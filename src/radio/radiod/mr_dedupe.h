/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Frame IDs seen recently, so a frame heard twice is delivered once. */
#ifndef MR_DEDUPE_H
#define MR_DEDUPE_H

#include "mr_common.h"

typedef struct { uint32_t node, id; } mr_frame_id_t;
typedef struct { mr_frame_id_t id; uint64_t expires_ms; } mr_dedupe_entry_t;
typedef struct {
    mr_dedupe_entry_t *ring;
    size_t cap, head, count;
    uint64_t evicted_early;
} mr_dedupe_t;

int mr_dedupe_init(mr_dedupe_t *d, size_t cap);
void mr_dedupe_free(mr_dedupe_t *d);
bool mr_dedupe_check_insert(mr_dedupe_t *d, mr_frame_id_t id, uint64_t ttl_ms, uint64_t now_ms);

#endif
