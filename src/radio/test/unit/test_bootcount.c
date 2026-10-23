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
    ck_assert_int_eq(mr_bootcount_next(path, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 1);
    ck_assert_int_eq(mr_bootcount_next(path, &v, why, sizeof why), MR_OK);
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
    ck_assert_int_ne(mr_bootcount_next(path, &v, why, sizeof why), MR_OK);
    ck_assert_uint_eq(v, 77);
    ck_assert_ptr_nonnull(strstr(why, "8 bytes"));
    unlink(path);
}
END_TEST

START_TEST(test_unwritable_directory_is_an_error)
{
    char why[128] = "";
    uint64_t v = 0;
    ck_assert_int_ne(mr_bootcount_next("/nonexistent-dir/x.boot", &v, why, sizeof why), MR_OK);
    ck_assert_int_gt((int)strlen(why), 0);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("bootcount");
    TCase *tc = tcase_create("bootcount");
    tcase_add_test(tc, test_counter_rises_across_calls);
    tcase_add_test(tc, test_wrong_length_file_is_an_error);
    tcase_add_test(tc, test_unwritable_directory_is_an_error);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
