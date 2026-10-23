/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Without relay support nothing is relayed, and relay frames heard on the
 * radio are dropped and counted as unauthenticated. */
#include "core_priv.h"

void mr_core_relay_send(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                        const uint8_t *data, size_t n, uint64_t now_ms)
{
    (void)c; (void)src; (void)id; (void)dest; (void)port; (void)data; (void)n; (void)now_ms;
}

int mr_core_relay_transmit_local(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                                 const uint8_t *data, size_t n, uint64_t now_ms)
{
    (void)c; (void)src; (void)id; (void)dest; (void)port; (void)data; (void)n; (void)now_ms;
    return RS(NO_ROUTE);
}

void mr_core_relay_received(mr_core_t *c, const MrReceived *r, uint64_t now_ms)
{
    (void)r; (void)now_ms;
    c->drops.relay_auth++;
}
