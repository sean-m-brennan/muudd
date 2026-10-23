/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include "mr_test.h"
#include "mr_dedupe.h"

START_TEST(test_seen_after_insert)
{
    mr_dedupe_t d;
    ck_assert_int_eq(mr_dedupe_init(&d, 16), MR_OK);
    mr_frame_id_t a = {0xB001, 7};
    ck_assert(!mr_dedupe_check_insert(&d, a, 600000, 1000));
    ck_assert(mr_dedupe_check_insert(&d, a, 600000, 2000));
    mr_dedupe_free(&d);
}
END_TEST

START_TEST(test_same_id_other_source_is_new)
{
    mr_dedupe_t d;
    mr_dedupe_init(&d, 16);
    mr_frame_id_t a = {0xB001, 7}, b = {0xB002, 7};
    ck_assert(!mr_dedupe_check_insert(&d, a, 600000, 1000));
    ck_assert(!mr_dedupe_check_insert(&d, b, 600000, 1000));
    mr_dedupe_free(&d);
}
END_TEST

START_TEST(test_expiry)
{
    mr_dedupe_t d;
    mr_dedupe_init(&d, 16);
    mr_frame_id_t a = {1, 1};
    ck_assert(!mr_dedupe_check_insert(&d, a, 600000, 1000));
    ck_assert(mr_dedupe_check_insert(&d, a, 600000, 600999));
    ck_assert(!mr_dedupe_check_insert(&d, a, 600000, 1201000));
    mr_dedupe_free(&d);
}
END_TEST

START_TEST(test_longer_ttl_retained)
{
    mr_dedupe_t d;
    mr_dedupe_init(&d, 16);
    mr_frame_id_t a = {1, 1}, b = {1, 2};
    mr_dedupe_check_insert(&d, a, 600000, 0);
    mr_dedupe_check_insert(&d, b, 3600000, 0);
    ck_assert(!mr_dedupe_check_insert(&d, a, 600000, 1200000));
    ck_assert(mr_dedupe_check_insert(&d, b, 3600000, 1200000));
    mr_dedupe_free(&d);
}
END_TEST

START_TEST(test_full_ring_evicts_oldest)
{
    mr_dedupe_t d;
    mr_dedupe_init(&d, 4);
    for (uint32_t i = 0; i < 5; i++) {
        mr_frame_id_t id = {9, i};
        ck_assert(!mr_dedupe_check_insert(&d, id, 600000, 1000));
    }
    ck_assert_uint_eq(d.evicted_early, 1);
    mr_frame_id_t first = {9, 0}, last = {9, 4};
    ck_assert(!mr_dedupe_check_insert(&d, first, 600000, 1000));
    ck_assert(mr_dedupe_check_insert(&d, last, 600000, 1000));
    mr_dedupe_free(&d);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("dedupe");
    TCase *tc = tcase_create("dedupe");
    tcase_add_test(tc, test_seen_after_insert);
    tcase_add_test(tc, test_same_id_other_source_is_new);
    tcase_add_test(tc, test_expiry);
    tcase_add_test(tc, test_longer_ttl_retained);
    tcase_add_test(tc, test_full_ring_evicts_oldest);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
