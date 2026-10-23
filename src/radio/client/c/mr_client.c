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

int mr_client_send(mr_client_t *c, const MrC2R *m)
{
    if (c->fd < 0)
        return MR_EIO;
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    if (n < 0)
        return MR_EINVAL;
    return mr_frame_write_fd(c->fd, buf, (size_t)n);
}

static void push_back(mr_client_t *c, MrR2C *m)
{
    if (c->count == MR_CLIENT_QUEUE) {
        muudd__radio__radiod_to_consumer__free_unpacked(c->q[c->head], NULL);
        c->head = (c->head + 1) % MR_CLIENT_QUEUE;
        c->count--;
    }
    c->q[(c->head + c->count) % MR_CLIENT_QUEUE] = m;
    c->count++;
}

static void push_front(mr_client_t *c, MrR2C *m)
{
    if (c->count == MR_CLIENT_QUEUE) {
        muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
        return;
    }
    c->head = (c->head + MR_CLIENT_QUEUE - 1) % MR_CLIENT_QUEUE;
    c->q[c->head] = m;
    c->count++;
}

static int on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    MrR2C *m = muudd__radio__radiod_to_consumer__unpack(NULL, n, buf);
    if (!m)
        return MR_EFRAME;
    push_back(ctx, m);
    return MR_OK;
}

MrR2C *mr_client_recv(mr_client_t *c, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)(timeout_ms > 0 ? timeout_ms : 0);
    for (;;) {
        if (c->count) {
            MrR2C *m = c->q[c->head];
            c->head = (c->head + 1) % MR_CLIENT_QUEUE;
            c->count--;
            return m;
        }
        uint64_t now = mr_now_ms();
        if (c->fd < 0 || now >= end)
            return NULL;
        struct pollfd p = {c->fd, POLLIN, 0};
        if (poll(&p, 1, (int)(end - now)) <= 0)
            continue;
        uint8_t buf[4096];
        ssize_t r = read(c->fd, buf, sizeof buf);
        if (r <= 0 || mr_decoder_feed(&c->dec, buf, (size_t)r, on_frame, c) != MR_OK) {
            close(c->fd);
            c->fd = -1;
        }
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

/* Waits for one reply; everything else that arrives meanwhile goes back on
 * the queue, ahead of anything newer. */
static MrR2C *wait_reply(mr_client_t *c, int msg_case, uint32_t req, int timeout_ms)
{
    MrR2C *side[MR_CLIENT_QUEUE];
    size_t ns = 0;
    MrR2C *found = NULL;
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    while (!found) {
        uint64_t now = mr_now_ms();
        if (now >= end)
            break;
        MrR2C *m = mr_client_recv(c, (int)(end - now));
        if (!m)
            break;
        if ((int)m->msg_case == msg_case && reply_req(m) == req)
            found = m;
        else if (ns < MR_CLIENT_QUEUE)
            side[ns++] = m;
        else
            muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
    }
    for (size_t i = ns; i-- > 0;)
        push_front(c, side[i]);
    return found;
}

int mr_client_claim(mr_client_t *c, uint32_t port, int policy, int timeout_ms)
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
    if (st == MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK) {
        bool known = false;
        for (size_t i = 0; i < c->n_claims; i++)
            if (c->claims[i].port == port)
                known = true;
        if (!known && c->n_claims < MR_NUM_PORTS) {
            c->claims[c->n_claims].port = port;
            c->claims[c->n_claims].policy = policy;
            c->n_claims++;
        }
    }
    return st;
}

int mr_client_reconnect(mr_client_t *c, int timeout_ms)
{
    if (c->fd >= 0)
        close(c->fd);
    c->fd = -1;
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
    size_t n = c->n_claims;
    c->n_claims = 0;
    for (size_t i = 0; i < n; i++)
        mr_client_claim(c, c->claims[i].port, c->claims[i].policy, 2000);
    return MR_OK;
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
