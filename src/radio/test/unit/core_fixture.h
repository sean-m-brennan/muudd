/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Drives an mr_core_t in tests: records everything it emits, and builds the
 * events it consumes. Recorded messages are packed and unpacked, so tests
 * see exactly what would cross a socket. */
#ifndef CORE_FIXTURE_H
#define CORE_FIXTURE_H

#include <sodium.h>
#include <stdlib.h>
#include <string.h>

#include "mr_core.h"
#include "mr_crypto.h"
#include "mr_test.h"

#define ST(x) MUUDD__RADIO__SEND_STATUS__##x
#define RS(x) MUUDD__RADIO__REFUSE_REASON__##x
#define R2C(x) MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_##x
#define R2D(x) MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_##x
#define RADIOD MUUDD__RADIO__POLICY__RADIOD_BRIDGED
#define CONSUMER MUUDD__RADIO__POLICY__CONSUMER_BRIDGED
#define K1 "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
#define K2 "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI="
#define K3 "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM="
#define FX_MAX 512
#define BCAST MR_BROADCAST

typedef struct { int cid; MrR2C *m; } fx_cmsg_t;
typedef struct { int peer; MrDatagram *d; } fx_pmsg_t;

typedef struct {
    mr_config_t cfg;
    mr_keypair_t kp;
    mr_core_t *core;
    fx_cmsg_t c[FX_MAX];
    size_t nc;
    MrR2D *d[FX_MAX];
    size_t nd;
    fx_pmsg_t p[FX_MAX];
    size_t np;
    uint64_t wall_ms;
} fx_t;

static inline void fx_to_consumer(void *ctx, int cid, const MrR2C *m)
{
    fx_t *f = ctx;
    static uint8_t buf[MR_FRAME_MAX];
    ck_assert_uint_lt(f->nc, FX_MAX);
    size_t n = muudd__radio__radiod_to_consumer__pack(m, buf);
    f->c[f->nc].cid = cid;
    f->c[f->nc].m = muudd__radio__radiod_to_consumer__unpack(NULL, n, buf);
    f->nc++;
}

static inline void fx_to_driver(void *ctx, const MrR2D *m)
{
    fx_t *f = ctx;
    static uint8_t buf[MR_FRAME_MAX];
    ck_assert_uint_lt(f->nd, FX_MAX);
    size_t n = muudd__radio__radiod_to_driver__pack(m, buf);
    f->d[f->nd++] = muudd__radio__radiod_to_driver__unpack(NULL, n, buf);
}

static inline void fx_to_peer(void *ctx, int peer, const MrDatagram *m)
{
    fx_t *f = ctx;
    static uint8_t buf[MR_FRAME_MAX];
    ck_assert_uint_lt(f->np, FX_MAX);
    size_t n = muudd__radio__bridge_datagram__pack(m, buf);
    f->p[f->np].peer = peer;
    f->p[f->np].d = muudd__radio__bridge_datagram__unpack(NULL, n, buf);
    f->np++;
}

static inline uint64_t fx_wall(void *ctx) { return ((fx_t *)ctx)->wall_ms; }

static inline fx_t *fx_new(const char *json, const uint8_t *relay_key)
{
    fx_t *f = calloc(1, sizeof *f);
    char err[256] = "";
    ck_assert_msg(mr_config_parse(json, &f->cfg, err, sizeof err) == MR_OK, "%s", err);
    crypto_box_keypair(f->kp.pk, f->kp.sk);
    f->wall_ms = 1700000000000ull;
    mr_core_io_t io = {f, fx_to_consumer, fx_to_driver, fx_to_peer, fx_wall};
    ck_assert_int_eq(mr_core_create(&f->cfg, &f->kp, relay_key, &io, &f->core), MR_OK);
    return f;
}

static inline void fx_clear(fx_t *f)
{
    for (size_t i = 0; i < f->nc; i++)
        muudd__radio__radiod_to_consumer__free_unpacked(f->c[i].m, NULL);
    for (size_t i = 0; i < f->nd; i++)
        muudd__radio__radiod_to_driver__free_unpacked(f->d[i], NULL);
    for (size_t i = 0; i < f->np; i++)
        muudd__radio__bridge_datagram__free_unpacked(f->p[i].d, NULL);
    f->nc = f->nd = f->np = 0;
}

static inline void fx_free(fx_t *f)
{
    fx_clear(f);
    mr_core_free(f->core);
    free(f);
}

static inline void fx_hello_full(fx_t *f, uint32_t node, uint32_t mtu, uint32_t bw_khz, const char *fatal,
                                 uint64_t now, int expect)
{
    Muudd__Radio__LoraParams lp = MUUDD__RADIO__LORA_PARAMS__INIT;
    lp.spreading_factor = 7;
    lp.bandwidth_hz = 500000;
    lp.coding_rate = 1;
    lp.preamble_symbols = 16;
    Muudd__Radio__RadioSettings rs = MUUDD__RADIO__RADIO_SETTINGS__INIT;
    rs.region = (char *)"US";
    rs.preset = (char *)"STUB";
    rs.bandwidth_khz = bw_khz;
    MrCaps caps = MUUDD__RADIO__CAPS__INIT;
    caps.name = (char *)"stub";
    caps.version = (char *)"1";
    caps.reach = MUUDD__RADIO__REACH__MESH;
    caps.mtu = mtu;
    caps.lora = &lp;
    caps.reports_link_quality = true;
    caps.supports_ack = true;
    caps.node_addr = node;
    caps.radio_settings = &rs;
    caps.fatal = (char *)(fatal ? fatal : "");
    ck_assert_int_eq(mr_core_driver_hello(f->core, &caps, now), expect);
}

static inline void fx_hello(fx_t *f, uint32_t node, uint64_t now)
{
    fx_hello_full(f, node, 233, 500, NULL, now, MR_OK);
}

static inline const MrR2C *fx_last_c(fx_t *f, int cid, int msg_case)
{
    for (size_t i = f->nc; i-- > 0;)
        if (f->c[i].cid == cid && (int)f->c[i].m->msg_case == msg_case)
            return f->c[i].m;
    return NULL;
}

static inline const MrClaimResult *fx_claim(fx_t *f, int cid, uint32_t port, int policy, uint64_t now)
{
    Muudd__Radio__Claim cl = MUUDD__RADIO__CLAIM__INIT;
    cl.req_id = 9000 + port;
    cl.port = port;
    cl.policy = policy;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM;
    m.claim = &cl;
    mr_core_consumer_msg(f->core, cid, &m, now);
    const MrR2C *r = fx_last_c(f, cid, R2C(CLAIM_RESULT));
    ck_assert_ptr_nonnull(r);
    return r->claim_result;
}

static inline void fx_send(fx_t *f, int cid, uint32_t req, uint32_t dest, uint32_t port, const void *data,
                           size_t n, bool ack, uint64_t now)
{
    MrSend s = MUUDD__RADIO__SEND__INIT;
    s.req_id = req;
    s.dest = dest;
    s.port = port;
    s.payload.data = (uint8_t *)data;
    s.payload.len = n;
    s.want_ack = ack;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND;
    m.send = &s;
    mr_core_consumer_msg(f->core, cid, &m, now);
}

static inline const MrStats *fx_stats(fx_t *f, int cid, uint64_t now)
{
    Muudd__Radio__GetStats g = MUUDD__RADIO__GET_STATS__INIT;
    g.req_id = 7777;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_GET_STATS;
    m.get_stats = &g;
    mr_core_consumer_msg(f->core, cid, &m, now);
    const MrR2C *r = fx_last_c(f, cid, R2C(STATS));
    ck_assert_ptr_nonnull(r);
    return r->stats;
}

static inline void fx_driver_result(fx_t *f, uint32_t dreq, int status, int reason, uint32_t pid, uint64_t now)
{
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = dreq;
    r.status = status;
    r.reason = reason;
    r.packet_id = pid;
    MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_SEND_RESULT;
    m.send_result = &r;
    mr_core_driver_msg(f->core, &m, now);
}

static inline void fx_received(fx_t *f, uint32_t src, uint32_t dest, uint32_t port, const void *data, size_t n,
                               uint32_t pid, uint64_t now)
{
    MrMeta meta = MUUDD__RADIO__META__INIT;
    meta.rssi_dbm = -80;
    meta.snr_db = 5.0f;
    meta.has_link_quality = true;
    meta.packet_id = pid;
    meta.protection = MUUDD__RADIO__PROTECTION__CHANNEL_KEY;
    MrReceived rv = MUUDD__RADIO__RECEIVED__INIT;
    rv.src = src;
    rv.dest = dest;
    rv.port = port;
    rv.payload.data = (uint8_t *)data;
    rv.payload.len = n;
    rv.meta = &meta;
    MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_RECEIVED;
    m.received = &rv;
    mr_core_driver_msg(f->core, &m, now);
}

/* nth (0-based) SendResult sent to cid for consumer request req */
static inline const MrSendResult *fx_result(fx_t *f, int cid, uint32_t req, size_t nth)
{
    for (size_t i = 0; i < f->nc; i++) {
        const MrR2C *m = f->c[i].m;
        if (f->c[i].cid == cid && m->msg_case == R2C(SEND_RESULT) && m->send_result->req_id == req && nth-- == 0)
            return m->send_result;
    }
    return NULL;
}

static inline size_t fx_count_results(fx_t *f, int cid, uint32_t req)
{
    size_t n = 0;
    while (fx_result(f, cid, req, n))
        n++;
    return n;
}

static inline const MrReceived *fx_rx(fx_t *f, int cid, size_t nth)
{
    for (size_t i = 0; i < f->nc; i++)
        if (f->c[i].cid == cid && f->c[i].m->msg_case == R2C(RECEIVED) && nth-- == 0)
            return f->c[i].m->received;
    return NULL;
}

static inline size_t fx_count_rx(fx_t *f, int cid)
{
    size_t n = 0;
    while (fx_rx(f, cid, n))
        n++;
    return n;
}

static inline const MrRadioStatus *fx_last_status(fx_t *f, int cid)
{
    const MrR2C *m = fx_last_c(f, cid, R2C(RADIO_STATUS));
    return m ? m->radio_status : NULL;
}

static inline const MrSend *fx_dsend(fx_t *f, size_t nth)
{
    for (size_t i = 0; i < f->nd; i++)
        if (f->d[i]->msg_case == R2D(SEND) && nth-- == 0)
            return f->d[i]->send;
    return NULL;
}

static inline size_t fx_count_dsend(fx_t *f)
{
    size_t n = 0;
    while (fx_dsend(f, n))
        n++;
    return n;
}

static inline const Muudd__Radio__Configure *fx_configure(fx_t *f)
{
    for (size_t i = 0; i < f->nd; i++)
        if (f->d[i]->msg_case == R2D(CONFIGURE))
            return f->d[i]->configure;
    return NULL;
}

/* ---- bridge helpers ---- */

static inline void fx_keepalive(fx_t *f, int peer, uint32_t node, uint32_t radio, const char *region, bool gw,
                                uint64_t seq, uint64_t now)
{
    MrKeepalive k = MUUDD__RADIO__KEEPALIVE__INIT;
    k.node_addr = node;
    k.radio_node_addr = radio;
    k.region = (char *)region;
    k.gateway = gw;
    k.radio_up = gw;
    k.boot_id = 1000;
    k.seq = seq;
    k.radio_mtu = gw ? 233 : 0;
    MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
    d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_KEEPALIVE;
    d.keepalive = &k;
    mr_core_bridge_datagram(f->core, peer, &d, now);
}

static inline void fx_frame_full(fx_t *f, int peer, int kind, uint32_t fnode, uint32_t fid, uint32_t src,
                                 uint32_t dest, uint32_t port, const void *data, size_t n, const char *origin,
                                 bool want_ack, uint32_t req, bool relay, uint64_t now)
{
    Muudd__Radio__FrameId id = MUUDD__RADIO__FRAME_ID__INIT;
    id.node = fnode;
    id.id = fid;
    MrBridgeFrame fr = MUUDD__RADIO__BRIDGE_FRAME__INIT;
    fr.frame_id = &id;
    fr.kind = kind;
    fr.src = src;
    fr.dest = dest;
    fr.port = port;
    fr.payload.data = (uint8_t *)data;
    fr.payload.len = n;
    fr.origin_region = (char *)(origin ? origin : "");
    fr.want_ack = want_ack;
    fr.req_id = req;
    fr.relay = relay;
    MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
    d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME;
    d.frame = &fr;
    mr_core_bridge_datagram(f->core, peer, &d, now);
}

#define KIND(x) MUUDD__RADIO__BRIDGE_KIND__##x

static inline void fx_deliver(fx_t *f, int peer, uint32_t src, uint32_t fid, uint32_t dest, uint32_t port,
                              const char *payload, const char *origin, uint64_t now)
{
    fx_frame_full(f, peer, KIND(DELIVER), src, fid, src, dest, port, payload, strlen(payload), origin, false, 0,
                  false, now);
}

static inline void fx_result_frame(fx_t *f, int peer, uint32_t req, int status, int reason, uint64_t now)
{
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = req;
    r.status = status;
    r.reason = reason;
    MrBridgeFrame fr = MUUDD__RADIO__BRIDGE_FRAME__INIT;
    fr.kind = KIND(RESULT);
    fr.req_id = req;
    fr.result = &r;
    MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
    d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME;
    d.frame = &fr;
    mr_core_bridge_datagram(f->core, peer, &d, now);
}

/* nth frame of the given kind sent to peer (peer -1 = any peer) */
static inline const MrBridgeFrame *fx_pframe(fx_t *f, int peer, int kind, size_t nth)
{
    for (size_t i = 0; i < f->np; i++) {
        const MrDatagram *d = f->p[i].d;
        if (d->msg_case != MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME)
            continue;
        if ((peer < 0 || f->p[i].peer == peer) && (int)d->frame->kind == kind && nth-- == 0)
            return d->frame;
    }
    return NULL;
}

static inline size_t fx_count_pframes(fx_t *f, int peer, int kind)
{
    size_t n = 0;
    while (fx_pframe(f, peer, kind, n))
        n++;
    return n;
}

static inline const MrKeepalive *fx_keepalive_sent(fx_t *f, int peer, size_t nth)
{
    for (size_t i = 0; i < f->np; i++) {
        const MrDatagram *d = f->p[i].d;
        if (f->p[i].peer == peer && d->msg_case == MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_KEEPALIVE && nth-- == 0)
            return d->keepalive;
    }
    return NULL;
}

#endif
