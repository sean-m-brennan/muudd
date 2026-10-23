/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The bridge's UDP socket: seals datagrams to peers and opens datagrams
 * from them. A datagram whose sender key is not a configured peer, or that
 * fails to open, is dropped and counted. Peer host names are resolved at
 * start and retried every 60 s while unresolved; resolution blocks, so
 * numeric addresses are preferable. Drops are logged at most once a minute
 * per cause. */
#ifndef MR_BRIDGE_IO_H
#define MR_BRIDGE_IO_H

#include <sys/socket.h>

#include "mr_core.h"

typedef struct {
    uint64_t last_ms;
    uint64_t pending;
    bool logged;
} mr_bridge_drop_log_t;

typedef struct {
    int fd;
    int family;
    mr_core_t *core;
    const mr_keypair_t *kp;
    mr_peers_t *peers;
    struct sockaddr_storage sa[MR_MAX_PEERS];
    socklen_t salen[MR_MAX_PEERS];
    uint64_t last_resolve_ms;
    mr_bridge_drop_log_t unknown_key, bad_seal; /* rate-limited drop logging */
} mr_bridge_io_t;

int mr_split_hostport(const char *s, char *host, size_t hcap, char *port, size_t pcap);
int mr_bridge_io_open(mr_bridge_io_t *b, const char *listen, const mr_keypair_t *kp, mr_core_t *core);
void mr_bridge_io_send(mr_bridge_io_t *b, int peer, const MrDatagram *d);
void mr_bridge_io_handle(mr_bridge_io_t *b, uint64_t now_ms);
void mr_bridge_io_tick(mr_bridge_io_t *b, uint64_t now_ms);
void mr_bridge_io_close(mr_bridge_io_t *b);

#endif
