/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Relay frames: how ip_fallback ports cross between IP islands over LoRa
 * when bridge peers are unreachable over IP. A relay frame keeps the
 * original source, destination, port and frame ID, sealed under the bridge
 * group's relay key, and is never relayed a second time. */
#include <string.h>

#include "core_priv.h"
#include "mr_crypto.h"

#define KIND(x) MUUDD__RADIO__BRIDGE_KIND__##x

int mr_core_relay_transmit_local(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                                 const uint8_t *data, size_t n, uint64_t now_ms)
{
    if (!c->has_relay_key)
        return RS(NO_ROUTE);
    if (n + MR_RELAY_OVERHEAD > mr_core_driver_mtu(c)) {
        c->drops.relay_too_big++;
        return RS(TOO_BIG);
    }
    mr_relay_hdr_t h = {
        .src = src,
        .dest = dest,
        .id = id,
        .ts = (uint32_t)(c->io.wall_ms(c->io.ctx) / 1000u),
        .port = (uint8_t)port,
    };
    uint8_t buf[MR_FRAME_MAX];
    int len = mr_relay_seal(c->relay_key, &h, data, n, buf, sizeof buf);
    if (len < 0)
        return RS(TOO_BIG);
    return mr_core_radio_transmit(c, MR_BROADCAST, MR_PORT_RELAY, buf, (size_t)len, false, port, true, now_ms,
                                  NULL);
}

void mr_core_relay_send(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                        const uint8_t *data, size_t n, uint64_t now_ms)
{
    if (!c->has_relay_key)
        return;
    if (mr_core_is_gateway(c)) {
        mr_core_relay_transmit_local(c, src, id, dest, port, data, n, now_ms);
        return;
    }
    if (c->cfg->has_driver)
        return;   /* a non-gateway radio node never bridges */
    int g = mr_peers_best_gateway(c->peers, NULL, now_ms);
    if (g < 0) {
        c->drops.relay_no_gateway++;
        return;
    }
    MrBridgeFrame f;
    Muudd__Radio__FrameId fid;
    mr_frame_id_t fr_id = {src, id};
    mr_core_fill_frame(&f, &fid, fr_id, KIND(TRANSMIT), src, dest, port, data, n);
    f.relay = true;
    mr_core_send_frame(c, g, &f);
}

void mr_core_relay_received(mr_core_t *c, const MrReceived *r, uint64_t now_ms)
{
    if (!c->has_relay_key) {
        c->drops.relay_auth++;
        return;
    }
    mr_relay_hdr_t h;
    uint8_t payload[MR_FRAME_MAX];
    size_t plen = 0;
    if (mr_relay_open(c->relay_key, r->payload.data, r->payload.len, &h, payload, sizeof payload, &plen) != MR_OK) {
        c->drops.relay_auth++;
        return;
    }
    if (!mr_relay_window_ok(h.ts, c->io.wall_ms(c->io.ctx), c->cfg->relay_window_s)) {
        c->drops.relay_window++;
        return;
    }
    const mr_port_cfg_t *pc = &c->cfg->ports[h.port];
    if (!pc->configured || !pc->ip_fallback) {
        c->drops.unclaimed++;
        return;
    }
    mr_frame_id_t id = {h.src, h.id};
    if (mr_dedupe_check_insert(&c->dedupe, id, mr_core_ttl(c, h.port), now_ms)) {
        c->drops.duplicate++;
        return;
    }
    MrMeta meta;
    if (r->meta)
        meta = *r->meta;
    else
        muudd__radio__meta__init(&meta);
    meta.via = MUUDD__RADIO__VIA__VIA_RELAY;
    meta.packet_id = h.id;
    meta.region = (char *)"";
    if (h.dest == MR_BROADCAST || h.dest == c->node_addr)
        mr_core_deliver_local(c, h.port, h.src, h.dest, payload, plen, &meta);
    if (!mr_core_is_gateway(c))
        return;
    MrBridgeFrame f;
    Muudd__Radio__FrameId fid;
    mr_core_fill_frame(&f, &fid, id, KIND(DELIVER), h.src, h.dest, h.port, payload, plen);
    f.meta = &meta;
    if (h.dest == MR_BROADCAST) {
        bool any_dead;
        mr_core_fanout(c, &f, NULL, now_ms, &any_dead);
    } else {
        int p = mr_peers_find_node(c->peers, h.dest);
        if (p >= 0 && mr_peers_live(c->peers, p, now_ms))
            mr_core_send_frame(c, p, &f);
    }
}
