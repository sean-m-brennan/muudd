/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <string.h>

#include "mr_ledger.h"

#define HOUR_US 3600000000.0

static void win_add(mr_window_t *w, uint64_t now_ms, uint64_t us)
{
    uint64_t m = now_ms / 60000u;
    unsigned b = (unsigned)(m % MR_LEDGER_BINS);
    if (w->minute[b] != m) {
        w->minute[b] = m;
        w->us[b] = 0;
    }
    w->us[b] += us;
}

static uint64_t win_sum(const mr_window_t *w, uint64_t now_ms)
{
    uint64_t m = now_ms / 60000u, sum = 0;
    for (unsigned b = 0; b < MR_LEDGER_BINS; b++)
        if (w->us[b] && w->minute[b] + MR_LEDGER_BINS > m)
            sum += w->us[b];
    return sum;
}

void mr_ledger_init(mr_ledger_t *l, double duty_cycle_pct)
{
    memset(l, 0, sizeof *l);
    l->cap_us = (uint64_t)(duty_cycle_pct / 100.0 * HOUR_US);
    l->bucket_cap_us = (double)l->cap_us * 10.0 / 3600.0;
    l->bucket_us = l->bucket_cap_us;
    l->refill_us_per_ms = (double)l->cap_us / 3600000.0;
}

void mr_ledger_set_port(mr_ledger_t *l, unsigned port, double share_pct)
{
    if (port < MR_NUM_PORTS) {
        l->port_set[port] = true;
        l->port_cap_us[port] = (uint64_t)(share_pct / 100.0 * (double)l->cap_us);
    }
}

void mr_ledger_set_max_frame(mr_ledger_t *l, uint64_t frame_us)
{
    if ((double)frame_us > l->bucket_cap_us) {
        l->bucket_us += (double)frame_us - l->bucket_cap_us;
        l->bucket_cap_us = (double)frame_us;
    }
}

static void refill(mr_ledger_t *l, uint64_t now_ms)
{
    if (!l->bucket_started) {
        l->bucket_started = true;
        l->bucket_ms = now_ms;
        return;
    }
    if (now_ms > l->bucket_ms) {
        l->bucket_us += (double)(now_ms - l->bucket_ms) * l->refill_us_per_ms;
        if (l->bucket_us > l->bucket_cap_us)
            l->bucket_us = l->bucket_cap_us;
        l->bucket_ms = now_ms;
    }
}

int mr_ledger_charge(mr_ledger_t *l, unsigned port, uint64_t us, bool relay, uint64_t now_ms)
{
    if (port >= MR_NUM_PORTS || !l->port_set[port])
        return MR_EBUDGET;
    if (us == 0)
        return MR_OK;
    refill(l, now_ms);
    if (win_sum(&l->total, now_ms) + us > l->cap_us)
        return MR_EBUDGET;
    if (win_sum(&l->port[port], now_ms) + us > l->port_cap_us[port])
        return MR_EBUDGET;
    if (l->bucket_us < (double)us)
        return MR_EBUDGET;
    l->bucket_us -= (double)us;
    win_add(&l->total, now_ms, us);
    win_add(&l->port[port], now_ms, us);
    if (relay)
        win_add(&l->relay[port], now_ms, us);
    return MR_OK;
}

uint64_t mr_ledger_used_us(const mr_ledger_t *l, int port, uint64_t now_ms)
{
    if (port < 0)
        return win_sum(&l->total, now_ms);
    if ((unsigned)port >= MR_NUM_PORTS)
        return 0;
    return win_sum(&l->port[port], now_ms);
}

uint64_t mr_ledger_relay_used_us(const mr_ledger_t *l, unsigned port, uint64_t now_ms)
{
    return port < MR_NUM_PORTS ? win_sum(&l->relay[port], now_ms) : 0;
}

uint64_t mr_ledger_cap_us(const mr_ledger_t *l, int port)
{
    if (port < 0)
        return l->cap_us;
    return (unsigned)port < MR_NUM_PORTS ? l->port_cap_us[port] : 0;
}
