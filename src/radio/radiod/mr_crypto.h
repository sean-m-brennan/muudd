/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The node's bridge keypair, sealed bridge datagrams (crypto_box, sender
 * authenticated), and relay frames over LoRa (XChaCha20-Poly1305 under the
 * bridge group's shared relay key). */
#ifndef MR_CRYPTO_H
#define MR_CRYPTO_H

#include "mr_common.h"

typedef struct { uint8_t pk[32], sk[32]; } mr_keypair_t;

#define MR_BRIDGE_OVERHEAD (32u + 24u + 16u)
#define MR_RELAY_HDR 17u

typedef struct {
    uint32_t src, dest, id, ts;
    uint8_t port;
} mr_relay_hdr_t;

/* Key files are opened without following symbolic links and refused with
 * MR_EPERM when group or others have any access (mode & 077); a wrong length
 * gives MR_EINVAL. */
int mr_keypair_load_or_create(const char *path, mr_keypair_t *kp);
int mr_relay_key_load(const char *path, uint8_t key[32]);
int mr_bridge_seal(const mr_keypair_t *self, const uint8_t peer_pk[32], const uint8_t *msg, size_t n,
                   uint8_t *out, size_t cap);
const uint8_t *mr_bridge_sender(const uint8_t *dgram, size_t n);
int mr_bridge_open(const mr_keypair_t *self, const uint8_t peer_pk[32], const uint8_t *dgram, size_t n,
                   uint8_t *out, size_t cap);
int mr_relay_seal(const uint8_t key[32], const mr_relay_hdr_t *h, const uint8_t *payload, size_t n,
                  uint8_t *out, size_t cap);
int mr_relay_open(const uint8_t key[32], const uint8_t *in, size_t n, mr_relay_hdr_t *h, uint8_t *payload,
                  size_t cap, size_t *payload_len);
bool mr_relay_window_ok(uint32_t ts, uint64_t wall_ms, uint32_t window_s);

#endif
