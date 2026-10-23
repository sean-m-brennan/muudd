/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The consumer Unix socket: accepts consumers, decodes their frames for the
 * core, and queues the core's replies to them. */
#ifndef MR_IPC_H
#define MR_IPC_H

#include <poll.h>

#include "mr_core.h"
#include "mr_framing.h"

#define MR_IPC_QUEUE 256

typedef struct {
    uint8_t *buf;
    size_t len;
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
    mr_ipc_client_t clients[MR_MAX_CONSUMERS];
} mr_ipc_t;

int mr_ipc_open(mr_ipc_t *ipc, const char *path, mr_core_t *core);
int mr_ipc_poll_fds(mr_ipc_t *ipc, struct pollfd *fds, int max);
void mr_ipc_handle(mr_ipc_t *ipc, const struct pollfd *fds, int n, uint64_t now_ms);
void mr_ipc_send(mr_ipc_t *ipc, int cid, const MrR2C *msg);
void mr_ipc_close(mr_ipc_t *ipc);

#endif
