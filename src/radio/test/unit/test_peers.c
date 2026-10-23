/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>
#include <string.h>

#include "mr_test.h"
#include "mr_peers.h"

#define K1 "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
#define K2 "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI="
#define K3 "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM="

static mr_config_t *cfg3(void)
{
    static const char *json =
        "{\"driver\": null, \"node_addr\": 3329,"
        " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"peers\": ["
        "   {\"addr\": \"127.0.0.1:2\", \"pubkey\": \"" K1 "\", \"priority\": 20},"
        "   {\"addr\": \"127.0.0.1:3\", \"pubkey\": \"" K2 "\", \"priority\": 10},"
        "   {\"addr\": \"127.0.0.1:4\", \"pubkey\": \"" K3 "\", \"priority\": 10}]}}";
    mr_config_t *c = calloc(1, sizeof *c);
    char err[256];
    ck_assert_int_eq(mr_config_parse(json, c, err, sizeof err), MR_OK);
    return c;
}

static MrKeepalive ka(uint32_t node, uint32_t radio, const char *region, bool gw, uint64_t boot, uint64_t seq)
{
    MrKeepalive k = MUUDD__RADIO__KEEPALIVE__INIT;
    k.node_addr = node;
    k.radio_node_addr = radio;
    k.region = (char *)region;
    k.gateway = gw;
    k.radio_up = gw;
    k.boot_id = boot;
    k.seq = seq;
    k.radio_mtu = 233;
    return k;
}

START_TEST(test_peers_skip_self)
{
    mr_config_t *c = cfg3();
    uint8_t own[32];
    memset(own, 2, sizeof own);
    mr_config_mark_self(c, own);
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, c);
    ck_assert_uint_eq(ps->n, 2);
    ck_assert_int_eq(mr_peers_find_pk(ps, own), -1);
    uint8_t k3[32];
    memset(k3, 3, sizeof k3);
    ck_assert_int_eq(mr_peers_find_pk(ps, k3), 1);
    free(ps);
    free(c);
}
END_TEST

START_TEST(test_keepalive_replay_rejected)
{
    mr_config_t *c = cfg3();
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, c);
    MrKeepalive k = ka(0xA1, 0xA1, "north", true, 1000, 5);
    ck_assert(mr_peers_accept_keepalive(ps, 0, &k, 1000));
    ck_assert(mr_peers_live(ps, 0, 15999));
    ck_assert(!mr_peers_live(ps, 0, 16001));
    /* replay of the same keepalive, and of an older one, 20 s later */
    ck_assert(!mr_peers_accept_keepalive(ps, 0, &k, 21000));
    MrKeepalive older = ka(0xA1, 0xA1, "north", true, 1000, 4);
    ck_assert(!mr_peers_accept_keepalive(ps, 0, &older, 21000));
    MrKeepalive old_boot = ka(0xA1, 0xA1, "north", true, 999, 100);
    ck_assert(!mr_peers_accept_keepalive(ps, 0, &old_boot, 21000));
    ck_assert(!mr_peers_live(ps, 0, 21000));
    /* the next real one, and a new boot starting at seq 1 */
    MrKeepalive next = ka(0xA1, 0xA1, "north", true, 1000, 6);
    ck_assert(mr_peers_accept_keepalive(ps, 0, &next, 22000));
    MrKeepalive reboot = ka(0xA1, 0xA1, "north", true, 2000, 1);
    ck_assert(mr_peers_accept_keepalive(ps, 0, &reboot, 23000));
    ck_assert(mr_peers_live(ps, 0, 23000));
    free(ps);
    free(c);
}
END_TEST

START_TEST(test_best_gateway)
{
    mr_config_t *c = cfg3();
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, c);
    MrKeepalive a = ka(0xA1, 0xA1, "north", true, 1, 1);   /* priority 20 */
    MrKeepalive b = ka(0xB2, 0xB2, "north", true, 1, 1);   /* priority 10 */
    MrKeepalive d = ka(0xA0, 0xA0, "north", true, 1, 1);   /* priority 10, lower node */
    mr_peers_accept_keepalive(ps, 0, &a, 1000);
    mr_peers_accept_keepalive(ps, 1, &b, 1000);
    ck_assert_int_eq(mr_peers_best_gateway(ps, "north", 1000), 1);
    mr_peers_accept_keepalive(ps, 2, &d, 1000);
    ck_assert_int_eq(mr_peers_best_gateway(ps, "north", 1000), 2);
    ck_assert_int_eq(mr_peers_best_gateway(ps, "south", 1000), -1);
    ck_assert_int_eq(mr_peers_best_gateway(ps, NULL, 1000), 2);
    /* peers 1 and 2 go silent; peer 0 keeps talking */
    MrKeepalive a2 = ka(0xA1, 0xA1, "north", true, 1, 2);
    mr_peers_accept_keepalive(ps, 0, &a2, 20000);
    ck_assert_int_eq(mr_peers_best_gateway(ps, "north", 20000), 0);
    /* a gateway whose radio is down is not chosen */
    MrKeepalive a3 = ka(0xA1, 0xA1, "north", true, 1, 3);
    a3.radio_up = false;
    mr_peers_accept_keepalive(ps, 0, &a3, 21000);
    ck_assert_int_eq(mr_peers_best_gateway(ps, "north", 21000), -1);
    free(ps);
    free(c);
}
END_TEST

START_TEST(test_regions_nodes_and_echo)
{
    mr_config_t *c = cfg3();
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, c);
    MrKeepalive a = ka(0xA1, 0xF1, "north", true, 1, 1);
    MrKeepalive b = ka(0xB2, 0xF2, "south", true, 1, 1);
    MrKeepalive d = ka(0x0D02, 0, "", false, 1, 1);
    mr_peers_accept_keepalive(ps, 0, &a, 1000);
    mr_peers_accept_keepalive(ps, 1, &b, 1000);
    mr_peers_accept_keepalive(ps, 2, &d, 1000);
    char regions[8][64];
    ck_assert_uint_eq(mr_peers_regions(ps, 1000, NULL, regions, 8), 2);
    ck_assert_uint_eq(mr_peers_regions(ps, 1000, "north", regions, 8), 1);
    ck_assert_str_eq(regions[0], "south");
    ck_assert_int_eq(mr_peers_find_node(ps, 0x0D02), 2);
    ck_assert_int_eq(mr_peers_find_node(ps, 0xF1), -1);
    ck_assert_int_eq(mr_peers_find_node(ps, 0), -1);
    ck_assert(mr_peers_is_peer_radio(ps, 0xF2));
    ck_assert(!mr_peers_is_peer_radio(ps, 0xB2));
    ck_assert(!mr_peers_is_peer_radio(ps, 0));
    ck_assert_uint_eq(mr_peers_min_gateway_mtu(ps), 233);
    ck_assert(!mr_peers_any_dead(ps, 1000));
    ck_assert(mr_peers_any_dead(ps, 17000));
    free(ps);
    free(c);
}
END_TEST

START_TEST(test_region_map)
{
    mr_config_t *c = cfg3();
    mr_peers_t *ps = calloc(1, sizeof *ps);
    mr_peers_init(ps, c);
    ck_assert_ptr_null(mr_peers_region_of(ps, 0xC002));
    mr_peers_note_region(ps, 0xC002, "south", 1000);
    ck_assert_str_eq(mr_peers_region_of(ps, 0xC002), "south");
    mr_peers_note_region(ps, 0xC002, "north", 2000);
    ck_assert_str_eq(mr_peers_region_of(ps, 0xC002), "north");
    for (uint32_t i = 0; i < MR_REGION_MAP; i++)
        mr_peers_note_region(ps, 0x10000 + i, "x", 3000 + i);
    ck_assert_ptr_null(mr_peers_region_of(ps, 0xC002));
    ck_assert_str_eq(mr_peers_region_of(ps, 0x10000 + MR_REGION_MAP - 1), "x");
    free(ps);
    free(c);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("peers");
    TCase *tc = tcase_create("peers");
    tcase_add_test(tc, test_peers_skip_self);
    tcase_add_test(tc, test_keepalive_replay_rejected);
    tcase_add_test(tc, test_best_gateway);
    tcase_add_test(tc, test_regions_nodes_and_echo);
    tcase_add_test(tc, test_region_map);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
