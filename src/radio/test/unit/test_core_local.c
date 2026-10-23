/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <string.h>

#include "core_fixture.h"

#define LOCAL_CFG(duty)                                                                                   \
    "{\"node_addr\": \"auto\", \"driver\": {\"exec\": \"/bin/true\", \"settings\": {\"channel\": \"muudd\"}}," \
    " \"duty_cycle_pct\": " #duty ","                                                                       \
    " \"ports\": {\"16\": {\"name\": \"telemetry\", \"share_pct\": 50, \"policy\": \"radiod\"},"            \
    "             \"32\": {\"name\": \"at\", \"share_pct\": 40, \"policy\": \"consumer\"}}}"

START_TEST(test_status_before_and_after_hello)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    const MrRadioStatus *st = fx_last_status(f, 1);
    ck_assert_ptr_nonnull(st);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_DOWN);
    ck_assert_str_eq(st->reason, "driver starting");
    fx_hello(f, 0xA1, 100);
    st = fx_last_status(f, 1);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_UP);
    ck_assert_ptr_nonnull(st->caps);
    ck_assert_uint_eq(st->caps->mtu, 233);
    ck_assert_uint_eq(mr_core_node_addr(f->core), 0xA1);
    const Muudd__Radio__Configure *conf = fx_configure(f);
    ck_assert_ptr_nonnull(conf);
    ck_assert_uint_eq(conf->n_settings, 1);
    ck_assert_str_eq(conf->settings[0]->key, "channel");
    ck_assert_str_eq(conf->settings[0]->value, "muudd");
    ck_assert(!conf->apply_radio_config);
    fx_free(f);
}
END_TEST

START_TEST(test_fatal_hello_keeps_reason)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello_full(f, 0xA1, 233, 500, "firmware 2.6.0 older than 2.7.15", 100, MR_EFATAL);
    const MrRadioStatus *st = fx_last_status(f, 1);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_DOWN);
    ck_assert(strstr(st->reason, "firmware 2.6.0") != NULL);
    mr_core_driver_down(f->core, "driver exited with status 0", 200);
    st = fx_last_status(f, 1);
    ck_assert(strstr(st->reason, "firmware 2.6.0") != NULL);
    fx_free(f);
}
END_TEST

START_TEST(test_bandwidth_guard)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello_full(f, 0xA1, 233, 250, NULL, 100, MR_EFATAL);
    const MrRadioStatus *st = fx_last_status(f, 1);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_DOWN);
    ck_assert(strstr(st->reason, "250 kHz below minimum 500 kHz") != NULL);
    fx_free(f);
}
END_TEST

START_TEST(test_claims)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    mr_core_consumer_connected(f->core, 2, 0);
    fx_hello(f, 0xA1, 0);
    const MrClaimResult *r = fx_claim(f, 1, 16, RADIOD, 0);
    ck_assert_int_eq(r->status, MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK);
    ck_assert_uint_eq(r->mtu, 233);
    ck_assert_int_eq(fx_claim(f, 2, 16, RADIOD, 0)->status, MUUDD__RADIO__CLAIM_STATUS__PORT_TAKEN);
    ck_assert_int_eq(fx_claim(f, 2, 99, RADIOD, 0)->status, MUUDD__RADIO__CLAIM_STATUS__PORT_UNCONFIGURED);
    ck_assert_int_eq(fx_claim(f, 2, 32, RADIOD, 0)->status, MUUDD__RADIO__CLAIM_STATUS__POLICY_MISMATCH);
    fx_free(f);
}
END_TEST

START_TEST(test_send_refusals)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, BCAST, 16, "x", 1, false, 0);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(RADIO_DOWN));
    fx_hello(f, 0xA1, 0);
    fx_send(f, 1, 2, BCAST, 32, "x", 1, false, 0);
    ck_assert_int_eq(fx_result(f, 1, 2, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 2, 0)->reason, RS(NOT_CLAIMED));
    static uint8_t big[234];
    fx_send(f, 1, 3, BCAST, 16, big, sizeof big, false, 0);
    ck_assert_int_eq(fx_result(f, 1, 3, 0)->reason, RS(TOO_BIG));
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_send_and_ack)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 5, 0xB1, 16, "hi", 2, true, 1000);
    const MrSend *s = fx_dsend(f, 0);
    ck_assert_ptr_nonnull(s);
    ck_assert_uint_eq(s->dest, 0xB1);
    ck_assert_uint_eq(s->port, 16);
    ck_assert(s->want_ack);
    ck_assert_mem_eq(s->payload.data, "hi", 2);
    uint32_t dreq = s->req_id;
    fx_driver_result(f, dreq, ST(SENT), 0, 77, 1100);
    ck_assert_int_eq(fx_result(f, 1, 5, 0)->status, ST(SENT));
    ck_assert_uint_eq(fx_result(f, 1, 5, 0)->packet_id, 77);
    fx_driver_result(f, dreq, ST(ACKED), 0, 77, 1200);
    ck_assert_int_eq(fx_result(f, 1, 5, 1)->status, ST(ACKED));

    fx_send(f, 1, 6, BCAST, 16, "b", 1, false, 2000);
    uint32_t dreq2 = fx_dsend(f, 1)->req_id;
    fx_driver_result(f, dreq2, ST(SENT), 0, 78, 2100);
    fx_driver_result(f, dreq2, ST(ACKED), 0, 78, 2200);
    ck_assert_uint_eq(fx_count_results(f, 1, 6), 1);
    fx_free(f);
}
END_TEST

START_TEST(test_over_budget)
{
    fx_t *f = fx_new(LOCAL_CFG(0.001), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    static uint8_t big[233];
    fx_send(f, 1, 1, BCAST, 16, big, sizeof big, false, 0);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(OVER_BUDGET));
    ck_assert_uint_eq(fx_stats(f, 1, 0)->drops->over_budget, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_received_delivery_and_dedupe)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_received(f, 0xB1, BCAST, 16, "t1", 2, 9, 1000);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    const MrReceived *rv = fx_rx(f, 1, 0);
    ck_assert_uint_eq(rv->src, 0xB1);
    ck_assert_int_eq(rv->meta->via, MUUDD__RADIO__VIA__VIA_RADIO);
    ck_assert_int_eq(rv->meta->rssi_dbm, -80);
    fx_received(f, 0xB1, BCAST, 16, "t1", 2, 9, 1500);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_received(f, 0xB1, BCAST, 17, "u", 1, 10, 1600);
    fx_received(f, 0xB1, 0xC3, 16, "v", 1, 11, 1700);
    fx_received(f, 0xB1, 0xA1, 16, "w", 1, 12, 1800);
    ck_assert_uint_eq(fx_count_rx(f, 1), 2);
    const MrStats *st = fx_stats(f, 1, 2000);
    ck_assert_uint_eq(st->drops->duplicate, 1);
    ck_assert_uint_eq(st->drops->unclaimed, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_disconnect_releases)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    mr_core_consumer_connected(f->core, 2, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    mr_core_consumer_disconnected(f->core, 1);
    ck_assert_int_eq(fx_claim(f, 2, 16, RADIOD, 0)->status, MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK);
    fx_free(f);
}
END_TEST

START_TEST(test_driver_down_fails_in_flight)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, BCAST, 16, "a", 1, false, 0);
    fx_send(f, 1, 2, 0xB1, 16, "b", 1, true, 0);
    fx_driver_result(f, fx_dsend(f, 1)->req_id, ST(SENT), 0, 5, 10);
    mr_core_driver_down(f->core, "driver exited with status 3", 20);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(RADIO_DOWN));
    ck_assert_int_eq(fx_result(f, 1, 2, 1)->status, ST(NO_ACK));
    const MrRadioStatus *st = fx_last_status(f, 1);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_DOWN);
    ck_assert_str_eq(st->reason, "driver exited with status 3");
    fx_free(f);
}
END_TEST

START_TEST(test_ack_timeout)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, 0xB1, 16, "a", 1, true, 1000);
    fx_driver_result(f, fx_dsend(f, 0)->req_id, ST(SENT), 0, 5, 1000);
    mr_core_tick(f->core, 120999);
    ck_assert_uint_eq(fx_count_results(f, 1, 1), 1);
    mr_core_tick(f->core, 121000);
    ck_assert_int_eq(fx_result(f, 1, 1, 1)->status, ST(NO_ACK));
    fx_free(f);
}
END_TEST

START_TEST(test_stats)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, BCAST, 16, "abc", 3, false, 1000);
    const MrStats *st = fx_stats(f, 1, 1000);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_UP);
    ck_assert_uint_gt(st->airtime_us, 0);
    ck_assert_uint_eq(st->cap_us, 360000000ull);
    ck_assert_uint_eq(st->n_ports, 2);
    ck_assert_uint_eq(st->ports[0]->port, 16);
    ck_assert_str_eq(st->ports[0]->name, "telemetry");
    ck_assert_uint_eq(st->ports[0]->airtime_us, st->airtime_us);
    ck_assert(!st->relay_active);
    fx_free(f);
}
END_TEST

START_TEST(test_lost_driver_result_times_out)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, BCAST, 16, "a", 1, false, 1000);
    mr_core_tick(f->core, 2000);
    ck_assert_uint_eq(fx_count_results(f, 1, 1), 0);
    mr_core_tick(f->core, 121999);
    ck_assert_uint_eq(fx_count_results(f, 1, 1), 0);
    mr_core_tick(f->core, 122000);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(RADIO_DOWN));
    fx_free(f);
}
END_TEST

START_TEST(test_fatal_hello_fails_in_flight)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, BCAST, 16, "a", 1, false, 10);
    fx_hello_full(f, 0xA1, 233, 500, "firmware too old", 20, MR_EFATAL);
    ck_assert_uint_eq(fx_count_results(f, 1, 1), 1);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(RADIO_DOWN));
    fx_free(f);
}
END_TEST

START_TEST(test_duplicate_sent_forwarded_once)
{
    fx_t *f = fx_new(LOCAL_CFG(10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_send(f, 1, 1, 0xB1, 16, "a", 1, true, 1000);
    uint32_t dreq = fx_dsend(f, 0)->req_id;
    fx_driver_result(f, dreq, ST(SENT), 0, 5, 1000);
    fx_driver_result(f, dreq, ST(SENT), 0, 5, 50000);
    ck_assert_uint_eq(fx_count_results(f, 1, 1), 1);
    mr_core_tick(f->core, 121000);
    ck_assert_int_eq(fx_result(f, 1, 1, 1)->status, ST(NO_ACK));
    fx_free(f);
}
END_TEST

int main(void)
{
    if (sodium_init() < 0)
        return 1;
    Suite *s = suite_create("core_local");
    TCase *tc = tcase_create("core_local");
    tcase_add_test(tc, test_status_before_and_after_hello);
    tcase_add_test(tc, test_fatal_hello_keeps_reason);
    tcase_add_test(tc, test_bandwidth_guard);
    tcase_add_test(tc, test_claims);
    tcase_add_test(tc, test_send_refusals);
    tcase_add_test(tc, test_send_and_ack);
    tcase_add_test(tc, test_over_budget);
    tcase_add_test(tc, test_received_delivery_and_dedupe);
    tcase_add_test(tc, test_disconnect_releases);
    tcase_add_test(tc, test_driver_down_fails_in_flight);
    tcase_add_test(tc, test_ack_timeout);
    tcase_add_test(tc, test_stats);
    tcase_add_test(tc, test_lost_driver_result_times_out);
    tcase_add_test(tc, test_fatal_hello_fails_in_flight);
    tcase_add_test(tc, test_duplicate_sent_forwarded_once);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
