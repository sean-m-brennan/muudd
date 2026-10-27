/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <pthread.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "mr_client.h"
#include "mr_proto.h"
#include "lora/lora_link.h"

typedef struct {
    uint32_t dest;
    bool want_ack;
    lora_frags_t frags;
} job_t;

struct lora_link {
    char path[108];
    uint32_t port;
    lora_log_fn log;
    void *log_arg;
    pthread_t th;
    pthread_mutex_t mu;
    atomic_bool stop;
    lora_link_state_t st;                    /* under mu */
    job_t jobs[LORA_LINK_JOBS_MAX];          /* under mu; jobs[0] is the one being sent */
    size_t n_jobs;
    lora_link_rx_t rx[LORA_LINK_RX_MAX];
    size_t n_rx;
    uint64_t rx_dropped;
    lora_link_result_t res[LORA_LINK_JOBS_MAX * 2];
    size_t n_res;
    uint64_t res_dropped;
};

static double _now(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

/* Log lines are formatted under mu and emitted after it is released, so the
 * callback never runs with the lock held. */
typedef struct { int level; char msg[256]; } pend_log_t;
typedef struct { pend_log_t e[4]; size_t n; } pend_logs_t;

static void _qlog(pend_logs_t *p, int level, const char *fmt, ...) __attribute__((format(printf, 3, 4)));
static void _qlog(pend_logs_t *p, int level, const char *fmt, ...)
{
    if (p->n >= sizeof(p->e) / sizeof(p->e[0]))
        return;
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(p->e[p->n].msg, sizeof(p->e[p->n].msg), fmt, ap);
    va_end(ap);
    p->e[p->n++].level = level;
}

static void _flush_logs(lora_link_t *l, pend_logs_t *p)
{
    for (size_t i = 0; i < p->n; i++)
        if (l->log != NULL)
            l->log(l->log_arg, p->e[i].level, p->e[i].msg);
    p->n = 0;
}

static void _recompute_up(lora_link_t *l)
{
    lora_link_state_t *s = &l->st;
    s->up = s->connected && s->claimed && s->radio_up && s->driver && s->node_addr != 0
            && s->mtu >= LORA_MIN_MTU;
}

/* Finish the job at the head of the queue (caller holds mu). */
static void _finish_head(lora_link_t *l, bool ok, size_t sent, const char *reason)
{
    if (l->n_jobs == 0)
        return;
    job_t *j = &l->jobs[0];
    if (l->n_res < sizeof(l->res) / sizeof(l->res[0])) {
        lora_link_result_t *r = &l->res[l->n_res++];
        r->ok = ok;
        r->sent = sent;
        r->total = j->frags.n;
        snprintf(r->reason, sizeof(r->reason), "%s", ok ? "" : reason);
    } else {
        l->res_dropped++;      /* nobody is polling; the newest result goes */
    }
    lora_frags_free(&j->frags);
    memmove(&l->jobs[0], &l->jobs[1], (l->n_jobs - 1) * sizeof(job_t));
    l->n_jobs--;
}

/* Fails every queued job; the head reports head_sent fragments already
 * confirmed (caller holds mu). */
static void _fail_all(lora_link_t *l, const char *reason, size_t head_sent)
{
    while (l->n_jobs > 0) {
        _finish_head(l, false, head_sent, reason);
        head_sent = 0;
    }
}

static int _send_claim(mr_client_t *c, uint32_t port, uint32_t req)
{
    Muudd__Radio__Claim cl = MUUDD__RADIO__CLAIM__INIT;
    cl.req_id = req;
    cl.port = port;
    cl.policy = MUUDD__RADIO__POLICY__CONSUMER_BRIDGED;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_CLAIM;
    m.claim = &cl;
    return mr_client_send(c, &m);
}

/* How long one write to radiod may wait for room in the socket. Past it the
 * connection counts as lost, which also bounds how long lora_link_stop waits
 * for a link thread caught in a send. */
#define LINK_SEND_TIMEOUT_MS 500

/* One fragment to put on the wire, copied out under mu so the (blocking) send
 * runs without it. The buffer stays valid because only the link thread ever
 * moves or frees jobs[0]; submit only appends behind it. */
typedef struct {
    bool want;
    uint32_t dest, req;
    bool want_ack;
    const uint8_t *buf;
    size_t len;
} tx_t;

static void _tx_set(tx_t *t, const job_t *j, size_t i, uint32_t req)
{
    t->want = true;
    t->dest = j->dest;
    t->req = req;
    t->want_ack = j->want_ack;
    t->buf = j->frags.buf[i];
    t->len = j->frags.len[i];
}

static int _send_frag(mr_client_t *c, uint32_t port, const tx_t *t)
{
    MrSend s = MUUDD__RADIO__SEND__INIT;
    s.req_id = t->req;
    s.dest = t->dest;
    s.port = port;
    s.payload.data = (uint8_t *)t->buf;
    s.payload.len = t->len;
    s.want_ack = t->want_ack;
    MrC2R m = MUUDD__RADIO__CONSUMER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__CONSUMER_TO_RADIOD__MSG_SEND;
    m.send = &s;
    return mr_client_send(c, &m);
}

/* Notes a port MTU below the floor, once per connection (caller holds mu). */
static void _check_mtu(lora_link_t *l, bool *warned, pend_logs_t *pl)
{
    if (*warned || l->st.mtu == 0 || l->st.mtu >= LORA_MIN_MTU)
        return;
    *warned = true;
    _qlog(pl, 1, "lora: radiod port %u has MTU %u, below %u; LoRa stays off",
          l->port, l->st.mtu, (unsigned)LORA_MIN_MTU);
}

static void *_run(void *arg)
{
    lora_link_t *l = arg;
    mr_client_t *c = NULL;
    double backoff = 0.1, next_try = 0;
    uint32_t req = 0, claim_req = 0, frag_req = 0;
    size_t frag_idx = 0;
    bool sending = false, refused_logged = false, mtu_warned = false;
    double frag_deadline = 0;
    pend_logs_t pl = {.n = 0};
    while (!atomic_load(&l->stop)) {
        double now = _now();
        if (c == NULL) {
            if (now < next_try) {
                usleep(50000);
                continue;
            }
            if (mr_client_connect(l->path, &c) != MR_OK) {
                c = NULL;
                next_try = now + backoff;
                backoff = backoff * 2 > 2.0 ? 2.0 : backoff * 2;
                continue;
            }
            mr_client_set_send_timeout(c, LINK_SEND_TIMEOUT_MS);
            backoff = 0.1;
            refused_logged = false;
            mtu_warned = false;
            claim_req = ++req;
            pthread_mutex_lock(&l->mu);
            memset(&l->st, 0, sizeof(l->st));
            l->st.connected = true;
            _qlog(&pl, 0, "lora: connected to radiod at %s (port %u)", l->path, l->port);
            pthread_mutex_unlock(&l->mu);
            _flush_logs(l, &pl);
            if (_send_claim(c, l->port, claim_req) != MR_OK)
                goto lost;
        }
        MrR2C *m = mr_client_recv(c, 50);         /* no lock held across client calls */
        if (m == NULL && !mr_client_connected(c))
            goto lost;
        tx_t tx = {.want = false};
        pthread_mutex_lock(&l->mu);
        if (m != NULL) {
            switch ((int)m->msg_case) {   /* int: radiod sends more cases than the link reads */
            case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_CLAIM_RESULT:
                if (m->claim_result->req_id != claim_req)
                    break;
                if (m->claim_result->status == MUUDD__RADIO__CLAIM_STATUS__CLAIM_OK) {
                    l->st.claimed = true;
                    l->st.mtu = m->claim_result->mtu;
                    _check_mtu(l, &mtu_warned, &pl);
                } else if (!refused_logged) {
                    refused_logged = true;
                    const ProtobufCEnumValue *v = protobuf_c_enum_descriptor_get_value(
                        &muudd__radio__claim_status__descriptor, m->claim_result->status);
                    _qlog(&pl, 2, "lora: radiod refused the claim (%s) of port %u; give it a "
                          "ports entry with policy consumer and no ip_fallback",
                          v ? v->name : "?", l->port);
                }
                break;
            case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RADIO_STATUS: {
                MrRadioStatus *s = m->radio_status;
                l->st.radio_up = s->state == MUUDD__RADIO__RADIO_STATE__STATE_UP;
                l->st.driver = s->caps != NULL && s->caps->node_addr != 0;
                l->st.node_addr = s->caps != NULL ? s->caps->node_addr : 0;
                for (size_t i = 0; i < s->n_mtus; i++)
                    if (s->mtus[i]->port == l->port) {
                        l->st.mtu = s->mtus[i]->mtu;
                        _check_mtu(l, &mtu_warned, &pl);
                    }
                break;
            }
            case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_RECEIVED: {
                MrReceived *r = m->received;
                if (r->port != l->port)
                    break;
                if (l->n_rx == LORA_LINK_RX_MAX) {
                    free(l->rx[0].payload);
                    memmove(&l->rx[0], &l->rx[1], (LORA_LINK_RX_MAX - 1) * sizeof(lora_link_rx_t));
                    l->n_rx--;
                    l->rx_dropped++;
                }
                uint8_t *p = malloc(r->payload.len ? r->payload.len : 1);
                if (p == NULL)
                    break;
                memcpy(p, r->payload.data, r->payload.len);
                l->rx[l->n_rx++] = (lora_link_rx_t){r->src, p, r->payload.len,
                                                    r->meta ? (int)r->meta->via : 0};
                break;
            }
            case MUUDD__RADIO__RADIOD_TO_CONSUMER__MSG_SEND_RESULT: {
                MrSendResult *sr = m->send_result;
                if (!sending || sr->req_id != frag_req)
                    break;         /* a later ACKED/NO_ACK for an earlier fragment */
                if (sr->status == MUUDD__RADIO__SEND_STATUS__REFUSED) {
                    const ProtobufCEnumValue *v = protobuf_c_enum_descriptor_get_value(
                        &muudd__radio__refuse_reason__descriptor, sr->reason);
                    _finish_head(l, false, frag_idx, v ? v->name : "REFUSED");
                    sending = false;
                } else if (++frag_idx == l->jobs[0].frags.n) {
                    _finish_head(l, true, frag_idx, "");
                    sending = false;
                } else {
                    frag_req = ++req;
                    _tx_set(&tx, &l->jobs[0], frag_idx, frag_req);
                }
                break;
            }
            default:
                break;
            }
            muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
        }
        _recompute_up(l);
        if (sending && !tx.want && _now() > frag_deadline) {
            _finish_head(l, false, frag_idx, "TIMEOUT");
            sending = false;
        }
        if (!sending && l->n_jobs > 0) {
            if (!l->st.up) {
                _fail_all(l, "RADIO_DOWN", 0);
            } else {
                sending = true;
                frag_idx = 0;
                frag_req = ++req;
                _tx_set(&tx, &l->jobs[0], 0, frag_req);
            }
        }
        pthread_mutex_unlock(&l->mu);
        _flush_logs(l, &pl);
        if (tx.want) {
            if (_send_frag(c, l->port, &tx) != MR_OK)
                goto lost;
            /* The wait for SEND_RESULT starts once the fragment is out, so a
             * slow write does not eat into it. */
            frag_deadline = _now() + 2.0;
        }
        continue;
lost:
        pthread_mutex_lock(&l->mu);
        if (l->st.connected)
            _qlog(&pl, 1, "lora: lost the connection to radiod at %s (port %u)", l->path, l->port);
        _fail_all(l, "RADIO_DOWN", sending ? frag_idx : 0);
        memset(&l->st, 0, sizeof(l->st));
        pthread_mutex_unlock(&l->mu);
        _flush_logs(l, &pl);
        sending = false;
        mr_client_close(c);
        c = NULL;
        next_try = _now() + backoff;
    }
    pthread_mutex_lock(&l->mu);
    _fail_all(l, "RADIO_DOWN", sending ? frag_idx : 0);
    pthread_mutex_unlock(&l->mu);
    if (c != NULL)
        mr_client_close(c);
    return NULL;
}

lora_link_t *lora_link_start(const char *socket_path, uint32_t port, lora_log_fn log, void *log_arg)
{
    lora_link_t *l = calloc(1, sizeof(*l));
    if (l == NULL)
        return NULL;
    snprintf(l->path, sizeof(l->path), "%s", socket_path);
    l->port = port;
    l->log = log;
    l->log_arg = log_arg;
    pthread_mutex_init(&l->mu, NULL);
    if (pthread_create(&l->th, NULL, _run, l) != 0) {
        free(l);
        return NULL;
    }
    return l;
}

void lora_link_stop(lora_link_t *l)
{
    if (l == NULL)
        return;
    atomic_store(&l->stop, true);
    pthread_join(l->th, NULL);
    for (size_t i = 0; i < l->n_rx; i++)
        free(l->rx[i].payload);
    pthread_mutex_destroy(&l->mu);
    free(l);
}

void lora_link_state(lora_link_t *l, lora_link_state_t *out)
{
    pthread_mutex_lock(&l->mu);
    *out = l->st;
    pthread_mutex_unlock(&l->mu);
}

int lora_link_submit(lora_link_t *l, uint32_t dest, lora_frags_t *frags, bool want_ack)
{
    pthread_mutex_lock(&l->mu);
    int rc = -1;
    if (l->st.up && l->n_jobs < LORA_LINK_JOBS_MAX && frags->n > 0) {
        l->jobs[l->n_jobs++] = (job_t){dest, want_ack, *frags};
        frags->n = 0;     /* ownership moved */
        rc = 0;
    }
    pthread_mutex_unlock(&l->mu);
    return rc;
}

size_t lora_link_poll_rx(lora_link_t *l, lora_link_rx_t *out, size_t max)
{
    pthread_mutex_lock(&l->mu);
    size_t n = l->n_rx < max ? l->n_rx : max;
    memcpy(out, l->rx, n * sizeof(lora_link_rx_t));
    memmove(&l->rx[0], &l->rx[n], (l->n_rx - n) * sizeof(lora_link_rx_t));
    l->n_rx -= n;
    pthread_mutex_unlock(&l->mu);
    return n;
}

size_t lora_link_poll_results(lora_link_t *l, lora_link_result_t *out, size_t max)
{
    pthread_mutex_lock(&l->mu);
    size_t n = l->n_res < max ? l->n_res : max;
    memcpy(out, l->res, n * sizeof(lora_link_result_t));
    memmove(&l->res[0], &l->res[n], (l->n_res - n) * sizeof(lora_link_result_t));
    l->n_res -= n;
    pthread_mutex_unlock(&l->mu);
    return n;
}

uint64_t lora_link_rx_dropped(lora_link_t *l)
{
    pthread_mutex_lock(&l->mu);
    uint64_t d = l->rx_dropped;
    pthread_mutex_unlock(&l->mu);
    return d;
}

uint64_t lora_link_results_dropped(lora_link_t *l)
{
    pthread_mutex_lock(&l->mu);
    uint64_t d = l->res_dropped;
    pthread_mutex_unlock(&l->mu);
    return d;
}

static void _ops_state(void *ctx, lora_link_state_t *out) { lora_link_state(ctx, out); }
static int _ops_submit(void *ctx, uint32_t dest, lora_frags_t *f, bool ack) { return lora_link_submit(ctx, dest, f, ack); }
static size_t _ops_rx(void *ctx, lora_link_rx_t *o, size_t m) { return lora_link_poll_rx(ctx, o, m); }
static size_t _ops_res(void *ctx, lora_link_result_t *o, size_t m) { return lora_link_poll_results(ctx, o, m); }
static uint64_t _ops_res_dropped(void *ctx) { return lora_link_results_dropped(ctx); }

lora_link_ops_t lora_link_ops(lora_link_t *l)
{
    return (lora_link_ops_t){l, _ops_state, _ops_submit, _ops_rx, _ops_res, _ops_res_dropped};
}
