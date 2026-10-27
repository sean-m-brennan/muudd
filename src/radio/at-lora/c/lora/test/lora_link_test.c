/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <pthread.h>
#include <stddef.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <linux/sockios.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#include "mr_framing.h"
#include "mr_proto.h"
#include "lora/lora_link.h"

/* The knobs are set before _fake_start and only read by the server thread.
 * What the server records is under mu (sends is atomic as well, for polling);
 * cfd and stop are shared with _fake_stop, so they are atomic. */
typedef struct {
    char path[108];
    int claim_status;        /* MUUDD__RADIO__CLAIM_STATUS__* */
    int die_after_sends;     /* close after this many Sends; 0 never */
    bool silent_sends;       /* never answer a Send */
    bool refuse_sends;       /* answer a Send with REFUSED / OVER_BUDGET */
    uint32_t status_mtu;     /* the RadioStatus MTU for the port; 0 means 233 */
    bool send_rx;            /* after the claim, deliver one frame on the port and one elsewhere */
    int rx_burst;            /* after the claim, deliver this many frames on the port */
    bool stall;              /* on the first Send, answer SENT for the next STALL_RESULTS
                                request ids, then stop reading */
    pthread_mutex_t mu;
    atomic_int sends;
    int claims;
    uint8_t payloads[8][256];
    size_t lens[8];
    bool want_ack[8];
    uint32_t dest[8];
    int lfd;
    atomic_int cfd;
    pthread_t th;
    atomic_bool stop;
} fake_t;

#define STALL_RESULTS 40     /* fewer than the client's receive queue holds */
#define FAKE_STALLED  1      /* _on_frame's return: stop reading this connection */

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

/* SEND_RESULT SENT for req_id, written without blocking: the link may be
 * stuck in its own send and not reading. */
static void _sent_nowait(fake_t *f, uint32_t req_id)
{
    MrSendResult sr = MUUDD__RADIO__SEND_RESULT__INIT;
    sr.req_id = req_id;
    sr.status = MUUDD__RADIO__SEND_STATUS__SENT;
    MrR2C m = MUUDD__RADIO__RADIOD_TO_CONSUMER__INIT;
    m.msg_case = MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT;
    m.send_result = &sr;
    uint8_t buf[2 + MR_FRAME_MAX];
    int n = mr_frame_pack(&m.base, buf, sizeof(buf));
    if (n > 0)
        (void)send(f->cfd, buf, (size_t)n, MSG_DONTWAIT | MSG_NOSIGNAL);
}

/* Waits (up to 2 s) until the link has read everything the fake wrote. */
static void _wait_drained(fake_t *f)
{
    for (int i = 0; i < 2000; i++) {
        int unread = 0;
        if (ioctl(f->cfd, SIOCOUTQ, &unread) != 0 || unread == 0)
            return;
        usleep(1000);
    }
}

static int _on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    fake_t *f = ctx;
    MrC2R *in = muudd__radio__consumer_to_radiod__unpack(NULL, n, buf);
    if (in == NULL)
        return -1;
    if (in->msg_case == MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM) {
        pthread_mutex_lock(&f->mu);
        f->claims++;
        pthread_mutex_unlock(&f->mu);
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
            /* Each frame waits until the link has read the last, so the
             * client's own queue never holds more than one and only the link's
             * queue can overflow, however the link thread is scheduled. */
            for (int i = 0; i < f->rx_burst; i++) {
                _wait_drained(f);
                _received(f, in->claim->port, (uint8_t)i);
            }
        }
    } else if (in->msg_case == MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND) {
        pthread_mutex_lock(&f->mu);
        int i = atomic_fetch_add(&f->sends, 1);
        if (i < 8) {
            memcpy(f->payloads[i], in->send->payload.data, in->send->payload.len);
            f->lens[i] = in->send->payload.len;
            f->want_ack[i] = in->send->want_ack;
            f->dest[i] = in->send->dest;
        }
        pthread_mutex_unlock(&f->mu);
        if (f->stall) {
            /* The link sends one fragment per answer and request ids count up
             * by one, so these answers keep it sending without being read. */
            for (uint32_t k = 0; k < STALL_RESULTS; k++)
                _sent_nowait(f, in->send->req_id + k);
            muudd__radio__consumer_to_radiod__free_unpacked(in, NULL);
            return FAKE_STALLED;
        }
        if (f->die_after_sends && i + 1 >= f->die_after_sends) {
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
    while (!atomic_load(&f->stop)) {
        int fd = accept(f->lfd, NULL, NULL);
        if (fd < 0)
            return NULL;
        atomic_store(&f->cfd, fd);
        mr_decoder_t dec;
        mr_decoder_init(&dec);
        uint8_t buf[4096];
        ssize_t r;
        int rc = 0;
        while ((r = read(fd, buf, sizeof(buf))) > 0)
            if ((rc = mr_decoder_feed(&dec, buf, (size_t)r, _on_frame, f)) != 0)
                break;
        while (rc == FAKE_STALLED && !atomic_load(&f->stop))
            usleep(10000);       /* connected, never reading */
        atomic_store(&f->cfd, -1);
        close(fd);
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
    atomic_store(&f->stop, false);
    pthread_create(&f->th, NULL, _serve, f);
}

static void _fake_stop(fake_t *f)
{
    atomic_store(&f->stop, true);
    shutdown(f->lfd, SHUT_RDWR);
    close(f->lfd);
    int fd = atomic_load(&f->cfd);
    if (fd >= 0)
        shutdown(fd, SHUT_RDWR);
    pthread_join(f->th, NULL);
    pthread_mutex_destroy(&f->mu);
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

/* Waits up to ms for the link to be connected with one claim refusal logged. */
static bool _wait_refused(lora_link_t *l, int ms)
{
    lora_link_state_t s;
    for (int i = 0; i < ms / 20; i++) {
        lora_link_state(l, &s);
        if (s.connected && g_errors == 1)
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
    pthread_mutex_init(&f->mu, NULL);
    atomic_init(&f->sends, 0);
    atomic_init(&f->cfd, -1);
    atomic_init(&f->stop, false);
}

static int _fake_claims(fake_t *f)
{
    pthread_mutex_lock(&f->mu);
    int n = f->claims;
    pthread_mutex_unlock(&f->mu);
    return n;
}

static double _ms_since(const struct timespec *t0)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)(t.tv_sec - t0->tv_sec) * 1e3 + (double)(t.tv_nsec - t0->tv_nsec) / 1e6;
}

/* Waits up to ms for one result. */
static size_t _wait_result(lora_link_t *l, lora_link_result_t *res, int ms)
{
    size_t got = 0;
    for (int i = 0; i < ms / 20 && (got = lora_link_poll_results(l, res, 1)) == 0; i++)
        usleep(20000);
    return got;
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
    pthread_mutex_lock(&f.mu);
    ck_assert_int_eq(atomic_load(&f.sends), 3);
    ck_assert(f.want_ack[0] && f.want_ack[2]);
    ck_assert_int_eq(f.dest[1], 0xB02);
    ck_assert_int_eq(f.payloads[1][3], 0x12);   /* fragment 1 of 3 */
    pthread_mutex_unlock(&f.mu);
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
    ck_assert(_wait_refused(l, 3000));
    usleep(200000);                               /* and it stays that way */
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
    ck_assert_int_eq(atomic_load(&f.sends), 1);
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

DEFINE_TEST(test_silent_send_ends_with_timeout)
{
    fake_t f;
    _fake_init(&f, "timeout");
    f.silent_sends = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    lora_frags_t fr = _frags(2);
    struct timespec t0;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    ck_assert_int_eq(lora_link_submit(l, 0xB02, &fr, true), 0);
    lora_link_result_t res = {0};
    ck_assert_int_eq((int)_wait_result(l, &res, 4000), 1);
    double ms = _ms_since(&t0);
    ck_assert(!res.ok);
    ck_assert_str_eq(res.reason, "TIMEOUT");
    ck_assert_int_eq((int)res.sent, 0);
    ck_assert_int_eq((int)res.total, 2);
    ck_assert(ms >= 1900.0);              /* two seconds without a SEND_RESULT */
    ck_assert_int_eq(atomic_load(&f.sends), 1);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_rx_overflow_drops_the_oldest_and_counts)
{
    fake_t f;
    _fake_init(&f, "rxfull");
    f.rx_burst = LORA_LINK_RX_MAX + 6;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    for (int i = 0; i < 150 && lora_link_rx_dropped(l) < 6; i++)
        usleep(20000);
    usleep(100000);                       /* nothing more arrives */
    ck_assert_int_eq((int)lora_link_rx_dropped(l), 6);
    lora_link_rx_t rx[LORA_LINK_RX_MAX + 6];
    size_t n = lora_link_poll_rx(l, rx, LORA_LINK_RX_MAX + 6);
    ck_assert_int_eq((int)n, LORA_LINK_RX_MAX);
    if (n == LORA_LINK_RX_MAX) {
        ck_assert_int_eq(rx[0].payload[0], 6);
        ck_assert_int_eq(rx[n - 1].payload[0], LORA_LINK_RX_MAX + 5);
    }
    for (size_t i = 0; i < n; i++)
        free(rx[i].payload);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_results_nobody_polls_are_dropped_and_counted)
{
    fake_t f;
    _fake_init(&f, "resfull");
    f.refuse_sends = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    const int total = LORA_LINK_JOBS_MAX * 2 + 8;
    int submitted = 0;
    for (int i = 0; i < 500 && submitted < total; i++) {
        lora_frags_t fr = _frags(1);
        if (lora_link_submit(l, 0xB02, &fr, false) == 0) {
            submitted++;
        } else {
            lora_frags_free(&fr);
            usleep(10000);
        }
    }
    ck_assert_int_eq(submitted, total);
    for (int i = 0; i < 150 && lora_link_results_dropped(l) < 8; i++)
        usleep(20000);
    ck_assert_int_eq((int)lora_link_results_dropped(l), 8);
    lora_link_result_t res[LORA_LINK_JOBS_MAX * 2 + 8];
    ck_assert_int_eq((int)lora_link_poll_results(l, res, LORA_LINK_JOBS_MAX * 2 + 8),
                     LORA_LINK_JOBS_MAX * 2);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_refused_claim_retried_only_after_reconnect)
{
    fake_t f;
    _fake_init(&f, "reclaim");
    f.claim_status = MUUDD__RADIO__CLAIM_STATUS__PORT_UNCONFIGURED;
    _fake_start(&f);
    g_errors = 0;
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait_refused(l, 3000));
    usleep(300000);                               /* time for a second claim, if any */
    lora_link_state_t s;
    lora_link_state(l, &s);
    ck_assert(s.connected && !s.claimed);
    ck_assert_int_eq(_fake_claims(&f), 1);       /* no re-claim on a live connection */
    ck_assert_int_eq(g_errors, 1);
    _fake_stop(&f);
    _fake_init(&f, "reclaim");                    /* the port is now configured */
    _fake_start(&f);
    ck_assert(_wait(l, true, 5000));
    ck_assert_int_eq(_fake_claims(&f), 1);       /* exactly one attempt after the restart */
    ck_assert_int_eq(g_errors, 1);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

/* Shrinks the send buffer of the link's socket to radiod, found as the socket
 * in this process whose peer is the fake's path; returns how many matched. */
static int _shrink_link_sndbuf(const fake_t *f)
{
    int found = 0;
    for (int fd = 3; fd < 1024; fd++) {
        struct sockaddr_un sa;
        socklen_t len = sizeof(sa);
        if (getpeername(fd, (struct sockaddr *)&sa, &len) != 0 || sa.sun_family != AF_UNIX
            || len <= offsetof(struct sockaddr_un, sun_path) || strcmp(sa.sun_path, f->path) != 0)
            continue;
        int sz = 1;                               /* the kernel raises it to its minimum */
        if (setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &sz, sizeof(sz)) == 0)
            found++;
    }
    return found;
}

/* The fd of the link's socket to radiod, or -1. */
static int _link_fd(const fake_t *f)
{
    for (int fd = 3; fd < 1024; fd++) {
        struct sockaddr_un sa;
        socklen_t len = sizeof(sa);
        if (getpeername(fd, (struct sockaddr *)&sa, &len) == 0 && sa.sun_family == AF_UNIX
            && len > offsetof(struct sockaddr_un, sun_path) && strcmp(sa.sun_path, f->path) == 0)
            return fd;
    }
    return -1;
}

/* True once the link is blocked in a send on fd: still connected, with unread
 * bytes in its socket that have stopped growing for 60 ms. The fake answers
 * far more fragments than the shrunken socket holds, so the link stops adding
 * bytes only because the socket is full. */
static bool _wait_send_blocked(lora_link_t *l, int fd, int ms)
{
    int last = -1, still = 0;
    for (int i = 0; i < ms / 10; i++) {
        int unread = 0;
        lora_link_state_t s;
        lora_link_state(l, &s);
        if (!s.connected || ioctl(fd, SIOCOUTQ, &unread) != 0)
            return false;
        still = (unread > 0 && unread == last) ? still + 1 : 0;
        if (still >= 6)
            return true;
        last = unread;
        usleep(10000);
    }
    return false;
}

/* radiod stops reading with the link mid-send: the send gives up within the
 * client's send timeout, so the link counts the connection lost and fails the
 * jobs instead of blocking for good. */
DEFINE_TEST(test_stalled_radiod_fails_the_send)
{
    fake_t f;
    _fake_init(&f, "stall");
    f.stall = true;
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    ck_assert_int_eq(_shrink_link_sndbuf(&f), 1);
    int submitted = 0;
    for (int i = 0; i < LORA_LINK_JOBS_MAX; i++) {
        lora_frags_t fr = _frags(3);
        if (lora_link_submit(l, 0xB02, &fr, false) == 0)
            submitted++;
        else
            lora_frags_free(&fr);
    }
    ck_assert_int_eq(submitted, LORA_LINK_JOBS_MAX);
    /* The jobs whose fragments fit in the socket finish; the rest fail. */
    int down = 0, results = 0;
    for (int i = 0; i < 150 && results < LORA_LINK_JOBS_MAX; i++) {
        lora_link_result_t res = {0};
        while (lora_link_poll_results(l, &res, 1) == 1) {
            results++;
            if (!res.ok && strcmp(res.reason, "RADIO_DOWN") == 0)
                down++;
        }
        usleep(20000);
    }
    ck_assert_int_eq(results, LORA_LINK_JOBS_MAX);
    ck_assert(down > 0);
    lora_link_stop(l);
    _fake_stop(&f);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_stop_returns_promptly)
{
    struct timespec t0;
    /* Up and idle. */
    fake_t f;
    _fake_init(&f, "stop");
    _fake_start(&f);
    lora_link_t *l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    clock_gettime(CLOCK_MONOTONIC, &t0);
    lora_link_stop(l);
    ck_assert(_ms_since(&t0) < 300.0);
    _fake_stop(&f);

    /* No radiod: between connection attempts. */
    l = lora_link_start("/tmp/lora_link_nobody_here.sock", 10, _log, NULL);
    usleep(300000);
    clock_gettime(CLOCK_MONOTONIC, &t0);
    lora_link_stop(l);
    ck_assert(_ms_since(&t0) < 300.0);

    /* radiod stopped reading and the link thread is waiting in a send. */
    _fake_init(&f, "stopstall");
    f.stall = true;
    _fake_start(&f);
    l = lora_link_start(f.path, 10, _log, NULL);
    ck_assert(_wait(l, true, 3000));
    ck_assert_int_eq(_shrink_link_sndbuf(&f), 1);
    int fd = _link_fd(&f);
    ck_assert(fd >= 0);
    for (int i = 0; i < LORA_LINK_JOBS_MAX; i++) {
        lora_frags_t fr = _frags(3);
        if (lora_link_submit(l, 0xB02, &fr, false) != 0)
            lora_frags_free(&fr);
    }
    ck_assert(_wait_send_blocked(l, fd, 2000));
    clock_gettime(CLOCK_MONOTONIC, &t0);
    lora_link_stop(l);
    ck_assert(_ms_since(&t0) < 1000.0);
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
          test_mtu_below_floor_stays_off_and_warns_once,
          test_silent_send_ends_with_timeout,
          test_rx_overflow_drops_the_oldest_and_counts,
          test_results_nobody_polls_are_dropped_and_counted,
          test_refused_claim_retried_only_after_reconnect,
          test_stalled_radiod_fails_the_send,
          test_stop_returns_promptly)
