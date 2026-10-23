/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <poll.h>
#include <sodium.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "core_priv.h"
#include "mr_ipc.h"
#include "mr_test.h"

#define CFG "{\"node_addr\": \"auto\", \"driver\": {\"exec\": \"/bin/true\"}}"

static mr_config_t g_cfg;
static mr_ipc_t g_ipc;
static mr_core_t *g_core;
static char g_path[108];

static void to_consumer(void *ctx, int cid, const MrR2C *m)
{
    (void)ctx;
    mr_ipc_send(&g_ipc, cid, m);
}
static void to_driver(void *ctx, const MrR2D *m) { (void)ctx; (void)m; }
static void to_peer(void *ctx, int peer, const MrDatagram *d) { (void)ctx; (void)peer; (void)d; }
static uint64_t wall(void *ctx) { (void)ctx; return 1700000000000ull; }

static void setup(const char *tag)
{
    char err[256];
    ck_assert_int_eq(mr_config_parse(CFG, &g_cfg, err, sizeof err), MR_OK);
    mr_keypair_t kp;
    crypto_box_keypair(kp.pk, kp.sk);
    mr_core_io_t io = {NULL, to_consumer, to_driver, to_peer, wall, NULL};
    ck_assert_int_eq(mr_core_create(&g_cfg, &kp, NULL, &io, &g_core), MR_OK);
    snprintf(g_path, sizeof g_path, "/tmp/mr_ipc_test_%d_%s.sock", (int)getpid(), tag);
    unlink(g_path);
    ck_assert_int_eq(mr_ipc_open(&g_ipc, g_path, g_core), MR_OK);
}

static void teardown(void)
{
    mr_ipc_close(&g_ipc);
    mr_core_free(g_core);
}

static int dial(const char *path)
{
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof sa.sun_path, "%s", path);
    if (connect(fd, (struct sockaddr *)&sa, sizeof sa) != 0) {
        close(fd);
        return -1;
    }
    return fd;
}

static void pump(void)
{
    struct pollfd fds[1 + MR_MAX_CONSUMERS];
    int n = mr_ipc_poll_fds(&g_ipc, fds, 1 + MR_MAX_CONSUMERS);
    poll(fds, (nfds_t)n, 50);
    mr_ipc_handle(&g_ipc, fds, n, mr_now_ms());
}

static void send_received(int cid, uint32_t src)
{
    static uint8_t payload[200];
    MrReceived rv = MUUDD__RADIO__RECEIVED__INIT;
    rv.src = src;
    rv.dest = MR_BROADCAST;
    rv.port = 16;
    rv.payload.data = payload;
    rv.payload.len = sizeof payload;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED;
    m.received = &rv;
    mr_ipc_send(&g_ipc, cid, &m);
}

typedef struct {
    bool got_reply;
    int frames;
} tally_t;

static int count_frame(void *p, const uint8_t *b, size_t n)
{
    tally_t *t = p;
    MrR2C *m = muudd__radio__radiod_to_consumer__unpack(NULL, n, b);
    ck_assert_ptr_nonnull(m);
    if (m->msg_case == MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT && m->send_result->req_id == 4242)
        t->got_reply = true;
    t->frames++;
    muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
    return 0;
}

START_TEST(test_overflow_drops_received_not_replies)
{
    setup("overflow");
    int fd = dial(g_path);
    ck_assert_int_ge(fd, 0);
    pump();
    ck_assert_int_ge(g_ipc.clients[0].fd, 0);
    /* fill the socket buffers until frames start to queue */
    uint32_t src = 1;
    while (g_ipc.clients[0].count == 0)
        send_received(0, src++);
    mr_core_reply(g_core, 0, 4242, MUUDD__RADIO__SEND_STATUS__SENT, 0, 0);
    for (int i = 0; i < 3 * MR_IPC_QUEUE; i++)
        send_received(0, src++);
    ck_assert_uint_eq(g_ipc.clients[0].count, MR_IPC_QUEUE);
    ck_assert_uint_ge(g_core->drops.queue_overflow, 2 * MR_IPC_QUEUE);

    /* drain everything: the reply must still be there */
    mr_decoder_t dec;
    mr_decoder_init(&dec);
    tally_t tally = {false, 0};
    for (int idle = 0; idle < 20;) {
        pump();
        uint8_t buf[65536];
        ssize_t r = recv(fd, buf, sizeof buf, MSG_DONTWAIT);
        if (r <= 0) {
            idle++;
            continue;
        }
        idle = 0;
        ck_assert_int_eq(mr_decoder_feed(&dec, buf, (size_t)r, count_frame, &tally), MR_OK);
    }
    ck_assert(tally.got_reply);
    ck_assert_int_gt(tally.frames, MR_IPC_QUEUE);
    close(fd);
    teardown();
}
END_TEST

START_TEST(test_unpackable_message_counted)
{
    setup("toobig");
    int fd = dial(g_path);
    pump();
    static uint8_t big[MR_FRAME_MAX + 100];
    MrReceived rv = MUUDD__RADIO__RECEIVED__INIT;
    rv.port = 16;
    rv.payload.data = big;
    rv.payload.len = sizeof big;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED;
    m.received = &rv;
    mr_ipc_send(&g_ipc, 0, &m);
    mr_ipc_send(&g_ipc, 0, &m);
    ck_assert_uint_eq(g_ipc.pack_failures, 2);
    close(fd);
    teardown();
}
END_TEST

START_TEST(test_refuses_a_live_socket)
{
    setup("live");
    static mr_ipc_t second;
    errno = 0;
    ck_assert_int_ne(mr_ipc_open(&second, g_path, g_core), MR_OK);
    ck_assert_int_eq(errno, EADDRINUSE);
    int fd = dial(g_path);   /* the first listener is untouched */
    ck_assert_int_ge(fd, 0);
    close(fd);
    teardown();
}
END_TEST

START_TEST(test_replaces_a_stale_socket)
{
    char path[108];
    snprintf(path, sizeof path, "/tmp/mr_ipc_test_%d_stale.sock", (int)getpid());
    unlink(path);
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof sa.sun_path, "%s", path);
    ck_assert_int_eq(bind(fd, (struct sockaddr *)&sa, sizeof sa), 0);
    close(fd);   /* the file stays behind, but nobody listens */
    char err[256];
    ck_assert_int_eq(mr_config_parse(CFG, &g_cfg, err, sizeof err), MR_OK);
    mr_keypair_t kp;
    crypto_box_keypair(kp.pk, kp.sk);
    mr_core_io_t io = {NULL, to_consumer, to_driver, to_peer, wall, NULL};
    ck_assert_int_eq(mr_core_create(&g_cfg, &kp, NULL, &io, &g_core), MR_OK);
    ck_assert_int_eq(mr_ipc_open(&g_ipc, path, g_core), MR_OK);
    teardown();
}
END_TEST

int main(void)
{
    if (sodium_init() < 0)
        return 1;
    Suite *s = suite_create("ipc");
    TCase *tc = tcase_create("ipc");
    tcase_add_test(tc, test_overflow_drops_received_not_replies);
    tcase_add_test(tc, test_unpackable_message_counted);
    tcase_add_test(tc, test_refuses_a_live_socket);
    tcase_add_test(tc, test_replaces_a_stale_socket);
    suite_add_tcase(s, tc);
    return mr_run_suite(s);
}
