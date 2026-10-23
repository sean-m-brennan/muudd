/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "mr_test.h"
#include "mr_bootcount.h"
#include "mr_common.h"

static void tmp_path(char *buf, size_t n, const char *name)
{
    snprintf(buf, n, "/tmp/mr_boot_%d_%s", (int)getpid(), name);
    unlink(buf);
}

START_TEST(test_counter_rises_across_calls)
{
    char path[128], why[128];
    tmp_path(path, sizeof path, "a");
    uint64_t v = 0;
    ck_assert_int_eq(mr_bootcount_next(path, 0, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 1);
    ck_assert_int_eq(mr_bootcount_next(path, 0, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 2);
    struct stat st;
    ck_assert_int_eq(stat(path, &st), 0);
    ck_assert_uint_eq(st.st_mode & 0777, 0600);
    ck_assert_int_eq((int)st.st_size, 8);
    FILE *f = fopen(path, "rb");
    unsigned char b[8];
    ck_assert_int_eq((int)fread(b, 1, 8, f), 8);
    fclose(f);
    ck_assert_uint_eq(b[7], 2);
    ck_assert_uint_eq(b[0], 0);
    unlink(path);
}
END_TEST

START_TEST(test_wrong_length_file_is_an_error)
{
    char path[128], why[128] = "";
    tmp_path(path, sizeof path, "bad");
    FILE *f = fopen(path, "wb");
    fputs("abc", f);
    fclose(f);
    uint64_t v = 77;
    ck_assert_int_ne(mr_bootcount_next(path, 0, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 77);
    ck_assert_ptr_nonnull(strstr(why, "8 bytes"));
    unlink(path);
}
END_TEST

START_TEST(test_unwritable_directory_is_an_error)
{
    char why[128] = "";
    uint64_t v = 0;
    ck_assert_int_ne(mr_bootcount_next("/nonexistent-dir/x.boot", 0, &v, why, sizeof why), MR_OK);
    ck_assert_int_gt((int)strlen(why), 0);
}
END_TEST

#define JAN_2025_MS 1735689600000ull

START_TEST(test_counter_follows_a_set_wall_clock)
{
    char path[128], why[128];
    tmp_path(path, sizeof path, "wall");
    uint64_t v = 0;
    /* a clock before 2025 is not trusted: plain increments */
    ck_assert_int_eq(mr_bootcount_next(path, JAN_2025_MS - 1, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 1);
    /* a set clock lifts the counter to at least the wall time in ms */
    uint64_t wall = 1790000000000ull;
    ck_assert_int_eq(mr_bootcount_next(path, wall, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, wall);
    /* so a start that fell back to its wall-clock time leaves nothing higher behind */
    uint64_t fallback = wall + 5000;
    ck_assert_int_eq(mr_bootcount_next(path, wall + 10000, &v, why, sizeof why), MR_OK);
    ck_assert_uint_gt(v, fallback);
    /* a clock that steps backwards never lowers it */
    uint64_t before = v;
    ck_assert_int_eq(mr_bootcount_next(path, wall - 100000, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, before + 1);
    ck_assert_int_eq(mr_bootcount_next(path, JAN_2025_MS, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, before + 2);
    unlink(path);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("bootcount");
    TCase *tc = tcase_create("bootcount");
    tcase_add_test(tc, test_counter_rises_across_calls);
    tcase_add_test(tc, test_wrong_length_file_is_an_error);
    tcase_add_test(tc, test_unwritable_directory_is_an_error);
    tcase_add_test(tc, test_counter_follows_a_set_wall_clock);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
