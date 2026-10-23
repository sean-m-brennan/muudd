/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "mr_test.h"
#include "mr_supervisor.h"

typedef struct {
    int hellos, downs, statuses, results;
    int fatal_hello;
    mr_sup_t *fatal_on_status;   /* when set, a Status makes the caller stop the driver */
    uint64_t hello_ms[16];
    char last_down[192];
    char last_stderr[256];
    int stderrs;
} rec_t;

static void on_stderr(void *ctx, const char *line)
{
    rec_t *r = ctx;
    r->stderrs++;
    snprintf(r->last_stderr, sizeof r->last_stderr, "%s", line);
}

static int on_hello(void *ctx, const MrCaps *caps)
{
    (void)caps;
    rec_t *r = ctx;
    if (r->hellos < 16)
        r->hello_ms[r->hellos] = mr_now_ms();
    r->hellos++;
    return r->fatal_hello ? MR_EFATAL : MR_OK;
}

static void on_msg(void *ctx, const MrD2R *m)
{
    rec_t *r = ctx;
    if (m->msg_case == MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_STATUS) {
        r->statuses++;
        if (r->fatal_on_status)
            mr_sup_fatal(r->fatal_on_status, "radio bandwidth 250 kHz below minimum 500 kHz", mr_now_ms());
    }
    if (m->msg_case == MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_SEND_RESULT)
        r->results++;
}

static void on_down(void *ctx, const char *reason)
{
    rec_t *r = ctx;
    r->downs++;
    snprintf(r->last_down, sizeof r->last_down, "%s", reason);
}

static mr_sup_t *start(rec_t *r, char *const *argv)
{
    mr_sup_cfg_t cfg;
    mr_sup_default_cfg(&cfg, argv);
    cfg.hello_timeout_ms = 300;
    cfg.status_period_ms = 100;
    cfg.missed_status = 3;
    cfg.kill_grace_ms = 200;
    cfg.backoff_min_ms = 100;
    cfg.backoff_max_ms = 400;
    cfg.backoff_reset_ms = 5000;
    mr_sup_io_t io = {r, on_hello, on_msg, on_down, on_stderr};
    mr_sup_t *s = NULL;
    ck_assert_int_eq(mr_sup_create(&cfg, &io, &s), MR_OK);
    return s;
}

static void run_for(mr_sup_t *s, uint64_t ms)
{
    uint64_t end = mr_now_ms() + ms;
    while (mr_now_ms() < end) {
        struct pollfd fds[2];
        int n = mr_sup_poll_fds(s, fds, 2);
        poll(fds, (nfds_t)n, 10);
        mr_sup_handle(s, fds, n, mr_now_ms());
        mr_sup_tick(s, mr_now_ms());
    }
}

START_TEST(test_up_and_heartbeats)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 500);
    ck_assert_int_eq(r.hellos, 1);
    ck_assert_int_ge(r.statuses, 5);
    ck_assert_int_eq(r.downs, 0);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_UP);
    mr_sup_free(s);
}
END_TEST

START_TEST(test_exit_restarts)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", "--exit-after-ms", "100", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 700);
    ck_assert_int_ge(r.downs, 1);
    ck_assert_str_eq(r.last_down, "driver exited with status 3");
    ck_assert_int_ge(r.hellos, 2);
    mr_sup_free(s);
}
END_TEST

START_TEST(test_backoff_grows)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", "--exit-after-ms", "20", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 1600);
    ck_assert_int_ge(r.hellos, 5);
    uint64_t d1 = r.hello_ms[1] - r.hello_ms[0];
    uint64_t d2 = r.hello_ms[2] - r.hello_ms[1];
    uint64_t d3 = r.hello_ms[3] - r.hello_ms[2];
    uint64_t d4 = r.hello_ms[4] - r.hello_ms[3];
    ck_assert_uint_ge(d1, 100);
    ck_assert_uint_gt(d2, d1 + 50);
    ck_assert_uint_gt(d3, d2 + 100);
    ck_assert_uint_lt(d4, d3 + 100);   /* capped at backoff_max_ms */
    mr_sup_free(s);
}
END_TEST

START_TEST(test_no_hello_killed)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--no-hello", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 600);
    ck_assert_int_ge(r.downs, 1);
    ck_assert_str_eq(r.last_down, "no Hello within 300 ms");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_missed_heartbeats)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--no-status", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 700);
    ck_assert_int_ge(r.downs, 1);
    ck_assert_str_eq(r.last_down, "missed heartbeats");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_malformed_frame)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", "--garbage", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 400);
    ck_assert_int_ge(r.downs, 1);
    ck_assert_str_eq(r.last_down, "malformed frame from driver");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_fatal_hello_not_restarted)
{
    rec_t r = {.fatal_hello = 1};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 800);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_FATAL);
    ck_assert_int_eq(r.hellos, 1);
    ck_assert_int_eq(r.downs, 1);
    ck_assert_str_eq(r.last_down, "driver reported a fatal error");
    r.fatal_hello = 0;
    mr_sup_reset_fatal(s, mr_now_ms());
    run_for(s, 300);
    ck_assert_int_eq(r.hellos, 2);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_UP);
    mr_sup_free(s);
}
END_TEST

START_TEST(test_exit_78_fatal)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", "--exit-after-ms", "50", "--exit-code", "78", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 600);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_FATAL);
    ck_assert_int_eq(r.hellos, 1);
    ck_assert_str_eq(r.last_down, "driver exited with status 78 (configuration error)");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_sigkill_after_grace)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--no-status", "--ignore-sigterm", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 1000);
    ck_assert_int_ge(r.downs, 1);
    ck_assert_str_eq(r.last_down, "missed heartbeats");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_send_reaches_driver)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 200);
    MrSend snd = MUUDD__RADIO__SEND__INIT;
    snd.req_id = 1;
    snd.dest = MR_BROADCAST;
    snd.port = 16;
    MrR2D m = MUUDD__RADIO__RADIOD_TO_DRIVER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_SEND;
    m.send = &snd;
    ck_assert_int_eq(mr_sup_send(s, &m), MR_OK);
    run_for(s, 200);
    ck_assert_int_eq(r.results, 1);
    mr_sup_free(s);
}
END_TEST

START_TEST(test_free_kills_child)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 200);
    pid_t pid = mr_sup_pid(s);
    ck_assert_int_gt(pid, 0);
    mr_sup_free(s);
    ck_assert_int_eq(kill(pid, 0), -1);
    ck_assert_int_eq(errno, ESRCH);
}
END_TEST

START_TEST(test_exit_78_wins_over_failed_send)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", "--exit-after-ms", "50", "--exit-code", "78", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 20);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_UP);
    usleep(150 * 1000);   /* driver exits; nothing reaps it yet */
    MrSend snd = MUUDD__RADIO__SEND__INIT;
    snd.req_id = 1;
    snd.dest = MR_BROADCAST;
    snd.port = 16;
    MrR2D m = MUUDD__RADIO__RADIOD_TO_DRIVER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_SEND;
    m.send = &snd;
    ck_assert_int_eq(mr_sup_send(s, &m), MR_EIO);
    run_for(s, 300);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_FATAL);
    ck_assert_str_eq(r.last_down, "driver exited with status 78 (configuration error)");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_stderr_forwarded)
{
    rec_t r = {0};
    char *argv[] = {"/nonexistent/driver", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 150);
    ck_assert_int_ge(r.stderrs, 1);
    ck_assert_ptr_nonnull(strstr(r.last_stderr, "cannot run /nonexistent/driver"));
    ck_assert_str_eq(r.last_down, "driver exited with status 127");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_caller_fatal_not_restarted)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 200);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_UP);
    mr_sup_fatal(s, "radio bandwidth 250 kHz below minimum 500 kHz", mr_now_ms());
    run_for(s, 800);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_FATAL);
    ck_assert_int_eq(r.hellos, 1);
    ck_assert_int_eq(r.downs, 1);
    ck_assert_str_eq(r.last_down, "radio bandwidth 250 kHz below minimum 500 kHz");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_fatal_from_status_callback)
{
    rec_t r = {0};
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    mr_sup_t *s = start(&r, argv);
    r.fatal_on_status = s;
    run_for(s, 800);
    ck_assert_int_eq(mr_sup_state(s), MR_SUP_FATAL);
    ck_assert_int_eq(r.hellos, 1);
    ck_assert_int_eq(r.statuses, 1);
    ck_assert_int_eq(r.downs, 1);
    ck_assert_str_eq(r.last_down, "radio bandwidth 250 kHz below minimum 500 kHz");
    mr_sup_free(s);
}
END_TEST

START_TEST(test_free_closes_stdin_first)
{
    char mark[128], script[256];
    snprintf(mark, sizeof mark, "/tmp/mr_sup_mark_%d", (int)getpid());
    unlink(mark);
    snprintf(script, sizeof script, "cat >/dev/null; touch %s", mark);
    rec_t r = {0};
    char *argv[] = {"/bin/sh", "-c", script, NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 100);
    pid_t pid = mr_sup_pid(s);
    ck_assert_int_gt(pid, 0);
    mr_sup_free(s);
    ck_assert_int_eq(access(mark, F_OK), 0);   /* it saw end of input and exited on its own */
    ck_assert_int_eq(kill(pid, 0), -1);
    unlink(mark);
}
END_TEST

START_TEST(test_free_kills_after_grace)
{
    rec_t r = {0};
    char *argv[] = {"/bin/sh", "-c", "trap '' TERM; while :; do sleep 0.05; done", NULL};
    mr_sup_t *s = start(&r, argv);
    run_for(s, 100);
    pid_t pid = mr_sup_pid(s);
    uint64_t t0 = mr_now_ms();
    mr_sup_free(s);
    uint64_t took = mr_now_ms() - t0;
    ck_assert_uint_ge(took, 180);    /* waited out the 200 ms grace */
    ck_assert_uint_lt(took, 1500);
    ck_assert_int_eq(kill(pid, 0), -1);
    ck_assert_int_eq(errno, ESRCH);
}
END_TEST

int main(void)
{
    signal(SIGPIPE, SIG_IGN);
    Suite *s = suite_create("supervisor");
    TCase *tc = tcase_create("supervisor");
    tcase_set_timeout(tc, 20);
    tcase_add_test(tc, test_up_and_heartbeats);
    tcase_add_test(tc, test_exit_restarts);
    tcase_add_test(tc, test_backoff_grows);
    tcase_add_test(tc, test_no_hello_killed);
    tcase_add_test(tc, test_missed_heartbeats);
    tcase_add_test(tc, test_malformed_frame);
    tcase_add_test(tc, test_fatal_hello_not_restarted);
    tcase_add_test(tc, test_exit_78_fatal);
    tcase_add_test(tc, test_sigkill_after_grace);
    tcase_add_test(tc, test_send_reaches_driver);
    tcase_add_test(tc, test_free_kills_child);
    tcase_add_test(tc, test_exit_78_wins_over_failed_send);
    tcase_add_test(tc, test_stderr_forwarded);
    tcase_add_test(tc, test_caller_fatal_not_restarted);
    tcase_add_test(tc, test_fatal_from_status_callback);
    tcase_add_test(tc, test_free_closes_stdin_first);
    tcase_add_test(tc, test_free_kills_after_grace);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
