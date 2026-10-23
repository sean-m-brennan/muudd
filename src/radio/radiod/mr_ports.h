/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Which consumer holds which port, and how much payload each port can carry. */
#ifndef MR_PORTS_H
#define MR_PORTS_H

#include "mr_config.h"

typedef struct { int claimant[MR_NUM_PORTS]; } mr_ports_t;

void mr_ports_init(mr_ports_t *p);
int mr_ports_claim(mr_ports_t *p, const mr_config_t *cfg, int cid, uint32_t port, int policy);
bool mr_ports_release(mr_ports_t *p, int cid, uint32_t port);
void mr_ports_release_all(mr_ports_t *p, int cid);
int mr_ports_claimant(const mr_ports_t *p, uint32_t port);
uint32_t mr_ports_mtu(const mr_config_t *cfg, uint32_t port, uint32_t driver_mtu);

#endif
