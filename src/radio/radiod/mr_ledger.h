/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Airtime budget: an overall duty-cycle cap and per-port shares over a
 * sliding hour, plus a token bucket that limits bursts. Refusals are final;
 * nothing is queued. */
#ifndef MR_LEDGER_H
#define MR_LEDGER_H

#include "mr_common.h"

#define MR_LEDGER_BINS 60

typedef struct {
    uint64_t minute[MR_LEDGER_BINS];
    uint64_t us[MR_LEDGER_BINS];
} mr_window_t;

typedef struct mr_ledger {
    uint64_t cap_us;
    uint64_t port_cap_us[MR_NUM_PORTS];
    bool port_set[MR_NUM_PORTS];
    mr_window_t total;
    mr_window_t port[MR_NUM_PORTS];
    mr_window_t relay[MR_NUM_PORTS];
    double bucket_us, bucket_cap_us, refill_us_per_ms;
    uint64_t bucket_ms;
    bool bucket_started;
} mr_ledger_t;

void mr_ledger_init(mr_ledger_t *l, double duty_cycle_pct);
void mr_ledger_set_port(mr_ledger_t *l, unsigned port, double share_pct);
void mr_ledger_set_max_frame(mr_ledger_t *l, uint64_t frame_us);
int mr_ledger_charge(mr_ledger_t *l, unsigned port, uint64_t airtime_us, bool relay, uint64_t now_ms);
uint64_t mr_ledger_used_us(const mr_ledger_t *l, int port, uint64_t now_ms);
uint64_t mr_ledger_relay_used_us(const mr_ledger_t *l, unsigned port, uint64_t now_ms);
uint64_t mr_ledger_cap_us(const mr_ledger_t *l, int port);

#endif
