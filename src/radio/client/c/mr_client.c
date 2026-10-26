/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "mr_client.h"
#include "mr_framing.h"

#define MR_CLIENT_QUEUE 64

struct mr_client {
    char path[108];
    int fd;
    mr_decoder_t dec;
    MrR2C *q[MR_CLIENT_QUEUE];
    size_t head, count;
    uint64_t dropped;
    uint32_t next_req;
    struct { uint32_t port; int policy; } claims[MR_NUM_PORTS];
    size_t n_claims;
};

static int dial(const char *path)
{
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0)
        return -1;
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof sa.sun_path, "%s", path);
    if (connect(fd, (struct sockaddr *)&sa, sizeof sa) != 0) {
        int e = errno;
        close(fd);
        errno = e;
        return -1;
    }
    return fd;
}

int mr_client_connect(const char *path, mr_client_t **out)
{
    if (strlen(path) >= sizeof(((mr_client_t *)0)->path)) {
        errno = ENAMETOOLONG;
        return MR_EINVAL;
    }
    mr_client_t *c = calloc(1, sizeof *c);
    if (!c)
        return MR_ENOMEM;
    snprintf(c->path, sizeof c->path, "%s", path);
    c->fd = dial(path);
    if (c->fd < 0) {
        free(c);
        return MR_EIO;
    }
    mr_decoder_init(&c->dec);
    *out = c;
    return MR_OK;
}

static void drop_queue(mr_client_t *c)
{
    while (c->count) {
        muudd__radio__radiod_to_consumer__free_unpacked(c->q[c->head], NULL);
        c->head = (c->head + 1) % MR_CLIENT_QUEUE;
        c->count--;
    }
}

void mr_client_close(mr_client_t *c)
{
    if (!c)
        return;
    if (c->fd >= 0)
        close(c->fd);
    drop_queue(c);
    free(c);
}

static void fail_link(mr_client_t *c)
{
    if (c->fd >= 0)
        close(c->fd);
    c->fd = -1;
}

int mr_client_send(mr_client_t *c, const MrC2R *m)
{
    if (c->fd < 0)
        return MR_EIO;
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    if (n < 0)
        return MR_EINVAL;
    size_t off = 0;
    while (off < (size_t)n) {
        /* MSG_NOSIGNAL: a peer that went away must not kill the consumer. */
        ssize_t w = send(c->fd, buf + off, (size_t)n - off, MSG_NOSIGNAL);
        if (w < 0 && errno == EINTR)
            continue;
        if (w <= 0) {
            fail_link(c);
            return MR_EIO;
        }
        off += (size_t)w;
    }
    return MR_OK;
}

uint64_t mr_client_dropped(const mr_client_t *c)
{
    return c->dropped;
}

bool mr_client_connected(const mr_client_t *c)
{
    return c != NULL && c->fd >= 0;
}

static void push_back(mr_client_t *c, MrR2C *m)
{
    if (c->count == MR_CLIENT_QUEUE) {
        muudd__radio__radiod_to_consumer__free_unpacked(c->q[c->head], NULL);
        c->head = (c->head + 1) % MR_CLIENT_QUEUE;
        c->count--;
        c->dropped++;
    }
    c->q[(c->head + c->count) % MR_CLIENT_QUEUE] = m;
    c->count++;
}

/* Removes the i-th queued message (0 is the oldest), keeping the order of the rest. */
static MrR2C *take_at(mr_client_t *c, size_t i)
{
    MrR2C *m = c->q[(c->head + i) % MR_CLIENT_QUEUE];
    for (; i + 1 < c->count; i++)
        c->q[(c->head + i) % MR_CLIENT_QUEUE] = c->q[(c->head + i + 1) % MR_CLIENT_QUEUE];
    c->count--;
    return m;
}

static int on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    MrR2C *m = muudd__radio__radiod_to_consumer__unpack(NULL, n, buf);
    if (!m)
        return MR_EFRAME;
    push_back(ctx, m);
    return MR_OK;
}

/* Waits up to timeout_ms for the socket and reads whatever is there into the
 * queue. A zero timeout still polls once. Returns 1 if bytes were read, 0 on
 * timeout, -1 if the link is down. */
static int pump(mr_client_t *c, int timeout_ms)
{
    if (c->fd < 0)
        return -1;
    struct pollfd p = {c->fd, POLLIN, 0};
    int pr = poll(&p, 1, timeout_ms);
    if (pr < 0)
        return errno == EINTR ? 0 : (fail_link(c), -1);
    if (pr == 0)
        return 0;
    uint8_t buf[4096];
    ssize_t r = read(c->fd, buf, sizeof buf);
    if (r < 0 && (errno == EINTR || errno == EAGAIN))
        return 0;
    if (r <= 0 || mr_decoder_feed(&c->dec, buf, (size_t)r, on_frame, c) != MR_OK) {
        fail_link(c);
        return -1;
    }
    return 1;
}

static int remaining_ms(uint64_t end)
{
    uint64_t now = mr_now_ms();
    return now >= end ? 0 : (int)(end - now);
}

MrR2C *mr_client_recv(mr_client_t *c, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)(timeout_ms > 0 ? timeout_ms : 0);
    for (;;) {
        if (c->count)
            return take_at(c, 0);
        if (pump(c, remaining_ms(end)) < 0 && !c->count)
            return NULL;
        if (!c->count && mr_now_ms() >= end)
            return NULL;
    }
}

static uint32_t reply_req(const MrR2C *m)
{
    switch (m->msg_case) {
    case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT: return m->claim_result->req_id;
    case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RELEASE_RESULT: return m->release_result->req_id;
    case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT: return m->send_result->req_id;
    case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_STATS: return m->stats->req_id;
    default: return 0;
    }
}

/* Waits for one reply and removes only it from the queue; everything else
 * stays queued in arrival order. */
static MrR2C *wait_reply(mr_client_t *c, int msg_case, uint32_t req, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    bool last = false;
    for (;;) {
        for (size_t i = 0; i < c->count; i++) {
            MrR2C *m = c->q[(c->head + i) % MR_CLIENT_QUEUE];
            if ((int)m->msg_case == msg_case && reply_req(m) == req)
                return take_at(c, i);
        }
        /* The scan above also covers what the final read or a closing link delivered. */
        if (last)
            return NULL;
        last = pump(c, remaining_ms(end)) < 0 || mr_now_ms() >= end;
    }
}

static int claim_once(mr_client_t *c, uint32_t port, int policy, int timeout_ms)
{
    Muudd__Radio__Claim cl = MUUDD__RADIO__CLAIM__INIT;
    cl.req_id = ++c->next_req;
    cl.port = port;
    cl.policy = policy;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM;
    m.claim = &cl;
    if (mr_client_send(c, &m) != MR_OK)
        return MR_EIO;
    MrR2C *r = wait_reply(c, MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT, cl.req_id, timeout_ms);
    if (!r)
        return MR_EIO;
    int st = r->claim_result->status;
    muudd__radio__radiod_to_consumer__free_unpacked(r, NULL);
    return st;
}

int mr_client_claim(mr_client_t *c, uint32_t port, int policy, int timeout_ms)
{
    int st = claim_once(c, port, policy, timeout_ms);
    if (st == MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK) {
        bool known = false;
        for (size_t i = 0; i < c->n_claims; i++)
            if (c->claims[i].port == port) {
                c->claims[i].policy = policy;
                known = true;
            }
        if (!known && c->n_claims < MR_NUM_PORTS) {
            c->claims[c->n_claims].port = port;
            c->claims[c->n_claims].policy = policy;
            c->n_claims++;
        }
    }
    return st;
}

/* Every claim ever held stays in the list, whether or not it could be
 * re-established, so a later reconnect tries it again. */
int mr_client_reconnect(mr_client_t *c, int timeout_ms)
{
    fail_link(c);
    mr_decoder_init(&c->dec);
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    useconds_t delay = 100000;
    while ((c->fd = dial(c->path)) < 0) {
        if (mr_now_ms() >= end)
            return MR_EIO;
        usleep(delay);
        if (delay < 2000000)
            delay *= 2;
    }
    int rc = MR_OK;
    for (size_t i = 0; i < c->n_claims; i++)
        if (claim_once(c, c->claims[i].port, c->claims[i].policy, 2000) != MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK)
            rc = MR_ERR;
    return rc;
}

MrR2C *mr_client_get_stats(mr_client_t *c, int timeout_ms)
{
    Muudd__Radio__GetStats g = MUUDD__RADIO__GET_STATS__INIT;
    g.req_id = ++c->next_req;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_GET_STATS;
    m.get_stats = &g;
    if (mr_client_send(c, &m) != MR_OK)
        return NULL;
    return wait_reply(c, MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_STATS, g.req_id, timeout_ms);
}
