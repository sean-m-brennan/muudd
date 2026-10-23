/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include "mr_test.h"
#include "mr_airtime.h"

static Muudd__Radio__LoraParams lp(uint32_t sf, uint32_t bw, uint32_t cr, uint32_t pre)
{
    Muudd__Radio__LoraParams p = MUUDD__RADIO__LORA_PARAMS__INIT;
    p.spreading_factor = sf;
    p.bandwidth_hz = bw;
    p.coding_rate = cr;
    p.preamble_symbols = pre;
    return p;
}

START_TEST(test_reference_values)
{
    struct { uint32_t pl, sf, bw, cr, pre; uint64_t us; } cases[] = {
        {10, 7, 125000, 1, 8, 41216},
        {51, 12, 125000, 1, 8, 2465792},
        {0, 7, 125000, 1, 8, 25856},
        {233, 11, 250000, 1, 16, 1992704},
        {233, 12, 125000, 4, 16, 13246464},
        {233, 7, 500000, 1, 16, 94272},
        {233, 11, 500000, 4, 16, 1524736},
        {50, 9, 250000, 1, 16, 180736},
        {233, 12, 62500, 4, 16, 26492928},
    };
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        Muudd__Radio__LoraParams p = lp(cases[i].sf, cases[i].bw, cases[i].cr, cases[i].pre);
        ck_assert_uint_eq(mr_airtime_lora_us(&p, cases[i].pl), cases[i].us);
    }
}
END_TEST

START_TEST(test_ldro_forced)
{
    Muudd__Radio__LoraParams p = lp(7, 125000, 1, 8);
    p.ldro = 1;
    ck_assert_uint_eq(mr_airtime_lora_us(&p, 10), 46336);
    p = lp(12, 125000, 1, 8);
    p.ldro = 2;
    ck_assert_uint_lt(mr_airtime_lora_us(&p, 51), 2465792);
}
END_TEST

START_TEST(test_bitrate)
{
    ck_assert_uint_eq(mr_airtime_bitrate_us(1000, 100), 800000);
    ck_assert_uint_eq(mr_airtime_bitrate_us(0, 100), 0);
    ck_assert_uint_eq(mr_airtime_bitrate_us(3, 1), 2666667);
}
END_TEST

START_TEST(test_caps_uses_lora_and_overhead)
{
    Muudd__Radio__LoraParams p = lp(7, 125000, 1, 8);
    MrCaps caps = MUUDD__RADIO__CAPS__INIT;
    caps.lora = &p;
    caps.overhead_bytes = 6;
    ck_assert_uint_eq(mr_airtime_caps_us(&caps, 4), 41216);
}
END_TEST

START_TEST(test_caps_falls_back_to_bitrate)
{
    Muudd__Radio__LoraParams bad = lp(13, 125000, 1, 8);
    MrCaps caps = MUUDD__RADIO__CAPS__INIT;
    caps.bitrate_bps = 1000;
    ck_assert_uint_eq(mr_airtime_caps_us(&caps, 100), 800000);
    caps.lora = &bad;
    ck_assert(!mr_lora_params_valid(&bad));
    ck_assert_uint_eq(mr_airtime_caps_us(&caps, 100), 800000);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("airtime");
    TCase *tc = tcase_create("airtime");
    tcase_add_test(tc, test_reference_values);
    tcase_add_test(tc, test_ldro_forced);
    tcase_add_test(tc, test_bitrate);
    tcase_add_test(tc, test_caps_uses_lora_and_overhead);
    tcase_add_test(tc, test_caps_falls_back_to_bitrate);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
