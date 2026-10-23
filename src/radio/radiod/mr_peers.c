/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdio.h>
#include <string.h>

#include "mr_peers.h"

void mr_peers_init(mr_peers_t *ps, const mr_config_t *cfg)
{
    memset(ps, 0, sizeof *ps);
    ps->dead_after_ms = cfg->dead_after_ms;
    for (size_t i = 0; i < cfg->n_peers; i++) {
        if (cfg->peers[i].is_self)
            continue;
        ps->peers[ps->n++].cfg = &cfg->peers[i];
    }
}

int mr_peers_find_pk(const mr_peers_t *ps, const uint8_t pk[32])
{
    for (size_t i = 0; i < ps->n; i++)
        if (memcmp(ps->peers[i].cfg->pubkey, pk, 32) == 0)
            return (int)i;
    return -1;
}

bool mr_peers_accept_keepalive(mr_peers_t *ps, int idx, const MrKeepalive *ka, uint64_t now_ms)
{
    mr_peer_t *p = &ps->peers[idx];
    if (p->heard) {
        if (ka->boot_id < p->boot_id)
            return false;
        if (ka->boot_id == p->boot_id && ka->seq <= p->seq)
            return false;
    }
    p->heard = true;
    p->boot_id = ka->boot_id;
    p->seq = ka->seq;
    p->last_seen_ms = now_ms;
    p->node_addr = ka->node_addr;
    p->radio_node_addr = ka->radio_node_addr;
    if (ka->radio_mtu)
        p->radio_mtu = ka->radio_mtu;
    snprintf(p->region, sizeof p->region, "%s", ka->region ? ka->region : "");
    p->gateway = ka->gateway;
    p->radio_up = ka->radio_up;
    return true;
}

bool mr_peers_live(const mr_peers_t *ps, int idx, uint64_t now_ms)
{
    const mr_peer_t *p = &ps->peers[idx];
    return p->heard && now_ms - p->last_seen_ms <= ps->dead_after_ms;
}

int mr_peers_find_node(const mr_peers_t *ps, uint32_t node_addr)
{
    if (node_addr == 0)
        return -1;
    for (size_t i = 0; i < ps->n; i++)
        if (ps->peers[i].heard && ps->peers[i].node_addr == node_addr)
            return (int)i;
    return -1;
}

bool mr_peers_is_gateway_radio(const mr_peers_t *ps, uint32_t radio_addr)
{
    if (radio_addr == 0)
        return false;
    for (size_t i = 0; i < ps->n; i++)
        if (ps->peers[i].heard && ps->peers[i].gateway && ps->peers[i].radio_node_addr == radio_addr)
            return true;
    return false;
}

int mr_peers_best_gateway(const mr_peers_t *ps, const char *region, uint64_t now_ms)
{
    int best = -1;
    for (size_t i = 0; i < ps->n; i++) {
        const mr_peer_t *p = &ps->peers[i];
        if (!mr_peers_live(ps, (int)i, now_ms) || !p->gateway || !p->radio_up)
            continue;
        if (region && strcmp(region, p->region) != 0)
            continue;
        if (best < 0) {
            best = (int)i;
            continue;
        }
        const mr_peer_t *b = &ps->peers[best];
        if (p->cfg->priority < b->cfg->priority ||
            (p->cfg->priority == b->cfg->priority && p->node_addr < b->node_addr))
            best = (int)i;
    }
    return best;
}

size_t mr_peers_regions(const mr_peers_t *ps, uint64_t now_ms, const char *exclude, char out[][64], size_t max)
{
    size_t n = 0;
    for (size_t i = 0; i < ps->n && n < max; i++) {
        const mr_peer_t *p = &ps->peers[i];
        if (!mr_peers_live(ps, (int)i, now_ms) || !p->gateway || !p->radio_up)
            continue;
        if (exclude && strcmp(exclude, p->region) == 0)
            continue;
        bool dup = false;
        for (size_t j = 0; j < n; j++)
            if (strcmp(out[j], p->region) == 0)
                dup = true;
        if (!dup)
            snprintf(out[n++], 64, "%s", p->region);
    }
    return n;
}

bool mr_peers_any_dead(const mr_peers_t *ps, uint64_t now_ms)
{
    for (size_t i = 0; i < ps->n; i++)
        if (!mr_peers_live(ps, (int)i, now_ms))
            return true;
    return false;
}

uint32_t mr_peers_min_gateway_mtu(const mr_peers_t *ps)
{
    uint32_t m = 0;
    for (size_t i = 0; i < ps->n; i++) {
        const mr_peer_t *p = &ps->peers[i];
        if (p->heard && p->gateway && p->radio_mtu && (m == 0 || p->radio_mtu < m))
            m = p->radio_mtu;
    }
    return m;
}

void mr_peers_note_region(mr_peers_t *ps, uint32_t node, const char *region, uint64_t now_ms)
{
    size_t slot = ps->rmap_n;
    for (size_t i = 0; i < ps->rmap_n; i++)
        if (ps->rmap[i].node == node)
            slot = i;
    if (slot == ps->rmap_n) {
        if (ps->rmap_n < MR_REGION_MAP) {
            ps->rmap_n++;
        } else {
            slot = 0;
            for (size_t i = 1; i < ps->rmap_n; i++)
                if (ps->rmap[i].t < ps->rmap[slot].t)
                    slot = i;
        }
    }
    ps->rmap[slot].node = node;
    ps->rmap[slot].t = now_ms;
    snprintf(ps->rmap[slot].region, sizeof ps->rmap[slot].region, "%s", region);
}

const char *mr_peers_region_of(const mr_peers_t *ps, uint32_t node)
{
    for (size_t i = 0; i < ps->rmap_n; i++)
        if (ps->rmap[i].node == node)
            return ps->rmap[i].region;
    return NULL;
}
