/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Bridge peers as learned from their keepalives: liveness, gateway choice,
 * and the region each LoRa source was last heard in. */
#ifndef MR_PEERS_H
#define MR_PEERS_H

#include "mr_config.h"

#define MR_REGION_MAP 1024

typedef struct {
    const mr_peer_cfg_t *cfg;
    bool heard;
    uint64_t last_seen_ms, boot_id, seq;
    uint32_t node_addr, radio_node_addr, radio_mtu;
    char region[64];
    bool gateway, radio_up;
} mr_peer_t;

typedef struct {
    mr_peer_t peers[MR_MAX_PEERS];
    size_t n;
    uint32_t dead_after_ms;
    struct { uint32_t node; char region[64]; uint64_t t; } rmap[MR_REGION_MAP];
    size_t rmap_n;
} mr_peers_t;

void mr_peers_init(mr_peers_t *ps, const mr_config_t *cfg);
int mr_peers_find_pk(const mr_peers_t *ps, const uint8_t pk[32]);
bool mr_peers_accept_keepalive(mr_peers_t *ps, int idx, const MrKeepalive *ka, uint64_t now_ms);
bool mr_peers_live(const mr_peers_t *ps, int idx, uint64_t now_ms);
int mr_peers_find_node(const mr_peers_t *ps, uint32_t node_addr);
/* True if radio_addr is the radio of a peer whose last keepalive said it is a gateway. */
bool mr_peers_is_gateway_radio(const mr_peers_t *ps, uint32_t radio_addr);
int mr_peers_best_gateway(const mr_peers_t *ps, const char *region, uint64_t now_ms);
size_t mr_peers_regions(const mr_peers_t *ps, uint64_t now_ms, const char *exclude, char out[][64], size_t max);
bool mr_peers_any_dead(const mr_peers_t *ps, uint64_t now_ms);
uint32_t mr_peers_min_gateway_mtu(const mr_peers_t *ps);
void mr_peers_note_region(mr_peers_t *ps, uint32_t node, const char *region, uint64_t now_ms);
const char *mr_peers_region_of(const mr_peers_t *ps, uint32_t node);

#endif
