/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include "mr_test.h"
#include "mr_common.h"
#include "mr_proto.h"

START_TEST(test_send_round_trip)
{
    uint8_t payload[] = {1, 2, 3};
    MrSend send = MUUDD__RADIO__SEND__INIT;
    send.req_id = 7;
    send.dest = MR_BROADCAST;
    send.port = 16;
    send.payload.data = payload;
    send.payload.len = sizeof payload;
    MrC2R msg = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    msg.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND;
    msg.send = &send;

    uint8_t buf[64];
    size_t n = muudd__radio__consumer_to_radiod__pack(&msg, buf);
    MrC2R *out = muudd__radio__consumer_to_radiod__unpack(NULL, n, buf);
    ck_assert_ptr_nonnull(out);
    ck_assert_int_eq(out->msg_case, MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND);
    ck_assert_uint_eq(out->send->req_id, 7);
    ck_assert_uint_eq(out->send->dest, MR_BROADCAST);
    ck_assert_uint_eq(out->send->port, 16);
    ck_assert_uint_eq(out->send->payload.len, 3);
    muudd__radio__consumer_to_radiod__free_unpacked(out, NULL);
}
END_TEST

START_TEST(test_clocks_advance)
{
    uint64_t a = mr_now_ms();
    ck_assert_uint_gt(mr_wall_ms(), 1700000000000ull);
    ck_assert_uint_ge(mr_now_ms(), a);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("proto");
    TCase *tc = tcase_create("proto");
    tcase_add_test(tc, test_send_round_trip);
    tcase_add_test(tc, test_clocks_advance);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
