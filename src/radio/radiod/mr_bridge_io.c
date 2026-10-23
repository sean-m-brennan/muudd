/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <netdb.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "mr_bridge_io.h"

#define RESOLVE_RETRY_MS 60000u
#define DROP_LOG_MS 60000u

int mr_split_hostport(const char *s, char *host, size_t hcap, char *port, size_t pcap)
{
    const char *colon = strrchr(s, ':');
    if (!colon || !colon[1])
        return MR_EINVAL;
    const char *h = s;
    size_t hl = (size_t)(colon - s);
    if (hl >= 2 && s[0] == '[' && s[hl - 1] == ']') {
        h = s + 1;
        hl -= 2;
    }
    size_t pl = strlen(colon + 1);
    if (hl >= hcap || pl >= pcap)
        return MR_EINVAL;
    memcpy(host, h, hl);
    host[hl] = 0;
    memcpy(port, colon + 1, pl + 1);
    return MR_OK;
}

static void resolve_peers(mr_bridge_io_t *b)
{
    for (size_t i = 0; i < b->peers->n; i++) {
        if (b->salen[i])
            continue;
        char host[128], port[16];
        if (mr_split_hostport(b->peers->peers[i].cfg->addr, host, sizeof host, port, sizeof port) != MR_OK)
            continue;
        struct addrinfo hints = {0}, *res = NULL;
        hints.ai_family = b->family;
        hints.ai_socktype = SOCK_DGRAM;
        hints.ai_flags = AI_NUMERICSERV | (b->family == AF_INET6 ? AI_V4MAPPED : 0);
        int gai = getaddrinfo(host, port, &hints, &res);
        if (gai != 0) {
            fprintf(stderr, "radiod: bridge: cannot resolve peer %s: %s\n", b->peers->peers[i].cfg->addr,
                    gai_strerror(gai));
            continue;
        }
        memcpy(&b->sa[i], res->ai_addr, res->ai_addrlen);
        b->salen[i] = res->ai_addrlen;
        freeaddrinfo(res);
    }
}

int mr_bridge_io_open(mr_bridge_io_t *b, const char *listen, const mr_keypair_t *kp, mr_core_t *core)
{
    memset(b, 0, sizeof *b);
    b->fd = -1;
    b->kp = kp;
    b->core = core;
    b->peers = mr_core_peers(core);
    char host[128], port[16];
    if (mr_split_hostport(listen, host, sizeof host, port, sizeof port) != MR_OK)
        return MR_EINVAL;
    struct addrinfo hints = {0}, *res = NULL;
    hints.ai_socktype = SOCK_DGRAM;
    hints.ai_flags = AI_PASSIVE | AI_NUMERICSERV;
    if (getaddrinfo(host[0] ? host : NULL, port, &hints, &res) != 0)
        return MR_EINVAL;
    int fd = socket(res->ai_family, SOCK_DGRAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd >= 0 && res->ai_family == AF_INET6) {
        int off = 0;
        setsockopt(fd, IPPROTO_IPV6, IPV6_V6ONLY, &off, sizeof off);
    }
    if (fd < 0 || bind(fd, res->ai_addr, res->ai_addrlen) != 0) {
        if (fd >= 0)
            close(fd);
        freeaddrinfo(res);
        return MR_EIO;
    }
    b->family = res->ai_family;
    freeaddrinfo(res);
    b->fd = fd;
    resolve_peers(b);
    return MR_OK;
}

void mr_bridge_io_send(mr_bridge_io_t *b, int peer, const MrDatagram *d)
{
    if (peer < 0 || (size_t)peer >= b->peers->n || !b->salen[peer])
        return;
    uint8_t plain[MR_FRAME_MAX], sealed[MR_FRAME_MAX + MR_BRIDGE_OVERHEAD];
    size_t n = muudd__radio__bridge_datagram__get_packed_size(d);
    if (n > sizeof plain)
        return;
    muudd__radio__bridge_datagram__pack(d, plain);
    int len = mr_bridge_seal(b->kp, b->peers->peers[peer].cfg->pubkey, plain, n, sealed, sizeof sealed);
    if (len > 0)
        sendto(b->fd, sealed, (size_t)len, MSG_DONTWAIT, (struct sockaddr *)&b->sa[peer], b->salen[peer]);
}

static void note_drop(mr_bridge_drop_log_t *l, const char *cause, const struct sockaddr_storage *from,
                      socklen_t fromlen, uint64_t now_ms)
{
    l->pending++;
    if (l->logged && now_ms - l->last_ms < DROP_LOG_MS)
        return;
    char host[64] = "?", port[16] = "?";
    getnameinfo((const struct sockaddr *)from, fromlen, host, sizeof host, port, sizeof port,
                NI_NUMERICHOST | NI_NUMERICSERV);
    fprintf(stderr, "radiod: bridge: dropped datagram from %s:%s: %s (%llu since last report)\n", host, port, cause,
            (unsigned long long)l->pending);
    l->logged = true;
    l->last_ms = now_ms;
    l->pending = 0;
}

void mr_bridge_io_handle(mr_bridge_io_t *b, uint64_t now_ms)
{
    uint8_t buf[MR_FRAME_MAX + MR_BRIDGE_OVERHEAD], plain[MR_FRAME_MAX];
    for (;;) {
        struct sockaddr_storage from;
        socklen_t fromlen = sizeof from;
        ssize_t n = recvfrom(b->fd, buf, sizeof buf, MSG_DONTWAIT, (struct sockaddr *)&from, &fromlen);
        if (n < 0)
            return;
        const uint8_t *pk = mr_bridge_sender(buf, (size_t)n);
        int peer = pk ? mr_peers_find_pk(b->peers, pk) : -1;
        if (peer < 0) {
            note_drop(&b->unknown_key, "unknown sender key", &from, fromlen, now_ms);
            mr_core_note_bridge_auth_failure(b->core);
            continue;
        }
        int m = mr_bridge_open(b->kp, pk, buf, (size_t)n, plain, sizeof plain);
        if (m < 0) {
            note_drop(&b->bad_seal, "failed to open", &from, fromlen, now_ms);
            mr_core_note_bridge_auth_failure(b->core);
            continue;
        }
        MrDatagram *d = muudd__radio__bridge_datagram__unpack(NULL, (size_t)m, plain);
        if (!d)
            continue;
        if (mr_core_bridge_datagram(b->core, peer, d, now_ms) == MR_EAUTH)
            note_drop(&b->replay, "stale or replayed frame", &from, fromlen, now_ms);
        muudd__radio__bridge_datagram__free_unpacked(d, NULL);
    }
}

void mr_bridge_io_tick(mr_bridge_io_t *b, uint64_t now_ms)
{
    if (now_ms - b->last_resolve_ms < RESOLVE_RETRY_MS)
        return;
    b->last_resolve_ms = now_ms;
    resolve_peers(b);
}

void mr_bridge_io_close(mr_bridge_io_t *b)
{
    if (b->fd >= 0) {
        close(b->fd);
        b->fd = -1;
    }
}
