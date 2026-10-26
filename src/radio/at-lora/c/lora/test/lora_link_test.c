/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#include "mr_framing.h"
#include "mr_proto.h"
#include "lora/lora_link.h"

typedef struct {
    char path[108];
    int claim_status;        /* MUUDD__RADIO__CLAIM_STATUS__* */
    int die_after_sends;     /* close after this many Sends; 0 never */
    bool silent_sends;       /* never answer a Send */
    bool refuse_sends;       /* answer a Send with REFUSED / OVER_BUDGET */
    uint32_t status_mtu;     /* the RadioStatus MTU for the port; 0 means 233 */
    bool send_rx;            /* after the claim, deliver one frame on the port and one elsewhere */
    int sends;
    uint8_t payloads[8][256];
    size_t lens[8];
    bool want_ack[8];
    uint32_t dest[8];
    int lfd, cfd;
    pthread_t th;
    volatile bool stop;
} fake_t;

static void _reply(fake_t *f, MrR2C *m)
{
    uint8_t buf[2 + MR_FRAME_MAX];
    int n = mr_frame_pack(&m->base, buf, sizeof(buf));
    if (n > 0)
        (void)mr_frame_write_fd(f->cfd, buf, (size_t)n);
}

static void _status(fake_t *f, uint32_t port)
{
    MrCaps caps = MUUDD__RADIO__CAPS__INIT;
    char name[] = "stub";
    caps.name = name;
    caps.node_addr = 0xA01;
    caps.mtu = 233;
    Muudd__Radio__PortMtu pm = MUUDD__RADIO__PORT_MTU__INIT, *pms[1] = {&pm};
    pm.port = port;
    pm.mtu = f->status_mtu ? f->status_mtu : 233;
    MrRadioStatus st = MUUDD__RADIO__RADIO_STATUS__INIT;
    st.state = MUUDD__RADIO__RADIO_STATE__STATE_UP;
    st.caps = &caps;
    st.n_mtus = 1;
    st.mtus = pms;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RADIO_STATUS;
    m.radio_status = &st;
    _reply(f, &m);
}

static void _received(fake_t *f, uint32_t port, uint8_t byte)
{
    MrReceived r = MUUDD__RADIO__RECEIVED__INIT;
    MrMeta meta = MUUDD__RADIO__META__INIT;
    r.src = 0xB02;
    r.dest = 0xA01;
    r.port = port;
    r.payload.data = &byte;
    r.payload.len = 1;
    r.meta = &meta;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED;
    m.received = &r;
    _reply(f, &m);
}

static int _on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    fake_t *f = ctx;
    MrC2R *in = muudd__radio__consumer_to_radiod__unpack(NULL, n, buf);
    if (in == NULL)
        return -1;
    if (in->msg_case == MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM) {
        MrClaimResult cr = MUUDD__RADIO__CLAIM_RESULT__INIT;
        cr.req_id = in->claim->req_id;
        cr.status = f->claim_status;
        cr.mtu = 233;
        MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
        m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT;
        m.claim_result = &cr;
        _reply(f, &m);
        if (f->claim_status == MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK) {
            _status(f, in->claim->port);
            if (f->send_rx) {
                _received(f, in->claim->port, 0x5a);
                _received(f, in->claim->port + 1, 0x77);
            }
        }
    } else if (in->msg_case == MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND) {
        int i = f->sends++;
        if (i < 8) {
            memcpy(f->payloads[i], in->send->payload.data, in->send->payload.len);
            f->lens[i] = in->send->payload.len;
            f->want_ack[i] = in->send->want_ack;
            f->dest[i] = in->send->dest;
        }
        if (f->die_after_sends && f->sends >= f->die_after_sends) {
            muudd__radio__consumer_to_radiod__free_unpacked(in, NULL);
            return -1;      /* the server closes the connection */
        }
        if (!f->silent_sends) {
            MrSendResult sr = MUUDD__RADIO__SEND_RESULT__INIT;
            sr.req_id = in->send->req_id;
            sr.status = f->refuse_sends ? MUUDD__RADIO__SEND_STATUS__REFUSED
                                        : MUUDD__RADIO__SEND_STATUS__SENT;
            sr.reason = MUUDD__RADIO__REFUSE_REASON__OVER_BUDGET;
            MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
            m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT;
            m.send_result = &sr;
            _reply(f, &m);
        }
    }
    muudd__radio__consumer_to_radiod__free_unpacked(in, NULL);
    return 0;
}

static void *_serve(void *arg)
{
    fake_t *f = arg;
    while (!f->stop) {
        f->cfd = accept(f->lfd, NULL, NULL);
        if (f->cfd < 0)
            return NULL;
        mr_decoder_t dec;
        mr_decoder_init(&dec);
        uint8_t buf[4096];
        ssize_t r;
        while ((r = read(f->cfd, buf, sizeof(buf))) > 0)
            if (mr_decoder_feed(&dec, buf, (size_t)r, _on_frame, f) != 0)
                break;
        close(f->cfd);
        f->cfd = -1;
        if (f->die_after_sends)
            return NULL;    /* stays down until the test restarts it */
    }
    return NULL;
}

static void _fake_start(fake_t *f)
{
    f->lfd = socket(AF_UNIX, SOCK_STREAM, 0);
    struct sockaddr_un sa = {.sun_family = AF_UNIX};
    snprintf(sa.sun_path, sizeof(sa.sun_path), "%s", f->path);
    unlink(f->path);
    ck_assert_int_eq(bind(f->lfd, (struct sockaddr *)&sa, sizeof(sa)), 0);
    ck_assert_int_eq(listen(f->lfd, 4), 0);
    f->stop = false;
    pthread_create(&f->th, NULL, _serve, f);
}

static void _fake_stop(fake_t *f)
{
    f->stop = true;
    shutdown(f->lfd, SHUT_RDWR);
    close(f->lfd);
    if (f->cfd >= 0)
        shutdown(f->cfd, SHUT_RDWR);
    pthread_join(f->th, NULL);
    unlink(f->path);
}

static int g_errors, g_warns;
static char g_last_error[256];
static void _log(void *arg, int level, const char *msg)
{
    (void)arg;
    if (level == 1 && strstr(msg, "below") != NULL)
        g_warns++;
    if (level == 2) {
        g_errors++;
        snprintf(g_last_error, sizeof(g_last_error), "%s", msg);
    }
}

static bool _wait(lora_link_t *l, bool want_up, int ms)
{
    lora_link_state_t s;
    for (int i = 0; i < ms / 20; i++) {
        lora_link_state(l, &s);
        if (s.up == want_up)
            return true;
        usleep(20000);
    }
    return false;
}

static void _fake_init(fake_t *f, const char *name)
{
    memset(f, 0, sizeof(*f));
    snprintf(f->path, sizeof(f->path), "/tmp/lora_link_%d_%s.sock", (int)getpid(), name);
    f->claim_status = MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK;
    f->cfd = -1;
}

static lora_frags_t _frags(int n)
{
    uint8_t msg[3 * 229];
    memset(msg, 0x42, sizeof(msg));
    lora_frags_t fr;
    ck_assert_int_eq(lora_fragment(msg, (size_t)(n * 229), LORA_T_PEER, 9, NULL, 233, 16, &fr), 0);
    return fr;
}

DEFINE_TEST(test_claims_reports_up_and_sends_in_order)
{
    fake_t f;
    _fake_init(&f, "up");
    f.send_rx = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    lora_link_state_t s;
    lora_link_state(l, &s);
    ck_assert_int_eq(s.node_addr, 0xA01);
    ck_assert_int_eq(s.mtu, 233);
    lora_frags_t fr = _frags(3);
    ck_assert_int_eq(lora_link_submit(l, 0xB02, &fr, true), 0);
    lora_link_result_t res = {0};
    size_t got = 0;
    for (int i = 0; i < 100 && (got = lora_link_poll_results(l, &res, 1)) == 0; i++)
        usleep(20000);
    ck_assert_int_eq((int)got, 1);
    ck_assert(res.ok);
    ck_assert_int_eq((int)res.sent, 3);
    ck_assert_int_eq(f.sends, 3);
    ck_assert(f.want_ack[0] && f.want_ack[2]);
    ck_assert_int_eq(f.dest[1], 0xB02);
    ck_assert_int_eq(f.payloads[1][3], 0x12);   /* fragment 1 of 3 */
    lora_link_rx_t rx[4] = {{0}};
    size_t n = lora_link_poll_rx(l, rx, 4);
    ck_assert_int_eq((int)n, 1);                 /* the other port's frame is ignored */
    ck_assert_int_eq(rx[0].src, 0xB02);
    ck_assert_int_eq(rx[0].payload[0], 0x5a);
    free(rx[0].payload);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_claim_refused_stays_inert)
{
    fake_t f;
    _fake_init(&f, "refused");
    f.claim_status = MUUDD__RADIO__CLAIM_STATUS__PORT_UNCONFIGURED;
    _fake_start(&f);
    g_errors = 0;
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    usleep(500000);
    lora_link_state_t s;
    lora_link_state(l, &s);
    ck_assert(s.connected && !s.claimed && !s.up);
    ck_assert_int_eq(g_errors, 1);
    ck_assert(strstr(g_last_error, "port 10") != NULL);
    lora_frags_t fr = _frags(1);
    ck_assert_int_eq(lora_link_submit(l, 0xB02, &fr, true), -1);
    lora_frags_free(&fr);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_server_dies_mid_job)
{
    fake_t f;
    _fake_init(&f, "dies");
    f.die_after_sends = 1;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    lora_frags_t fr = _frags(3);
    ck_assert_int_eq(lora_link_submit(l, 0xB02, &fr, true), 0);
    lora_link_result_t res = {0};
    size_t got = 0;
    for (int i = 0; i < 200 && (got = lora_link_poll_results(l, &res, 1)) == 0; i++)
        usleep(20000);
    ck_assert_int_eq((int)got, 1);
    ck_assert(!res.ok);
    ck_assert_str_eq(res.reason, "RADIO_DOWN");
    ck_assert(_wait(l, false, 2000));
    _fake_stop(&f);
    _fake_init(&f, "dies");
    _fake_start(&f);
    ck_assert(_wait(l, true, 5000));      /* reconnected and re-claimed */
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_job_queue_full_refuses)
{
    fake_t f;
    _fake_init(&f, "full");
    f.silent_sends = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    int accepted = 0;
    for (int i = 0; i < LORA_LINK_JOBS_MAX + 4; i++) {
        lora_frags_t fr = _frags(1);
        if (lora_link_submit(l, 0xB02, &fr, false) == 0)
            accepted++;
        else
            lora_frags_free(&fr);
    }
    ck_assert_int_eq(accepted, LORA_LINK_JOBS_MAX);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_refused_send_ends_with_the_reason)
{
    fake_t f;
    _fake_init(&f, "refuse");
    f.refuse_sends = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    lora_frags_t fr = _frags(3);
    ck_assert_int_eq(lora_link_submit(l, 0xB02, &fr, true), 0);
    lora_link_result_t res = {0};
    size_t got = 0;
    for (int i = 0; i < 200 && (got = lora_link_poll_results(l, &res, 1)) == 0; i++)
        usleep(20000);
    ck_assert_int_eq((int)got, 1);
    ck_assert(!res.ok);
    ck_assert_str_eq(res.reason, "OVER_BUDGET");
    ck_assert_int_eq((int)res.sent, 0);
    ck_assert_int_eq((int)res.total, 3);
    ck_assert_int_eq(f.sends, 1);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_status_mtu_replaces_the_claim_mtu)
{
    fake_t f;
    _fake_init(&f, "mtu");
    f.status_mtu = 100;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    lora_link_state_t s;
    lora_link_state(l, &s);
    ck_assert_int_eq(s.mtu, 100);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_mtu_below_floor_stays_off_and_warns_once)
{
    fake_t f;
    _fake_init(&f, "lowmtu");
    f.status_mtu = 32;
    _fake_start(&f);
    g_warns = 0;
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(!_wait(l, true, 600));
    lora_link_state_t s;
    lora_link_state(l, &s);
    ck_assert(s.claimed && !s.up);
    ck_assert_int_eq(g_warns, 1);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

RUN_TESTS(LORA_LINK,
          test_claims_reports_up_and_sends_in_order,
          test_claim_refused_stays_inert,
          test_server_dies_mid_job,
          test_job_queue_full_refuses,
          test_refused_send_ends_with_the_reason,
          test_status_mtu_replaces_the_claim_mtu,
          test_mtu_below_floor_stays_off_and_warns_once)
