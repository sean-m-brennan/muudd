/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <stdlib.h>
#include <string.h>

#include "lora/lora_frame.h"
#include "lora/lora_reasm.h"

static lora_rx_status_t _add(lora_reasm_t *r, uint32_t src, const char *hex, double now,
                             char *msg_hex)
{
    uint8_t b[64];
    size_t n = strlen(hex) / 2;
    for (size_t i = 0; i < n; i++) {
        unsigned v;
        sscanf(hex + 2 * i, "%2x", &v);
        b[i] = (uint8_t)v;
    }
    lora_frame_t f;
    ck_assert_int_eq(lora_frame_decode(b, n, &f), 0);
    uint8_t *m = NULL;
    size_t mlen = 0;
    lora_rx_status_t st = lora_reasm_add(r, src, &f, now, &m, &mlen);
    if (msg_hex != NULL) {
        msg_hex[0] = '\0';
        for (size_t i = 0; m != NULL && i < mlen; i++)
            sprintf(msg_hex + 2 * i, "%02x", m[i]);
    }
    free(m);
    return st;
}

DEFINE_TEST(test_out_of_order_completes_then_duplicates)
{
    lora_reasm_t *r = lora_reasm_new(20, 90);
    char m[64];
    ck_assert_int_eq(_add(r, 1, "11010222cc", 0, m), LORA_RX_PARTIAL);
    ck_assert_int_eq(_add(r, 1, "11010202aa", 1, m), LORA_RX_PARTIAL);
    ck_assert_int_eq(_add(r, 1, "11010212bb", 2, m), LORA_RX_COMPLETE);
    ck_assert_str_eq(m, "aabbcc");
    ck_assert_int_eq(_add(r, 1, "11010202aa", 3, m), LORA_RX_DUPLICATE);
    ck_assert_int_eq((int)lora_reasm_stats(r).duplicate, 1);
    lora_reasm_free(r);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_nack_once_after_the_gap_then_timeout)
{
    lora_reasm_t *r = lora_reasm_new(20, 90);
    lora_nack_t n[4];
    ck_assert_int_eq(_add(r, 1, "11010202aa", 0, NULL), LORA_RX_PARTIAL);
    ck_assert_int_eq((int)lora_reasm_tick(r, 19, n, 4), 0);
    ck_assert_int_eq((int)lora_reasm_tick(r, 20, n, 4), 1);
    ck_assert_int_eq(n[0].src, 1);
    ck_assert_int_eq(n[0].msg_id, 258);
    ck_assert_int_eq(n[0].missing, 6);
    ck_assert_int_eq((int)lora_reasm_tick(r, 50, n, 4), 0);
    ck_assert_int_eq((int)lora_reasm_tick(r, 90, n, 4), 0);
    ck_assert_int_eq((int)lora_reasm_stats(r).timeouts, 1);
    ck_assert_int_eq(_add(r, 1, "11010212bb", 91, NULL), LORA_RX_DUPLICATE);
    lora_reasm_free(r);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_group_is_never_nacked)
{
    lora_reasm_t *r = lora_reasm_new(20, 90);
    lora_nack_t n[4];
    ck_assert_int_eq(_add(r, 1, "12000901a1b2c3d4aa", 0, NULL), LORA_RX_PARTIAL);
    ck_assert_int_eq((int)lora_reasm_tick(r, 30, n, 4), 0);
    lora_reasm_free(r);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_caps_evict_oldest)
{
    lora_reasm_t *r = lora_reasm_new(20, 90);
    const char *five[] = {"1100010101", "1100020102", "1100030103", "1100040104", "1100050105"};
    for (int i = 0; i < 5; i++)
        ck_assert_int_eq(_add(r, 1, five[i], i, NULL), LORA_RX_PARTIAL);
    ck_assert_int_eq((int)lora_reasm_stats(r).evicted, 1);
    ck_assert_int_eq(_add(r, 1, "1100011101", 6, NULL), LORA_RX_DUPLICATE);
    /* 16 overall: four sources of four, then a fifth source evicts one more. */
    lora_reasm_t *g = lora_reasm_new(20, 90);
    char hex[16];
    for (uint32_t src = 1; src <= 4; src++)
        for (int m = 1; m <= 4; m++) {
            snprintf(hex, sizeof(hex), "11%04x01%02x", m, m);
            ck_assert_int_eq(_add(g, src, hex, src * 10 + m, NULL), LORA_RX_PARTIAL);
        }
    ck_assert_int_eq(_add(g, 5, "1100090109", 100, NULL), LORA_RX_PARTIAL);
    ck_assert_int_eq((int)lora_reasm_stats(g).evicted, 1);
    lora_reasm_free(r);
    lora_reasm_free(g);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_hold_serves_once_and_expires)
{
    lora_hold_t *h = lora_hold_new(120);
    uint8_t msg[3] = {0xaa, 0xbb, 0xcc};
    lora_frags_t f, got;
    ck_assert_int_eq(lora_fragment(msg, 3, LORA_T_PEER, 258, NULL, 64, 16, &f), 0);
    lora_hold_put(h, 9, 258, &f, 0);
    ck_assert_int_eq((int)lora_hold_serve(h, 9, 258, 1, 10, &got), 1);
    lora_frags_free(&got);
    ck_assert_int_eq((int)lora_hold_serve(h, 9, 258, 1, 11, &got), 0);
    ck_assert_int_eq((int)lora_hold_served(h), 1);
    lora_frags_t f2;
    ck_assert_int_eq(lora_fragment(msg, 3, LORA_T_PEER, 259, NULL, 64, 16, &f2), 0);
    lora_hold_put(h, 9, 259, &f2, 0);
    ck_assert_int_eq((int)lora_hold_serve(h, 9, 259, 1, 120, &got), 0);
    lora_hold_free(h);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_hold_serves_the_newest_for_a_repeated_key)
{
    lora_hold_t *h = lora_hold_new(120);
    uint8_t a[1] = {0xaa}, b[1] = {0xbb};
    lora_frags_t f1, f2, got;
    ck_assert_int_eq(lora_fragment(a, 1, LORA_T_PEER, 1, NULL, 64, 16, &f1), 0);
    ck_assert_int_eq(lora_fragment(b, 1, LORA_T_PEER, 1, NULL, 64, 16, &f2), 0);
    lora_hold_put(h, 9, 1, &f1, 0);
    lora_hold_put(h, 9, 1, &f2, 1);
    ck_assert_int_eq((int)lora_hold_serve(h, 9, 1, 1, 2, &got), 1);
    ck_assert_int_eq(got.buf[0][got.len[0] - 1], 0xbb);
    lora_frags_free(&got);
    lora_hold_free(h);
}
END_TEST_DEFINITION()

RUN_TESTS(LORA_REASM,
          test_out_of_order_completes_then_duplicates,
          test_nack_once_after_the_gap_then_timeout,
          test_group_is_never_nacked,
          test_caps_evict_oldest,
          test_hold_serves_once_and_expires,
          test_hold_serves_the_newest_for_a_repeated_key)
