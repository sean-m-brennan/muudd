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

/* Reads the 8-byte big-endian counter at path (a missing file counts as 0),
 * increments it, stores it atomically (temporary file, fsync, rename, mode
 * 0600) and returns the new value in *out. On failure returns a negative
 * MR_* code, sets errno and, when reason is not NULL, writes a message. */
int mr_bootcount_next(const char *path, uint64_t *out, char *reason, size_t cap);

#endif
