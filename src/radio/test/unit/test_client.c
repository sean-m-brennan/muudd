/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <unistd.h>

#include "mr_client.h"
#include "mr_framing.h"
#include "mr_test.h"

static char sock_path[108];

static int listen_on(const char *tag)
{
    snprintf(sock_path, sizeof sock_path, "/tmp/mr_client_test_%d_%s.sock", (int)getpid(), tag);
    unlink(sock_path);
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof sa.sun_path, "%s", sock_path);
    ck_assert_int_eq(bind(fd, (struct sockaddr *)&sa, sizeof sa), 0);
    ck_assert_int_eq(listen(fd, 4), 0);
    return fd;
}

static int read_full(int fd, uint8_t *buf, size_t n)
{
    size_t off = 0;
    while (off < n) {
        ssize_t r = read(fd, buf + off, n - off);
        if (r <= 0)
            return -1;
        off += (size_t)r;
    }
    return 0;
}

/* Reads one client frame; NULL on EOF. */
static MrC2R *read_msg(int fd)
{
    uint8_t hdr[2], buf[MR_FRAME_MAX];
    if (read_full(fd, hdr, 2) != 0)
        return NULL;
    size_t len = ((size_t)hdr[0] << 8) | hdr[1];
    if (read_full(fd, buf, len) != 0)
        return NULL;
    return muudd__radio__consumer_to_radiod__unpack(NULL, len, buf);
}

static void write_msg(int fd, const MrR2C *m)
{
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    if (n < 0 || write(fd, buf, (size_t)n) != n)
        _exit(90);
}

static void write_received(int fd, uint32_t port)
{
    Muudd__Radio__Received rx = MUUDD__RADIO__RECEIVED__INIT;
    rx.port = port;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED;
    m.received = &rx;
    write_msg(fd, &m);
}

static void write_claim_result(int fd, uint32_t req, int status)
{
    Muudd__Radio__ClaimResult cr = MUUDD__RADIO__CLAIM_RESULT__INIT;
    cr.req_id = req;
    cr.status = status;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT;
    m.claim_result = &cr;
    write_msg(fd, &m);
}

static void expect_port(mr_client_t *c, uint32_t port)
{
    MrR2C *m = mr_client_recv(c, 1000);
    ck_assert_ptr_nonnull(m);
    ck_assert_int_eq(m->msg_case, MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED);
    ck_assert_uint_eq(m->received->port, port);
    muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
}

START_TEST(test_side_messages_kept_in_order)
{
    int lfd = listen_on("order");
    pid_t pid = fork();
    if (pid == 0) {
        int fd = accept(lfd, NULL, NULL);
        MrC2R *req = read_msg(fd);
        if (!req || req->msg_case != MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_GET_STATS)
            _exit(91);
        write_received(fd, 1);
        write_received(fd, 2);
        Muudd__Radio__Stats st = MUUDD__RADIO__STATS__INIT;
        st.req_id = req->get_stats->req_id;
        char reason[] = "";
        st.reason = reason;
        MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
        m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_STATS;
        m.stats = &st;
        write_msg(fd, &m);
        write_received(fd, 3);
        MrC2R *x = read_msg(fd);
        _exit(x ? 92 : 0);
    }
    close(lfd);
    mr_client_t *c;
    ck_assert_int_eq(mr_client_connect(sock_path, &c), MR_OK);
    MrR2C *st = mr_client_get_stats(c, 2000);
    ck_assert_ptr_nonnull(st);
    muudd__radio__radiod_to_consumer__free_unpacked(st, NULL);
    expect_port(c, 1);
    expect_port(c, 2);
    expect_port(c, 3);
    ck_assert_uint_eq(mr_client_dropped(c), 0);
    mr_client_close(c);
    int status;
    waitpid(pid, &status, 0);
    ck_assert_int_eq(WEXITSTATUS(status), 0);
    unlink(sock_path);
}
END_TEST

START_TEST(test_recv_zero_timeout_sees_pending)
{
    int lfd = listen_on("zero");
    pid_t pid = fork();
    if (pid == 0) {
        int fd = accept(lfd, NULL, NULL);
        write_received(fd, 7);
        MrC2R *x = read_msg(fd);
        _exit(x ? 92 : 0);
    }
    close(lfd);
    mr_client_t *c;
    ck_assert_int_eq(mr_client_connect(sock_path, &c), MR_OK);
    usleep(200000);
    expect_port(c, 7);
    ck_assert_ptr_null(mr_client_recv(c, 0));
    mr_client_close(c);
    waitpid(pid, NULL, 0);
    unlink(sock_path);
}
END_TEST

/* Deliberately does not ignore SIGPIPE: a plain write would kill the test. */
START_TEST(test_send_after_server_close_is_eio)
{
    int lfd = listen_on("pipe");
    pid_t pid = fork();
    if (pid == 0) {
        int fd = accept(lfd, NULL, NULL);
        close(fd);
        _exit(0);
    }
    close(lfd);
    mr_client_t *c;
    ck_assert_int_eq(mr_client_connect(sock_path, &c), MR_OK);
    waitpid(pid, NULL, 0);
    Muudd__Radio__GetStats g = MUUDD__RADIO__GET_STATS__INIT;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_GET_STATS;
    m.get_stats = &g;
    ck_assert_int_eq(mr_client_send(c, &m), MR_EIO);
    ck_assert_int_eq(mr_client_send(c, &m), MR_EIO);
    mr_client_close(c);
    unlink(sock_path);
}
END_TEST

START_TEST(test_reconnect_keeps_failed_claims)
{
    int lfd = listen_on("claims");
    pid_t pid = fork();
    if (pid == 0) {
        /* Connection 0 accepts both claims, 1 refuses port 6, 2 accepts both.
         * Every connection must see both claims. */
        for (int conn = 0; conn < 3; conn++) {
            int fd = accept(lfd, NULL, NULL);
            int seen = 0;
            MrC2R *m;
            while ((m = read_msg(fd)) != NULL) {
                if (m->msg_case != MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM)
                    _exit(93);
                int refuse = conn == 1 && m->claim->port == 6;
                write_claim_result(fd, m->claim->req_id,
                                   refuse ? MUUDD__RADIO__CLAIM_STATUS__PORT_TAKEN
                                          : MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK);
                seen++;
            }
            close(fd);
            if (seen != 2)
                _exit(94);
        }
        _exit(0);
    }
    close(lfd);
    mr_client_t *c;
    ck_assert_int_eq(mr_client_connect(sock_path, &c), MR_OK);
    ck_assert_int_eq(mr_client_claim(c, 5, MUUDD__RADIO__POLICY__RADIOD_BRIDGED, 2000), 0);
    ck_assert_int_eq(mr_client_claim(c, 6, MUUDD__RADIO__POLICY__RADIOD_BRIDGED, 2000), 0);
    ck_assert_int_eq(mr_client_reconnect(c, 2000), MR_ERR);
    ck_assert_int_eq(mr_client_reconnect(c, 2000), MR_OK);
    mr_client_close(c);
    int status;
    waitpid(pid, &status, 0);
    ck_assert_int_eq(WEXITSTATUS(status), 0);
    unlink(sock_path);
}
END_TEST

int main(void)
{
    Suite *s = suite_create("client");
    TCase *tc = tcase_create("client");
    tcase_set_timeout(tc, 30);
    tcase_add_test(tc, test_side_messages_kept_in_order);
    tcase_add_test(tc, test_recv_zero_timeout_sees_pending);
    tcase_add_test(tc, test_send_after_server_close_is_eio);
    tcase_add_test(tc, test_reconnect_keeps_failed_claims);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
