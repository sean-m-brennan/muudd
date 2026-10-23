/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Runs the radio driver as a child process on pipes: starts it, checks its
 * Hello and heartbeats, kills it when it misbehaves, and restarts it with
 * backoff unless it reported a fatal error. */
#ifndef MR_SUPERVISOR_H
#define MR_SUPERVISOR_H

#include <poll.h>
#include <sys/types.h>

#include "mr_common.h"
#include "mr_proto.h"

typedef enum { MR_SUP_IDLE, MR_SUP_STARTING, MR_SUP_UP, MR_SUP_KILLING, MR_SUP_BACKOFF, MR_SUP_FATAL } mr_sup_state_t;

typedef struct {
    char *const *argv;
    uint32_t hello_timeout_ms, status_period_ms, missed_status, kill_grace_ms;
    uint32_t backoff_min_ms, backoff_max_ms, backoff_reset_ms;
} mr_sup_cfg_t;

typedef struct {
    void *ctx;
    int (*on_hello)(void *ctx, const MrCaps *caps);
    void (*on_msg)(void *ctx, const MrD2R *msg);
    void (*on_down)(void *ctx, const char *reason);
    void (*on_stderr)(void *ctx, const char *line);
} mr_sup_io_t;

typedef struct mr_sup mr_sup_t;

/* The caller must ignore SIGPIPE: mr_sup_send writes to a pipe whose reader
 * may already have exited. */

void mr_sup_default_cfg(mr_sup_cfg_t *cfg, char *const *argv);
int mr_sup_create(const mr_sup_cfg_t *cfg, const mr_sup_io_t *io, mr_sup_t **out);
void mr_sup_free(mr_sup_t *s);
int mr_sup_poll_fds(mr_sup_t *s, struct pollfd *fds, int max);
void mr_sup_handle(mr_sup_t *s, const struct pollfd *fds, int n, uint64_t now_ms);
void mr_sup_tick(mr_sup_t *s, uint64_t now_ms);
int mr_sup_send(mr_sup_t *s, const MrR2D *msg);
void mr_sup_reset_fatal(mr_sup_t *s, uint64_t now_ms);
mr_sup_state_t mr_sup_state(const mr_sup_t *s);
pid_t mr_sup_pid(const mr_sup_t *s);

#endif
