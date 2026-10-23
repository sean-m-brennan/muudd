/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* radiod's routing core. It owns no sockets and reads no clocks: the caller
 * feeds it events with the current monotonic time, and it answers through
 * the callbacks in mr_core_io_t. */
#ifndef MR_CORE_H
#define MR_CORE_H

#include "mr_config.h"
#include "mr_crypto.h"
#include "mr_peers.h"
#include "mr_proto.h"

#define MR_MAX_CONSUMERS 64

typedef struct mr_core mr_core_t;

typedef struct {
    void *ctx;
    void (*to_consumer)(void *ctx, int cid, const MrR2C *msg);
    void (*to_driver)(void *ctx, const MrR2D *msg);
    void (*to_peer)(void *ctx, int peer, const MrDatagram *msg);
    uint64_t (*wall_ms)(void *ctx);
} mr_core_io_t;

int mr_core_create(const mr_config_t *cfg, const mr_keypair_t *kp, const uint8_t *relay_key,
                   const mr_core_io_t *io, mr_core_t **out);
void mr_core_free(mr_core_t *c);
mr_peers_t *mr_core_peers(mr_core_t *c);
uint32_t mr_core_node_addr(const mr_core_t *c);
/* boot_id carried in keepalives; must rise across restarts */
void mr_core_set_boot_id(mr_core_t *c, uint64_t boot_id);

void mr_core_consumer_connected(mr_core_t *c, int cid, uint64_t now_ms);
void mr_core_consumer_disconnected(mr_core_t *c, int cid);
void mr_core_consumer_msg(mr_core_t *c, int cid, const MrC2R *msg, uint64_t now_ms);

int mr_core_driver_hello(mr_core_t *c, const MrCaps *caps, uint64_t now_ms);
void mr_core_driver_msg(mr_core_t *c, const MrD2R *msg, uint64_t now_ms);
void mr_core_driver_down(mr_core_t *c, const char *reason, uint64_t now_ms);

void mr_core_bridge_datagram(mr_core_t *c, int peer, const MrDatagram *msg, uint64_t now_ms);
void mr_core_note_bridge_auth_failure(mr_core_t *c);
void mr_core_note_queue_overflow(mr_core_t *c, uint64_t n);

void mr_core_tick(mr_core_t *c, uint64_t now_ms);

#endif
