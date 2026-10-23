/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Without bridge support a bridged send has no route, and bridge traffic is
 * ignored. */
#include "core_priv.h"

void mr_core_bridged_send(mr_core_t *c, int cid, const MrSend *s, uint64_t now_ms)
{
    (void)now_ms;
    mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(NO_ROUTE), 0);
}

void mr_core_gateway_heard(mr_core_t *c, const MrReceived *r, uint64_t now_ms) { (void)c; (void)r; (void)now_ms; }
void mr_core_bridge_datagram(mr_core_t *c, int peer, const MrDatagram *d, uint64_t now_ms)
{
    (void)c; (void)peer; (void)d; (void)now_ms;
}
void mr_core_bridge_tick(mr_core_t *c, uint64_t now_ms) { (void)c; (void)now_ms; }
void mr_core_agg_update(mr_core_t *c, mr_pending_t *a, int status, int reason, uint32_t packet_id)
{
    (void)c; (void)a; (void)status; (void)reason; (void)packet_id;
}
void mr_core_send_result_frame(mr_core_t *c, int peer, uint32_t req, int status, int reason, uint32_t packet_id)
{
    (void)c; (void)peer; (void)req; (void)status; (void)reason; (void)packet_id;
}
