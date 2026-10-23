/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdio.h>
#include <stdlib.h>

#include "mr_test.h"
#include "spawn.h"

#define D2R(x) MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_##x
#define ST(x) MUUDD__RADIO__SEND_STATUS__##x

static void send_msg(child_t *c, uint32_t req, uint32_t dest, size_t len, bool ack)
{
    static uint8_t payload[512];
    MrSend s = MUUDD__RADIO__SEND__INIT;
    s.req_id = req;
    s.dest = dest;
    s.port = 16;
    s.payload.data = payload;
    s.payload.len = len;
    s.want_ack = ack;
    MrR2D m = MUUDD__RADIO__RADIOD_TO_DRIVER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_SEND;
    m.send = &s;
    child_send(c, &m);
}

static MrD2R *next_of(child_t *c, int msg_case, int timeout_ms)
{
    uint64_t end = mr_now_ms() + (uint64_t)timeout_ms;
    while (mr_now_ms() < end) {
        MrD2R *m = child_next(c, 50);
        if (!m)
            continue;
        if ((int)m->msg_case == msg_case)
            return m;
        muudd__radio__driver_to_radiod__free_unpacked(m, NULL);
    }
    return NULL;
}

START_TEST(test_hello_fields)
{
    char *argv[] = {MR_STUB_DRIVER, NULL};
    child_t c;
    child_start(&c, argv);
    MrD2R *m = child_next(&c, 2000);
    ck_assert_ptr_nonnull(m);
    ck_assert_int_eq(m->msg_case, D2R(HELLO));
    ck_assert_str_eq(m->hello->name, "stub");
    ck_assert_uint_eq(m->hello->node_addr, 0xA01);
    ck_assert_uint_eq(m->hello->mtu, 233);
    ck_assert_int_eq(m->hello->reach, MUUDD__RADIO__REACH__MESH);
    ck_assert_uint_eq(m->hello->radio_settings->bandwidth_khz, 500);
    ck_assert_uint_gt(m->hello->bitrate_bps, 0);
    ck_assert_str_eq(m->hello->fatal, "");
    muudd__radio__driver_to_radiod__free_unpacked(m, NULL);
    child_close_stdin(&c);
    ck_assert_int_eq(child_wait(&c, 2000), 0);
}
END_TEST

START_TEST(test_loopback_results)
{
    char *argv[] = {MR_STUB_DRIVER, NULL};
    child_t c;
    child_start(&c, argv);
    muudd__radio__driver_to_radiod__free_unpacked(child_next(&c, 2000), NULL);
    send_msg(&c, 1, 0xB2, 4, true);
    MrD2R *a = next_of(&c, D2R(SEND_RESULT), 2000);
    MrD2R *b = next_of(&c, D2R(SEND_RESULT), 2000);
    ck_assert_int_eq(a->send_result->status, ST(SENT));
    ck_assert_int_eq(b->send_result->status, ST(ACKED));
    ck_assert_uint_eq(a->send_result->packet_id, b->send_result->packet_id);
    muudd__radio__driver_to_radiod__free_unpacked(a, NULL);
    muudd__radio__driver_to_radiod__free_unpacked(b, NULL);
    send_msg(&c, 2, MR_BROADCAST, 4, true);
    a = next_of(&c, D2R(SEND_RESULT), 2000);
    b = next_of(&c, D2R(SEND_RESULT), 2000);
    ck_assert_int_eq(b->send_result->status, ST(NO_ACK));
    muudd__radio__driver_to_radiod__free_unpacked(a, NULL);
    muudd__radio__driver_to_radiod__free_unpacked(b, NULL);
    send_msg(&c, 3, MR_BROADCAST, 234, false);
    a = next_of(&c, D2R(SEND_RESULT), 2000);
    ck_assert_int_eq(a->send_result->status, ST(REFUSED));
    ck_assert_int_eq(a->send_result->reason, MUUDD__RADIO__REFUSE_REASON__TOO_BIG);
    muudd__radio__driver_to_radiod__free_unpacked(a, NULL);
    child_close_stdin(&c);
    ck_assert_int_eq(child_wait(&c, 2000), 0);
}
END_TEST

START_TEST(test_status_heartbeat)
{
    char *argv[] = {MR_STUB_DRIVER, "--status-ms", "50", NULL};
    child_t c;
    child_start(&c, argv);
    MrD2R *m = next_of(&c, D2R(STATUS), 1000);
    ck_assert_ptr_nonnull(m);
    ck_assert_int_eq(m->status->state, MUUDD__RADIO__RADIO_STATE__STATE_UP);
    muudd__radio__driver_to_radiod__free_unpacked(m, NULL);
    child_close_stdin(&c);
    child_wait(&c, 2000);
}
END_TEST

START_TEST(test_medium)
{
    char dir[] = "/tmp/mr_medium_XXXXXX";
    ck_assert_ptr_nonnull(mkdtemp(dir));
    char *argv_a[] = {MR_STUB_DRIVER, "--medium", dir, "--node-addr", "0xA1", NULL};
    char *argv_b[] = {MR_STUB_DRIVER, "--medium", dir, "--node-addr", "0xB2", NULL};
    child_t a, b;
    child_start(&a, argv_a);
    child_start(&b, argv_b);
    muudd__radio__driver_to_radiod__free_unpacked(child_next(&a, 2000), NULL);
    muudd__radio__driver_to_radiod__free_unpacked(child_next(&b, 2000), NULL);
    send_msg(&a, 1, MR_BROADCAST, 3, false);
    MrD2R *sent = next_of(&a, D2R(SEND_RESULT), 2000);
    MrD2R *rx = next_of(&b, D2R(RECEIVED), 2000);
    ck_assert_ptr_nonnull(rx);
    ck_assert_uint_eq(rx->received->src, 0xA1);
    ck_assert_uint_eq(rx->received->dest, MR_BROADCAST);
    ck_assert_uint_eq(rx->received->port, 16);
    ck_assert_uint_eq(rx->received->meta->packet_id, sent->send_result->packet_id);
    muudd__radio__driver_to_radiod__free_unpacked(sent, NULL);
    muudd__radio__driver_to_radiod__free_unpacked(rx, NULL);
    send_msg(&a, 2, 0xB2, 3, true);
    muudd__radio__driver_to_radiod__free_unpacked(next_of(&a, D2R(SEND_RESULT), 2000), NULL);
    MrD2R *ack = next_of(&a, D2R(SEND_RESULT), 2000);
    ck_assert_int_eq(ack->send_result->status, ST(ACKED));
    muudd__radio__driver_to_radiod__free_unpacked(ack, NULL);
    send_msg(&a, 3, 0xC3, 3, true);
    muudd__radio__driver_to_radiod__free_unpacked(next_of(&a, D2R(SEND_RESULT), 2000), NULL);
    ack = next_of(&a, D2R(SEND_RESULT), 2000);
    ck_assert_int_eq(ack->send_result->status, ST(NO_ACK));
    muudd__radio__driver_to_radiod__free_unpacked(ack, NULL);
    child_close_stdin(&a);
    child_close_stdin(&b);
    child_wait(&a, 2000);
    child_wait(&b, 2000);
    rmdir(dir);
}
END_TEST

START_TEST(test_exit_code)
{
    char *argv[] = {MR_STUB_DRIVER, "--exit-after-ms", "100", "--exit-code", "78", NULL};
    child_t c;
    child_start(&c, argv);
    ck_assert_int_eq(child_wait(&c, 2000), 78);
}
END_TEST

START_TEST(test_no_fd_leak_between_children)
{
    char *argv[] = {MR_STUB_DRIVER, NULL};
    child_t a, b;
    child_start(&a, argv);
    child_start(&b, argv);
    muudd__radio__driver_to_radiod__free_unpacked(child_next(&a, 2000), NULL);
    muudd__radio__driver_to_radiod__free_unpacked(child_next(&b, 2000), NULL);
    child_close_stdin(&a);
    ck_assert_int_eq(child_wait(&a, 2000), 0);
    ck_assert_int_eq(waitpid(b.pid, NULL, WNOHANG), 0);   /* b is still running */
    child_close_stdin(&b);
    ck_assert_int_eq(child_wait(&b, 2000), 0);
}
END_TEST

int main(void)
{
    signal(SIGPIPE, SIG_IGN);
    Suite *s = suite_create("stub");
    TCase *tc = tcase_create("stub");
    tcase_set_timeout(tc, 20);
    tcase_add_test(tc, test_hello_fields);
    tcase_add_test(tc, test_loopback_results);
    tcase_add_test(tc, test_status_heartbeat);
    tcase_add_test(tc, test_medium);
    tcase_add_test(tc, test_exit_code);
    tcase_add_test(tc, test_no_fd_leak_between_children);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
