/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Internals shared by core.c, core_bridge.c and core_relay.c. */
#ifndef MR_CORE_PRIV_H
#define MR_CORE_PRIV_H

#include "mr_airtime.h"
#include "mr_core.h"
#include "mr_dedupe.h"
#include "mr_ledger.h"
#include "mr_ports.h"

#define ST(x) MUUDD__RADIO__SEND_STATUS__##x
#define RS(x) MUUDD__RADIO__REFUSE_REASON__##x

#define MR_PENDING_MAX 1024
#define MR_AGG_TIMEOUT_MS 60000u
#define MR_ACK_TIMEOUT_MS 120000u

/* What an outstanding send is waiting for.
 *   LOCAL:     our radio's result, owed to a local consumer.
 *   REMOTE:    our radio's result, owed to the peer that sent TRANSMIT.
 *   LOCAL_AGG: our radio's result, feeding an aggregate (AGG).
 *   AGG:       results from several gateways, owed to a local consumer. */
typedef enum { PEND_FREE = 0, PEND_LOCAL, PEND_REMOTE, PEND_LOCAL_AGG, PEND_AGG } mr_pend_kind_t;

typedef struct {
    mr_pend_kind_t kind;
    uint32_t key;               /* driver req id, or aggregate id for AGG */
    int cid;
    uint32_t consumer_req;
    bool want_ack;
    int peer;
    uint32_t peer_req;
    uint32_t agg;               /* LOCAL_AGG: the aggregate it feeds */
    unsigned remaining;         /* AGG: gateways still to answer */
    bool sent_reported, acked_reported;
    int first_refusal;
    uint64_t deadline_ms;       /* 0 = none */
} mr_pending_t;

typedef struct {
    uint64_t unclaimed, queue_overflow, duplicate, bridge_auth, relay_auth, relay_window, relay_no_gateway,
        over_budget, relay_too_big;
} mr_drops_t;

struct mr_core {
    const mr_config_t *cfg;
    mr_keypair_t kp;
    uint8_t relay_key[32];
    bool has_relay_key;
    mr_core_io_t io;
    uint32_t node_addr;
    bool driver_up, fatal;
    char reason[160];
    MrCaps *caps;               /* deep copy of the last accepted Hello */
    mr_ports_t ports;
    mr_ledger_t *ledger;
    mr_dedupe_t dedupe;
    mr_dedupe_t relay_req_dedupe;   /* relay requests share frame IDs with the plain transmit */
    mr_peers_t *peers;
    mr_pending_t pending[MR_PENDING_MAX];
    uint32_t next_driver_req, next_seq, next_agg;
    uint64_t boot_id, ka_seq, last_ka_ms;
    bool ka_started;
    bool connected[MR_MAX_CONSUMERS];
    bool last_up;
    uint32_t last_mtu;
    mr_drops_t drops;
};

bool mr_core_is_gateway(const mr_core_t *c);
bool mr_core_up(const mr_core_t *c, uint64_t now_ms);
uint32_t mr_core_driver_mtu(const mr_core_t *c);
uint64_t mr_core_ttl(const mr_core_t *c, uint32_t port);
int mr_core_radio_transmit(mr_core_t *c, uint32_t dest, uint32_t port, const uint8_t *data, size_t n,
                           bool want_ack, uint32_t charge_port, bool relay, uint64_t now_ms, uint32_t *driver_req);
bool mr_core_deliver_local(mr_core_t *c, uint32_t port, uint32_t src, uint32_t dest, const uint8_t *data,
                           size_t n, const MrMeta *meta);
void mr_core_reply(mr_core_t *c, int cid, uint32_t req, int status, int reason, uint32_t packet_id);
mr_pending_t *mr_core_pending_new(mr_core_t *c, mr_pend_kind_t kind, uint32_t key);
mr_pending_t *mr_core_pending_find(mr_core_t *c, mr_pend_kind_t kind, uint32_t key);
void mr_core_pending_free(mr_pending_t *p);
void mr_core_status_all(mr_core_t *c, uint64_t now_ms);

/* core_bridge.c */
void mr_core_bridged_send(mr_core_t *c, int cid, const MrSend *s, uint64_t now_ms);
void mr_core_gateway_heard(mr_core_t *c, const MrReceived *r, uint64_t now_ms);
void mr_core_bridge_tick(mr_core_t *c, uint64_t now_ms);
void mr_core_agg_update(mr_core_t *c, mr_pending_t *agg, int status, int reason, uint32_t packet_id);
void mr_core_send_result_frame(mr_core_t *c, int peer, uint32_t req, int status, int reason, uint32_t packet_id);
void mr_core_fill_frame(MrBridgeFrame *f, Muudd__Radio__FrameId *fid, mr_frame_id_t id, int kind, uint32_t src,
                        uint32_t dest, uint32_t port, const uint8_t *data, size_t n);
void mr_core_send_frame(mr_core_t *c, int peer, MrBridgeFrame *f);
unsigned mr_core_fanout(mr_core_t *c, MrBridgeFrame *f, const bool *skip, uint64_t now_ms, bool *any_dead);

/* core_relay.c */
void mr_core_relay_send(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                        const uint8_t *data, size_t n, uint64_t now_ms);
int mr_core_relay_transmit_local(mr_core_t *c, uint32_t src, uint32_t id, uint32_t dest, uint32_t port,
                                 const uint8_t *data, size_t n, uint64_t now_ms);
void mr_core_relay_received(mr_core_t *c, const MrReceived *r, uint64_t now_ms);

#endif
