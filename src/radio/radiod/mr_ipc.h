/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The consumer Unix socket: accepts consumers, decodes their frames for the
 * core, and queues the core's replies to them. When a consumer's queue is
 * full, the oldest queued Received frame is dropped, so replies survive a
 * slow reader; only a queue of nothing but replies loses its oldest reply. */
#ifndef MR_IPC_H
#define MR_IPC_H

#include <poll.h>

#include "mr_core.h"
#include "mr_framing.h"

#define MR_IPC_QUEUE 256

typedef struct {
    uint8_t *buf;
    size_t len;
    bool received;   /* a Received frame, the first kind dropped on overflow */
} mr_ipc_frame_t;

typedef struct {
    int fd;
    bool dead;
    mr_decoder_t dec;
    mr_ipc_frame_t q[MR_IPC_QUEUE];
    size_t head, count, off;
} mr_ipc_client_t;

typedef struct {
    int listen_fd;
    char path[108];
    mr_core_t *core;
    uint64_t now;
    uint64_t pack_failures, pack_unreported, pack_log_ms;   /* messages too big to frame */
    bool pack_logged;
    mr_ipc_client_t clients[MR_MAX_CONSUMERS];
} mr_ipc_t;

/* Fails with errno EADDRINUSE if another process is listening on path; a
 * socket file nobody listens on is replaced. */
int mr_ipc_open(mr_ipc_t *ipc, const char *path, mr_core_t *core);
int mr_ipc_poll_fds(mr_ipc_t *ipc, struct pollfd *fds, int max);
void mr_ipc_handle(mr_ipc_t *ipc, const struct pollfd *fds, int n, uint64_t now_ms);
void mr_ipc_send(mr_ipc_t *ipc, int cid, const MrR2C *msg);
void mr_ipc_close(mr_ipc_t *ipc);

#endif
