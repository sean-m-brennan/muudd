/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>
#include <string.h>

#include "mr_test.h"
#include "mr_config.h"

#define K1 "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
#define K2 "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI="

static const char *FULL =
    "{\"node_addr\": \"auto\","
    " \"driver\": {\"exec\": \"/usr/lib/muudd/meshtastic-driver\","
    "              \"args\": [\"--port\", \"/dev/ttyACM0\"],"
    "              \"settings\": {\"channel\": \"muudd\"},"
    "              \"apply_radio_config\": false},"
    " \"duty_cycle_pct\": 10, \"min_bandwidth_khz\": 500,"
    " \"socket\": \"/tmp/r.sock\","
    " \"ports\": {\"1\":  {\"name\": \"text\", \"share_pct\": 20, \"policy\": \"radiod\", \"ip_fallback\": true},"
    "             \"16\": {\"name\": \"telemetry\", \"share_pct\": 30, \"policy\": \"radiod\", \"cross_region\": true},"
    "             \"32\": {\"name\": \"at\", \"share_pct\": 40, \"policy\": \"consumer\"}},"
    " \"bridge\": {\"listen\": \"[::]:4403\", \"gateway\": true, \"region\": \"valley-north\","
    "              \"priority\": 10, \"key_file\": \"/tmp/k\", \"relay_key_file\": \"/tmp/rk\","
    "              \"relay_window_s\": 900,"
    "              \"peers\": [{\"addr\": \"10.0.0.5:4403\", \"pubkey\": \"" K1 "\", \"region\": \"valley-north\", \"priority\": 1},"
    "                         {\"addr\": \"10.0.0.6:4403\", \"pubkey\": \"" K2 "\"}]}}";

static mr_config_t *cfg_new(void) { return calloc(1, sizeof(mr_config_t)); }

START_TEST(test_full_example)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_msg(mr_config_parse(FULL, c, err, sizeof err) == MR_OK, "%s", err);
    ck_assert(c->node_addr_auto);
    ck_assert(c->has_driver);
    ck_assert_str_eq(c->driver_exec, "/usr/lib/muudd/meshtastic-driver");
    ck_assert_uint_eq(c->n_driver_args, 2);
    ck_assert_str_eq(c->driver_args[1], "/dev/ttyACM0");
    ck_assert_uint_eq(c->n_settings, 1);
    ck_assert_str_eq(c->setting_keys[0], "channel");
    ck_assert_str_eq(c->setting_vals[0], "muudd");
    ck_assert_str_eq(c->socket_path, "/tmp/r.sock");
    ck_assert(c->ports[1].configured && c->ports[1].ip_fallback);
    ck_assert(c->ports[16].cross_region);
    ck_assert_int_eq(c->ports[32].policy, MUUDD__RADIO__POLICY__CONSUMER_BRIDGED);
    ck_assert(!c->ports[17].configured);
    ck_assert(c->has_bridge && c->gateway);
    ck_assert_str_eq(c->region, "valley-north");
    ck_assert_int_eq(c->priority, 10);
    ck_assert_uint_eq(c->relay_window_s, 900);
    ck_assert_uint_eq(c->n_peers, 2);
    ck_assert_uint_eq(c->peers[0].pubkey[0], 1);
    ck_assert_int_eq(c->peers[0].priority, 1);
    ck_assert_int_eq(c->peers[1].priority, 100);
    free(c);
}
END_TEST

START_TEST(test_defaults)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_msg(mr_config_parse("{\"driver\": {\"exec\": \"/bin/true\"}}", c, err, sizeof err) == MR_OK, "%s", err);
    ck_assert(c->node_addr_auto);
    ck_assert(c->duty_cycle_pct == 10.0);
    ck_assert_uint_eq(c->min_bandwidth_khz, 500);
    ck_assert_str_eq(c->socket_path, "/run/muudd/radio.sock");
    ck_assert_str_eq(c->key_file, "/etc/muudd/radio/radiod.key");
    ck_assert_uint_eq(c->relay_window_s, 600);
    ck_assert_uint_eq(c->keepalive_ms, 5000);
    ck_assert_uint_eq(c->dead_after_ms, 15000);
    ck_assert_int_eq(c->priority, 100);
    ck_assert(!c->has_bridge);
    free(c);
}
END_TEST

static void expect_error(const char *json, const char *field)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_int_eq(mr_config_parse(json, c, err, sizeof err), MR_EINVAL);
    ck_assert_msg(strstr(err, field) == err, "error \"%s\" does not start with \"%s\"", err, field);
    free(c);
}

#define DRV "\"driver\": {\"exec\": \"/bin/true\"}"

START_TEST(test_invalid_files_name_their_field)
{
    expect_error("{" DRV ", \"ports\": {\"0\": {\"name\": \"x\", \"policy\": \"radiod\"}}}", "ports.0");
    expect_error("{" DRV ", \"ports\": {\"2\": {\"name\": \"x\", \"policy\": \"radiod\"}}}", "ports.2");
    expect_error("{" DRV ", \"ports\": {\"256\": {\"name\": \"x\", \"policy\": \"radiod\"}}}", "ports.256");
    expect_error("{" DRV ", \"ports\": {\"16\": {\"name\": \"a\", \"share_pct\": 60, \"policy\": \"radiod\"},"
                 " \"17\": {\"name\": \"b\", \"share_pct\": 50, \"policy\": \"radiod\"}}}", "ports:");
    expect_error("{" DRV ", \"ports\": {\"16\": {\"name\": \"a\", \"policy\": \"both\"}}}", "ports.16.policy");
    expect_error("{" DRV ", \"ports\": {\"16\": {\"name\": \"a\", \"policy\": \"consumer\", \"ip_fallback\": true}}}",
                 "ports.16.ip_fallback");
    expect_error("{" DRV ", \"ports\": {\"16\": {\"name\": \"a\", \"policy\": \"radiod\", \"ip_fallback\": true}},"
                 " \"bridge\": {\"listen\": \"127.0.0.1:1\"}}", "bridge.relay_key_file");
    expect_error("{" DRV ", \"colour\": 1}", "colour");
    expect_error("{\"driver\": {\"args\": []}}", "driver.exec");
    expect_error("{\"driver\": null, \"bridge\": {\"listen\": \"127.0.0.1:1\"}}", "node_addr");
    expect_error("{\"driver\": null, \"node_addr\": 5}", "bridge");
    expect_error("{\"driver\": null, \"node_addr\": 5, \"bridge\": {\"listen\": \"127.0.0.1:1\", \"gateway\": true, \"region\": \"r\"}}",
                 "bridge.gateway");
    expect_error("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_window_s\": 604801}}", "bridge.relay_window_s");
    expect_error("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_window_s\": -1}}", "bridge.relay_window_s");
    expect_error("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"gateway\": true}}", "bridge.region");
    expect_error("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"keepalive_ms\": 5000, \"dead_after_ms\": 5000}}",
                 "bridge.dead_after_ms");
    expect_error("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"peers\": [{\"addr\": \"x:1\", \"pubkey\": \"AQID\"}]}}",
                 "bridge.peers[0].pubkey");
    expect_error("{" DRV ", \"bridge\": {\"peers\": []}}", "bridge.listen");
    expect_error("{" DRV ", \"node_addr\": 4294967295}", "node_addr");
    expect_error("not json", "line 1");
}
END_TEST

START_TEST(test_window_bounds_accepted)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_int_eq(mr_config_parse("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_window_s\": 0}}",
                                     c, err, sizeof err), MR_OK);
    ck_assert_uint_eq(c->relay_window_s, 0);
    ck_assert_int_eq(mr_config_parse("{" DRV ", \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_window_s\": 604800}}",
                                     c, err, sizeof err), MR_OK);
    ck_assert_uint_eq(c->relay_window_s, 604800);
    free(c);
}
END_TEST

START_TEST(test_mark_self)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_int_eq(mr_config_parse(FULL, c, err, sizeof err), MR_OK);
    uint8_t own[32];
    memset(own, 2, sizeof own);
    ck_assert_uint_eq(mr_config_mark_self(c, own), 1);
    ck_assert(!c->peers[0].is_self);
    ck_assert(c->peers[1].is_self);
    free(c);
}
END_TEST

START_TEST(test_load_missing_file)
{
    mr_config_t *c = cfg_new();
    char err[256] = "";
    ck_assert_int_eq(mr_config_load("/nonexistent/radiod.json", c, err, sizeof err), MR_EIO);
    ck_assert(strstr(err, "/nonexistent/radiod.json") != NULL);
    free(c);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("config");
    TCase *tc = tcase_create("config");
    tcase_add_test(tc, test_full_example);
    tcase_add_test(tc, test_defaults);
    tcase_add_test(tc, test_invalid_files_name_their_field);
    tcase_add_test(tc, test_window_bounds_accepted);
    tcase_add_test(tc, test_mark_self);
    tcase_add_test(tc, test_load_missing_file);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
