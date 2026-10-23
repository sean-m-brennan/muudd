/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <grp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#include "mr_ipc.h"

#define MR_IPC_SNDBUF 32768

int mr_ipc_open(mr_ipc_t *ipc, const char *path, mr_core_t *core)
{
    memset(ipc, 0, sizeof *ipc);
    ipc->listen_fd = -1;
    ipc->core = core;
    for (int i = 0; i < MR_MAX_CONSUMERS; i++)
        ipc->clients[i].fd = -1;
    if (strlen(path) >= sizeof ipc->path) {
        errno = ENAMETOOLONG;
        return MR_EINVAL;
    }
    snprintf(ipc->path, sizeof ipc->path, "%s", path);
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0)
        return MR_EIO;
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof sa.sun_path, "%s", path);
    unlink(path);
    if (bind(fd, (struct sockaddr *)&sa, sizeof sa) != 0 || listen(fd, 16) != 0) {
        close(fd);
        return MR_EIO;
    }
    chmod(path, 0660);
    struct group *g = getgrnam("muudd-radio");
    if (g && chown(path, (uid_t)-1, g->gr_gid) != 0)
        fprintf(stderr, "radiod: cannot give %s to group muudd-radio: %s\n", path, strerror(errno));
    ipc->listen_fd = fd;
    return MR_OK;
}

int mr_ipc_poll_fds(mr_ipc_t *ipc, struct pollfd *fds, int max)
{
    int n = 0;
    if (ipc->listen_fd >= 0 && n < max)
        fds[n++] = (struct pollfd){ipc->listen_fd, POLLIN, 0};
    for (int i = 0; i < MR_MAX_CONSUMERS && n < max; i++) {
        mr_ipc_client_t *c = &ipc->clients[i];
        if (c->fd >= 0)
            fds[n++] = (struct pollfd){c->fd, (short)(POLLIN | (c->count ? POLLOUT : 0)), 0};
    }
    return n;
}

static void flush(mr_ipc_client_t *c)
{
    while (c->count && !c->dead) {
        mr_ipc_frame_t *f = &c->q[c->head];
        ssize_t w = send(c->fd, f->buf + c->off, f->len - c->off, MSG_NOSIGNAL);
        if (w < 0) {
            if (errno == EINTR)
                continue;
            if (errno != EAGAIN)
                c->dead = true;
            return;
        }
        c->off += (size_t)w;
        if (c->off == f->len) {
            free(f->buf);
            c->head = (c->head + 1) % MR_IPC_QUEUE;
            c->count--;
            c->off = 0;
        }
    }
}

void mr_ipc_send(mr_ipc_t *ipc, int cid, const MrR2C *m)
{
    if (cid < 0 || cid >= MR_MAX_CONSUMERS)
        return;
    mr_ipc_client_t *c = &ipc->clients[cid];
    if (c->fd < 0 || c->dead)
        return;
    uint8_t tmp[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, tmp, sizeof tmp);
    if (n < 0)
        return;
    if (c->count == MR_IPC_QUEUE) {
        /* Drop the oldest frame that has not been partly written. */
        size_t victim = c->off ? (c->head + 1) % MR_IPC_QUEUE : c->head;
        free(c->q[victim].buf);
        if (victim != c->head)
            c->q[victim] = c->q[c->head];
        c->head = (c->head + 1) % MR_IPC_QUEUE;
        c->count--;
        mr_core_note_queue_overflow(ipc->core, 1);
    }
    uint8_t *buf = malloc((size_t)n);
    if (!buf)
        return;
    memcpy(buf, tmp, (size_t)n);
    c->q[(c->head + c->count) % MR_IPC_QUEUE] = (mr_ipc_frame_t){buf, (size_t)n};
    c->count++;
    flush(c);
}

static void disconnect(mr_ipc_t *ipc, int cid)
{
    mr_ipc_client_t *c = &ipc->clients[cid];
    if (c->fd < 0)
        return;
    close(c->fd);
    c->fd = -1;
    c->dead = false;
    while (c->count) {
        free(c->q[c->head].buf);
        c->head = (c->head + 1) % MR_IPC_QUEUE;
        c->count--;
    }
    c->off = 0;
    mr_core_consumer_disconnected(ipc->core, cid);
}

static void accept_all(mr_ipc_t *ipc, uint64_t now)
{
    for (;;) {
        int fd = accept4(ipc->listen_fd, NULL, NULL, SOCK_CLOEXEC | SOCK_NONBLOCK);
        if (fd < 0)
            return;
        int cid = -1;
        for (int i = 0; i < MR_MAX_CONSUMERS; i++)
            if (ipc->clients[i].fd < 0) {
                cid = i;
                break;
            }
        if (cid < 0) {
            close(fd);
            continue;
        }
        int sndbuf = MR_IPC_SNDBUF;
        setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &sndbuf, sizeof sndbuf);
        mr_ipc_client_t *c = &ipc->clients[cid];
        c->fd = fd;
        c->dead = false;
        c->head = c->count = c->off = 0;
        mr_decoder_init(&c->dec);
        mr_core_consumer_connected(ipc->core, cid, now);
    }
}

typedef struct {
    mr_ipc_t *ipc;
    int cid;
} frame_ctx_t;

static int on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    frame_ctx_t *fc = ctx;
    MrC2R *m = muudd__radio__consumer_to_radiod__unpack(NULL, n, buf);
    if (!m)
        return MR_EFRAME;
    mr_core_consumer_msg(fc->ipc->core, fc->cid, m, fc->ipc->now);
    muudd__radio__consumer_to_radiod__free_unpacked(m, NULL);
    return MR_OK;
}

static void read_client(mr_ipc_t *ipc, int cid)
{
    mr_ipc_client_t *c = &ipc->clients[cid];
    uint8_t buf[4096];
    while (c->fd >= 0 && !c->dead) {
        ssize_t r = read(c->fd, buf, sizeof buf);
        if (r == 0) {
            c->dead = true;
            return;
        }
        if (r < 0) {
            if (errno == EINTR)
                continue;
            if (errno != EAGAIN)
                c->dead = true;
            return;
        }
        frame_ctx_t fc = {ipc, cid};
        if (mr_decoder_feed(&c->dec, buf, (size_t)r, on_frame, &fc) != MR_OK) {
            c->dead = true;
            return;
        }
    }
}

void mr_ipc_handle(mr_ipc_t *ipc, const struct pollfd *fds, int n, uint64_t now_ms)
{
    ipc->now = now_ms;
    for (int i = 0; i < n; i++) {
        if (!fds[i].revents)
            continue;
        if (fds[i].fd == ipc->listen_fd) {
            accept_all(ipc, now_ms);
            continue;
        }
        for (int cid = 0; cid < MR_MAX_CONSUMERS; cid++) {
            mr_ipc_client_t *c = &ipc->clients[cid];
            if (c->fd != fds[i].fd)
                continue;
            if (fds[i].revents & POLLOUT)
                flush(c);
            if (fds[i].revents & (POLLIN | POLLHUP | POLLERR))
                read_client(ipc, cid);
            break;
        }
    }
    for (int cid = 0; cid < MR_MAX_CONSUMERS; cid++)
        if (ipc->clients[cid].fd >= 0 && ipc->clients[cid].dead)
            disconnect(ipc, cid);
}

void mr_ipc_close(mr_ipc_t *ipc)
{
    for (int cid = 0; cid < MR_MAX_CONSUMERS; cid++)
        disconnect(ipc, cid);
    if (ipc->listen_fd >= 0) {
        close(ipc->listen_fd);
        ipc->listen_fd = -1;
        unlink(ipc->path);
    }
}
