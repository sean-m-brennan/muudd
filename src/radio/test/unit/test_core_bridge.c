/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <string.h>

#include "core_fixture.h"

#define PORTS                                                                                          \
    " \"ports\": {\"16\": {\"name\": \"telemetry\", \"share_pct\": 30, \"policy\": \"radiod\"},"       \
    "             \"17\": {\"name\": \"wide\", \"share_pct\": 30, \"policy\": \"radiod\", \"cross_region\": true}," \
    "             \"32\": {\"name\": \"at\", \"share_pct\": 30, \"policy\": \"consumer\"}},"

/* A gateway in region north with two peers (priorities 20 and 30). */
#define GW_CFG(prio, duty)                                                                             \
    "{\"node_addr\": \"auto\", \"driver\": {\"exec\": \"/bin/true\"}, \"duty_cycle_pct\": " #duty ","   \
    PORTS                                                                                              \
    " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"gateway\": true, \"region\": \"north\", \"priority\": " #prio "," \
    "   \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"            \
    "               {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 30}]}}"

/* A driverless node 0x0D01 with three peers (priorities 20, 10, 30). */
#define D_CFG                                                                                          \
    "{\"node_addr\": 3329, \"driver\": null," PORTS                                                    \
    " \"bridge\": {\"listen\": \"127.0.0.1:1\","                                                       \
    "   \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"            \
    "               {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 10},"            \
    "               {\"addr\": \"127.0.0.1:4\", \"pubkey\": \"" K3 "\", \"priority\": 30}]}}"

START_TEST(test_keepalives_sent)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    fx_hello(f, 0xA1, 0);
    mr_core_tick(f->core, 1000);
    const MrKeepalive *k = fx_keepalive_sent(f, 0, 0);
    ck_assert_ptr_nonnull(k);
    ck_assert_ptr_nonnull(fx_keepalive_sent(f, 1, 0));
    ck_assert_uint_eq(k->node_addr, 0xA1);
    ck_assert_uint_eq(k->radio_node_addr, 0xA1);
    ck_assert_str_eq(k->region, "north");
    ck_assert(k->gateway && k->radio_up);
    ck_assert_uint_eq(k->radio_mtu, 233);
    ck_assert_uint_eq(k->seq, 1);
    mr_core_tick(f->core, 3000);
    ck_assert_ptr_null(fx_keepalive_sent(f, 0, 1));
    mr_core_tick(f->core, 6000);
    ck_assert_uint_eq(fx_keepalive_sent(f, 0, 1)->seq, 2);
    fx_free(f);
}
END_TEST

START_TEST(test_keepalive_boot_id)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    fx_hello(f, 0xA1, 0);
    mr_core_set_boot_id(f->core, 42);
    mr_core_tick(f->core, 1000);
    const MrKeepalive *k = fx_keepalive_sent(f, 0, 0);
    ck_assert_ptr_nonnull(k);
    ck_assert_uint_eq(k->boot_id, 42);
    fx_free(f);
}
END_TEST

START_TEST(test_driverless_status_follows_gateways)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    ck_assert_str_eq(fx_last_status(f, 1)->reason, "no live gateway");
    ck_assert_uint_eq(fx_claim(f, 1, 16, RADIOD, 0)->mtu, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    mr_core_tick(f->core, 1000);
    const MrRadioStatus *st = fx_last_status(f, 1);
    ck_assert_int_eq(st->state, MUUDD__RADIO__RADIO_STATE__STATE_UP);
    ck_assert_uint_eq(st->n_mtus, 1);
    ck_assert_uint_eq(st->mtus[0]->port, 16);
    ck_assert_uint_eq(st->mtus[0]->mtu, 233);
    mr_core_tick(f->core, 17000);
    ck_assert_int_eq(fx_last_status(f, 1)->state, MUUDD__RADIO__RADIO_STATE__STATE_DOWN);
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_forwards_heard_broadcast)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xF2, "north", true, 1, 1000);
    fx_received(f, 0xC0, BCAST, 16, "t", 1, 5, 1000);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 1, KIND(DELIVER)), 1);
    const MrBridgeFrame *fr = fx_pframe(f, 0, KIND(DELIVER), 0);
    ck_assert_uint_eq(fr->frame_id->node, 0xC0);
    ck_assert_uint_eq(fr->frame_id->id, 5);
    ck_assert_uint_eq(fr->src, 0xC0);
    ck_assert_str_eq(fr->origin_region, "north");
    fx_received(f, 0xF2, BCAST, 16, "echo", 4, 6, 1100);   /* peer gateway's own transmission */
    fx_received(f, 0xC0, BCAST, 32, "at", 2, 7, 1200);     /* consumer-bridged port */
    fx_received(f, 0xC0, 0xA1, 16, "dm", 2, 8, 1300);      /* addressed to this gateway */
    ck_assert_uint_eq(fx_count_pframes(f, -1, KIND(DELIVER)), 2);
    fx_free(f);
}
END_TEST

START_TEST(test_dedupe_across_gateways)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_deliver(f, 0, 0xC0, 5, BCAST, 16, "t", "north", 1000);
    fx_deliver(f, 1, 0xC0, 5, BCAST, 16, "t", "north", 1001);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    const MrReceived *rv = fx_rx(f, 1, 0);
    ck_assert_int_eq(rv->meta->via, MUUDD__RADIO__VIA__VIA_BRIDGE);
    ck_assert_str_eq(rv->meta->region, "north");
    ck_assert_uint_eq(fx_stats(f, 1, 1002)->drops->duplicate, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_driverless_broadcast_fanout)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xB2, "north", true, 1, 1000);
    fx_keepalive(f, 2, 0x0D02, 0, "", false, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 1, BCAST, 16, "hello", 5, false, 1000);
    ck_assert_uint_eq(fx_count_pframes(f, 1, KIND(TRANSMIT)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(TRANSMIT)), 0);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 2, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 1, KIND(DELIVER)), 0);
    const MrBridgeFrame *t = fx_pframe(f, 1, KIND(TRANSMIT), 0);
    ck_assert_uint_eq(t->src, 0x0D01);
    ck_assert_uint_eq(t->frame_id->node, 0x0D01);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(SENT));
    fx_free(f);
}
END_TEST

START_TEST(test_failover)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xB2, "north", true, 1, 1000);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 2, 20000);
    fx_claim(f, 1, 16, RADIOD, 20000);
    fx_send(f, 1, 1, BCAST, 16, "x", 1, false, 20000);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(TRANSMIT)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 1, KIND(TRANSMIT)) + fx_count_pframes(f, 1, KIND(DELIVER)), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_unicast_known_region)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xB2, "south", true, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_deliver(f, 1, 0xC2, 1, BCAST, 16, "hi", "south", 1000);
    fx_send(f, 1, 7, 0xC2, 16, "dm", 2, true, 1100);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(TRANSMIT)), 0);
    const MrBridgeFrame *t = fx_pframe(f, 1, KIND(TRANSMIT), 0);
    ck_assert_ptr_nonnull(t);
    ck_assert(t->want_ack);
    ck_assert_uint_ne(t->req_id, 0);
    fx_result_frame(f, 1, t->req_id, ST(SENT), 0, 1200);
    ck_assert_int_eq(fx_result(f, 1, 7, 0)->status, ST(SENT));
    fx_result_frame(f, 1, t->req_id, ST(ACKED), 0, 1300);
    ck_assert_int_eq(fx_result(f, 1, 7, 1)->status, ST(ACKED));
    ck_assert_uint_eq(fx_count_results(f, 1, 7), 2);
    fx_free(f);
}
END_TEST

START_TEST(test_unicast_unknown_region_aggregates)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xB2, "south", true, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 8, 0xE5, 16, "dm", 2, false, 1000);
    const MrBridgeFrame *a = fx_pframe(f, 0, KIND(TRANSMIT), 0);
    const MrBridgeFrame *b = fx_pframe(f, 1, KIND(TRANSMIT), 0);
    ck_assert_ptr_nonnull(a);
    ck_assert_ptr_nonnull(b);
    ck_assert_uint_eq(a->req_id, b->req_id);
    uint32_t req = a->req_id;
    fx_result_frame(f, 0, req, ST(REFUSED), RS(OVER_BUDGET), 1100);
    ck_assert_uint_eq(fx_count_results(f, 1, 8), 0);
    fx_result_frame(f, 1, req, ST(SENT), 0, 1200);
    ck_assert_uint_eq(fx_count_results(f, 1, 8), 1);
    ck_assert_int_eq(fx_result(f, 1, 8, 0)->status, ST(SENT));
    fx_free(f);
}
END_TEST

START_TEST(test_unicast_all_refused)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0xB2, 0xB2, "south", true, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 9, 0xE5, 16, "dm", 2, true, 1000);
    uint32_t req = fx_pframe(f, 0, KIND(TRANSMIT), 0)->req_id;
    fx_result_frame(f, 0, req, ST(REFUSED), RS(OVER_BUDGET), 1100);
    fx_result_frame(f, 1, req, ST(REFUSED), RS(RADIO_DOWN), 1200);
    ck_assert_uint_eq(fx_count_results(f, 1, 9), 1);
    ck_assert_int_eq(fx_result(f, 1, 9, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 9, 0)->reason, RS(OVER_BUDGET));
    fx_free(f);
}
END_TEST

START_TEST(test_unicast_no_gateway)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    /* No gateway has ever been heard, so the driverless node's MTU is 0. */
    fx_send(f, 1, 1, 0xE5, 16, "", 0, false, 1000);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(NO_ROUTE));
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_handles_transmit)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 3, 0x0D01, BCAST, 16, "b", 1, "", false, 0, false, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    ck_assert_uint_eq(fx_dsend(f, 0)->dest, BCAST);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    ck_assert_uint_eq(fx_rx(f, 1, 0)->src, 0x0D01);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(RESULT)), 0);

    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 4, 0x0D01, 0xC2, 16, "u", 1, "", true, 44, false, 1100);
    uint32_t dreq = fx_dsend(f, 1)->req_id;
    ck_assert(fx_dsend(f, 1)->want_ack);
    fx_driver_result(f, dreq, ST(SENT), 0, 9, 1200);
    const MrBridgeFrame *r = fx_pframe(f, 0, KIND(RESULT), 0);
    ck_assert_uint_eq(r->req_id, 44);
    ck_assert_int_eq(r->result->status, ST(SENT));
    fx_driver_result(f, dreq, ST(ACKED), 0, 9, 1300);
    ck_assert_int_eq(fx_pframe(f, 0, KIND(RESULT), 1)->result->status, ST(ACKED));
    fx_free(f);
}
END_TEST

START_TEST(test_transmit_over_budget)
{
    fx_t *f = fx_new(GW_CFG(10, 0.001), NULL);
    fx_hello(f, 0xA1, 0);
    static uint8_t big[200];
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 5, 0x0D01, 0xC2, 16, big, sizeof big, "", false, 45, false, 1000);
    const MrBridgeFrame *r = fx_pframe(f, 0, KIND(RESULT), 0);
    ck_assert_ptr_nonnull(r);
    ck_assert_int_eq(r->result->status, ST(REFUSED));
    ck_assert_int_eq(r->result->reason, RS(OVER_BUDGET));
    fx_free(f);
}
END_TEST

START_TEST(test_cross_region)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 1, 0xB2, 0xF2, "north", true, 1, 1000);   /* priority 30 */
    fx_deliver(f, 0, 0xC2, 1, BCAST, 17, "w", "south", 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    ck_assert_uint_eq(fx_dsend(f, 0)->port, 17);
    fx_deliver(f, 0, 0xC2, 2, BCAST, 16, "n", "south", 1000);
    fx_deliver(f, 0, 0xC3, 3, BCAST, 17, "h", "north", 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    fx_free(f);

    /* Same, but this gateway (priority 50) is not the best in its region. */
    f = fx_new(GW_CFG(50, 10), NULL);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0xB1, 0xF1, "north", true, 1, 1000);   /* priority 20 */
    fx_deliver(f, 1, 0xC2, 1, BCAST, 17, "w", "south", 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_deliver_to_peer_node_with_ack)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 2, 0x0D02, 0, "", false, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 3, 0x0D02, 16, "p", 1, true, 1000);
    const MrBridgeFrame *d = fx_pframe(f, 2, KIND(DELIVER), 0);
    ck_assert_ptr_nonnull(d);
    ck_assert(d->want_ack);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(TRANSMIT)) + fx_count_pframes(f, 0, KIND(DELIVER)), 0);
    ck_assert_int_eq(fx_result(f, 1, 3, 0)->status, ST(SENT));
    fx_result_frame(f, 2, d->req_id, ST(ACKED), 0, 1100);
    ck_assert_int_eq(fx_result(f, 1, 3, 1)->status, ST(ACKED));
    fx_free(f);
}
END_TEST

START_TEST(test_deliver_ack_without_claimant)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    fx_hello(f, 0xA1, 0);
    fx_frame_full(f, 0, KIND(DELIVER), 0x0D01, 6, 0x0D01, 0xA1, 16, "p", 1, "", true, 46, false, 1000);
    const MrBridgeFrame *r = fx_pframe(f, 0, KIND(RESULT), 0);
    ck_assert_uint_eq(r->req_id, 46);
    ck_assert_int_eq(r->result->status, ST(NO_ACK));
    fx_free(f);
}
END_TEST

START_TEST(test_dead_peer_node_without_fallback)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 2, 0x0D02, 0, "", false, 1, 1000);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 2, 20000);
    fx_claim(f, 1, 16, RADIOD, 20000);
    fx_send(f, 1, 4, 0x0D02, 16, "p", 1, false, 20000);
    ck_assert_int_eq(fx_result(f, 1, 4, 0)->reason, RS(NO_ROUTE));
    fx_free(f);
}
END_TEST

START_TEST(test_aggregate_timeout)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 5, 0xE5, 16, "dm", 2, true, 1000);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 2, 60000);
    mr_core_tick(f->core, 60999);
    ck_assert_uint_eq(fx_count_results(f, 1, 5), 0);
    mr_core_tick(f->core, 61000);
    ck_assert_int_eq(fx_result(f, 1, 5, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 5, 0)->reason, RS(NO_ROUTE));
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_drops_live_peer_echo)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    mr_core_consumer_connected(f->core, 2, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_claim(f, 2, 32, CONSUMER, 0);
    fx_keepalive(f, 1, 0xB2, 0xF2, "north", true, 1, 1000);
    fx_received(f, 0xF2, BCAST, 16, "echo", 4, 6, 1000);    /* live peer: already on IP */
    ck_assert_uint_eq(fx_count_rx(f, 1), 0);
    fx_received(f, 0xF2, BCAST, 32, "at", 2, 7, 1000);      /* consumer-bridged: never on the bridge */
    ck_assert_uint_eq(fx_count_rx(f, 2), 1);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 19000);     /* another peer stays live */
    fx_received(f, 0xF2, BCAST, 16, "quiet", 5, 8, 20000);  /* peer silent: deliver, don't forward */
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    ck_assert_uint_eq(fx_count_pframes(f, -1, KIND(DELIVER)), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_agg_no_ack_only)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_claim(f, 1, 16, RADIOD, 1000);
    fx_send(f, 1, 6, 0xE5, 16, "dm", 2, true, 1000);
    uint32_t req = fx_pframe(f, 0, KIND(TRANSMIT), 0)->req_id;
    fx_result_frame(f, 0, req, ST(NO_ACK), 0, 1100);
    ck_assert_uint_eq(fx_count_results(f, 1, 6), 2);
    ck_assert_int_eq(fx_result(f, 1, 6, 0)->status, ST(SENT));
    ck_assert_int_eq(fx_result(f, 1, 6, 1)->status, ST(NO_ACK));
    fx_free(f);
}
END_TEST

START_TEST(test_consumer_port_not_bridged)
{
    fx_t *f = fx_new(D_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_claim(f, 1, 32, CONSUMER, 0);
    fx_deliver(f, 0, 0xC0, 1, BCAST, 32, "x", "north", 1000);
    ck_assert_uint_eq(fx_count_rx(f, 1), 0);
    fx_free(f);
}
END_TEST

/* A radio node with a bridge that is not a gateway, with two peers. */
#define NG_CFG                                                                                         \
    "{\"node_addr\": \"auto\", \"driver\": {\"exec\": \"/bin/true\"}," PORTS                         \
    " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"gateway\": false, \"region\": \"north\","          \
    "   \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"            \
    "               {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 30}]}}"

START_TEST(test_gateway_forwards_non_gateway_radio_peer)
{
    fx_t *f = fx_new(GW_CFG(10, 10), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_keepalive(f, 0, 0x0E01, 0xE1, "north", false, 1, 1000);   /* non-gateway radio node */
    fx_keepalive(f, 1, 0x0D01, 0, "", false, 1, 1000);           /* driverless node */
    fx_received(f, 0xE1, BCAST, 16, "r", 1, 5, 1000);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 1, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(DELIVER)), 0);   /* never back to its source */
    ck_assert_uint_eq(fx_stats(f, 1, 1001)->drops->duplicate, 0);
    fx_free(f);
}
END_TEST

START_TEST(test_non_gateway_drops_live_gateway_copy)
{
    fx_t *f = fx_new(NG_CFG, NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xE1, 0);
    fx_claim(f, 1, 16, RADIOD, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);      /* live gateway */
    fx_keepalive(f, 1, 0x0E02, 0xE2, "north", false, 1, 1000);   /* another non-gateway radio node */
    fx_received(f, 0xA1, BCAST, 16, "copy", 4, 5, 1000);         /* the gateway's LoRa copy */
    ck_assert_uint_eq(fx_count_rx(f, 1), 0);
    fx_received(f, 0xA1, 0xE1, 16, "dm", 2, 7, 1000);            /* not a broadcast: delivered */
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_received(f, 0xE2, BCAST, 16, "peer", 4, 8, 1000);         /* non-gateway peer: delivered */
    fx_received(f, 0xC0, BCAST, 16, "hs", 2, 9, 1000);           /* handset: delivered */
    ck_assert_uint_eq(fx_count_rx(f, 1), 3);
    ck_assert_uint_eq(fx_count_pframes(f, -1, KIND(DELIVER)), 0);
    fx_free(f);
}
END_TEST

int main(void)
{
    if (sodium_init() < 0)
        return 1;
    Suite *s = suite_create("core_bridge");
    TCase *tc = tcase_create("core_bridge");
    tcase_add_test(tc, test_keepalives_sent);
    tcase_add_test(tc, test_keepalive_boot_id);
    tcase_add_test(tc, test_driverless_status_follows_gateways);
    tcase_add_test(tc, test_gateway_forwards_heard_broadcast);
    tcase_add_test(tc, test_dedupe_across_gateways);
    tcase_add_test(tc, test_driverless_broadcast_fanout);
    tcase_add_test(tc, test_failover);
    tcase_add_test(tc, test_unicast_known_region);
    tcase_add_test(tc, test_unicast_unknown_region_aggregates);
    tcase_add_test(tc, test_unicast_all_refused);
    tcase_add_test(tc, test_unicast_no_gateway);
    tcase_add_test(tc, test_gateway_handles_transmit);
    tcase_add_test(tc, test_transmit_over_budget);
    tcase_add_test(tc, test_cross_region);
    tcase_add_test(tc, test_deliver_to_peer_node_with_ack);
    tcase_add_test(tc, test_deliver_ack_without_claimant);
    tcase_add_test(tc, test_dead_peer_node_without_fallback);
    tcase_add_test(tc, test_aggregate_timeout);
    tcase_add_test(tc, test_gateway_drops_live_peer_echo);
    tcase_add_test(tc, test_agg_no_ack_only);
    tcase_add_test(tc, test_consumer_port_not_bridged);
    tcase_add_test(tc, test_gateway_forwards_non_gateway_radio_peer);
    tcase_add_test(tc, test_non_gateway_drops_live_gateway_copy);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
