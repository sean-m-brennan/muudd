/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The link to muudd-radiod. One thread owns the radiod consumer client; every
 * public call here only touches mutex-protected queues and state, so the
 * network process never blocks on the radio. */
#ifndef LORA_LINK_H
#define LORA_LINK_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lora/lora_frame.h"

#define LORA_LINK_RX_MAX   64   /* queued received frames; the oldest is dropped and counted */
#define LORA_LINK_JOBS_MAX 16   /* outstanding messages, the one being sent included */
#define LORA_BROADCAST     0xFFFFFFFFu

/* up = connected, claimed, radio up, a local driver present (caps set and
 * node_addr non-zero) and an MTU of at least LORA_MIN_MTU. */
typedef struct { bool connected, claimed, radio_up, driver; uint32_t node_addr, mtu; bool up; } lora_link_state_t;
/* A frame received on the claimed port; payload is malloc'd, the caller frees it. */
typedef struct { uint32_t src; uint8_t *payload; size_t len; int via; } lora_link_rx_t;
/* The outcome of one submitted message: ok, or the reason's name (a radiod
 * REFUSED reason such as OVER_BUDGET, TIMEOUT, or RADIO_DOWN). */
typedef struct { bool ok; size_t sent, total; char reason[16]; } lora_link_result_t;
typedef void (*lora_log_fn)(void *arg, int level, const char *msg);   /* 0 info, 1 warn, 2 error */
typedef struct lora_link lora_link_t;

/* Starts the link thread: connects (retry backoff 0.1 s doubling to 2 s) and
 * claims the port CONSUMER_BRIDGED. NULL on failure. */
lora_link_t *lora_link_start(const char *socket_path, uint32_t port, lora_log_fn log, void *log_arg);
/* Stops the thread (within about 50 ms, or about half a second when radiod
 * has stopped reading and a send is waiting for room), fails anything
 * outstanding with RADIO_DOWN, and frees the link. */
void lora_link_stop(lora_link_t *l);
void lora_link_state(lora_link_t *l, lora_link_state_t *out);
/* Takes ownership of the fragments on success (0). -1, fragments untouched,
 * when the link is not up or LORA_LINK_JOBS_MAX messages are outstanding. */
int lora_link_submit(lora_link_t *l, uint32_t dest, lora_frags_t *frags, bool want_ack);
/* Moves up to max received frames out of the queue; returns how many. */
size_t lora_link_poll_rx(lora_link_t *l, lora_link_rx_t *out, size_t max);
/* Moves up to max finished-message results out, oldest first. The queue holds
 * LORA_LINK_JOBS_MAX * 2; a result finished while it is full is dropped. */
size_t lora_link_poll_results(lora_link_t *l, lora_link_result_t *out, size_t max);
/* Received frames dropped because the queue was full. */
uint64_t lora_link_rx_dropped(lora_link_t *l);
/* Results dropped because the result queue was full. */
uint64_t lora_link_results_dropped(lora_link_t *l);

typedef struct {
    void *ctx;
    void (*state)(void *ctx, lora_link_state_t *out);
    int (*submit)(void *ctx, uint32_t dest, lora_frags_t *frags, bool want_ack);
    size_t (*poll_rx)(void *ctx, lora_link_rx_t *out, size_t max);
    size_t (*poll_results)(void *ctx, lora_link_result_t *out, size_t max);
    uint64_t (*results_dropped)(void *ctx);    /* may be NULL: nothing is dropped */
} lora_link_ops_t;
/* The same operations as a table, so the engine can run against a fake. */
lora_link_ops_t lora_link_ops(lora_link_t *l);

#endif
