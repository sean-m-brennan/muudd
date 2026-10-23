/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "mr_bootcount.h"
#include "mr_common.h"

static int fail(char *reason, size_t cap, int rc, const char *what)
{
    int saved = errno;
    if (reason && cap)
        snprintf(reason, cap, "%s", what[0] ? what : strerror(saved));
    errno = saved;
    return rc;
}

static int read_counter(const char *path, uint64_t *v, char *reason, size_t cap)
{
    *v = 0;
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        if (errno == ENOENT)
            return MR_OK;
        return fail(reason, cap, MR_EIO, "");
    }
    uint8_t b[9];
    ssize_t n;
    do
        n = read(fd, b, sizeof b);
    while (n < 0 && errno == EINTR);
    int saved = errno;
    close(fd);
    if (n < 0) {
        errno = saved;
        return fail(reason, cap, MR_EIO, "");
    }
    if (n != 8) {
        errno = EINVAL;
        return fail(reason, cap, MR_EFRAME, "counter file is not exactly 8 bytes");
    }
    for (int i = 0; i < 8; i++)
        *v = (*v << 8) | b[i];
    return MR_OK;
}

static int write_counter(const char *path, uint64_t v, char *reason, size_t cap)
{
    char tmp[600];
    if (snprintf(tmp, sizeof tmp, "%s.tmp", path) >= (int)sizeof tmp) {
        errno = ENAMETOOLONG;
        return fail(reason, cap, MR_EINVAL, "");
    }
    int fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (fd < 0)
        return fail(reason, cap, MR_EIO, "");
    uint8_t b[8];
    for (int i = 0; i < 8; i++)
        b[i] = (uint8_t)(v >> (56 - 8 * i));
    ssize_t w;
    do
        w = write(fd, b, sizeof b);
    while (w < 0 && errno == EINTR);
    if (w != (ssize_t)sizeof b || fsync(fd) != 0) {
        if (w >= 0 && w != (ssize_t)sizeof b)
            errno = EIO;
        int saved = errno;
        close(fd);
        unlink(tmp);
        errno = saved;
        return fail(reason, cap, MR_EIO, "");
    }
    if (close(fd) != 0 || rename(tmp, path) != 0) {
        int saved = errno;
        unlink(tmp);
        errno = saved;
        return fail(reason, cap, MR_EIO, "");
    }
    return MR_OK;
}

int mr_bootcount_next(const char *path, uint64_t wall_ms, uint64_t *out, char *reason, size_t cap)
{
    uint64_t v;
    int rc = read_counter(path, &v, reason, cap);
    if (rc != MR_OK)
        return rc;
    if (v == UINT64_MAX) {
        errno = ERANGE;
        return fail(reason, cap, MR_EINVAL, "counter overflow");
    }
    v++;
    if (wall_ms >= MR_BOOTCOUNT_PLAUSIBLE_MS && wall_ms > v)
        v = wall_ms;
    rc = write_counter(path, v, reason, cap);
    if (rc != MR_OK)
        return rc;
    *out = v;
    return MR_OK;
}
