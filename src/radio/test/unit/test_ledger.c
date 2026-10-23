/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>

#include "mr_test.h"
#include "mr_ledger.h"

static mr_ledger_t *mk(double duty)
{
    mr_ledger_t *l = calloc(1, sizeof *l);
    mr_ledger_init(l, duty);
    return l;
}

START_TEST(test_port_share_enforced_over_the_hour)
{
    mr_ledger_t *l = mk(10.0);            /* 360 s per hour */
    mr_ledger_set_port(l, 16, 50.0);      /* 180 s per hour */
    mr_ledger_set_max_frame(l, 2000000);  /* bucket holds one 2 s frame */
    ck_assert_uint_eq(mr_ledger_cap_us(l, -1), 360000000ull);
    ck_assert_uint_eq(mr_ledger_cap_us(l, 16), 180000000ull);
    uint64_t t = 1000;
    for (int i = 0; i < 90; i++, t += 21000)
        ck_assert_int_eq(mr_ledger_charge(l, 16, 2000000, false, t), MR_OK);
    ck_assert_int_eq(mr_ledger_charge(l, 16, 2000000, false, t), MR_EBUDGET);
    ck_assert_uint_eq(mr_ledger_used_us(l, 16, t), 180000000ull);
    t += 61u * 60000u;
    ck_assert_int_eq(mr_ledger_charge(l, 16, 2000000, false, t), MR_OK);
    free(l);
}
END_TEST

START_TEST(test_total_cap_enforced)
{
    mr_ledger_t *l = mk(1.0);             /* 36 s per hour */
    mr_ledger_set_port(l, 16, 100.0);
    mr_ledger_set_port(l, 17, 100.0);
    mr_ledger_set_max_frame(l, 36000000);
    ck_assert_int_eq(mr_ledger_charge(l, 16, 36000000, false, 1000), MR_OK);
    /* 20 minutes later the bucket has refilled 12 s, port 17 has used
     * nothing, and only the overall hourly cap stands in the way. */
    ck_assert_int_eq(mr_ledger_charge(l, 17, 1000, false, 1000 + 1200000), MR_EBUDGET);
    free(l);
}
END_TEST

START_TEST(test_bucket_admits_max_frame)
{
    mr_ledger_t *l = mk(10.0);
    mr_ledger_set_port(l, 16, 100.0);
    mr_ledger_set_max_frame(l, 13246464);  /* 233 bytes at SF12/125 kHz */
    ck_assert_int_eq(mr_ledger_charge(l, 16, 13246464, false, 1000), MR_OK);
    ck_assert_int_eq(mr_ledger_charge(l, 16, 1000000, false, 1000), MR_EBUDGET);
    ck_assert_int_eq(mr_ledger_charge(l, 16, 1000000, false, 11000), MR_OK);
    free(l);
}
END_TEST

START_TEST(test_unconfigured_port_refused)
{
    mr_ledger_t *l = mk(10.0);
    ck_assert_int_eq(mr_ledger_charge(l, 5, 1, false, 1000), MR_EBUDGET);
    ck_assert_int_eq(mr_ledger_charge(l, 300, 1, false, 1000), MR_EBUDGET);
    free(l);
}
END_TEST

START_TEST(test_relay_accounted_separately)
{
    mr_ledger_t *l = mk(10.0);
    mr_ledger_set_port(l, 18, 50.0);
    ck_assert_int_eq(mr_ledger_charge(l, 18, 100000, true, 1000), MR_OK);
    ck_assert_int_eq(mr_ledger_charge(l, 18, 50000, false, 2000), MR_OK);
    ck_assert_uint_eq(mr_ledger_used_us(l, 18, 2000), 150000);
    ck_assert_uint_eq(mr_ledger_relay_used_us(l, 18, 2000), 100000);
    ck_assert_uint_eq(mr_ledger_used_us(l, -1, 2000), 150000);
    free(l);
}
END_TEST

START_TEST(test_zero_airtime_always_admitted)
{
    mr_ledger_t *l = mk(0.0);
    mr_ledger_set_port(l, 16, 100.0);
    ck_assert_int_eq(mr_ledger_charge(l, 16, 0, false, 1000), MR_OK);
    free(l);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("ledger");
    TCase *tc = tcase_create("ledger");
    tcase_add_test(tc, test_port_share_enforced_over_the_hour);
    tcase_add_test(tc, test_total_cap_enforced);
    tcase_add_test(tc, test_bucket_admits_max_frame);
    tcase_add_test(tc, test_unconfigured_port_refused);
    tcase_add_test(tc, test_relay_accounted_separately);
    tcase_add_test(tc, test_zero_airtime_always_admitted);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
