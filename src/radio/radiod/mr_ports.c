/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include "mr_ports.h"

void mr_ports_init(mr_ports_t *p)
{
    for (unsigned i = 0; i < MR_NUM_PORTS; i++)
        p->claimant[i] = -1;
}

int mr_ports_claim(mr_ports_t *p, const mr_config_t *cfg, int cid, uint32_t port, int policy)
{
    if (port >= MR_NUM_PORTS || !cfg->ports[port].configured)
        return MUUDD__RADIO__CLAIM_STATUS__PORT_UNCONFIGURED;
    if (cfg->ports[port].policy != policy)
        return MUUDD__RADIO__CLAIM_STATUS__POLICY_MISMATCH;
    if (p->claimant[port] != -1 && p->claimant[port] != cid)
        return MUUDD__RADIO__CLAIM_STATUS__PORT_TAKEN;
    p->claimant[port] = cid;
    return MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK;
}

bool mr_ports_release(mr_ports_t *p, int cid, uint32_t port)
{
    if (port >= MR_NUM_PORTS || p->claimant[port] != cid)
        return false;
    p->claimant[port] = -1;
    return true;
}

void mr_ports_release_all(mr_ports_t *p, int cid)
{
    for (unsigned i = 0; i < MR_NUM_PORTS; i++)
        if (p->claimant[i] == cid)
            p->claimant[i] = -1;
}

int mr_ports_claimant(const mr_ports_t *p, uint32_t port)
{
    return port < MR_NUM_PORTS ? p->claimant[port] : -1;
}

uint32_t mr_ports_mtu(const mr_config_t *cfg, uint32_t port, uint32_t driver_mtu)
{
    if (port >= MR_NUM_PORTS || !cfg->ports[port].configured)
        return 0;
    if (!cfg->ports[port].ip_fallback)
        return driver_mtu;
    return driver_mtu > MR_RELAY_OVERHEAD ? driver_mtu - MR_RELAY_OVERHEAD : 0;
}
