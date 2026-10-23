/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Runs a driver executable on pipes and speaks the driver protocol to it. */
#ifndef SPAWN_H
#define SPAWN_H

#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#include "mr_common.h"
#include "mr_framing.h"
#include "mr_proto.h"

typedef struct {
    pid_t pid;
    int in, out;
    mr_decoder_t dec;
    MrD2R *ready;
} child_t;

static inline void child_start(child_t *c, char *const argv[])
{
    int in[2], out[2];
    ck_assert_int_eq(pipe(in), 0);
    ck_assert_int_eq(pipe(out), 0);
    c->pid = fork();
    if (c->pid == 0) {
        dup2(in[0], 0);
        dup2(out[1], 1);
        close(in[1]);
        close(out[0]);
        execv(argv[0], argv);
        _exit(127);
    }
    close(in[0]);
    close(out[1]);
    c->in = in[1];
    c->out = out[0];
    c->ready = NULL;
    mr_decoder_init(&c->dec);
}

static inline void child_send(child_t *c, const MrR2D *m)
{
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    ck_assert_int_gt(n, 0);
    ck_assert_int_eq(mr_frame_write_fd(c->in, buf, (size_t)n), MR_OK);
}

static inline int child_on_frame(void *ctx, const uint8_t *f, size_t n)
{
    child_t *c = ctx;
    c->ready = muudd__radio__driver_to_radiod__unpack(NULL, n, f);
    return 1;   /* stop after one frame; the decoder keeps the rest */
}

/* Reads one byte at a time so no second frame is left half-consumed. */
static inline MrD2R *child_next(child_t *c, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    while (mr_now_ms() < end) {
        struct pollfd p = {c->out, POLLIN, 0};
        if (poll(&p, 1, 10) <= 0)
            continue;
        uint8_t b;
        if (read(c->out, &b, 1) != 1)
            return NULL;
        int rc = mr_decoder_feed(&c->dec, &b, 1, child_on_frame, c);
        if (rc == 1) {
            MrD2R *m = c->ready;
            c->ready = NULL;
            return m;
        }
        ck_assert_int_eq(rc, MR_OK);
    }
    return NULL;
}

static inline void child_close_stdin(child_t *c)
{
    close(c->in);
    c->in = -1;
}

static inline int child_wait(child_t *c, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    int st;
    while (mr_now_ms() < end) {
        if (waitpid(c->pid, &st, WNOHANG) == c->pid)
            return WIFEXITED(st) ? WEXITSTATUS(st) : 128 + WTERMSIG(st);
        usleep(5000);
    }
    kill(c->pid, SIGKILL);
    waitpid(c->pid, &st, 0);
    return -1;
}

#endif
