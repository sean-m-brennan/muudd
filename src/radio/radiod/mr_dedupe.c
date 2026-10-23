/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>

#include "mr_dedupe.h"

int mr_dedupe_init(mr_dedupe_t *d, size_t cap)
{
    d->ring = calloc(cap, sizeof *d->ring);
    if (!d->ring)
        return MR_ENOMEM;
    d->cap = cap;
    d->head = d->count = 0;
    d->evicted_early = 0;
    return MR_OK;
}

void mr_dedupe_free(mr_dedupe_t *d)
{
    free(d->ring);
    d->ring = NULL;
}

bool mr_dedupe_check_insert(mr_dedupe_t *d, mr_frame_id_t id, uint64_t ttl_ms, uint64_t now_ms)
{
    while (d->count && d->ring[d->head].expires_ms <= now_ms) {
        d->head = (d->head + 1) % d->cap;
        d->count--;
    }
    for (size_t i = 0; i < d->count; i++) {
        const mr_dedupe_entry_t *e = &d->ring[(d->head + i) % d->cap];
        if (e->expires_ms > now_ms && e->id.node == id.node && e->id.id == id.id)
            return true;
    }
    if (d->count == d->cap) {
        if (d->ring[d->head].expires_ms > now_ms)
            d->evicted_early++;
        d->head = (d->head + 1) % d->cap;
        d->count--;
    }
    mr_dedupe_entry_t *slot = &d->ring[(d->head + d->count) % d->cap];
    slot->id = id;
    slot->expires_ms = now_ms + ttl_ms;
    d->count++;
    return false;
}
