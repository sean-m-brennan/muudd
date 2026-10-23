/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#include "mr_framing.h"
#include "mr_supervisor.h"

#define SUP_FATAL_RC 1
#define EX_CONFIG_STATUS 78

struct mr_sup {
    mr_sup_cfg_t cfg;
    mr_sup_io_t io;
    mr_sup_state_t state;
    pid_t pid;
    int in_fd, out_fd, err_fd;
    mr_decoder_t dec;
    char errbuf[1024];
    size_t errlen;
    uint64_t now, started_ms, last_status_ms, kill_deadline_ms, next_start_ms, backoff_ms;
    bool got_hello, fatal_pending;
    char down_reason[160];
};

void mr_sup_default_cfg(mr_sup_cfg_t *cfg, char *const *argv)
{
    cfg->argv = argv;
    cfg->hello_timeout_ms = 10000;
    cfg->status_period_ms = 5000;
    cfg->missed_status = 3;
    cfg->kill_grace_ms = 2000;
    cfg->backoff_min_ms = 1000;
    cfg->backoff_max_ms = 60000;
    cfg->backoff_reset_ms = 600000;
}

int mr_sup_create(const mr_sup_cfg_t *cfg, const mr_sup_io_t *io, mr_sup_t **out)
{
    mr_sup_t *s = calloc(1, sizeof *s);
    if (!s)
        return MR_ENOMEM;
    s->cfg = *cfg;
    s->io = *io;
    s->state = MR_SUP_IDLE;
    s->in_fd = s->out_fd = s->err_fd = -1;
    s->backoff_ms = cfg->backoff_min_ms;
    *out = s;
    return MR_OK;
}

static void close_fd(int *fd)
{
    if (*fd >= 0) {
        close(*fd);
        *fd = -1;
    }
}

static void set_nonblock(int fd)
{
    fcntl(fd, F_SETFL, fcntl(fd, F_GETFL) | O_NONBLOCK);
}

static void schedule_restart(mr_sup_t *s, uint64_t now)
{
    s->next_start_ms = now + s->backoff_ms;
    s->backoff_ms = s->backoff_ms * 2 > s->cfg.backoff_max_ms ? s->cfg.backoff_max_ms : s->backoff_ms * 2;
    s->state = MR_SUP_BACKOFF;
}

static void start_child(mr_sup_t *s, uint64_t now)
{
    int in[2], out[2], err[2];
    if (pipe2(in, O_CLOEXEC) != 0)
        goto fail;
    if (pipe2(out, O_CLOEXEC) != 0) {
        close(in[0]);
        close(in[1]);
        goto fail;
    }
    if (pipe2(err, O_CLOEXEC) != 0) {
        close(in[0]);
        close(in[1]);
        close(out[0]);
        close(out[1]);
        goto fail;
    }
    pid_t pid = fork();
    if (pid == 0) {
        dup2(in[0], 0);
        dup2(out[1], 1);
        dup2(err[1], 2);
        signal(SIGPIPE, SIG_DFL);
        execv(s->cfg.argv[0], s->cfg.argv);
        dprintf(2, "cannot run %s: %s\n", s->cfg.argv[0], strerror(errno));
        _exit(127);
    }
    close(in[0]);
    close(out[1]);
    close(err[1]);
    if (pid < 0) {
        close(in[1]);
        close(out[0]);
        close(err[0]);
        goto fail;
    }
    s->pid = pid;
    s->in_fd = in[1];
    s->out_fd = out[0];
    s->err_fd = err[0];
    set_nonblock(s->in_fd);
    set_nonblock(s->out_fd);
    set_nonblock(s->err_fd);
    s->state = MR_SUP_STARTING;
    s->started_ms = s->last_status_ms = now;
    s->got_hello = s->fatal_pending = false;
    s->down_reason[0] = 0;
    s->errlen = 0;
    mr_decoder_init(&s->dec);
    return;
fail:
    if (s->io.on_stderr)
        s->io.on_stderr(s->io.ctx, "cannot start driver: out of processes or file descriptors");
    schedule_restart(s, now);
}

static void begin_kill(mr_sup_t *s, const char *reason, bool fatal, uint64_t now)
{
    if (s->pid <= 0 || s->state == MR_SUP_KILLING)
        return;
    snprintf(s->down_reason, sizeof s->down_reason, "%s", reason);
    s->fatal_pending = fatal;
    kill(s->pid, SIGTERM);
    s->state = MR_SUP_KILLING;
    s->kill_deadline_ms = now + s->cfg.kill_grace_ms;
}

static int on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    mr_sup_t *s = ctx;
    MrD2R *m = muudd__radio__driver_to_radiod__unpack(NULL, n, buf);
    if (!m)
        return MR_EFRAME;
    int rc = MR_OK;
    bool is_hello = m->msg_case == MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_HELLO;
    if (!s->got_hello) {
        if (!is_hello) {
            rc = MR_EFRAME;
        } else {
            s->got_hello = true;
            s->state = MR_SUP_UP;
            s->last_status_ms = s->now;
            if (s->io.on_hello(s->io.ctx, m->hello) == MR_EFATAL)
                rc = SUP_FATAL_RC;
        }
    } else if (is_hello) {
        rc = MR_EFRAME;
    } else {
        if (m->msg_case == MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_STATUS)
            s->last_status_ms = s->now;
        s->io.on_msg(s->io.ctx, m);
    }
    muudd__radio__driver_to_radiod__free_unpacked(m, NULL);
    return rc;
}

static void read_stdout(mr_sup_t *s, uint64_t now)
{
    uint8_t buf[4096];
    for (;;) {
        ssize_t r = read(s->out_fd, buf, sizeof buf);
        if (r == 0) {
            close_fd(&s->out_fd);
            return;
        }
        if (r < 0) {
            if (errno == EINTR)
                continue;
            if (errno != EAGAIN)
                close_fd(&s->out_fd);
            return;
        }
        if (s->state == MR_SUP_KILLING)
            continue;
        int rc = mr_decoder_feed(&s->dec, buf, (size_t)r, on_frame, s);
        if (rc == SUP_FATAL_RC) {
            begin_kill(s, "driver reported a fatal error", true, now);
            return;
        }
        if (rc != MR_OK) {
            begin_kill(s, "malformed frame from driver", false, now);
            return;
        }
    }
}

static void read_stderr(mr_sup_t *s)
{
    char buf[1024];
    for (;;) {
        ssize_t r = read(s->err_fd, buf, sizeof buf);
        if (r == 0) {
            close_fd(&s->err_fd);
            return;
        }
        if (r < 0) {
            if (errno == EINTR)
                continue;
            if (errno != EAGAIN)
                close_fd(&s->err_fd);
            return;
        }
        for (ssize_t i = 0; i < r; i++) {
            bool newline = buf[i] == '\n';
            if (newline || s->errlen == sizeof s->errbuf - 1) {
                s->errbuf[s->errlen] = 0;
                if (s->io.on_stderr)
                    s->io.on_stderr(s->io.ctx, s->errbuf);
                s->errlen = 0;
                if (newline)
                    continue;
            }
            s->errbuf[s->errlen++] = buf[i];
        }
    }
}

int mr_sup_poll_fds(mr_sup_t *s, struct pollfd *fds, int max)
{
    int n = 0;
    if (s->out_fd >= 0 && n < max)
        fds[n++] = (struct pollfd){s->out_fd, POLLIN, 0};
    if (s->err_fd >= 0 && n < max)
        fds[n++] = (struct pollfd){s->err_fd, POLLIN, 0};
    return n;
}

void mr_sup_handle(mr_sup_t *s, const struct pollfd *fds, int n, uint64_t now_ms)
{
    s->now = now_ms;
    for (int i = 0; i < n; i++) {
        if (!fds[i].revents)
            continue;
        if (s->out_fd >= 0 && fds[i].fd == s->out_fd)
            read_stdout(s, now_ms);
        else if (s->err_fd >= 0 && fds[i].fd == s->err_fd)
            read_stderr(s);
    }
}

static void child_exited(mr_sup_t *s, int status, uint64_t now)
{
    s->pid = 0;
    close_fd(&s->in_fd);
    close_fd(&s->out_fd);
    close_fd(&s->err_fd);
    bool fatal = s->fatal_pending;
    char reason[192];
    if (s->down_reason[0]) {
        snprintf(reason, sizeof reason, "%s", s->down_reason);
    } else if (WIFEXITED(status) && WEXITSTATUS(status) == EX_CONFIG_STATUS) {
        fatal = true;
        snprintf(reason, sizeof reason, "driver exited with status 78 (configuration error)");
    } else if (WIFEXITED(status)) {
        snprintf(reason, sizeof reason, "driver exited with status %d", WEXITSTATUS(status));
    } else {
        snprintf(reason, sizeof reason, "driver killed by signal %d", WTERMSIG(status));
    }
    s->down_reason[0] = 0;
    s->fatal_pending = false;
    if (s->io.on_down)
        s->io.on_down(s->io.ctx, reason);
    if (fatal) {
        s->state = MR_SUP_FATAL;
        return;
    }
    if (s->got_hello && now - s->started_ms >= s->cfg.backoff_reset_ms)
        s->backoff_ms = s->cfg.backoff_min_ms;
    schedule_restart(s, now);
}

void mr_sup_tick(mr_sup_t *s, uint64_t now)
{
    s->now = now;
    if (s->pid > 0) {
        int status;
        if (waitpid(s->pid, &status, WNOHANG) == s->pid)
            child_exited(s, status, now);
    }
    switch (s->state) {
    case MR_SUP_IDLE:
        start_child(s, now);
        break;
    case MR_SUP_BACKOFF:
        if (now >= s->next_start_ms)
            start_child(s, now);
        break;
    case MR_SUP_STARTING:
        if (now - s->started_ms >= s->cfg.hello_timeout_ms) {
            char reason[64];
            snprintf(reason, sizeof reason, "no Hello within %u ms", s->cfg.hello_timeout_ms);
            begin_kill(s, reason, false, now);
        }
        break;
    case MR_SUP_UP:
        if (now - s->last_status_ms >= (uint64_t)s->cfg.status_period_ms * s->cfg.missed_status)
            begin_kill(s, "missed heartbeats", false, now);
        break;
    case MR_SUP_KILLING:
        if (s->pid > 0 && now >= s->kill_deadline_ms) {
            kill(s->pid, SIGKILL);
            s->kill_deadline_ms = UINT64_MAX;
        }
        break;
    case MR_SUP_FATAL:
        break;
    }
}

int mr_sup_send(mr_sup_t *s, const MrR2D *m)
{
    if (s->in_fd < 0 || s->state != MR_SUP_UP)
        return MR_EIO;
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    if (n < 0)
        return MR_EINVAL;
    ssize_t w = write(s->in_fd, buf, (size_t)n);
    if (w != n) {
        begin_kill(s, "driver is not reading its input", false, s->now);
        return MR_EIO;
    }
    return MR_OK;
}

void mr_sup_reset_fatal(mr_sup_t *s, uint64_t now_ms)
{
    if (s->state != MR_SUP_FATAL)
        return;
    s->backoff_ms = s->cfg.backoff_min_ms;
    s->next_start_ms = now_ms;
    s->state = MR_SUP_BACKOFF;
}

mr_sup_state_t mr_sup_state(const mr_sup_t *s) { return s->state; }
pid_t mr_sup_pid(const mr_sup_t *s) { return s->pid; }

void mr_sup_free(mr_sup_t *s)
{
    if (!s)
        return;
    if (s->pid > 0) {
        kill(s->pid, SIGKILL);
        waitpid(s->pid, NULL, 0);
    }
    close_fd(&s->in_fd);
    close_fd(&s->out_fd);
    close_fd(&s->err_fd);
    free(s);
}
