/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "lora/lora_addrmap.h"

#define A "aaaaaaaa-0000-4000-8000-000000000001"
#define B "bbbbbbbb-0000-4000-8000-000000000002"

/* fsync and rename are defined here, ahead of libc, so a test can record the
 * order lora_addrmap_save calls them in: each call notes what it touched (a
 * file or a directory, by inode) and then makes the real system call. */
#define CALLS_MAX 8
static bool g_record;
static int g_ncalls;
static const char *g_call[CALLS_MAX];
static ino_t g_ino[CALLS_MAX];

static void _note(const char *what, ino_t ino)
{
    if (g_record && g_ncalls < CALLS_MAX) {
        g_call[g_ncalls] = what;
        g_ino[g_ncalls++] = ino;
    }
}

int fsync(int fd)
{
    struct stat st;
    if (fstat(fd, &st) == 0)
        _note(S_ISDIR(st.st_mode) ? "fsync dir" : "fsync file", st.st_ino);
    return (int)syscall(SYS_fsync, fd);
}

int rename(const char *from, const char *to)
{
    struct stat st;
    if (stat(from, &st) == 0)
        _note("rename", st.st_ino);
    return (int)syscall(SYS_renameat, AT_FDCWD, from, AT_FDCWD, to);
}

static bool _only_b(const char *uuid, void *arg)
{
    (void)arg;
    return strcmp(uuid, B) == 0;
}

DEFINE_TEST(test_accept_replay_and_lookup)
{
    lora_addrmap_t *m = lora_addrmap_new();
    uint32_t addr;
    char u[LORA_UUID_LEN + 1];
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x1001, 100, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x1002, 100, NULL, NULL), LORA_ANN_STALE);
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x1002, 101, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert(lora_addrmap_addr_of(m, A, &addr));
    ck_assert_int_eq(addr, 0x1002);
    ck_assert(!lora_addrmap_uuid_of(m, 0x1001, u));
    ck_assert(lora_addrmap_uuid_of(m, 0x1002, u));
    ck_assert_str_eq(u, A);
    ck_assert_int_eq(lora_addrmap_announce(m, "AAAAAAAA-0000-4000-8000-000000000001", 0x1003, 102,
                                           NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert(lora_addrmap_addr_of(m, A, &addr) && addr == 0x1003);
    ck_assert_int_eq(lora_addrmap_announce(m, "nope", 1, 1, NULL, NULL), LORA_ANN_BAD);
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0, 1, NULL, NULL), LORA_ANN_BAD);
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0xFFFFFFFFu, 1, NULL, NULL), LORA_ANN_BAD);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_conflict_fails_closed_and_departed_holder_is_replaced)
{
    lora_addrmap_t *m = lora_addrmap_new();
    uint32_t addr;
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x2000, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0x2000, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert(!lora_addrmap_addr_of(m, A, &addr));
    ck_assert(lora_addrmap_addr_conflicted(m, 0x2000));
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0x2001, 2, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert(lora_addrmap_addr_of(m, A, &addr) && addr == 0x2000);
    /* A departs; B takes A's radio: A's stale entry goes, no conflict. */
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0x2000, 3, _only_b, NULL), LORA_ANN_ACCEPTED);
    ck_assert(!lora_addrmap_addr_of(m, A, &addr));
    ck_assert(lora_addrmap_addr_of(m, B, &addr) && addr == 0x2000);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_full_and_remove)
{
    lora_addrmap_t *m = lora_addrmap_new();
    char u[LORA_UUID_LEN + 1];
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++) {
        snprintf(u, sizeof(u), "00000000-0000-4000-8000-%012x", i);
        ck_assert_int_eq(lora_addrmap_announce(m, u, 0x100 + i, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    }
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x9999, 1, NULL, NULL), LORA_ANN_FULL);
    snprintf(u, sizeof(u), "00000000-0000-4000-8000-%012x", 0);
    ck_assert(lora_addrmap_remove(m, u));
    ck_assert(!lora_addrmap_remove(m, u));
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x9999, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_persistence_round_trip_and_corrupt_file)
{
    char path[] = "/tmp/lora_addrmap_testXXXXXX";
    int fd = mkstemp(path);
    ck_assert(fd >= 0);
    close(fd);
    unlink(path);
    lora_addrmap_t *m = lora_addrmap_new(), *back = lora_addrmap_new();
    uint64_t self = 7;
    ck_assert_int_eq(lora_addrmap_load(back, path, &self), 0);   /* absent */
    ck_assert_int_eq((int)self, 0);
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x3000, 5, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_save(m, path, 1234), 0);
    ck_assert_int_eq(lora_addrmap_load(back, path, &self), 0);
    ck_assert_int_eq((int)self, 1234);
    uint32_t addr;
    ck_assert(lora_addrmap_addr_of(back, A, &addr) && addr == 0x3000);
    ck_assert_int_eq(lora_addrmap_announce(back, A, 0x3001, 5, NULL, NULL), LORA_ANN_STALE);
    FILE *f = fopen(path, "w");
    fputs("{not json", f);
    fclose(f);
    lora_addrmap_t *bad = lora_addrmap_new();
    ck_assert_int_eq(lora_addrmap_load(bad, path, &self), -1);
    ck_assert_int_eq((int)lora_addrmap_snapshot(bad, NULL, 0), 0);
    unlink(path);
    lora_addrmap_free(m);
    lora_addrmap_free(back);
    lora_addrmap_free(bad);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_load_range_conflicts_and_unreadable)
{
    char path[] = "/tmp/lora_addrmap_rngXXXXXX";
    int fd = mkstemp(path);
    ck_assert(fd >= 0);
    close(fd);
    const char *bodies[] = {
        "{\"version\":1,\"self_issued_ms\":-1,\"peers\":{}}",
        "{\"version\":1.0,\"self_issued_ms\":0,\"peers\":{}}",
    };
    lora_addrmap_t *m = lora_addrmap_new();
    uint64_t self = 9;
    for (size_t i = 0; i < sizeof(bodies) / sizeof(bodies[0]); i++) {
        FILE *f = fopen(path, "w");
        fputs(bodies[i], f);
        fclose(f);
        ck_assert_int_eq(lora_addrmap_load(m, path, &self), -1);
        ck_assert_int_eq((int)lora_addrmap_snapshot(m, NULL, 0), 0);
        ck_assert_int_eq((int)self, 0);
    }
    /* A bad entry is skipped; the file is otherwise fine. */
    const char *entries[] = {
        "{\"version\":1,\"self_issued_ms\":3,\"peers\":{\"" A "\":{\"addr\":1,\"issued_ms\":-1}}}",
        "{\"version\":1,\"self_issued_ms\":3,\"peers\":{\"" A "\":{\"addr\":4294967295,\"issued_ms\":1}}}",
        "{\"version\":1,\"self_issued_ms\":3,\"peers\":{\"" A "\":{\"addr\":4294967297,\"issued_ms\":1}}}",
        "{\"version\":1,\"self_issued_ms\":3,\"peers\":{\"" A "\":{\"addr\":0,\"issued_ms\":1}}}",
        "{\"version\":1,\"self_issued_ms\":3,\"peers\":{\"" A "\":5}}",
    };
    for (size_t i = 0; i < sizeof(entries) / sizeof(entries[0]); i++) {
        FILE *f = fopen(path, "w");
        fputs(entries[i], f);
        fclose(f);
        ck_assert_int_eq(lora_addrmap_load(m, path, &self), 1);
        ck_assert_int_eq((int)lora_addrmap_snapshot(m, NULL, 0), 0);
        ck_assert_int_eq((int)self, 3);
    }
    /* two entries sharing an address survive save and load, still conflicted */
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x2000, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0x2000, 1, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_save(m, path, 0), 0);
    lora_addrmap_t *back = lora_addrmap_new();
    ck_assert_int_eq(lora_addrmap_load(back, path, &self), 0);
    ck_assert(lora_addrmap_addr_conflicted(back, 0x2000));
    ck_assert_int_eq((int)lora_addrmap_snapshot(back, NULL, 0), 2);
    unlink(path);
    /* an existing path that cannot be read as a file is an error, not "absent" */
    ck_assert_int_eq(lora_addrmap_load(back, "/tmp", &self), -1);
    lora_addrmap_free(m);
    lora_addrmap_free(back);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_load_skips_a_bad_entry_and_keeps_the_rest)
{
    char path[] = "/tmp/lora_addrmap_skipXXXXXX";
    int fd = mkstemp(path);
    ck_assert(fd >= 0);
    close(fd);
    FILE *f = fopen(path, "w");
    fputs("{\"version\":1,\"self_issued_ms\":77,\"peers\":{"
          "\"" A "\":{\"addr\":4660,\"issued_ms\":5},"
          "\"" B "\":{\"addr\":0,\"issued_ms\":5},"
          "\"not-a-uuid\":{\"addr\":7,\"issued_ms\":5}}}", f);
    fclose(f);
    lora_addrmap_t *m = lora_addrmap_new();
    uint64_t self = 0;
    ck_assert_int_eq(lora_addrmap_load(m, path, &self), 2);
    ck_assert_int_eq((int)self, 77);
    uint32_t addr = 0;
    ck_assert(lora_addrmap_addr_of(m, A, &addr) && addr == 4660);
    ck_assert(!lora_addrmap_addr_of(m, B, &addr));
    ck_assert_int_eq((int)lora_addrmap_snapshot(m, NULL, 0), 1);
    unlink(path);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_save_syncs_the_file_then_renames_then_syncs_the_directory)
{
    char dir[] = "/tmp/lora_addrmap_orderXXXXXX";
    ck_assert(mkdtemp(dir) != NULL);
    char path[sizeof(dir) + 16];
    snprintf(path, sizeof(path), "%s/p.json", dir);
    lora_addrmap_t *m = lora_addrmap_new();
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0x3000, 5, NULL, NULL), LORA_ANN_ACCEPTED);
    g_ncalls = 0;
    g_record = true;
    int rc = lora_addrmap_save(m, path, 1);
    g_record = false;
    ck_assert_int_eq(rc, 0);
    struct stat file, d;
    ck_assert_int_eq(stat(path, &file), 0);
    ck_assert_int_eq(stat(dir, &d), 0);
    ck_assert_int_eq(g_ncalls, 3);
    ck_assert_str_eq(g_call[0], "fsync file");
    ck_assert(g_ino[0] == file.st_ino);
    ck_assert_str_eq(g_call[1], "rename");
    ck_assert(g_ino[1] == file.st_ino);
    ck_assert_str_eq(g_call[2], "fsync dir");
    ck_assert(g_ino[2] == d.st_ino);
    unlink(path);
    rmdir(dir);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

/* jansson's limit: 2048 levels with the root object, even in a key the map
 * never reads; brackets in a string do not count. */
DEFINE_TEST(test_load_refuses_a_file_nested_past_2048_levels)
{
    char path[] = "/tmp/lora_addrmap_deepXXXXXX";
    int fd = mkstemp(path);
    ck_assert(fd >= 0);
    close(fd);
    lora_addrmap_t *m = lora_addrmap_new();
    uint64_t self = 0;
    uint32_t addr = 0;
    /* arrays under the root object; 0 is a string holding 3000 "[{" pairs */
    const int arrays[] = {2047, 2048, 0};
    for (size_t c = 0; c < sizeof(arrays) / sizeof(arrays[0]); c++) {
        FILE *f = fopen(path, "w");
        ck_assert(f != NULL);
        fputs("{\"version\":1,\"self_issued_ms\":5,\"x\":", f);
        if (arrays[c] == 0) {
            fputc('"', f);
            for (int i = 0; i < 3000; i++)
                fputs("[{", f);
            fputc('"', f);
        }
        for (int i = 0; i < arrays[c]; i++)
            fputc('[', f);
        for (int i = 0; i < arrays[c]; i++)
            fputc(']', f);
        fputs(",\"peers\":{\"" A "\":{\"addr\":4097,\"issued_ms\":5}}}", f);
        fclose(f);
        int rc = lora_addrmap_load(m, path, &self);
        if (arrays[c] == 2048) {
            ck_assert_int_eq(rc, -1);
            ck_assert_int_eq((int)lora_addrmap_snapshot(m, NULL, 0), 0);
            ck_assert_int_eq((int)self, 0);
        } else {
            ck_assert_int_eq(rc, 0);
            ck_assert_int_eq((int)self, 5);
            ck_assert(lora_addrmap_addr_of(m, A, &addr) && addr == 4097);
        }
    }
    unlink(path);
    lora_addrmap_free(m);
}
END_TEST_DEFINITION()

RUN_TESTS(LORA_ADDRMAP,
          test_accept_replay_and_lookup,
          test_conflict_fails_closed_and_departed_holder_is_replaced,
          test_full_and_remove,
          test_persistence_round_trip_and_corrupt_file,
          test_load_range_conflicts_and_unreadable,
          test_load_skips_a_bad_entry_and_keeps_the_rest,
          test_save_syncs_the_file_then_renames_then_syncs_the_directory,
          test_load_refuses_a_file_nested_past_2048_levels)
