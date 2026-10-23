/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "core_priv.h"

/* ---- lifecycle ---- */

int mr_core_create(const mr_config_t *cfg, const mr_keypair_t *kp, const uint8_t *relay_key,
                   const mr_core_io_t *io, mr_core_t **out)
{
    mr_core_t *c = calloc(1, sizeof *c);
    if (!c)
        return MR_ENOMEM;
    c->cfg = cfg;
    c->kp = *kp;
    c->io = *io;
    if (relay_key) {
        memcpy(c->relay_key, relay_key, 32);
        c->has_relay_key = true;
    }
    c->node_addr = cfg->node_addr_auto ? 0 : cfg->node_addr;
    snprintf(c->reason, sizeof c->reason, "%s", cfg->has_driver ? "driver starting" : "no live gateway");
    mr_ports_init(&c->ports);
    c->ledger = calloc(1, sizeof *c->ledger);
    c->peers = calloc(1, sizeof *c->peers);
    if (!c->ledger || !c->peers || mr_dedupe_init(&c->dedupe_short, MR_DEDUPE_CAP) != MR_OK ||
        mr_dedupe_init(&c->dedupe_long, MR_DEDUPE_CAP) != MR_OK ||
        mr_dedupe_init(&c->relay_req_dedupe, 4096) != MR_OK) {
        mr_core_free(c);
        return MR_ENOMEM;
    }
    mr_ledger_init(c->ledger, cfg->duty_cycle_pct);
    for (unsigned p = 0; p < MR_NUM_PORTS; p++)
        if (cfg->ports[p].configured)
            mr_ledger_set_port(c->ledger, p, cfg->ports[p].share_pct);
    mr_peers_init(c->peers, cfg);
    c->boot_id = io->wall_ms(io->ctx);
    *out = c;
    return MR_OK;
}

void mr_core_free(mr_core_t *c)
{
    if (!c)
        return;
    if (c->caps)
        muudd__radio__caps__free_unpacked(c->caps, NULL);
    mr_dedupe_free(&c->dedupe_short);
    mr_dedupe_free(&c->dedupe_long);
    mr_dedupe_free(&c->relay_req_dedupe);
    free(c->ledger);
    free(c->peers);
    free(c);
}

mr_peers_t *mr_core_peers(mr_core_t *c) { return c->peers; }
uint32_t mr_core_node_addr(const mr_core_t *c) { return c->node_addr; }
const char *mr_core_reason(const mr_core_t *c) { return c->reason; }
void mr_core_set_boot_id(mr_core_t *c, uint64_t boot_id) { c->boot_id = boot_id; }
void mr_core_note_bridge_auth_failure(mr_core_t *c) { c->drops.bridge_auth++; }
void mr_core_note_queue_overflow(mr_core_t *c, uint64_t n) { c->drops.queue_overflow += n; }

/* ---- shared helpers ---- */

bool mr_core_is_gateway(const mr_core_t *c)
{
    return c->cfg->has_bridge && c->cfg->gateway && c->cfg->has_driver;
}

bool mr_core_up(const mr_core_t *c, uint64_t now_ms)
{
    if (c->cfg->has_driver)
        return c->driver_up;
    return mr_peers_best_gateway(c->peers, NULL, now_ms) >= 0;
}

uint32_t mr_core_driver_mtu(const mr_core_t *c)
{
    if (c->cfg->has_driver)
        return c->driver_up && c->caps ? c->caps->mtu : 0;
    return mr_peers_min_gateway_mtu(c->peers);
}

uint64_t mr_core_ttl(const mr_core_t *c, uint32_t port)
{
    uint64_t ttl = MR_DEDUPE_TTL_MS;
    if (port < MR_NUM_PORTS && c->cfg->ports[port].ip_fallback && c->has_relay_key) {
        uint64_t w = (uint64_t)c->cfg->relay_window_s * 1000u;
        if (w > ttl)
            ttl = w;
    }
    return ttl;
}

bool mr_core_seen(mr_core_t *c, mr_frame_id_t id, uint32_t port, uint64_t now_ms)
{
    uint64_t ttl = mr_core_ttl(c, port);
    mr_dedupe_t *d = ttl > MR_DEDUPE_TTL_MS ? &c->dedupe_long : &c->dedupe_short;
    bool seen = mr_dedupe_check_insert(d, id, ttl, now_ms);
    uint64_t evicted = c->dedupe_short.evicted_early + c->dedupe_long.evicted_early;
    if (evicted > c->evicted_logged &&
        (!c->evict_log_started || now_ms - c->evict_log_ms >= MR_LOG_EVERY_MS)) {
        if (c->io.log) {
            char msg[160];
            snprintf(msg, sizeof msg,
                     "dedupe cache full: %llu unexpired frame IDs evicted (%llu ten-minute, %llu long-lived)",
                     (unsigned long long)(evicted - c->evicted_logged),
                     (unsigned long long)c->dedupe_short.evicted_early,
                     (unsigned long long)c->dedupe_long.evicted_early);
            c->io.log(c->io.ctx, msg);
        }
        c->evicted_logged = evicted;
        c->evict_log_started = true;
        c->evict_log_ms = now_ms;
    }
    return seen;
}

void mr_core_reply(mr_core_t *c, int cid, uint32_t req, int status, int reason, uint32_t packet_id)
{
    if (cid < 0)
        return;
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = req;
    r.status = status;
    r.reason = reason;
    r.packet_id = packet_id;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT;
    m.send_result = &r;
    c->io.to_consumer(c->io.ctx, cid, &m);
}

mr_pending_t *mr_core_pending_new(mr_core_t *c, mr_pend_kind_t kind, uint32_t key)
{
    for (size_t i = 0; i < MR_PENDING_MAX; i++) {
        mr_pending_t *p = &c->pending[i];
        if (p->kind == PEND_FREE) {
            memset(p, 0, sizeof *p);
            p->kind = kind;
            p->key = key;
            p->cid = -1;
            p->peer = -1;
            return p;
        }
    }
    return NULL;
}

mr_pending_t *mr_core_pending_find(mr_core_t *c, mr_pend_kind_t kind, uint32_t key)
{
    for (size_t i = 0; i < MR_PENDING_MAX; i++)
        if (c->pending[i].kind == kind && c->pending[i].key == key)
            return &c->pending[i];
    return NULL;
}

void mr_core_pending_free(mr_pending_t *p) { p->kind = PEND_FREE; }

static bool driver_kind(mr_pend_kind_t k)
{
    return k == PEND_LOCAL || k == PEND_REMOTE || k == PEND_LOCAL_AGG;
}

static mr_pending_t *find_driver_pending(mr_core_t *c, uint32_t dreq)
{
    for (size_t i = 0; i < MR_PENDING_MAX; i++)
        if (driver_kind(c->pending[i].kind) && c->pending[i].key == dreq)
            return &c->pending[i];
    return NULL;
}

/* Forward a driver outcome to whoever the pending entry is owed to. */
static void forward_driver_outcome(mr_core_t *c, mr_pending_t *p, int status, int reason, uint32_t packet_id)
{
    switch (p->kind) {
    case PEND_LOCAL:
        mr_core_reply(c, p->cid, p->consumer_req, status, reason, packet_id);
        break;
    case PEND_REMOTE:
        mr_core_send_result_frame(c, p->peer, p->peer_req, status, reason, packet_id);
        break;
    case PEND_LOCAL_AGG: {
        mr_pending_t *a = mr_core_pending_find(c, PEND_AGG, p->agg);
        if (a)
            mr_core_agg_update(c, a, status, reason, packet_id);
        break;
    }
    default:
        break;
    }
}

int mr_core_radio_transmit(mr_core_t *c, uint32_t dest, uint32_t port, const uint8_t *data, size_t n,
                           bool want_ack, uint32_t charge_port, bool relay, uint64_t now_ms, uint32_t *driver_req)
{
    if (!c->cfg->has_driver || !c->driver_up || !c->caps)
        return RS(RADIO_DOWN);
    if (n > c->caps->mtu)
        return RS(TOO_BIG);
    if (mr_ledger_charge(c->ledger, charge_port, mr_airtime_caps_us(c->caps, n), relay, now_ms) != MR_OK) {
        c->drops.over_budget++;
        return RS(OVER_BUDGET);
    }
    MrSend s = MUUDD__RADIO__SEND__INIT;
    s.req_id = ++c->next_driver_req;
    s.dest = dest;
    s.port = port;
    s.payload.data = (uint8_t *)data;
    s.payload.len = n;
    s.want_ack = want_ack;
    MrR2D m = MUUDD__RADIO__RADIOD_TO_DRIVER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_SEND;
    m.send = &s;
    c->io.to_driver(c->io.ctx, &m);
    if (driver_req)
        *driver_req = s.req_id;
    return RS(REASON_NONE);
}

bool mr_core_deliver_local(mr_core_t *c, uint32_t port, uint32_t src, uint32_t dest, const uint8_t *data,
                           size_t n, const MrMeta *meta)
{
    int cid = mr_ports_claimant(&c->ports, port);
    if (cid < 0) {
        c->drops.unclaimed++;
        return false;
    }
    MrReceived rv = MUUDD__RADIO__RECEIVED__INIT;
    rv.src = src;
    rv.dest = dest;
    rv.port = port;
    rv.payload.data = (uint8_t *)data;
    rv.payload.len = n;
    rv.meta = (MrMeta *)meta;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED;
    m.received = &rv;
    c->io.to_consumer(c->io.ctx, cid, &m);
    return true;
}

/* ---- status and stats ---- */

static void send_status(mr_core_t *c, int cid, uint64_t now_ms)
{
    bool up = mr_core_up(c, now_ms);
    uint32_t dmtu = mr_core_driver_mtu(c);
    Muudd__Radio__PortMtu mtus[MR_NUM_PORTS];
    Muudd__Radio__PortMtu *ptrs[MR_NUM_PORTS];
    size_t n = 0;
    for (unsigned p = 0; p < MR_NUM_PORTS; p++) {
        if (mr_ports_claimant(&c->ports, p) != cid)
            continue;
        muudd__radio__port_mtu__init(&mtus[n]);
        mtus[n].port = p;
        mtus[n].mtu = mr_ports_mtu(c->cfg, p, dmtu);
        ptrs[n] = &mtus[n];
        n++;
    }
    MrRadioStatus st = MUUDD__RADIO__RADIO_STATUS__INIT;
    st.state = up ? MUUDD__RADIO__RADIO_STATE__STATE_UP : MUUDD__RADIO__RADIO_STATE__STATE_DOWN;
    st.reason = (char *)(up ? "" : c->reason);
    st.caps = c->caps;
    st.n_mtus = n;
    st.mtus = ptrs;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RADIO_STATUS;
    m.radio_status = &st;
    c->io.to_consumer(c->io.ctx, cid, &m);
}

void mr_core_status_all(mr_core_t *c, uint64_t now_ms)
{
    for (int i = 0; i < MR_MAX_CONSUMERS; i++)
        if (c->connected[i])
            send_status(c, i, now_ms);
    c->last_up = mr_core_up(c, now_ms);
    c->last_mtu = mr_core_driver_mtu(c);
}

static bool any_fallback_port(const mr_core_t *c)
{
    for (unsigned p = 0; p < MR_NUM_PORTS; p++)
        if (c->cfg->ports[p].ip_fallback)
            return true;
    return false;
}

static void send_stats(mr_core_t *c, int cid, uint32_t req, uint64_t now_ms)
{
    bool up = mr_core_up(c, now_ms);
    Muudd__Radio__PortStats ps[MR_NUM_PORTS];
    Muudd__Radio__PortStats *pp[MR_NUM_PORTS];
    size_t np = 0;
    for (unsigned p = 0; p < MR_NUM_PORTS; p++) {
        if (!c->cfg->ports[p].configured)
            continue;
        muudd__radio__port_stats__init(&ps[np]);
        ps[np].port = p;
        ps[np].name = (char *)c->cfg->ports[p].name;
        ps[np].airtime_us = mr_ledger_used_us(c->ledger, (int)p, now_ms);
        ps[np].relay_airtime_us = mr_ledger_relay_used_us(c->ledger, p, now_ms);
        ps[np].cap_us = mr_ledger_cap_us(c->ledger, (int)p);
        pp[np] = &ps[np];
        np++;
    }
    Muudd__Radio__PeerStats prs[MR_MAX_PEERS];
    Muudd__Radio__PeerStats *ppr[MR_MAX_PEERS];
    for (size_t i = 0; i < c->peers->n; i++) {
        const mr_peer_t *p = &c->peers->peers[i];
        muudd__radio__peer_stats__init(&prs[i]);
        prs[i].addr = (char *)p->cfg->addr;
        prs[i].node_addr = p->node_addr;
        prs[i].region = (char *)p->region;
        prs[i].live = mr_peers_live(c->peers, (int)i, now_ms);
        prs[i].gateway = p->gateway;
        ppr[i] = &prs[i];
    }
    Muudd__Radio__DropCounters d = MUUDD__RADIO__DROP_COUNTERS__INIT;
    d.unclaimed = c->drops.unclaimed;
    d.queue_overflow = c->drops.queue_overflow;
    d.duplicate = c->drops.duplicate;
    d.bridge_auth = c->drops.bridge_auth;
    d.relay_auth = c->drops.relay_auth;
    d.relay_window = c->drops.relay_window;
    d.relay_no_gateway = c->drops.relay_no_gateway;
    d.over_budget = c->drops.over_budget;
    d.relay_too_big = c->drops.relay_too_big;
    d.bridge_replay = c->drops.bridge_replay;
    d.dedupe_evicted = c->dedupe_short.evicted_early + c->dedupe_long.evicted_early;
    MrStats st = MUUDD__RADIO__STATS__INIT;
    st.req_id = req;
    st.state = up ? MUUDD__RADIO__RADIO_STATE__STATE_UP : MUUDD__RADIO__RADIO_STATE__STATE_DOWN;
    st.reason = (char *)(up ? "" : c->reason);
    st.caps = c->caps;
    st.airtime_us = mr_ledger_used_us(c->ledger, -1, now_ms);
    st.cap_us = mr_ledger_cap_us(c->ledger, -1);
    st.n_ports = np;
    st.ports = pp;
    st.n_peers = c->peers->n;
    st.peers = ppr;
    st.drops = &d;
    st.relay_active = c->has_relay_key && c->cfg->has_bridge && any_fallback_port(c) &&
                      mr_peers_any_dead(c->peers, now_ms);
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_STATS;
    m.stats = &st;
    c->io.to_consumer(c->io.ctx, cid, &m);
}

/* ---- consumers ---- */

void mr_core_consumer_connected(mr_core_t *c, int cid, uint64_t now_ms)
{
    if (cid < 0 || cid >= MR_MAX_CONSUMERS)
        return;
    c->connected[cid] = true;
    send_status(c, cid, now_ms);
}

void mr_core_consumer_disconnected(mr_core_t *c, int cid)
{
    if (cid < 0 || cid >= MR_MAX_CONSUMERS)
        return;
    c->connected[cid] = false;
    mr_ports_release_all(&c->ports, cid);
    for (size_t i = 0; i < MR_PENDING_MAX; i++)
        if (c->pending[i].kind != PEND_FREE && c->pending[i].cid == cid)
            c->pending[i].cid = -1;
}

static void on_send(mr_core_t *c, int cid, const MrSend *s, uint64_t now_ms)
{
    if (s->port >= MR_NUM_PORTS || mr_ports_claimant(&c->ports, s->port) != cid) {
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(NOT_CLAIMED), 0);
        return;
    }
    if (s->payload.len > mr_ports_mtu(c->cfg, s->port, mr_core_driver_mtu(c))) {
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), mr_core_up(c, now_ms) ? RS(TOO_BIG) : RS(RADIO_DOWN), 0);
        return;
    }
    const mr_port_cfg_t *pc = &c->cfg->ports[s->port];
    bool non_gateway_radio = c->cfg->has_driver && !c->cfg->gateway;
    if (pc->policy == MUUDD__RADIO__POLICY__RADIOD_BRIDGED && c->cfg->has_bridge && !non_gateway_radio) {
        mr_core_bridged_send(c, cid, s, now_ms);
        return;
    }
    mr_pending_t *p = mr_core_pending_new(c, PEND_LOCAL, 0);
    if (!p) {
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), RS(OVER_BUDGET), 0);
        return;
    }
    uint32_t dreq = 0;
    int r = mr_core_radio_transmit(c, s->dest, s->port, s->payload.data, s->payload.len, s->want_ack, s->port,
                                   false, now_ms, &dreq);
    if (r != RS(REASON_NONE)) {
        mr_core_pending_free(p);
        mr_core_reply(c, cid, s->req_id, ST(REFUSED), r, 0);
        return;
    }
    p->key = dreq;
    p->cid = cid;
    p->consumer_req = s->req_id;
    p->want_ack = s->want_ack;
}

void mr_core_consumer_msg(mr_core_t *c, int cid, const MrC2R *m, uint64_t now_ms)
{
    switch (m->msg_case) {
    case MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM: {
        int st = mr_ports_claim(&c->ports, c->cfg, cid, m->claim->port, m->claim->policy);
        MrClaimResult r = MUUDD__RADIO__CLAIM_RESULT__INIT;
        r.req_id = m->claim->req_id;
        r.status = st;
        if (st == MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK)
            r.mtu = mr_ports_mtu(c->cfg, m->claim->port, mr_core_driver_mtu(c));
        MrR2C out = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
        out.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT;
        out.claim_result = &r;
        c->io.to_consumer(c->io.ctx, cid, &out);
        break;
    }
    case MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_RELEASE: {
        mr_ports_release(&c->ports, cid, m->release->port);
        Muudd__Radio__ReleaseResult r = MUUDD__RADIO__RELEASE_RESULT__INIT;
        r.req_id = m->release->req_id;
        MrR2C out = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
        out.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RELEASE_RESULT;
        out.release_result = &r;
        c->io.to_consumer(c->io.ctx, cid, &out);
        break;
    }
    case MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND:
        on_send(c, cid, m->send, now_ms);
        break;
    case MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_GET_STATS:
        send_stats(c, cid, m->get_stats->req_id, now_ms);
        break;
    default:
        break;
    }
}

/* ---- driver ---- */

static MrCaps *copy_caps(const MrCaps *caps)
{
    size_t n = muudd__radio__caps__get_packed_size(caps);
    uint8_t *buf = malloc(n ? n : 1);
    if (!buf)
        return NULL;
    muudd__radio__caps__pack(caps, buf);
    MrCaps *out = muudd__radio__caps__unpack(NULL, n, buf);
    free(buf);
    return out;
}

static void send_configure(mr_core_t *c)
{
    Muudd__Radio__Configure__SettingsEntry entries[MR_MAX_SETTINGS];
    Muudd__Radio__Configure__SettingsEntry *ptrs[MR_MAX_SETTINGS];
    for (size_t i = 0; i < c->cfg->n_settings; i++) {
        muudd__radio__configure__settings_entry__init(&entries[i]);
        entries[i].key = (char *)c->cfg->setting_keys[i];
        entries[i].value = (char *)c->cfg->setting_vals[i];
        ptrs[i] = &entries[i];
    }
    Muudd__Radio__Configure conf = MUUDD__RADIO__CONFIGURE__INIT;
    conf.n_settings = c->cfg->n_settings;
    conf.settings = ptrs;
    conf.apply_radio_config = c->cfg->apply_radio_config;
    MrR2D m = MUUDD__RADIO__RADIOD_TO_DRIVER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_CONFIGURE;
    m.configure = &conf;
    c->io.to_driver(c->io.ctx, &m);
}

static void fail_in_flight(mr_core_t *c);

static bool bandwidth_too_low(mr_core_t *c, uint32_t bw)
{
    if (bw >= c->cfg->min_bandwidth_khz)
        return false;
    snprintf(c->reason, sizeof c->reason, "radio bandwidth %u kHz below minimum %u kHz", bw,
             c->cfg->min_bandwidth_khz);
    return true;
}

static int go_fatal(mr_core_t *c, uint64_t now_ms)
{
    c->driver_up = false;
    c->fatal = true;
    fail_in_flight(c);
    mr_core_status_all(c, now_ms);
    return MR_EFATAL;
}

int mr_core_driver_hello(mr_core_t *c, const MrCaps *caps, uint64_t now_ms)
{
    uint32_t bw = caps->radio_settings ? caps->radio_settings->bandwidth_khz : 0;
    if (caps->fatal && caps->fatal[0]) {
        snprintf(c->reason, sizeof c->reason, "driver fatal: %s", caps->fatal);
        return go_fatal(c, now_ms);
    }
    if (bandwidth_too_low(c, bw))
        return go_fatal(c, now_ms);
    MrCaps *copy = copy_caps(caps);
    if (!copy) {
        snprintf(c->reason, sizeof c->reason, "out of memory");
        return go_fatal(c, now_ms);
    }
    if (c->caps)
        muudd__radio__caps__free_unpacked(c->caps, NULL);
    c->caps = copy;
    if (c->cfg->node_addr_auto)
        c->node_addr = caps->node_addr;
    mr_ledger_set_max_frame(c->ledger, mr_airtime_caps_us(c->caps, c->caps->mtu));
    c->driver_up = true;
    c->fatal = false;
    c->reason[0] = 0;
    send_configure(c);
    mr_core_status_all(c, now_ms);
    return MR_OK;
}

static void on_driver_result(mr_core_t *c, const MrSendResult *r, uint64_t now_ms)
{
    mr_pending_t *p = find_driver_pending(c, r->req_id);
    if (!p)
        return;
    if (r->status == ST(SENT) && p->sent_reported)
        return;   /* repeated SENT: already reported, keep the ack deadline */
    bool terminal = r->status != ST(SENT) || !p->want_ack;
    forward_driver_outcome(c, p, r->status, r->reason, r->packet_id);
    if (terminal) {
        mr_core_pending_free(p);
    } else {
        p->sent_reported = true;
        p->deadline_ms = now_ms + MR_ACK_TIMEOUT_MS;
    }
}

/* True if radio_addr belongs to a peer gateway we can currently reach over
 * IP. Such a peer has already put whatever it transmits on the bridge. A
 * non-gateway radio node puts nothing on the bridge, so it never counts. */
static bool live_gateway_radio(const mr_core_t *c, uint32_t radio_addr, uint64_t now_ms)
{
    if (radio_addr == 0)
        return false;
    for (size_t i = 0; i < c->peers->n; i++) {
        const mr_peer_t *p = &c->peers->peers[i];
        if (p->heard && p->gateway && p->radio_node_addr == radio_addr && mr_peers_live(c->peers, (int)i, now_ms))
            return true;
    }
    return false;
}

static void on_driver_received(mr_core_t *c, const MrReceived *r, uint64_t now_ms)
{
    if (r->port >= MR_NUM_PORTS) {
        c->drops.unclaimed++;
        return;
    }
    if (r->port == MR_PORT_RELAY) {
        mr_core_relay_received(c, r, now_ms);
        return;
    }
    mr_frame_id_t id = {r->src, r->meta ? r->meta->packet_id : 0};
    if (mr_core_seen(c, id, r->port, now_ms)) {
        c->drops.duplicate++;
        return;
    }
    const mr_port_cfg_t *pc = &c->cfg->ports[r->port];
    bool bridged_broadcast = r->dest == MR_BROADCAST && c->cfg->has_bridge && pc->configured &&
                             pc->policy == MUUDD__RADIO__POLICY__RADIOD_BRIDGED;
    /* Any radio node with a bridge gets a live gateway's broadcasts over IP
     * as well, so the LoRa copy would be a second delivery. */
    if (bridged_broadcast && live_gateway_radio(c, r->src, now_ms)) {
        c->drops.duplicate++;
        return;
    }
    MrMeta meta;
    if (r->meta)
        meta = *r->meta;
    else
        muudd__radio__meta__init(&meta);
    meta.via = MUUDD__RADIO__VIA__VIA_RADIO;
    if (r->dest == MR_BROADCAST || r->dest == c->node_addr)
        mr_core_deliver_local(c, r->port, r->src, r->dest, r->payload.data, r->payload.len, &meta);
    if (bridged_broadcast && mr_core_is_gateway(c))
        mr_core_gateway_heard(c, r, now_ms);
}

/* A periodic Status: keeps the cached radio settings current, enforces the
 * bandwidth minimum, and follows the driver's own view of whether the radio
 * is usable. */
static int on_driver_status(mr_core_t *c, const Muudd__Radio__DriverStatus *st, uint64_t now_ms)
{
    if (c->fatal || !c->caps)
        return c->fatal ? MR_EFATAL : MR_OK;
    if (st->radio_settings) {
        if (bandwidth_too_low(c, st->radio_settings->bandwidth_khz))
            return go_fatal(c, now_ms);
        MrCaps tmp = *c->caps;
        tmp.radio_settings = st->radio_settings;
        MrCaps *copy = copy_caps(&tmp);
        if (copy) {
            muudd__radio__caps__free_unpacked(c->caps, NULL);
            c->caps = copy;
        }
    }
    if (st->state == MUUDD__RADIO__RADIO_STATE__STATE_DOWN) {
        if (c->driver_up) {
            snprintf(c->reason, sizeof c->reason, "%s",
                     st->reason && st->reason[0] ? st->reason : "driver reported the radio down");
            c->driver_up = false;
            fail_in_flight(c);
            mr_core_status_all(c, now_ms);
        }
    } else if (!c->driver_up) {
        c->driver_up = true;
        c->reason[0] = 0;
        mr_core_status_all(c, now_ms);
    }
    return MR_OK;
}

int mr_core_driver_msg(mr_core_t *c, const MrD2R *m, uint64_t now_ms)
{
    switch (m->msg_case) {
    case MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_SEND_RESULT:
        on_driver_result(c, m->send_result, now_ms);
        break;
    case MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_RECEIVED:
        on_driver_received(c, m->received, now_ms);
        break;
    case MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_STATUS:
        if (m->status)
            return on_driver_status(c, m->status, now_ms);
        break;
    default:
        break;
    }
    return MR_OK;
}

static void complete_driver_pending(mr_core_t *c, mr_pending_t *p, int status, int reason)
{
    forward_driver_outcome(c, p, status, reason, 0);
    mr_core_pending_free(p);
}

/* Settle every send still waiting on the driver: acked sends were already
 * reported as sent and end as NO_ACK, the rest are refused as radio down. */
static void fail_in_flight(mr_core_t *c)
{
    for (size_t i = 0; i < MR_PENDING_MAX; i++) {
        mr_pending_t *p = &c->pending[i];
        if (!driver_kind(p->kind))
            continue;
        if (p->sent_reported)
            complete_driver_pending(c, p, ST(NO_ACK), RS(REASON_NONE));
        else
            complete_driver_pending(c, p, ST(REFUSED), RS(RADIO_DOWN));
    }
}

void mr_core_driver_down(mr_core_t *c, const char *reason, uint64_t now_ms)
{
    if (!c->fatal)
        snprintf(c->reason, sizeof c->reason, "%s", reason);
    c->driver_up = false;
    fail_in_flight(c);
    mr_core_status_all(c, now_ms);
}

/* ---- time ---- */

void mr_core_tick(mr_core_t *c, uint64_t now_ms)
{
    for (size_t i = 0; i < MR_PENDING_MAX; i++) {
        mr_pending_t *p = &c->pending[i];
        if (!driver_kind(p->kind))
            continue;
        if (!p->deadline_ms)
            p->deadline_ms = now_ms + MR_ACK_TIMEOUT_MS;   /* bounds a lost driver result */
        else if (now_ms >= p->deadline_ms) {
            if (p->sent_reported)
                complete_driver_pending(c, p, ST(NO_ACK), RS(REASON_NONE));
            else
                complete_driver_pending(c, p, ST(REFUSED), RS(RADIO_DOWN));
        }
    }
    if (c->cfg->has_bridge)
        mr_core_bridge_tick(c, now_ms);
    if (mr_core_up(c, now_ms) != c->last_up || mr_core_driver_mtu(c) != c->last_mtu)
        mr_core_status_all(c, now_ms);
}
