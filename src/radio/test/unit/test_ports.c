/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>

#include "mr_test.h"
#include "mr_ports.h"

static mr_config_t *cfg(void)
{
    static const char *json =
        "{\"driver\": {\"exec\": \"/bin/true\"},"
        " \"ports\": {\"16\": {\"name\": \"t\", \"share_pct\": 10, \"policy\": \"radiod\"},"
        "             \"18\": {\"name\": \"f\", \"share_pct\": 10, \"policy\": \"radiod\", \"ip_fallback\": true},"
        "             \"32\": {\"name\": \"at\", \"share_pct\": 10, \"policy\": \"consumer\"}},"
        " \"bridge\": {\"listen\": \"127.0.0.1:1\", \"relay_key_file\": \"/tmp/rk\"}}";
    mr_config_t *c = calloc(1, sizeof *c);
    char err[256];
    ck_assert_int_eq(mr_config_parse(json, c, err, sizeof err), MR_OK);
    return c;
}

#define OK MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK
#define TAKEN MUUDD__RADIO__CLAIM_STATUS__PORT_TAKEN
#define UNCONF MUUDD__RADIO__CLAIM_STATUS__PORT_UNCONFIGURED
#define MISMATCH MUUDD__RADIO__CLAIM_STATUS__POLICY_MISMATCH
#define RADIOD MUUDD__RADIO__POLICY__RADIOD_BRIDGED
#define CONSUMER MUUDD__RADIO__POLICY__CONSUMER_BRIDGED

START_TEST(test_claim_rules)
{
    mr_config_t *c = cfg();
    mr_ports_t p;
    mr_ports_init(&p);
    ck_assert_int_eq(mr_ports_claim(&p, c, 1, 16, RADIOD), OK);
    ck_assert_int_eq(mr_ports_claim(&p, c, 1, 16, RADIOD), OK);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 16, RADIOD), TAKEN);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 17, RADIOD), UNCONF);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 0, RADIOD), UNCONF);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 2, RADIOD), UNCONF);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 300, RADIOD), UNCONF);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 32, RADIOD), MISMATCH);
    ck_assert_int_eq(mr_ports_claim(&p, c, 2, 32, CONSUMER), OK);
    ck_assert_int_eq(mr_ports_claimant(&p, 32), 2);
    free(c);
}
END_TEST

START_TEST(test_release)
{
    mr_config_t *c = cfg();
    mr_ports_t p;
    mr_ports_init(&p);
    mr_ports_claim(&p, c, 1, 16, RADIOD);
    mr_ports_claim(&p, c, 1, 18, RADIOD);
    ck_assert(!mr_ports_release(&p, 2, 16));
    ck_assert(mr_ports_release(&p, 1, 16));
    ck_assert_int_eq(mr_ports_claimant(&p, 16), -1);
    mr_ports_release_all(&p, 1);
    ck_assert_int_eq(mr_ports_claimant(&p, 18), -1);
    ck_assert_int_eq(mr_ports_claim(&p, c, 3, 18, RADIOD), OK);
    free(c);
}
END_TEST

START_TEST(test_mtu)
{
    mr_config_t *c = cfg();
    ck_assert_uint_eq(mr_ports_mtu(c, 16, 233), 233);
    ck_assert_uint_eq(mr_ports_mtu(c, 18, 233), 176);
    ck_assert_uint_eq(mr_ports_mtu(c, 18, 57), 0);
    ck_assert_uint_eq(mr_ports_mtu(c, 18, 50), 0);
    ck_assert_uint_eq(mr_ports_mtu(c, 18, 0), 0);
    ck_assert_uint_eq(mr_ports_mtu(c, 300, 233), 0);
    free(c);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("ports");
    TCase *tc = tcase_create("ports");
    tcase_add_test(tc, test_claim_rules);
    tcase_add_test(tc, test_release);
    tcase_add_test(tc, test_mtu);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
