/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* A persistent start counter: boot_id values must rise across restarts even
 * if the wall clock steps backwards, so the bridge counts its own starts. */
#ifndef MR_BOOTCOUNT_H
#define MR_BOOTCOUNT_H

#include <stddef.h>
#include <stdint.h>

/* Wall-clock times before 2025-01-01 (Unix ms) are taken as an unset clock. */
#define MR_BOOTCOUNT_PLAUSIBLE_MS 1735689600000ull

/* Reads the 8-byte big-endian counter at path (a missing file counts as 0),
 * advances it, stores it atomically (temporary file, fsync, rename, mode
 * 0600) and returns the new value in *out. The new value is the stored one
 * plus 1, or wall_ms if that is larger and the clock looks set, so the
 * counter never falls behind a start that had to use the wall clock as its
 * boot_id. On failure returns a negative MR_* code, sets errno and, when
 * reason is not NULL, writes a message. */
int mr_bootcount_next(const char *path, uint64_t wall_ms, uint64_t *out, char *reason, size_t cap);

#endif
