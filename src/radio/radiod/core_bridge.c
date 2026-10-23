/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Bridging between radiods over IP: bridged sends, the DELIVER, TRANSMIT
 * and RESULT frames, keepalives, and aggregation of results from several
 * gateways. */
#include <string.h>

#include "core_priv.h"

#define KIND(x) MUUDD__RADIO__BRIDGE_KIND__##x

void mr_core_fill_frame(MrBridgeFrame *f, Muudd__Radio__FrameId *fid, mr_frame_id_t id, int kind, uint32_t src,
                        uint32_t dest, uint32_t port, const uint8_t *data, size_t n)
{
    muudd__radio__bridge_frame__init(f);
    muudd__radio__frame_id__init(fid);
    fid->node = id.node;
    fid->id = id.id;
    f->frame_id = fid;
    f->kind = kind;
    f->src = src;
    f->dest = dest;
    f->port = port;
    f->payload.data = (uint8_t *)data;
    f->payload.len = n;
}

void mr_core_send_frame(mr_core_t *c, int peer, MrBridgeFrame *f)
{
    MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
    d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME;
    d.frame = f;
    c->io.to_peer(c->io.ctx, peer, &d);
}

unsigned mr_core_fanout(mr_core_t *c, MrBridgeFrame *f, const bool *skip, uint64_t now_ms, bool *any_dead)
{
    unsigned sent = 0;
    *any_dead = false;
    for (size_t i = 0; i < c->peers->n; i++) {
        if (!mr_peers_live(c->peers, (int)i, now_ms)) {
            *any_dead = true;
            continue;
        }
        if (skip && skip[i])
            continue;
        mr_core_send_frame(c, (int)i, f);
        sent++;
    }
    return sent;
}

void mr_core_send_result_frame(mr_core_t *c, int peer, uint32_t req, int status, int reason, uint32_t packet_id)
{
    if (peer < 0)
        return;
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = req;
    r.status = status;
    r.reason = reason;
    r.packet_id = packet_id;
    MrBridgeFrame f = MUUDD__RADIO__BRIDGE_FRAME__INIT;
    f.kind = KIND(RESULT);
    f.req_id = req;
    f.result = &r;
    mr_core_send_frame(c, peer, &f);
}

/* ---- aggregation ---- */

static mr_pending_t *new_agg(mr_core_t *c, int cid, const MrSend *s, unsigned remaining, uint64_t now_ms)
{
    mr_pending_t *a = mr_core_pending_new(c, PEND_AGG, ++c->next_agg);
    if (!a)
        return NULL;
    a->cid = cid;
    a->consumer_req = s->req_id;
    a->want_ack = s->want_ack;
    a->remaining = remaining;
    a->deadline_ms = now_ms + MR_AGG_TIMEOUT_MS;
    return a;
}

static void agg_finish(mr_core_t *c, mr_pending_t *a)
{
    int refusal = a->first_refusal ? a->first_refusal : RS(NO_ROUTE);
    if (a->want_ack) {
        if (!a->acked_reported) {
            if (a->sent_reported)
                mr_core_reply(c, a->cid, a->consumer_req, ST(NO_ACK), RS(REASON_NONE), 0);
            else
                mr_core_reply(c, a->cid, a->consumer_req, ST(REFUSED), refusal, 0);
        }
    } else if (!a->sent_reported) {
        mr_core_reply(c, a->cid, a->consumer_req, ST(REFUSED), refusal, 0);
    }
    mr_core_pending_free(a);
}

void mr_core_agg_update(mr_core_t *c, mr_pending_t *a, int status, int reason, uint32_t packet_id)
{
    if ((status == ST(SENT) || status == ST(ACKED) || status == ST(NO_ACK)) && !a->sent_reported) {
        a->sent_reported = true;
        mr_core_reply(c, a->cid, a->consumer_req, ST(SENT), RS(REASON_NONE), packet_id);
    }
    if (status == ST(ACKED) && a->want_ack && !a->acked_reported) {
        a->acked_reported = true;
        mr_core_reply(c, a->cid, a->consumer_req, ST(ACKED), RS(REASON_NONE), packet_id);
        mr_core_pending_free(a);
        return;
    }
    if (status == ST(REFUSED) && !a->first_refusal)
        a->first_refusal = reason;
    bool gateway_done = status != ST(SENT) || !a->want_ack;
    if (!gateway_done)
        return;
    if (a->remaining > 0)
        a->remaining--;
    if (a->remaining == 0)
        agg_finish(c, a);
}

/* ---- consumer sends ---- */

static bool top_of_own_region(mr_core_t *c, uint64_t now_ms)
{
    int best = mr_peers_best_gateway(c->peers, c->cfg->region, now_ms);
    if (best < 0)
        return true;
    const mr_peer_t *p = &c->peers->peers[best];
    if (p->cfg->priority < c->cfg->priority)
        return false;
    if (p->cfg->priority == c->cfg->priority && p->node_addr < c->node_addr)
        return false;
    return true;
}

static void send_to_peer_node(mr_core_t *c, int cid, const MrSend *s, MrBridgeFrame *f, mr_frame_id_t id,
                              int peer, uint64_t now_ms)
{
    const mr_port_cfg_t *pc = &c->cfg->ports[s->port];
    if (mr_peers_live(c->peers, peer, now_ms)) {
        if (s->want_ack) {
            mr_pending_t *a = new_agg(c, cid, s, 1, now_ms);
            if (!a) {
                mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(OVER_BUDGET), 0);
                return;
            }
            a->sent_reported = true;
            f->want_ack = true;
            f->req_id = a->key;
        }
        mr_core_send_frame(c, peer, f);
        mr_core_reply(c, cid, s->req_id, ST(SENT), RS(REASON_NONE), 0);
        return;
    }
    if (pc->ip_fallback && c->has_relay_key) {
        mr_core_relay_send(c, c->node_addr, id.id, s->dest, s->port, s->payload.data, s->payload.len, now_ms);
        mr_core_reply(c, cid, s->req_id, ST(SENT), RS(REASON_NONE), 0);
        if (s->want_ack)
            mr_core_reply(c, cid, s->req_id, ST(NO_ACK), RS(REASON_NONE), 0);
        return;
    }
    mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(NO_ROUTE), 0);
}

static void broadcast_send(mr_core_t *c, int cid, const MrSend *s, MrBridgeFrame *f, mr_frame_id_t id,
                           uint64_t now_ms)
{
    bool gw = mr_core_is_gateway(c);
    bool skip[MR_MAX_PEERS] = {false};
    unsigned dispatched = 0;
    int local_refusal = RS(REASON_NONE);
    if (gw) {
        local_refusal = mr_core_radio_transmit(c, MR_BROADCAST, s->port, s->payload.data, s->payload.len, false,
                                               s->port, false, now_ms, NULL);
        if (local_refusal == RS(REASON_NONE))
            dispatched++;
    }
    char regions[MR_MAX_PEERS][64];
    size_t nr = mr_peers_regions(c->peers, now_ms, gw ? c->cfg->region : NULL, regions, MR_MAX_PEERS);
    f->kind = KIND(TRANSMIT);
    for (size_t i = 0; i < nr; i++) {
        int g = mr_peers_best_gateway(c->peers, regions[i], now_ms);
        if (g < 0)
            continue;
        mr_core_send_frame(c, g, f);
        skip[g] = true;
        dispatched++;
    }
    f->kind = KIND(DELIVER);
    bool any_dead = false;
    dispatched += mr_core_fanout(c, f, skip, now_ms, &any_dead);
    if (any_dead && c->cfg->ports[s->port].ip_fallback)
        mr_core_relay_send(c, c->node_addr, id.id, MR_BROADCAST, s->port, s->payload.data, s->payload.len,
                           now_ms);
    if (dispatched) {
        mr_core_reply(c, cid, s->req_id, ST(SENT), RS(REASON_NONE), 0);
        if (s->want_ack)   /* a broadcast is never acknowledged */
            mr_core_reply(c, cid, s->req_id, ST(NO_ACK), RS(REASON_NONE), 0);
    } else
        mr_core_reply(c, cid, s->req_id, ST(REFUSED),
                      local_refusal != RS(REASON_NONE) ? local_refusal : RS(NO_ROUTE), 0);
}

static void unicast_radio_send(mr_core_t *c, int cid, const MrSend *s, MrBridgeFrame *f, uint64_t now_ms)
{
    bool gw = mr_core_is_gateway(c);
    const char *region = mr_peers_region_of(c->peers, s->dest);
    int targets[MR_MAX_PEERS];
    size_t nt = 0;
    bool local = false;
    if (region) {
        if (gw && strcmp(region, c->cfg->region) == 0) {
            local = true;
        } else {
            int g = mr_peers_best_gateway(c->peers, region, now_ms);
            if (g >= 0)
                targets[nt++] = g;
        }
    } else {
        local = gw;
        char regions[MR_MAX_PEERS][64];
        size_t nr = mr_peers_regions(c->peers, now_ms, gw ? c->cfg->region : NULL, regions, MR_MAX_PEERS);
        for (size_t i = 0; i < nr; i++) {
            int g = mr_peers_best_gateway(c->peers, regions[i], now_ms);
            if (g >= 0)
                targets[nt++] = g;
        }
    }
    unsigned total = (unsigned)nt + (local ? 1u : 0u);
    if (total == 0) {
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(NO_ROUTE), 0);
        return;
    }
    mr_pending_t *a = new_agg(c, cid, s, total, now_ms);
    if (!a) {
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(OVER_BUDGET), 0);
        return;
    }
    uint32_t agg = a->key;
    f->kind = KIND(TRANSMIT);
    f->want_ack = s->want_ack;
    f->req_id = agg;
    for (size_t i = 0; i < nt; i++)
        mr_core_send_frame(c, targets[i], f);
    if (!local)
        return;
    mr_pending_t *lp = mr_core_pending_new(c, PEND_LOCAL_AGG, 0);
    uint32_t dreq = 0;
    int r = lp ? mr_core_radio_transmit(c, s->dest, s->port, s->payload.data, s->payload.len, s->want_ack, s->port,
                                        false, now_ms, &dreq)
               : RS(OVER_BUDGET);
    if (r != RS(REASON_NONE)) {
        if (lp)
            mr_core_pending_free(lp);
        a = mr_core_pending_find(c, PEND_AGG, agg);
        if (a)
            mr_core_agg_update(c, a, ST(REFUSED), r, 0);
        return;
    }
    lp->key = dreq;
    lp->agg = agg;
    lp->want_ack = s->want_ack;
}

void mr_core_bridged_send(mr_core_t *c, int cid, const MrSend *s, uint64_t now_ms)
{
    mr_frame_id_t id = {c->node_addr, ++c->next_seq};
    mr_dedupe_check_insert(&c->dedupe, id, mr_core_ttl(c, s->port), now_ms);
    MrBridgeFrame f;
    Muudd__Radio__FrameId fid;
    mr_core_fill_frame(&f, &fid, id, KIND(DELIVER), c->node_addr, s->dest, s->port, s->payload.data,
                       s->payload.len);
    int peer = s->dest == MR_BROADCAST ? -1 : mr_peers_find_node(c->peers, s->dest);
    if (peer >= 0)
        send_to_peer_node(c, cid, s, &f, id, peer, now_ms);
    else if (s->dest == MR_BROADCAST)
        broadcast_send(c, cid, s, &f, id, now_ms);
    else
        unicast_radio_send(c, cid, s, &f, now_ms);
}

/* ---- heard on our radio ---- */

void mr_core_gateway_heard(mr_core_t *c, const MrReceived *r, uint64_t now_ms)
{
    if (mr_peers_is_gateway_radio(c->peers, r->src))
        return;   /* echo rule: that gateway already put this frame on IP */
    uint32_t pid = r->meta ? r->meta->packet_id : 0;
    mr_frame_id_t id = {r->src, pid};
    MrBridgeFrame f;
    Muudd__Radio__FrameId fid;
    mr_core_fill_frame(&f, &fid, id, KIND(DELIVER), r->src, r->dest, r->port, r->payload.data, r->payload.len);
    MrMeta meta;
    if (r->meta)
        meta = *r->meta;
    else
        muudd__radio__meta__init(&meta);
    meta.region = (char *)c->cfg->region;
    f.meta = &meta;
    f.origin_region = (char *)c->cfg->region;
    /* A non-gateway radio node that is also a bridge peer heard its own
     * frame on air; it needs no copy back. */
    bool skip[MR_MAX_PEERS] = {false};
    for (size_t i = 0; i < c->peers->n; i++)
        if (r->src && c->peers->peers[i].heard && c->peers->peers[i].radio_node_addr == r->src)
            skip[i] = true;
    bool any_dead = false;
    mr_core_fanout(c, &f, skip, now_ms, &any_dead);
    if (any_dead && c->cfg->ports[r->port].ip_fallback)
        mr_core_relay_transmit_local(c, r->src, pid, r->dest, r->port, r->payload.data, r->payload.len, now_ms);
}

/* ---- frames from peers ---- */

static void cross_region(mr_core_t *c, const MrBridgeFrame *f, uint64_t now_ms)
{
    if (!mr_core_is_gateway(c) || !c->cfg->ports[f->port].cross_region)
        return;
    if (!f->origin_region || !f->origin_region[0] || strcmp(f->origin_region, c->cfg->region) == 0)
        return;
    if (!top_of_own_region(c, now_ms))
        return;
    mr_core_radio_transmit(c, f->dest, f->port, f->payload.data, f->payload.len, false, f->port, false, now_ms,
                           NULL);
}

static void on_transmit(mr_core_t *c, int peer, const MrBridgeFrame *f, const MrMeta *meta, uint64_t now_ms)
{
    if (f->dest == MR_BROADCAST || f->dest == c->node_addr)
        mr_core_deliver_local(c, f->port, f->src, f->dest, f->payload.data, f->payload.len, meta);
    mr_pending_t *p = NULL;
    if (f->req_id) {
        p = mr_core_pending_new(c, PEND_REMOTE, 0);
        if (!p) {
            mr_core_send_result_frame(c, peer, f->req_id, ST(REFUSED), RS(OVER_BUDGET), 0);
            return;
        }
    }
    uint32_t dreq = 0;
    int r = mr_core_radio_transmit(c, f->dest, f->port, f->payload.data, f->payload.len, f->want_ack, f->port,
                                   false, now_ms, &dreq);
    if (r != RS(REASON_NONE)) {
        if (p) {
            mr_core_pending_free(p);
            mr_core_send_result_frame(c, peer, f->req_id, ST(REFUSED), r, 0);
        }
        return;
    }
    if (p) {
        p->key = dreq;
        p->peer = peer;
        p->peer_req = f->req_id;
        p->want_ack = f->want_ack;
    }
}

static void on_frame(mr_core_t *c, int peer, const MrBridgeFrame *f, uint64_t now_ms)
{
    if (f->kind == KIND(RESULT)) {
        mr_pending_t *a = mr_core_pending_find(c, PEND_AGG, f->req_id);
        if (a && f->result)
            mr_core_agg_update(c, a, f->result->status, f->result->reason, f->result->packet_id);
        return;
    }
    if (!f->frame_id || f->port >= MR_NUM_PORTS || !c->cfg->ports[f->port].configured ||
        c->cfg->ports[f->port].policy != MUUDD__RADIO__POLICY__RADIOD_BRIDGED)
        return;   /* consumer-bridged ports never travel over the bridge */
    mr_frame_id_t id = {f->frame_id->node, f->frame_id->id};
    if (f->kind == KIND(TRANSMIT) && f->relay) {
        /* A relay request shares its frame ID with the plain transmit of the same
         * message, so it has its own dedupe cache and is never delivered locally. */
        if (mr_dedupe_check_insert(&c->relay_req_dedupe, id, mr_core_ttl(c, f->port), now_ms)) {
            c->drops.duplicate++;
            return;
        }
        int r = mr_core_relay_transmit_local(c, f->src, f->frame_id->id, f->dest, f->port, f->payload.data,
                                             f->payload.len, now_ms);
        if (f->req_id)
            mr_core_send_result_frame(c, peer, f->req_id, r == RS(REASON_NONE) ? ST(SENT) : ST(REFUSED), r, 0);
        return;
    }
    if (mr_dedupe_check_insert(&c->dedupe, id, mr_core_ttl(c, f->port), now_ms)) {
        c->drops.duplicate++;
        return;
    }
    MrMeta meta;
    if (f->meta)
        meta = *f->meta;
    else
        muudd__radio__meta__init(&meta);
    if (meta.via != MUUDD__RADIO__VIA__VIA_RELAY)
        meta.via = MUUDD__RADIO__VIA__VIA_BRIDGE;   /* a relayed frame stays marked as relayed */
    if (f->origin_region && f->origin_region[0]) {
        meta.region = f->origin_region;
        mr_peers_note_region(c->peers, f->src, f->origin_region, now_ms);
    }
    if (f->kind == KIND(DELIVER)) {
        bool delivered = false;
        if (f->dest == MR_BROADCAST || f->dest == c->node_addr)
            delivered = mr_core_deliver_local(c, f->port, f->src, f->dest, f->payload.data, f->payload.len, &meta);
        if (f->want_ack)
            mr_core_send_result_frame(c, peer, f->req_id, delivered ? ST(ACKED) : ST(NO_ACK), RS(REASON_NONE), 0);
        cross_region(c, f, now_ms);
    } else if (f->kind == KIND(TRANSMIT)) {
        on_transmit(c, peer, f, &meta, now_ms);
    }
}

void mr_core_bridge_datagram(mr_core_t *c, int peer, const MrDatagram *d, uint64_t now_ms)
{
    if (peer < 0 || (size_t)peer >= c->peers->n)
        return;
    if (d->msg_case == MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_KEEPALIVE && d->keepalive)
        mr_peers_accept_keepalive(c->peers, peer, d->keepalive, now_ms);
    else if (d->msg_case == MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME && d->frame)
        on_frame(c, peer, d->frame, now_ms);
}

/* ---- time ---- */

void mr_core_bridge_tick(mr_core_t *c, uint64_t now_ms)
{
    if (!c->ka_started || now_ms - c->last_ka_ms >= c->cfg->keepalive_ms) {
        c->ka_started = true;
        c->last_ka_ms = now_ms;
        MrKeepalive ka = MUUDD__RADIO__KEEPALIVE__INIT;
        ka.node_addr = c->node_addr;
        ka.radio_node_addr = c->driver_up && c->caps ? c->caps->node_addr : 0;
        ka.region = (char *)c->cfg->region;
        ka.gateway = mr_core_is_gateway(c);
        ka.radio_up = c->driver_up;
        ka.boot_id = c->boot_id;
        ka.seq = ++c->ka_seq;
        ka.radio_mtu = c->driver_up && c->caps ? c->caps->mtu : 0;
        MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
        d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_KEEPALIVE;
        d.keepalive = &ka;
        for (size_t i = 0; i < c->peers->n; i++)
            c->io.to_peer(c->io.ctx, (int)i, &d);
    }
    for (size_t i = 0; i < MR_PENDING_MAX; i++) {
        mr_pending_t *p = &c->pending[i];
        if (p->kind == PEND_AGG && p->deadline_ms && now_ms >= p->deadline_ms)
            agg_finish(c, p);
    }
}
