/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <sodium.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "mr_test.h"
#include "mr_crypto.h"
#include "mr_peers.h"

static void tmp_path(char *buf, size_t n, const char *name)
{
    snprintf(buf, n, "/tmp/mr_crypto_%d_%s", (int)getpid(), name);
    unlink(buf);
}

START_TEST(test_keypair_created_then_reloaded)
{
    char path[128];
    tmp_path(path, sizeof path, "key");
    mr_keypair_t a, b;
    ck_assert_int_eq(mr_keypair_load_or_create(path, &a), MR_OK);
    struct stat st;
    ck_assert_int_eq(stat(path, &st), 0);
    ck_assert_uint_eq(st.st_mode & 0777, 0600);
    ck_assert_int_eq(mr_keypair_load_or_create(path, &b), MR_OK);
    ck_assert_mem_eq(a.pk, b.pk, 32);
    ck_assert_mem_eq(a.sk, b.sk, 32);
    unlink(path);
}
END_TEST

static void write_n(const char *path, size_t n)
{
    FILE *f = fopen(path, "wb");
    for (size_t i = 0; i < n; i++) fputc((int)i, f);
    fclose(f);
    chmod(path, 0600);
}

START_TEST(test_keypair_rejects_wrong_length_file)
{
    char path[128];
    tmp_path(path, sizeof path, "badkey");
    mr_keypair_t k;
    write_n(path, 64);
    ck_assert_int_eq(mr_keypair_load_or_create(path, &k), MR_EINVAL);
    write_n(path, 10);
    ck_assert_int_eq(mr_keypair_load_or_create(path, &k), MR_EINVAL);
    unlink(path);
}
END_TEST

START_TEST(test_relay_key_must_be_32_bytes)
{
    char path[128];
    tmp_path(path, sizeof path, "rk");
    write_n(path, 5);
    uint8_t key[32];
    ck_assert_int_eq(mr_relay_key_load(path, key), MR_EINVAL);
    write_n(path, 32);
    ck_assert_int_eq(mr_relay_key_load(path, key), MR_OK);
    ck_assert_uint_eq(key[31], 31);
    unlink(path);
}
END_TEST

static void keypair(mr_keypair_t *k) { crypto_box_keypair(k->pk, k->sk); }

START_TEST(test_bridge_round_trip)
{
    mr_keypair_t a, b;
    keypair(&a);
    keypair(&b);
    uint8_t dgram[128], plain[64];
    int n = mr_bridge_seal(&a, b.pk, (const uint8_t *)"keepalive", 9, dgram, sizeof dgram);
    ck_assert_int_eq(n, (int)MR_BRIDGE_OVERHEAD + 9);
    ck_assert_mem_eq(mr_bridge_sender(dgram, (size_t)n), a.pk, 32);
    int m = mr_bridge_open(&b, a.pk, dgram, (size_t)n, plain, sizeof plain);
    ck_assert_int_eq(m, 9);
    ck_assert_mem_eq(plain, "keepalive", 9);
}
END_TEST

START_TEST(test_bridge_wrong_key_and_tamper)
{
    mr_keypair_t a, b, c;
    keypair(&a);
    keypair(&b);
    keypair(&c);
    uint8_t dgram[128], plain[64];
    int n = mr_bridge_seal(&a, b.pk, (const uint8_t *)"x", 1, dgram, sizeof dgram);
    ck_assert_int_eq(mr_bridge_open(&c, a.pk, dgram, (size_t)n, plain, sizeof plain), MR_EAUTH);
    dgram[n - 1] ^= 1;
    ck_assert_int_eq(mr_bridge_open(&b, a.pk, dgram, (size_t)n, plain, sizeof plain), MR_EAUTH);
    ck_assert_ptr_null(mr_bridge_sender(dgram, 10));
    ck_assert_int_eq(mr_bridge_open(&b, a.pk, dgram, 20, plain, sizeof plain), MR_EINVAL);
}
END_TEST

START_TEST(test_relay_round_trip_at_reduced_mtu)
{
    uint8_t key[32], out[233], payload[176], back[233];
    randombytes_buf(key, sizeof key);
    for (size_t i = 0; i < sizeof payload; i++) payload[i] = (uint8_t)i;
    mr_relay_hdr_t h = {.src = 0x0D01, .dest = 0xFFFFFFFF, .id = 77, .ts = 1700000000, .port = 18};
    int n = mr_relay_seal(key, &h, payload, sizeof payload, out, sizeof out);
    ck_assert_int_eq(n, 233);
    mr_relay_hdr_t g;
    size_t plen = 0;
    ck_assert_int_eq(mr_relay_open(key, out, (size_t)n, &g, back, sizeof back, &plen), MR_OK);
    ck_assert_uint_eq(plen, 176);
    ck_assert_mem_eq(back, payload, 176);
    ck_assert_uint_eq(g.src, 0x0D01);
    ck_assert_uint_eq(g.dest, 0xFFFFFFFF);
    ck_assert_uint_eq(g.id, 77);
    ck_assert_uint_eq(g.ts, 1700000000);
    ck_assert_uint_eq(g.port, 18);
    ck_assert_int_eq(mr_relay_seal(key, &h, payload, 177, out, sizeof out), MR_EINVAL);
}
END_TEST

START_TEST(test_relay_wrong_key_and_tamper)
{
    uint8_t key[32], other[32], out[128], back[128];
    randombytes_buf(key, sizeof key);
    randombytes_buf(other, sizeof other);
    mr_relay_hdr_t h = {.src = 1, .dest = 2, .id = 3, .ts = 4, .port = 18};
    int n = mr_relay_seal(key, &h, (const uint8_t *)"hi", 2, out, sizeof out);
    mr_relay_hdr_t g;
    size_t plen;
    ck_assert_int_eq(mr_relay_open(other, out, (size_t)n, &g, back, sizeof back, &plen), MR_EAUTH);
    out[30] ^= 0x40;
    ck_assert_int_eq(mr_relay_open(key, out, (size_t)n, &g, back, sizeof back, &plen), MR_EAUTH);
    ck_assert_int_eq(mr_relay_open(key, out, 56, &g, back, sizeof back, &plen), MR_EINVAL);
}
END_TEST

START_TEST(test_relay_window)
{
    uint64_t now = 1700000000000ull;
    ck_assert(mr_relay_window_ok(1700000000, now, 600));
    ck_assert(mr_relay_window_ok(1700000000 - 600, now, 600));
    ck_assert(!mr_relay_window_ok(1700000000 - 601, now, 600));
    ck_assert(mr_relay_window_ok(1700000000 + 600, now, 600));
    ck_assert(!mr_relay_window_ok(1700000000 + 601, now, 600));
    ck_assert(mr_relay_window_ok(1, now, 0));
}
END_TEST

START_TEST(test_key_files_refuse_loose_modes_and_links)
{
    char path[128], link[128];
    tmp_path(path, sizeof path, "modes");
    tmp_path(link, sizeof link, "modes_link");
    uint8_t key[32];
    mr_keypair_t kp;
    write_n(path, 32);
    ck_assert_int_eq(mr_relay_key_load(path, key), MR_OK);
    ck_assert_int_eq(mr_keypair_load_or_create(path, &kp), MR_OK);
    static const mode_t loose[] = {0640, 0604, 0620, 0602, 0644};
    for (size_t i = 0; i < sizeof loose / sizeof loose[0]; i++) {
        chmod(path, loose[i]);
        ck_assert_int_eq(mr_relay_key_load(path, key), MR_EPERM);
        ck_assert_int_eq(mr_keypair_load_or_create(path, &kp), MR_EPERM);
    }
    chmod(path, 0400);
    ck_assert_int_eq(mr_relay_key_load(path, key), MR_OK);
    chmod(path, 0600);
    ck_assert_int_eq(symlink(path, link), 0);
    ck_assert_int_ne(mr_relay_key_load(link, key), MR_OK);
    ck_assert_int_ne(mr_keypair_load_or_create(link, &kp), MR_OK);
    unlink(link);
    unlink(path);
}
END_TEST

static size_t sealed_frame(const mr_keypair_t *from, const mr_keypair_t *to, uint64_t boot, uint64_t seq,
                           uint8_t *out, size_t cap)
{
    Muudd__Radio__FrameId id = MUUDD__RADIO__FRAME_ID__INIT;
    id.node = 0xC0;
    id.id = 1;
    MrBridgeFrame fr = MUUDD__RADIO__BRIDGE_FRAME__INIT;
    fr.frame_id = &id;
    fr.port = 16;
    fr.boot_id = boot;
    fr.seq = seq;
    MrDatagram d = MUUDD__RADIO__BRIDGE_DATAGRAM__INIT;
    d.msg_case = MUUDD__RADIO__BRIDGE_DATAGRAM__MSG_FRAME;
    d.frame = &fr;
    uint8_t plain[256];
    size_t n = muudd__radio__bridge_datagram__pack(&d, plain);
    int m = mr_bridge_seal(from, to->pk, plain, n, out, cap);
    ck_assert_int_gt(m, 0);
    return (size_t)m;
}

static bool open_and_accept(mr_peers_t *ps, const mr_keypair_t *self, const mr_keypair_t *from,
                            const uint8_t *dgram, size_t n)
{
    uint8_t plain[256];
    int m = mr_bridge_open(self, from->pk, dgram, n, plain, sizeof plain);
    ck_assert_int_gt(m, 0);   /* sealing alone cannot tell a replay apart */
    MrDatagram *d = muudd__radio__bridge_datagram__unpack(NULL, (size_t)m, plain);
    ck_assert_ptr_nonnull(d);
    bool ok = mr_peers_accept_frame(ps, 0, d->frame->boot_id, d->frame->seq);
    muudd__radio__bridge_datagram__free_unpacked(d, NULL);
    return ok;
}

START_TEST(test_bridge_replayed_frame)
{
    mr_keypair_t a, b;
    keypair(&a);
    keypair(&b);
    char json[512], b64[64];
    sodium_bin2base64(b64, sizeof b64, a.pk, 32, sodium_base64_VARIANT_ORIGINAL);
    snprintf(json, sizeof json,
             "{\"driver\": null, \"node_addr\": 3329, \"bridge\": {\"listen\": \"127.0.0.1:1\","
             " \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"%s\"}]}}", b64);
    mr_config_t *cfg = calloc(1, sizeof *cfg);
    char err[256];
    ck_assert_int_eq(mr_config_parse(json, cfg, err, sizeof err), MR_OK);
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, cfg);
    MrKeepalive k = MUUDD__RADIO__KEEPALIVE__INIT;
    k.boot_id = 5;
    k.seq = 1;
    k.region = (char *)"";
    mr_peers_accept_keepalive(ps, 0, &k, 1000);

    uint8_t first[256], old[256];
    size_t n1 = sealed_frame(&a, &b, 5, 1, first, sizeof first);
    ck_assert(open_and_accept(ps, &b, &a, first, n1));
    ck_assert(!open_and_accept(ps, &b, &a, first, n1));   /* the captured datagram again */

    size_t n2 = sealed_frame(&a, &b, 4, 2, old, sizeof old);   /* from an earlier boot */
    ck_assert(!open_and_accept(ps, &b, &a, old, n2));
    free(ps);
    free(cfg);
}
END_TEST

int main(void)
{
    if (sodium_init() < 0)
        return 1;
    Suite *s = suite_create("crypto");
    TCase *tc = tcase_create("crypto");
    tcase_add_test(tc, test_keypair_created_then_reloaded);
    tcase_add_test(tc, test_keypair_rejects_wrong_length_file);
    tcase_add_test(tc, test_relay_key_must_be_32_bytes);
    tcase_add_test(tc, test_bridge_round_trip);
    tcase_add_test(tc, test_bridge_wrong_key_and_tamper);
    tcase_add_test(tc, test_relay_round_trip_at_reduced_mtu);
    tcase_add_test(tc, test_relay_wrong_key_and_tamper);
    tcase_add_test(tc, test_relay_window);
    tcase_add_test(tc, test_key_files_refuse_loose_modes_and_links);
    tcase_add_test(tc, test_bridge_replayed_frame);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
