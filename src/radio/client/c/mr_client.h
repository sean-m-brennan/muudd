/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* A small blocking client for radiod's consumer socket. */
#ifndef MR_CLIENT_H
#define MR_CLIENT_H

#include "mr_common.h"
#include "mr_proto.h"

typedef struct mr_client mr_client_t;

int mr_client_connect(const char *path, mr_client_t **out);
void mr_client_close(mr_client_t *c);
int mr_client_send(mr_client_t *c, const MrC2R *msg);
MrR2C *mr_client_recv(mr_client_t *c, int timeout_ms);
int mr_client_claim(mr_client_t *c, uint32_t port, int policy, int timeout_ms);
int mr_client_reconnect(mr_client_t *c, int timeout_ms);
/* Messages discarded because the receive queue was full. */
uint64_t mr_client_dropped(const mr_client_t *c);
/* True while the socket to radiod is open; false once the client saw it fail
 * (a reconnect opens it again). */
bool mr_client_connected(const mr_client_t *c);
MrR2C *mr_client_get_stats(mr_client_t *c, int timeout_ms);

#endif
