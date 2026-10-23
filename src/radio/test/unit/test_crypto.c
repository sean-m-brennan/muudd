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
    FILE *f = fopen(path, "wb");
    fwrite("short", 1, 5, f);
    fclose(f);
    uint8_t key[32];
    ck_assert_int_eq(mr_relay_key_load(path, key), MR_EINVAL);
    f = fopen(path, "wb");
    for (int i = 0; i < 32; i++) fputc(i, f);
    fclose(f);
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
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
