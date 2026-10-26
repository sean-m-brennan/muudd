/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Reassembly of fragmented messages with dedupe and NACK timers, and the
 * sender's hold that answers one NACK. */
#ifndef LORA_REASM_H
#define LORA_REASM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lora/lora_frame.h"

#define LORA_REASM_MAX     16      /* messages in progress overall */
#define LORA_REASM_PER_SRC  4      /* messages in progress per source */
#define LORA_DEDUPE_MAX   256      /* dedupe entries kept; the oldest goes first */
#define LORA_DEDUPE_S   600.0      /* seconds a finished or dropped key is remembered */
#define LORA_HOLD_MAX      16      /* messages the send hold keeps; the oldest goes first */

typedef enum { LORA_RX_PARTIAL = 0, LORA_RX_COMPLETE, LORA_RX_DUPLICATE, LORA_RX_BAD } lora_rx_status_t;
typedef struct { uint64_t timeouts, evicted, duplicate, bad; } lora_reasm_stats_t;
/* A NACK to send: the bitmap of fragment indexes still missing from (src, msg_id). */
typedef struct { uint32_t src; uint16_t msg_id; uint16_t missing; } lora_nack_t;
typedef struct lora_reasm lora_reasm_t;

/** A reassembler that NACKs a quiet unicast after @p nack_gap_s and drops any message
 *  @p reassembly_s after its first fragment. */
lora_reasm_t *lora_reasm_new(double nack_gap_s, double reassembly_s);
void lora_reasm_free(lora_reasm_t *r);
/** Take one data frame from @p src at @p now. DUPLICATE when the key is in dedupe; BAD when
 *  type, count or group tag disagree with the entry (the entry is dropped); COMPLETE with a
 *  malloc'd message in @p msg (caller frees) once every fragment is held; else PARTIAL. */
lora_rx_status_t lora_reasm_add(lora_reasm_t *r, uint32_t src, const lora_frame_t *f,
                                double now, uint8_t **msg, size_t *len);
/** Drop timed-out entries and emit one NACK per quiet DATA_PEER entry never NACKed, in
 *  creation order, up to @p max. @return the number of NACKs written. */
size_t lora_reasm_tick(lora_reasm_t *r, double now, lora_nack_t *out, size_t max);
lora_reasm_stats_t lora_reasm_stats(const lora_reasm_t *r);

typedef struct lora_hold lora_hold_t;
/** A hold that keeps each message @p hold_s seconds. */
lora_hold_t *lora_hold_new(double hold_s);
void lora_hold_free(lora_hold_t *h);
/** Hold @p frags for (dest, msg_id); the hold takes ownership (@p frags->n becomes 0). */
void lora_hold_put(lora_hold_t *h, uint32_t dest, uint16_t msg_id, lora_frags_t *frags, double now);
/** Answer one NACK: copies of the held fragments whose bit is set in @p missing, in index
 *  order (free with lora_frags_free); 0 for an unknown, already served or expired message. */
size_t lora_hold_serve(lora_hold_t *h, uint32_t dest, uint16_t msg_id, uint16_t missing,
                       double now, lora_frags_t *out);
/** How many held messages have been served. */
uint64_t lora_hold_served(const lora_hold_t *h);

#endif
