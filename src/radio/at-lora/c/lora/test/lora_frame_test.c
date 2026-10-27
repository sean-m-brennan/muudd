/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <string.h>

#include "lora/lora_frame.h"

static void _pattern(uint8_t *m, size_t n)
{
    for (size_t i = 0; i < n; i++)
        m[i] = (uint8_t)(i % 256);
}

DEFINE_TEST(test_fragment_peer_three_pieces)
{
    uint8_t msg[500];
    _pattern(msg, sizeof(msg));
    lora_frags_t f;
    ck_assert_int_eq(lora_fragment(msg, sizeof(msg), LORA_T_PEER, 0x0102, NULL, 233, 8, &f), 0);
    ck_assert_int_eq((int)f.n, 3);
    ck_assert_int_eq((int)f.len[0], 233);
    ck_assert_int_eq((int)f.len[1], 233);
    ck_assert_int_eq((int)f.len[2], 46);
    const uint8_t h0[] = {0x11, 0x01, 0x02, 0x02}, h2[] = {0x11, 0x01, 0x02, 0x22};
    ck_assert_int_eq(memcmp(f.buf[0], h0, 4), 0);
    ck_assert_int_eq(memcmp(f.buf[2], h2, 4), 0);
    ck_assert_int_eq(memcmp(f.buf[2] + 4, msg + 458, 42), 0);
    lora_frags_free(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_fragment_limits)
{
    uint8_t msg[1833];
    _pattern(msg, sizeof(msg));
    lora_frags_t f;
    ck_assert_int_eq(lora_fragment(msg, 1832, LORA_T_PEER, 1, NULL, 233, 8, &f), 0);
    ck_assert_int_eq((int)f.n, 8);
    lora_frags_free(&f);
    ck_assert_int_eq(lora_fragment(msg, 1833, LORA_T_PEER, 1, NULL, 233, 8, &f), -1);
    ck_assert_int_eq(lora_fragment(msg, 10, LORA_T_PEER, 1, NULL, 63, 8, &f), -1);
    ck_assert_int_eq(lora_fragment(msg, 0, LORA_T_PEER, 1, NULL, 233, 8, &f), -1);
    ck_assert_int_eq(lora_fragment(msg, 10, LORA_T_GROUP, 1, NULL, 233, 8, &f), -1);
    ck_assert_int_eq(lora_fragment(msg, 10, LORA_T_NACK, 1, NULL, 233, 8, &f), -1);
    ck_assert_int_eq((int)lora_max_message(LORA_T_GROUP, 233, 8), 1800);
    ck_assert_int_eq((int)lora_payload_per_frag(LORA_T_PEER, 63), 0);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_max_frags_is_clamped)
{
    uint8_t msg[3665];
    _pattern(msg, sizeof(msg));
    lora_frags_t f;
    /* Zero and negative count as one fragment, anything above 16 as 16. */
    for (int mf = -2; mf <= 0; mf++) {
        ck_assert_int_eq(lora_fragment(msg, 229, LORA_T_PEER, 1, NULL, 233, mf, &f), 0);
        ck_assert_int_eq((int)f.n, 1);
        lora_frags_free(&f);
        ck_assert_int_eq(lora_fragment(msg, 230, LORA_T_PEER, 1, NULL, 233, mf, &f), -1);
        ck_assert_int_eq((int)lora_max_message(LORA_T_PEER, 233, mf), 229);
    }
    ck_assert_int_eq(lora_fragment(msg, 3664, LORA_T_PEER, 1, NULL, 233, 17, &f), 0);
    ck_assert_int_eq((int)f.n, 16);
    lora_frags_free(&f);
    ck_assert_int_eq(lora_fragment(msg, 3665, LORA_T_PEER, 1, NULL, 233, 17, &f), -1);
    ck_assert_int_eq((int)lora_max_message(LORA_T_PEER, 233, 1 << 30), 3664);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_decode_group_and_nack)
{
    const uint8_t g[] = {0x12, 0x00, 0x07, 0x11, 0xa1, 0xb2, 0xc3, 0xd4, 0xff};
    lora_frame_t fr;
    ck_assert_int_eq(lora_frame_decode(g, sizeof(g), &fr), 0);
    ck_assert_int_eq(fr.type, LORA_T_GROUP);
    ck_assert_int_eq(fr.msg_id, 7);
    ck_assert_int_eq(fr.idx, 1);
    ck_assert_int_eq(fr.count, 2);
    ck_assert_int_eq(fr.tag[0], 0xa1);
    ck_assert_int_eq((int)fr.payload_len, 1);
    uint8_t n[5];
    ck_assert_int_eq(lora_encode_nack(5, 3, n, sizeof(n)), 5);
    ck_assert_int_eq(lora_frame_decode(n, 5, &fr), 0);
    ck_assert_int_eq(fr.type, LORA_T_NACK);
    ck_assert_int_eq(fr.missing, 3);
    ck_assert_int_eq(lora_encode_nack(5, 0, n, sizeof(n)), -1);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_decode_refusals)
{
    lora_frame_t fr;
    const uint8_t v2[] = {0x21, 0x01, 0x02, 0x00, 0xff};
    const uint8_t t5[] = {0x15, 0x00, 0x01, 0x00, 0xff};
    const uint8_t idx[] = {0x11, 0x01, 0x02, 0x20, 0xff};
    const uint8_t empty[] = {0x11, 0x01, 0x02, 0x00};
    const uint8_t nshort[] = {0x14, 0x00, 0x05, 0x00};
    const uint8_t nzero[] = {0x14, 0x00, 0x05, 0x00, 0x00};
    ck_assert_int_eq(lora_frame_decode(v2, sizeof(v2), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(t5, sizeof(t5), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(idx, sizeof(idx), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(empty, sizeof(empty), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(nshort, sizeof(nshort), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(nzero, sizeof(nzero), &fr), -1);
    ck_assert_int_eq(lora_frame_decode(v2, 0, &fr), -1);
}
END_TEST_DEFINITION()

RUN_TESTS(LORA_FRAME,
          test_fragment_peer_three_pieces,
          test_fragment_limits,
          test_max_frags_is_clamped,
          test_decode_group_and_nack,
          test_decode_refusals)
