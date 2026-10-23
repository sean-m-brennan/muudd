/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <string.h>

#include "core_fixture.h"
#include "core_priv.h"

#define RPORTS                                                                                         \
    " \"ports\": {\"16\": {\"name\": \"telemetry\", \"share_pct\": 30, \"policy\": \"radiod\"},"       \
    "             \"18\": {\"name\": \"fallback\", \"share_pct\": 30, \"policy\": \"radiod\", \"ip_fallback\": true}},"

#define GW_R(window)                                                                                   \
    "{\"node_addr\": \"auto\", \"driver\": {\"exec\": \"/bin/true\"}," RPORTS                           \
    " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"gateway\": true, \"region\": \"north\", \"priority\": 10," \
    "   \"relay_key_file\": \"/unused\", \"relay_window_s\": " #window ","                              \
    "   \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"            \
    "               {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 30}]}}"

#define D_R                                                                                            \
    "{\"node_addr\": 3329, \"driver\": null," RPORTS                                                   \
    " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_key_file\": \"/unused\","                      \
    "   \"peers\": [{\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"            \
    "               {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 30}]}}"

static uint8_t KEY[32];

static size_t seal(const uint8_t *key, uint32_t src, uint32_t dest, uint8_t port, uint32_t id, uint32_t ts,
                   const char *payload, uint8_t *out)
{
    mr_relay_hdr_t h = {.src = src, .dest = dest, .id = id, .ts = ts, .port = port};
    int n = mr_relay_seal(key, &h, (const uint8_t *)payload, strlen(payload), out, 256);
    ck_assert_int_gt(n, 0);
    return (size_t)n;
}

static uint32_t now_s(fx_t *f) { return (uint32_t)(f->wall_ms / 1000); }

START_TEST(test_gateway_relays_when_peer_dead)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_received(f, 0xC0, BCAST, 18, "temp", 4, 5, 1000);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    const MrSend *s = fx_dsend(f, 0);
    ck_assert_uint_eq(s->port, MR_PORT_RELAY);
    ck_assert_uint_eq(s->dest, BCAST);
    ck_assert_uint_eq(s->payload.len, 4 + MR_RELAY_OVERHEAD);
    mr_relay_hdr_t h;
    uint8_t body[64];
    size_t blen;
    ck_assert_int_eq(mr_relay_open(KEY, s->payload.data, s->payload.len, &h, body, sizeof body, &blen), MR_OK);
    ck_assert_uint_eq(h.src, 0xC0);
    ck_assert_uint_eq(h.dest, BCAST);
    ck_assert_uint_eq(h.port, 18);
    ck_assert_uint_eq(h.id, 5);
    ck_assert_uint_eq(h.ts, now_s(f));
    ck_assert_mem_eq(body, "temp", 4);
    fx_free(f);
}
END_TEST

START_TEST(test_no_relay_without_fallback_or_dead_peer)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_received(f, 0xC0, BCAST, 16, "temp", 4, 5, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    fx_keepalive(f, 1, 0x0D02, 0, "", false, 1, 1000);
    fx_received(f, 0xC0, BCAST, 18, "temp", 4, 6, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_driverless_requests_relay)
{
    fx_t *f = fx_new(D_R, KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    ck_assert_uint_eq(fx_claim(f, 1, 18, RADIOD, 1000)->mtu, 176);
    fx_send(f, 1, 1, BCAST, 18, "x", 1, false, 1000);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(TRANSMIT)), 2);
    const MrBridgeFrame *plain = fx_pframe(f, 0, KIND(TRANSMIT), 0);
    const MrBridgeFrame *relay = fx_pframe(f, 0, KIND(TRANSMIT), 1);
    ck_assert(!plain->relay);
    ck_assert(relay->relay);
    ck_assert_uint_eq(relay->src, 0x0D01);
    ck_assert_uint_eq(relay->frame_id->id, plain->frame_id->id);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(SENT));
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_transmits_relay_request)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 9, 0x0D01, BCAST, 18, "x", 1, "", false, 0, true, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    const MrSend *s = fx_dsend(f, 0);
    ck_assert_uint_eq(s->port, MR_PORT_RELAY);
    mr_relay_hdr_t h;
    uint8_t body[8];
    size_t blen;
    ck_assert_int_eq(mr_relay_open(KEY, s->payload.data, s->payload.len, &h, body, sizeof body, &blen), MR_OK);
    ck_assert_uint_eq(h.src, 0x0D01);
    ck_assert_uint_eq(h.id, 9);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_receipt)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    uint8_t buf[256];
    size_t n = seal(KEY, 0x0D99, BCAST, 18, 4, now_s(f), "far", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 1234, 1000);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    const MrReceived *rv = fx_rx(f, 1, 0);
    ck_assert_uint_eq(rv->src, 0x0D99);
    ck_assert_uint_eq(rv->port, 18);
    ck_assert_int_eq(rv->meta->via, MUUDD__RADIO__VIA__VIA_RELAY);
    ck_assert_mem_eq(rv->payload.data, "far", 3);
    ck_assert_uint_eq(fx_count_pframes(f, 0, KIND(DELIVER)), 1);
    ck_assert_uint_eq(fx_pframe(f, 0, KIND(DELIVER), 0)->src, 0x0D99);
    ck_assert_uint_eq(fx_count_dsend(f), 0);   /* never relayed again, though peer 1 is dead */
    fx_free(f);
}
END_TEST

START_TEST(test_relay_receipt_duplicate)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_deliver(f, 0, 0x0D99, 4, BCAST, 18, "far", "", 1000);
    uint8_t buf[256];
    size_t n = seal(KEY, 0x0D99, BCAST, 18, 4, now_s(f), "far", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 1235, 1100);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    ck_assert_uint_eq(fx_stats(f, 1, 1200)->drops->duplicate, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_bad_key_and_window)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    uint8_t other[32], buf[256];
    memset(other, 0x55, sizeof other);
    size_t n = seal(other, 0x0D99, BCAST, 18, 1, now_s(f), "x", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 1, 1000);
    n = seal(KEY, 0x0D99, BCAST, 18, 2, now_s(f) - 601, "x", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 2, 1000);
    n = seal(KEY, 0x0D99, BCAST, 18, 3, now_s(f) - 600, "x", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 3, 1000);
    const MrStats *st = fx_stats(f, 1, 1000);
    ck_assert_uint_eq(st->drops->relay_auth, 1);
    ck_assert_uint_eq(st->drops->relay_window, 1);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_free(f);

    f = fx_new(GW_R(0), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    n = seal(KEY, 0x0D99, BCAST, 18, 4, 1, "old", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 4, 1000);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_mtu_reduced)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    ck_assert_uint_eq(fx_claim(f, 1, 18, RADIOD, 0)->mtu, 176);
    ck_assert_uint_eq(fx_claim(f, 1, 16, RADIOD, 0)->mtu, 233);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_mtu_underflow)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello_full(f, 0xA1, 50, 500, NULL, 0, MR_OK);
    ck_assert_uint_eq(fx_claim(f, 1, 18, RADIOD, 0)->mtu, 0);
    fx_send(f, 1, 1, BCAST, 18, "x", 1, false, 0);
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->status, ST(REFUSED));
    ck_assert_int_eq(fx_result(f, 1, 1, 0)->reason, RS(TOO_BIG));
    fx_free(f);
}
END_TEST

START_TEST(test_relay_without_key)
{
    fx_t *f = fx_new(GW_R(600), NULL);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    uint8_t buf[256];
    size_t n = seal(KEY, 0x0D99, BCAST, 18, 1, now_s(f), "x", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 1, 1000);
    ck_assert_uint_eq(fx_stats(f, 1, 1000)->drops->relay_auth, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_dedupe_follows_window)
{
    fx_t *f = fx_new(GW_R(3600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    uint8_t buf[256];
    size_t n = seal(KEY, 0x0D99, BCAST, 18, 7, now_s(f), "x", buf);
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 1, 1000);
    f->wall_ms += 20u * 60000u;
    fx_received(f, 0xF9, BCAST, MR_PORT_RELAY, buf, n, 2, 1000 + 20u * 60000u);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_free(f);
}
END_TEST

START_TEST(test_unicast_to_dead_peer_relays)
{
    fx_t *f = fx_new(D_R, KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 1, 1000);
    fx_keepalive(f, 1, 0x0D02, 0, "", false, 1, 1000);
    fx_keepalive(f, 0, 0xA1, 0xA1, "north", true, 2, 20000);
    fx_claim(f, 1, 18, RADIOD, 20000);
    fx_send(f, 1, 2, 0x0D02, 18, "p", 1, true, 20000);
    const MrBridgeFrame *r = fx_pframe(f, 0, KIND(TRANSMIT), 0);
    ck_assert_ptr_nonnull(r);
    ck_assert(r->relay);
    ck_assert_uint_eq(r->dest, 0x0D02);
    ck_assert_int_eq(fx_result(f, 1, 2, 0)->status, ST(SENT));
    ck_assert_int_eq(fx_result(f, 1, 2, 1)->status, ST(NO_ACK));
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_plain_then_relay_same_id)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 9, 0x0D01, BCAST, 18, "x", 1, "", false, 0, false, 1000);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 9, 0x0D01, BCAST, 18, "x", 1, "", false, 0, true, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 2);
    ck_assert_uint_eq(fx_dsend(f, 0)->port, 18);
    const MrSend *s = fx_dsend(f, 1);
    ck_assert_uint_eq(s->port, MR_PORT_RELAY);
    mr_relay_hdr_t h;
    uint8_t body[8];
    size_t blen;
    ck_assert_int_eq(mr_relay_open(KEY, s->payload.data, s->payload.len, &h, body, sizeof body, &blen), MR_OK);
    ck_assert_uint_eq(h.src, 0x0D01);
    ck_assert_uint_eq(h.dest, BCAST);
    ck_assert_uint_eq(h.port, 18);
    ck_assert_uint_eq(h.id, 9);
    /* the same relay request again is a duplicate */
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 9, 0x0D01, BCAST, 18, "x", 1, "", false, 0, true, 1100);
    ck_assert_uint_eq(fx_count_dsend(f), 2);
    ck_assert_uint_eq(fx_stats(f, 1, 1200)->drops->duplicate, 1);
    fx_free(f);
}
END_TEST

START_TEST(test_gateway_lone_unicast_relay_request)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 10, 0x0D01, 0x0D02, 18, "x", 1, "", false, 0, true, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 1);
    ck_assert_uint_eq(fx_dsend(f, 0)->port, MR_PORT_RELAY);
    ck_assert_uint_eq(fx_count_rx(f, 1), 0);
    fx_free(f);
}
END_TEST

START_TEST(test_short_ttl_flood_keeps_long_ttl_ids)
{
    fx_t *f = fx_new(GW_R(3600), KEY);
    mr_core_consumer_connected(f->core, 1, 0);
    fx_hello(f, 0xA1, 0);
    fx_claim(f, 1, 18, RADIOD, 0);
    /* a small short-TTL cache, so the flood below overflows it quickly */
    mr_dedupe_free(&f->core->dedupe_short);
    ck_assert_int_eq(mr_dedupe_init(&f->core->dedupe_short, 64), MR_OK);
    fx_received(f, 0xC0, BCAST, 18, "long", 4, 1, 1000);   /* ip_fallback port: one-hour TTL */
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    for (uint32_t i = 0; i < 200; i++)
        fx_received(f, 0xC1, BCAST, 16, "s", 1, 100 + i, 1000);   /* ten-minute TTL */
    ck_assert_uint_eq(f->nlog, 1);   /* evicting live entries is logged once, then rate-capped */
    ck_assert_ptr_nonnull(strstr(f->last_log, "dedupe"));
    ck_assert_uint_gt(fx_stats(f, 1, 1000)->drops->dedupe_evicted, 0);
    fx_received(f, 0xC0, BCAST, 18, "long", 4, 1, 1000 + 20u * 60000u);
    ck_assert_uint_eq(fx_count_rx(f, 1), 1);
    fx_free(f);
}
END_TEST

START_TEST(test_relay_request_needs_fallback_port)
{
    fx_t *f = fx_new(GW_R(600), KEY);
    fx_hello(f, 0xA1, 0);
    fx_keepalive(f, 0, 0x0D01, 0, "", false, 1, 1000);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 11, 0x0D01, BCAST, 16, "x", 1, "", false, 0, true, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    fx_frame_full(f, 0, KIND(TRANSMIT), 0x0D01, 12, 0x0D01, BCAST, 16, "x", 1, "", false, 61, true, 1000);
    ck_assert_uint_eq(fx_count_dsend(f), 0);
    const MrBridgeFrame *r = fx_pframe(f, 0, KIND(RESULT), 0);
    ck_assert_ptr_nonnull(r);
    ck_assert_int_eq(r->result->status, ST(REFUSED));
    ck_assert_int_eq(r->result->reason, RS(NO_ROUTE));
    fx_free(f);
}
END_TEST

int main(void)
{
    if (sodium_init() < 0)
        return 1;
    randombytes_buf(KEY, sizeof KEY);
    Suite *s = suite_create("core_relay");
    TCase *tc = tcase_create("core_relay");
    tcase_add_test(tc, test_gateway_relays_when_peer_dead);
    tcase_add_test(tc, test_no_relay_without_fallback_or_dead_peer);
    tcase_add_test(tc, test_driverless_requests_relay);
    tcase_add_test(tc, test_gateway_transmits_relay_request);
    tcase_add_test(tc, test_relay_receipt);
    tcase_add_test(tc, test_relay_receipt_duplicate);
    tcase_add_test(tc, test_relay_bad_key_and_window);
    tcase_add_test(tc, test_relay_mtu_reduced);
    tcase_add_test(tc, test_relay_mtu_underflow);
    tcase_add_test(tc, test_relay_without_key);
    tcase_add_test(tc, test_relay_dedupe_follows_window);
    tcase_add_test(tc, test_short_ttl_flood_keeps_long_ttl_ids);
    tcase_add_test(tc, test_relay_request_needs_fallback_port);
    tcase_add_test(tc, test_unicast_to_dead_peer_relays);
    tcase_add_test(tc, test_gateway_plain_then_relay_same_id);
    tcase_add_test(tc, test_gateway_lone_unicast_relay_request);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
