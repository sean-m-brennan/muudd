/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <string.h>

#include "mr_test.h"
#include "mr_framing.h"
#include "mr_proto.h"

typedef struct { int count; size_t lens[8]; uint8_t last[64]; } sink_t;

static int sink(void *ctx, const uint8_t *frame, size_t len)
{
    sink_t *s = ctx;
    s->lens[s->count++] = len;
    memcpy(s->last, frame, len < sizeof s->last ? len : sizeof s->last);
    return 0;
}

START_TEST(test_round_trip)
{
    uint8_t out[16];
    int n = mr_frame_encode((const uint8_t *)"abc", 3, out, sizeof out);
    ck_assert_int_eq(n, 5);
    ck_assert_uint_eq(out[0], 0);
    ck_assert_uint_eq(out[1], 3);
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, out, (size_t)n, sink, &s), MR_OK);
    ck_assert_int_eq(s.count, 1);
    ck_assert_uint_eq(s.lens[0], 3);
    ck_assert_mem_eq(s.last, "abc", 3);
}
END_TEST

START_TEST(test_byte_by_byte)
{
    uint8_t out[16];
    int n = mr_frame_encode((const uint8_t *)"hello", 5, out, sizeof out);
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    for (int i = 0; i < n; i++)
        ck_assert_int_eq(mr_decoder_feed(&d, out + i, 1, sink, &s), MR_OK);
    ck_assert_int_eq(s.count, 1);
    ck_assert_mem_eq(s.last, "hello", 5);
}
END_TEST

START_TEST(test_two_frames_one_feed_and_empty_frame)
{
    uint8_t out[32];
    int a = mr_frame_encode((const uint8_t *)"xy", 2, out, sizeof out);
    int b = mr_frame_encode(NULL, 0, out + a, sizeof out - (size_t)a);
    ck_assert_int_eq(b, 2);
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, out, (size_t)(a + b), sink, &s), MR_OK);
    ck_assert_int_eq(s.count, 2);
    ck_assert_uint_eq(s.lens[0], 2);
    ck_assert_uint_eq(s.lens[1], 0);
}
END_TEST

START_TEST(test_oversized_header_rejected)
{
    uint8_t junk[] = {0xFF, 0xFF, 1, 2, 3};
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, junk, sizeof junk, sink, &s), MR_EFRAME);
    ck_assert_int_eq(s.count, 0);
}
END_TEST

START_TEST(test_header_just_over_limit_rejected)
{
    uint8_t hdr[] = {(uint8_t)((MR_FRAME_MAX + 1) >> 8), (uint8_t)(MR_FRAME_MAX + 1)};
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, hdr, 2, sink, &s), MR_EFRAME);
}
END_TEST

START_TEST(test_truncated_frame_waits)
{
    uint8_t out[16];
    int n = mr_frame_encode((const uint8_t *)"abcdef", 6, out, sizeof out);
    mr_decoder_t d; sink_t s = {0};
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, out, (size_t)n - 1, sink, &s), MR_OK);
    ck_assert_int_eq(s.count, 0);
    ck_assert_int_eq(mr_decoder_feed(&d, out + n - 1, 1, sink, &s), MR_OK);
    ck_assert_int_eq(s.count, 1);
}
END_TEST

START_TEST(test_encode_limits)
{
    static uint8_t big[MR_FRAME_MAX + 1];
    static uint8_t out[MR_FRAME_MAX + 8];
    ck_assert_int_eq(mr_frame_encode(big, sizeof big, out, sizeof out), -1);
    ck_assert_int_eq(mr_frame_encode(big, 10, out, 11), -1);
    ck_assert_int_eq(mr_frame_encode(big, MR_FRAME_MAX, out, sizeof out), (int)MR_FRAME_MAX + 2);
}
END_TEST

static int stop_cb(void *ctx, const uint8_t *f, size_t n) { (void)ctx; (void)f; (void)n; return 42; }

START_TEST(test_callback_error_propagates)
{
    uint8_t out[8];
    int n = mr_frame_encode((const uint8_t *)"a", 1, out, sizeof out);
    mr_decoder_t d;
    mr_decoder_init(&d);
    ck_assert_int_eq(mr_decoder_feed(&d, out, (size_t)n, stop_cb, NULL), 42);
}
END_TEST

START_TEST(test_pack_message)
{
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = 9;
    r.status = MUUDD__RADIO__SEND_STATUS__ACKED;
    uint8_t out[64];
    int n = mr_frame_pack(&r.base, out, sizeof out);
    ck_assert_int_gt(n, 2);
    size_t len = ((size_t)out[0] << 8) | out[1];
    ck_assert_uint_eq(len, (size_t)n - 2);
    MrSendResult *u = muudd__radio__send_result__unpack(NULL, len, out + 2);
    ck_assert_uint_eq(u->req_id, 9);
    ck_assert_int_eq(u->status, MUUDD__RADIO__SEND_STATUS__ACKED);
    muudd__radio__send_result__free_unpacked(u, NULL);
    ck_assert_int_eq(mr_frame_pack(&r.base, out, 3), -1);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("framing");
    TCase *tc = tcase_create("framing");
    tcase_add_test(tc, test_round_trip);
    tcase_add_test(tc, test_byte_by_byte);
    tcase_add_test(tc, test_two_frames_one_feed_and_empty_frame);
    tcase_add_test(tc, test_oversized_header_rejected);
    tcase_add_test(tc, test_header_just_over_limit_rejected);
    tcase_add_test(tc, test_truncated_frame_waits);
    tcase_add_test(tc, test_encode_limits);
    tcase_add_test(tc, test_callback_error_propagates);
    tcase_add_test(tc, test_pack_message);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
