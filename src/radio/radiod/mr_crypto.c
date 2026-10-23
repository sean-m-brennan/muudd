/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <fcntl.h>
#include <sodium.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "mr_crypto.h"

/* Read a 32-byte secret, refusing links and files others can reach. */
static int read_key_file(const char *path, uint8_t out[32])
{
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0)
        return MR_EIO;
    struct stat st;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode)) {
        close(fd);
        errno = EINVAL;
        return MR_EIO;
    }
    if (st.st_mode & 077) {
        close(fd);
        errno = EACCES;
        return MR_EPERM;
    }
    uint8_t buf[33];
    ssize_t n = read(fd, buf, sizeof buf);
    close(fd);
    if (n != 32) {
        sodium_memzero(buf, sizeof buf);
        return MR_EINVAL;
    }
    memcpy(out, buf, 32);
    sodium_memzero(buf, sizeof buf);
    return MR_OK;
}

int mr_keypair_load_or_create(const char *path, mr_keypair_t *kp)
{
    if (sodium_init() < 0)
        return MR_ERR;
    int rc = read_key_file(path, kp->sk);
    if (rc == MR_EIO && errno == ENOENT) {
        randombytes_buf(kp->sk, 32);
        int fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
        if (fd < 0)
            return MR_EIO;
        ssize_t n = write(fd, kp->sk, 32);
        int sync_rc = fsync(fd);
        close(fd);
        if (n != 32 || sync_rc != 0) {
            /* A partial key file would fail every later start. */
            unlink(path);
            sodium_memzero(kp->sk, 32);
            return MR_EIO;
        }
    } else if (rc != MR_OK) {
        return rc;
    }
    crypto_scalarmult_base(kp->pk, kp->sk);
    return MR_OK;
}

int mr_relay_key_load(const char *path, uint8_t key[32])
{
    return read_key_file(path, key);
}

int mr_bridge_seal(const mr_keypair_t *self, const uint8_t peer_pk[32], const uint8_t *msg, size_t n,
                   uint8_t *out, size_t cap)
{
    size_t total = MR_BRIDGE_OVERHEAD + n;
    if (total > cap)
        return MR_EINVAL;
    memcpy(out, self->pk, 32);
    uint8_t *nonce = out + 32;
    randombytes_buf(nonce, crypto_box_NONCEBYTES);
    if (crypto_box_easy(out + 32 + crypto_box_NONCEBYTES, msg, n, nonce, peer_pk, self->sk) != 0)
        return MR_ERR;
    return (int)total;
}

const uint8_t *mr_bridge_sender(const uint8_t *dgram, size_t n)
{
    return n >= MR_BRIDGE_OVERHEAD ? dgram : NULL;
}

int mr_bridge_open(const mr_keypair_t *self, const uint8_t peer_pk[32], const uint8_t *dgram, size_t n,
                   uint8_t *out, size_t cap)
{
    if (n < MR_BRIDGE_OVERHEAD || n - MR_BRIDGE_OVERHEAD > cap)
        return MR_EINVAL;
    const uint8_t *nonce = dgram + 32;
    const uint8_t *c = nonce + crypto_box_NONCEBYTES;
    size_t clen = n - 32 - crypto_box_NONCEBYTES;
    if (crypto_box_open_easy(out, c, clen, nonce, peer_pk, self->sk) != 0)
        return MR_EAUTH;
    return (int)(clen - crypto_box_MACBYTES);
}

static void put32(uint8_t *p, uint32_t v)
{
    p[0] = (uint8_t)(v >> 24);
    p[1] = (uint8_t)(v >> 16);
    p[2] = (uint8_t)(v >> 8);
    p[3] = (uint8_t)v;
}

static uint32_t get32(const uint8_t *p)
{
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) | ((uint32_t)p[2] << 8) | p[3];
}

int mr_relay_seal(const uint8_t key[32], const mr_relay_hdr_t *h, const uint8_t *payload, size_t n,
                  uint8_t *out, size_t cap)
{
    size_t total = MR_RELAY_OVERHEAD + n;
    if (total > cap || n > MR_FRAME_MAX)
        return MR_EINVAL;
    uint8_t plain[MR_RELAY_HDR + MR_FRAME_MAX];
    put32(plain, h->src);
    put32(plain + 4, h->dest);
    plain[8] = h->port;
    put32(plain + 9, h->id);
    put32(plain + 13, h->ts);
    if (n)
        memcpy(plain + MR_RELAY_HDR, payload, n);
    randombytes_buf(out, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES);
    unsigned long long clen = 0;
    crypto_aead_xchacha20poly1305_ietf_encrypt(out + 24, &clen, plain, MR_RELAY_HDR + n, NULL, 0, NULL, out, key);
    return (int)(24 + clen);
}

int mr_relay_open(const uint8_t key[32], const uint8_t *in, size_t n, mr_relay_hdr_t *h, uint8_t *payload,
                  size_t cap, size_t *payload_len)
{
    if (n < MR_RELAY_OVERHEAD || n > MR_RELAY_OVERHEAD + MR_FRAME_MAX)
        return MR_EINVAL;
    uint8_t plain[MR_RELAY_HDR + MR_FRAME_MAX];
    unsigned long long plen = 0;
    if (crypto_aead_xchacha20poly1305_ietf_decrypt(plain, &plen, NULL, in + 24, n - 24, NULL, 0, in, key) != 0)
        return MR_EAUTH;
    size_t body = (size_t)plen - MR_RELAY_HDR;
    if (body > cap)
        return MR_EINVAL;
    h->src = get32(plain);
    h->dest = get32(plain + 4);
    h->port = plain[8];
    h->id = get32(plain + 9);
    h->ts = get32(plain + 13);
    memcpy(payload, plain + MR_RELAY_HDR, body);
    *payload_len = body;
    return MR_OK;
}

bool mr_relay_window_ok(uint32_t ts, uint64_t wall_ms, uint32_t window_s)
{
    if (window_s == 0)
        return true;
    int64_t d = (int64_t)(wall_ms / 1000u) - (int64_t)ts;
    if (d < 0)
        d = -d;
    return d <= (int64_t)window_s;
}
