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

/* How long a send waits for room in the socket before giving up, unless
 * mr_client_set_send_timeout says otherwise. */
#define MR_CLIENT_SEND_TIMEOUT_MS 5000

int mr_client_connect(const char *path, mr_client_t **out);
void mr_client_close(mr_client_t *c);
/* MR_EIO when the link is down, the write fails, or radiod leaves no room in
 * the socket for the send timeout; the last two close the link (errno is
 * ETIMEDOUT for a timeout). Claims, stats and reconnects send the same way. */
int mr_client_send(mr_client_t *c, const MrC2R *msg);
/* Sets the send timeout in milliseconds for this client: 0 never waits for
 * room, a negative value waits as long as it takes. 0 is not a non-blocking
 * mode with retries: the first send that finds the socket full fails and
 * closes the link, and its claims go with it until mr_client_reconnect claims
 * them again. */
void mr_client_set_send_timeout(mr_client_t *c, int timeout_ms);
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
