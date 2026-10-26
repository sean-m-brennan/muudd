/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The engine against a fake link and a fake environment: the twin of the
 * Python test_engine.py. */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <jansson.h>

#include "lora/lora_addrmap.h"
#include "lora/lora_engine.h"
#include "lora/lora_frame.h"

#define ME "00000000-0000-4000-8000-000000000000"
#define A  "aaaaaaaa-0000-4000-8000-000000000001"
#define B  "bbbbbbbb-0000-4000-8000-000000000002"
#define C  "cccccccc-0000-4000-8000-000000000003"
static const uint8_t TAG[4] = {0xa1, 0xb2, 0xc3, 0xd4};

/* ---- fake link ---- */
typedef struct { uint32_t dest; lora_frags_t frags; bool ack; } sent_t;
static struct {
    lora_link_state_t st;
    bool accept;
    sent_t sent[32];
    size_t n_sent;
    lora_link_rx_t rx[32];
    size_t n_rx;
    lora_link_result_t res[8];
    size_t n_res;
} L;

static void fl_state(void *c, lora_link_state_t *o) { (void)c; *o = L.st; }
static int fl_submit(void *c, uint32_t dest, lora_frags_t *f, bool ack)
{
    (void)c;
    if (!L.accept || !L.st.up || L.n_sent == 32)
        return -1;
    L.sent[L.n_sent++] = (sent_t){dest, *f, ack};
    f->n = 0;
    return 0;
}
static size_t fl_rx(void *c, lora_link_rx_t *o, size_t m)
{
    (void)c;
    size_t n = L.n_rx < m ? L.n_rx : m;
    memcpy(o, L.rx, n * sizeof(*o));
    memmove(&L.rx[0], &L.rx[n], (L.n_rx - n) * sizeof(*o));   /* the rest stay queued */
    L.n_rx -= n;
    return n;
}
static size_t fl_res(void *c, lora_link_result_t *o, size_t m)
{
    (void)c;
    size_t n = L.n_res < m ? L.n_res : m;
    memcpy(o, L.res, n * sizeof(*o));
    L.n_res = 0;
    return n;
}
static void _rx(uint32_t src, const uint8_t *p, size_t len, int via)
{
    uint8_t *c = malloc(len);
    memcpy(c, p, len);
    L.rx[L.n_rx++] = (lora_link_rx_t){src, c, len, via};
}
static void _link_up(uint32_t node)
{
    for (size_t i = 0; i < L.n_sent; i++)
        lora_frags_free(&L.sent[i].frags);
    memset(&L, 0, sizeof(L));
    L.st = (lora_link_state_t){true, true, true, true, node, 233, true};
    L.accept = true;
}

/* ---- fake environment ---- */
#define LOGS_MAX 32
static struct {
    double t;
    uint64_t wall;
    double quiet_a, quiet_b;
    char roster[LORA_ADDRMAP_MAX][37];
    size_t n_roster;
    bool proto, have_tag;
    int log_level[LOGS_MAX];
    char log_msg[LOGS_MAX][512];
    size_t n_log;
} E;

static void fe_me(void *a, char out[37]) { (void)a; strcpy(out, ME); }
static size_t fe_members(void *a, char (*out)[37], size_t max)
{
    (void)a;
    size_t n = E.n_roster < max ? E.n_roster : max;
    memcpy(out, E.roster, n * 37);
    return n;
}
static double fe_quiet(void *a, const char *u)
{
    (void)a;
    return strcmp(u, A) == 0 ? E.quiet_a : strcmp(u, B) == 0 ? E.quiet_b : 0.0;
}
static bool fe_proto(void *a) { (void)a; return E.proto; }
static bool fe_tag(void *a, uint8_t t[4]) { (void)a; memcpy(t, TAG, 4); return E.have_tag; }
static uint64_t fe_wall(void *a) { (void)a; return E.wall; }
static double fe_now(void *a) { (void)a; return E.t; }
static void fe_log(void *a, int l, const char *m)
{
    (void)a;
    if (E.n_log == LOGS_MAX)
        return;
    E.log_level[E.n_log] = l;
    snprintf(E.log_msg[E.n_log], sizeof(E.log_msg[0]), "%s", m);
    E.n_log++;
}

/* Logged lines at @p level containing @p needle. */
static int _logged(int level, const char *needle)
{
    int n = 0;
    for (size_t i = 0; i < E.n_log; i++)
        if (E.log_level[i] == level && strstr(E.log_msg[i], needle) != NULL)
            n++;
    return n;
}

static const lora_link_ops_t OPS = {NULL, fl_state, fl_submit, fl_rx, fl_res};
static const lora_env_t ENV = {NULL, fe_me, fe_members, fe_quiet, fe_proto, fe_tag, fe_wall, fe_now, fe_log};

static char g_path[] = "/tmp/lora_engine_testXXXXXX";

static lora_engine_t *_engine(const char *allow_json, bool announce)
{
    _link_up(0xA01);
    memset(&E, 0, sizeof(E));
    E.t = 1000.0;
    E.wall = 1700000000000ULL;
    E.quiet_a = 500.0;
    strcpy(E.roster[0], A);
    strcpy(E.roster[1], B);
    E.n_roster = 2;
    E.proto = E.have_tag = true;
    lora_cfg_t cfg;
    char f[64], w[160], text[256];
    snprintf(text, sizeof(text), "{\"port\": 10, \"allow\": %s}", allow_json);
    ck_assert_int_eq(lora_config_parse(text, &cfg, f, sizeof(f), w, sizeof(w)), 0);
    lora_engine_t *e = lora_engine_new(&cfg, OPS, ENV, g_path);
    if (announce) {
        json_t *a = json_pack("{s:i,s:i}", "lora_addr", 0xB02, "issued_ms", 5);
        json_t *b = json_pack("{s:i,s:i}", "lora_addr", 0xC03, "issued_ms", 5);
        ck_assert_int_eq(lora_engine_on_announce(e, A, a, true), LORA_ANN_ACCEPTED);
        ck_assert_int_eq(lora_engine_on_announce(e, B, b, true), LORA_ANN_ACCEPTED);
        json_decref(a);
        json_decref(b);
    }
    return e;
}

static void _done(lora_engine_t *e)
{
    lora_engine_free(e);
    unlink(g_path);
    _link_up(0);
}

/* Announce if due, as the glue does after a successful send. */
static bool _announce(lora_engine_t *e, uint32_t *addr, uint64_t *issued)
{
    if (!lora_engine_announce_due(e, addr, issued))
        return false;
    lora_engine_announce_sent(e, *addr, *issued);
    return true;
}

/* The self_issued_ms persisted at g_path. */
static uint64_t _persisted_self(void)
{
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self = 0;
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 0);
    lora_addrmap_free(back);
    return self;
}

/* A stalled 3-fragment unicast from 0xB02 (only its first fragment arrived). */
static void _stalled_unicast(lora_engine_t *e)
{
    uint8_t msg[500] = {0};
    lora_frags_t f;
    ck_assert_int_eq(lora_fragment(msg, sizeof(msg), LORA_T_PEER, 9, NULL, 233, 8, &f), 0);
    _rx(0xB02, f.buf[0], f.len[0], 0);
    lora_delivery_t d[4];
    ck_assert_int_eq((int)lora_engine_poll(e, d, 4), 0);
    lora_frags_free(&f);
}

static const char *ALLOW = "[\"identity/group_key_update\", \"identity/presence\"]";

DEFINE_TEST(test_unicast_to_a_quiet_peer_goes_by_lora_and_is_held)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint8_t frame[600];
    memset(frame, 0x5e, sizeof(frame));
    ck_assert(lora_engine_on_unicast(e, A, "identity", "group_key_update", frame, sizeof(frame)));
    sent_t *s = &L.sent[L.n_sent - 1];
    ck_assert_int_eq(s->dest, 0xB02);
    ck_assert(s->ack);
    ck_assert_int_eq((int)s->frags.n, 3);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.msgs"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.frames"), 3);
    lora_frame_t f0;
    ck_assert_int_eq(lora_frame_decode(s->frags.buf[0], s->frags.len[0], &f0), 0);
    uint8_t nack[5];
    lora_encode_nack(f0.msg_id, 0x2, nack, sizeof(nack));
    _rx(0xB02, nack, 5, 0);
    lora_delivery_t d[4];
    ck_assert_int_eq((int)lora_engine_poll(e, d, 4), 0);
    sent_t *r = &L.sent[L.n_sent - 1];
    ck_assert_int_eq(r->dest, 0xB02);
    ck_assert(r->ack);
    ck_assert_int_eq((int)r->frags.n, 1);
    ck_assert_int_eq(r->frags.buf[0][3], 0x12);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.nack.served"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.frames"), 4);   /* 3 sent + 1 resent */
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_refused_resend_counts_abandoned)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint8_t frame[600] = {0};
    ck_assert(lora_engine_on_unicast(e, A, "identity", "group_key_update", frame, sizeof(frame)));
    lora_frame_t f0;
    sent_t *s = &L.sent[L.n_sent - 1];
    ck_assert_int_eq(lora_frame_decode(s->frags.buf[0], s->frags.len[0], &f0), 0);
    uint8_t nack[5];
    lora_encode_nack(f0.msg_id, 0x6, nack, sizeof(nack));
    _rx(0xB02, nack, 5, 0);
    L.accept = false;
    lora_delivery_t d[4];
    lora_engine_poll(e, d, 4);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.nack.served"), 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.frames"), 3);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_unicast_is_skipped_when_any_condition_fails)
{
    lora_engine_t *e = _engine(ALLOW, true);
    const uint8_t x[1] = {1};
    ck_assert(!lora_engine_on_unicast(e, B, "identity", "group_key_update", x, 1));   /* not quiet */
    ck_assert(!lora_engine_on_unicast(e, A, "identity", "peer_dm", x, 1));            /* not allowed */
    E.proto = false;
    ck_assert(!lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1));   /* JSON group */
    E.proto = true;
    memset(&L.st, 0, sizeof(L.st));
    ck_assert(!lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1));   /* link down */
    ck_assert_int_eq((int)L.n_sent, 0);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_job_queue_full_counts_abandoned)
{
    lora_engine_t *e = _engine(ALLOW, true);
    L.accept = false;
    const uint8_t x[1] = {1};
    ck_assert(!lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 1);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_fanout_needs_a_quiet_member)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint8_t g[10];
    memset(g, 'g', sizeof(g));
    ck_assert(lora_engine_on_fanout(e, false, "identity", "presence", g, sizeof(g)));
    sent_t *s = &L.sent[L.n_sent - 1];
    ck_assert_int_eq(s->dest, LORA_BROADCAST);
    ck_assert(!s->ack);
    ck_assert_int_eq(memcmp(s->frags.buf[0] + 4, TAG, 4), 0);
    E.quiet_a = 0.0;
    ck_assert(!lora_engine_on_fanout(e, false, "identity", "presence", g, 1));
    E.quiet_a = 500.0;
    E.proto = false;
    ck_assert(!lora_engine_on_fanout(e, false, "identity", "presence", g, 1));   /* JSON group */
    ck_assert(lora_engine_on_fanout(e, true, "identity", "presence", g, 1));     /* broadcast ignores format */
    ck_assert(!lora_engine_on_fanout(e, true, "identity", "peer_dm", g, 1));     /* not allowed */
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_received_frames_are_delivered_once_on_their_channel)
{
    lora_engine_t *e = _engine(ALLOW, true);
    const uint8_t hello[5] = {'h', 'e', 'l', 'l', 'o'};
    lora_frags_t g, p;
    ck_assert_int_eq(lora_fragment(hello, 5, LORA_T_GROUP, 77, TAG, 233, 8, &g), 0);
    ck_assert_int_eq(lora_fragment(hello, 5, LORA_T_PEER, 78, NULL, 233, 8, &p), 0);
    _rx(0xB02, g.buf[0], g.len[0], 0);
    _rx(0xB02, g.buf[0], g.len[0], 0);
    _rx(0xB02, p.buf[0], p.len[0], 0);
    lora_delivery_t d[4];
    size_t n = lora_engine_poll(e, d, 4);
    ck_assert_int_eq((int)n, 2);
    ck_assert_int_eq(d[0].channel, LORA_DELIVER_GROUP);
    ck_assert_str_eq(d[0].uuid, A);
    ck_assert_int_eq((int)d[0].len, 5);
    ck_assert_int_eq(memcmp(d[0].msg, hello, 5), 0);
    ck_assert_int_eq(d[1].channel, LORA_DELIVER_PEER);
    ck_assert_str_eq(d[1].uuid, A);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.duplicate"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.delivered"), 2);
    for (size_t i = 0; i < n; i++)
        free(d[i].msg);
    lora_frags_free(&g);
    lora_frags_free(&p);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_poll_takes_no_more_frames_than_it_can_return)
{
    lora_engine_t *e = _engine(ALLOW, true);
    const uint8_t x[1] = {'x'};
    for (uint16_t i = 0; i < 20; i++) {
        lora_frags_t p;
        ck_assert_int_eq(lora_fragment(x, 1, LORA_T_PEER, (uint16_t)(100 + i), NULL, 233, 8, &p), 0);
        _rx(0xB02, p.buf[0], p.len[0], 0);
        lora_frags_free(&p);
    }
    lora_delivery_t d[16];
    size_t n = lora_engine_poll(e, d, 16);
    ck_assert_int_eq((int)n, 16);
    ck_assert_int_eq((int)L.n_rx, 4);
    for (size_t i = 0; i < n; i++)
        free(d[i].msg);
    n = lora_engine_poll(e, d, 16);
    ck_assert_int_eq((int)n, 4);
    for (size_t i = 0; i < n; i++)
        free(d[i].msg);
    ck_assert_int_eq((int)L.n_rx, 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.delivered"), 20);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.frames"), 20);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_receive_side_refusals_are_counted)
{
    lora_engine_t *e = _engine(ALLOW, true);
    const uint8_t x[1] = {'x'}, zz[4] = {'z', 'z', 'z', 'z'}, bad[1] = {0x99};
    lora_frags_t good, foreign;
    ck_assert_int_eq(lora_fragment(x, 1, LORA_T_PEER, 1, NULL, 233, 8, &good), 0);
    ck_assert_int_eq(lora_fragment(x, 1, LORA_T_GROUP, 2, zz, 233, 8, &foreign), 0);
    _rx(0xD04, good.buf[0], good.len[0], 0);         /* unknown source */
    _rx(0xB02, bad, 1, 0);                           /* bad frame */
    _rx(0xB02, good.buf[0], good.len[0], 1);         /* not via the radio */
    _rx(0xB02, foreign.buf[0], foreign.len[0], 0);   /* foreign tag */
    lora_delivery_t d[4];
    ck_assert_int_eq((int)lora_engine_poll(e, d, 4), 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.unknown_src"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.bad_frame"), 2);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.foreign_tag"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.frames"), 4);
    lora_frags_free(&good);
    lora_frags_free(&foreign);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_nack_is_sent_for_a_stalled_unicast)
{
    lora_engine_t *e = _engine(ALLOW, true);
    _stalled_unicast(e);
    E.t += 20;
    lora_delivery_t d[4];
    lora_engine_poll(e, d, 4);
    sent_t *s = &L.sent[L.n_sent - 1];
    lora_frame_t n;
    ck_assert_int_eq(s->dest, 0xB02);
    ck_assert(!s->ack);
    ck_assert_int_eq(lora_frame_decode(s->frags.buf[0], s->frags.len[0], &n), 0);
    ck_assert_int_eq(n.type, LORA_T_NACK);
    ck_assert_int_eq(n.missing, 0x6);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_sent_nack_counts_a_frame)
{
    lora_engine_t *e = _engine(ALLOW, true);
    _stalled_unicast(e);
    E.t += 20;
    lora_delivery_t d[4];
    lora_engine_poll(e, d, 4);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.nack.sent"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.frames"), 1);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_refused_nack_counts_abandoned)
{
    lora_engine_t *e = _engine(ALLOW, true);
    _stalled_unicast(e);
    L.accept = false;
    E.t += 20;
    lora_delivery_t d[4];
    lora_engine_poll(e, d, 4);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.nack.sent"), 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.frames"), 0);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_announce_due_rules)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint32_t addr, addr2;
    uint64_t issued, issued2;
    ck_assert(lora_engine_announce_due(e, &addr, &issued));
    ck_assert_int_eq(addr, 0xA01);
    ck_assert(issued == E.wall);
    ck_assert(lora_engine_announce_due(e, &addr2, &issued2));   /* no side effects until sent */
    ck_assert(addr2 == addr && issued2 == issued);
    lora_engine_announce_sent(e, addr, issued);
    ck_assert(!lora_engine_announce_due(e, &addr, &issued));
    strcpy(E.roster[E.n_roster++], C);
    E.t += 30;
    ck_assert(!lora_engine_announce_due(e, &addr, &issued));    /* within 60 s of the last */
    E.t += 31;
    ck_assert(_announce(e, &addr, &issued));                    /* a new member, 61 s later */
    E.t += 600;
    ck_assert(_announce(e, &addr, &issued));                    /* announce_s */
    L.st.node_addr = 0xA09;
    ck_assert(_announce(e, &addr, &issued));                    /* address changed */
    ck_assert_int_eq(addr, 0xA09);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 4);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_announce_not_due_before_the_group_is_known)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint32_t addr;
    uint64_t issued;
    E.have_tag = false;
    ck_assert(!lora_engine_announce_due(e, &addr, &issued));
    E.have_tag = true;
    ck_assert(lora_engine_announce_due(e, &addr, &issued));
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_announce_due_again_after_link_down_up_with_the_same_address)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint32_t addr;
    uint64_t issued;
    ck_assert(_announce(e, &addr, &issued));
    ck_assert(!lora_engine_announce_due(e, &addr, &issued));
    lora_link_state_t up = L.st;
    memset(&L.st, 0, sizeof(L.st));
    ck_assert(!lora_engine_announce_due(e, &addr, &issued));
    L.st = up;
    ck_assert(lora_engine_announce_due(e, &addr, &issued));
    ck_assert_int_eq(addr, 0xA01);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_unsent_announcement_stays_due_and_keeps_issued_ms)
{
    lora_engine_t *e = _engine(ALLOW, true);
    uint32_t addr, addr2;
    uint64_t issued, issued2;
    ck_assert(lora_engine_announce_due(e, &addr, &issued));
    E.t += 1;
    ck_assert(lora_engine_announce_due(e, &addr2, &issued2));
    ck_assert(addr2 == addr && issued2 == issued);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 0);
    ck_assert(_persisted_self() == 0);
    lora_engine_announce_sent(e, addr, issued);
    ck_assert(_persisted_self() == issued);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_issued_ms_survives_restart_with_clock_behind)
{
    lora_engine_t *e1 = _engine("[]", false);
    uint32_t addr;
    uint64_t first, second;
    ck_assert(_announce(e1, &addr, &first));
    lora_engine_free(e1);           /* keeps the file */
    lora_cfg_t cfg;
    char f[64], w[160];
    ck_assert_int_eq(lora_config_parse("{\"port\": 10}", &cfg, f, sizeof(f), w, sizeof(w)), 0);
    E.wall -= 3600000ULL;           /* rebooted, clock an hour behind */
    lora_engine_t *e2 = lora_engine_new(&cfg, OPS, ENV, g_path);
    ck_assert(lora_engine_announce_due(e2, &addr, &second));
    ck_assert(second == first + 1);
    _done(e2);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_announce_acceptance_rules)
{
    lora_engine_t *e = _engine(ALLOW, true);
    json_t *ok = json_pack("{s:i,s:i}", "lora_addr", 7, "issued_ms", 9);
    json_t *str = json_pack("{s:s,s:i}", "lora_addr", "7", "issued_ms", 9);
    json_t *old = json_pack("{s:i,s:i}", "lora_addr", 0xB02, "issued_ms", 5);
    ck_assert_int_eq(lora_engine_on_announce(e, ME, ok, true), LORA_ANN_SELF);
    ck_assert_int_eq(lora_engine_on_announce(e, "dddddddd-0000-4000-8000-000000000004", ok, true),
                     LORA_ANN_REFUSED);
    ck_assert_int_eq(lora_engine_on_announce(e, A, str, true), LORA_ANN_BAD);
    ck_assert_int_eq(lora_engine_on_announce(e, A, NULL, true), LORA_ANN_BAD);
    json_t *zero = json_pack("{s:i,s:i}", "lora_addr", 0, "issued_ms", 9);
    json_t *bcast = json_pack("{s:I,s:i}", "lora_addr", (json_int_t)0xFFFFFFFF, "issued_ms", 9);
    json_t *wide = json_pack("{s:I,s:i}", "lora_addr", (json_int_t)0x100000002LL, "issued_ms", 9);
    json_t *neg = json_pack("{s:i,s:i}", "lora_addr", 7, "issued_ms", -1);
    ck_assert_int_eq(lora_engine_on_announce(e, A, zero, true), LORA_ANN_BAD);
    ck_assert_int_eq(lora_engine_on_announce(e, A, bcast, true), LORA_ANN_BAD);
    ck_assert_int_eq(lora_engine_on_announce(e, A, wide, true), LORA_ANN_BAD);
    ck_assert_int_eq(lora_engine_on_announce(e, A, neg, true), LORA_ANN_BAD);
    json_decref(zero);
    json_decref(bcast);
    json_decref(wide);
    json_decref(neg);
    ck_assert_int_eq(lora_engine_on_announce(e, A, old, true), LORA_ANN_STALE);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.stale"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.accepted"), 2);
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self;
    uint32_t addr;
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 0);
    ck_assert(lora_addrmap_addr_of(back, A, &addr) && addr == 0xB02);
    lora_addrmap_free(back);
    json_decref(ok);
    json_decref(str);
    json_decref(old);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_departed_holder_is_replaced_by_a_member)
{
    lora_engine_t *e = _engine(ALLOW, true);
    strcpy(E.roster[0], C);          /* A left the roster, C joined */
    json_t *c = json_pack("{s:i,s:i}", "lora_addr", 0xB02, "issued_ms", 9);
    ck_assert_int_eq(lora_engine_on_announce(e, C, c, true), LORA_ANN_ACCEPTED);
    json_decref(c);
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self;
    uint32_t addr = 0;
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 0);
    ck_assert(!lora_addrmap_addr_of(back, A, &addr));
    ck_assert(lora_addrmap_addr_of(back, C, &addr) && addr == 0xB02);
    lora_addrmap_free(back);
    lora_frags_t p;
    const uint8_t x[1] = {'x'};
    ck_assert_int_eq(lora_fragment(x, 1, LORA_T_PEER, 3, NULL, 233, 8, &p), 0);
    _rx(0xB02, p.buf[0], p.len[0], 0);
    lora_delivery_t d[4];
    ck_assert_int_eq((int)lora_engine_poll(e, d, 4), 1);
    ck_assert_str_eq(d[0].uuid, C);
    free(d[0].msg);
    lora_frags_free(&p);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_conflict_warns_once_and_counts_received_frames)
{
    lora_engine_t *e = _engine(ALLOW, true);
    json_t *a = json_pack("{s:i,s:i}", "lora_addr", 0xC03, "issued_ms", 9);
    ck_assert_int_eq(lora_engine_on_announce(e, A, a, true), LORA_ANN_ACCEPTED);
    json_decref(a);
    ck_assert_int_eq(_logged(1, ""), 1);
    ck_assert_int_eq(_logged(1, A), 1);
    ck_assert_int_eq(_logged(1, B), 1);
    ck_assert_int_eq(_logged(1, "both claim LoRa address 0xc03"), 1);
    lora_frags_t p;
    const uint8_t x[1] = {'x'};
    ck_assert_int_eq(lora_fragment(x, 1, LORA_T_PEER, 1, NULL, 233, 8, &p), 0);
    _rx(0xC03, p.buf[0], p.len[0], 0);
    lora_delivery_t d[4];
    ck_assert_int_eq((int)lora_engine_poll(e, d, 4), 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.rx.conflict"), 1);
    lora_frags_free(&p);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_json_group_is_logged_once)
{
    lora_engine_t *e = _engine(ALLOW, true);
    E.proto = false;
    const uint8_t x[1] = {'x'};
    lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1);
    lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1);
    lora_engine_on_fanout(e, false, "identity", "presence", x, 1);
    ck_assert_int_eq(_logged(0, "JSON"), 1);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_summary_due_every_300_s)
{
    lora_engine_t *e = _engine(ALLOW, true);
    char line[256];
    ck_assert(!lora_engine_summary_due(e, line, sizeof(line)));
    E.t += 299;
    ck_assert(!lora_engine_summary_due(e, line, sizeof(line)));
    E.t += 2;
    ck_assert(lora_engine_summary_due(e, line, sizeof(line)));
    ck_assert(strncmp(line, "lora: link up", 13) == 0);
    ck_assert(strstr(line, "2 peers mapped") != NULL);
    ck_assert(!lora_engine_summary_due(e, line, sizeof(line)));
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_stats_shape)
{
    lora_engine_t *e = _engine(ALLOW, true);
    json_t *s = lora_engine_stats(e);
    ck_assert_ptr_nonnull(s);
    ck_assert_int_eq((int)json_object_size(s), 3);
    json_t *link = json_object_get(s, "link");
    ck_assert(json_is_true(json_object_get(link, "up")));
    ck_assert(json_is_true(json_object_get(link, "connected")));
    ck_assert(json_is_true(json_object_get(link, "claimed")));
    ck_assert_int_eq(json_integer_value(json_object_get(link, "node_addr")), 0xA01);
    ck_assert_int_eq(json_integer_value(json_object_get(link, "mtu")), 233);
    ck_assert_int_eq((int)json_array_size(json_object_get(s, "addrmap")), 2);
    ck_assert_int_eq(json_integer_value(json_object_get(json_object_get(s, "counters"),
                                                        "lora.announce.accepted")), 2);
    json_decref(s);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_exclusion_removes_and_persists)
{
    lora_engine_t *e = _engine(ALLOW, true);
    lora_engine_on_exclusion(e, A, true);
    const uint8_t x[1] = {1};
    ck_assert(!lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1));
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self;
    uint32_t addr;
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 0);
    ck_assert(!lora_addrmap_addr_of(back, A, &addr));
    lora_addrmap_free(back);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_refusals_from_the_link_are_counted_by_reason)
{
    lora_engine_t *e = _engine(ALLOW, true);
    L.res[L.n_res++] = (lora_link_result_t){false, 1, 3, "OVER_BUDGET"};
    lora_delivery_t d[4];
    lora_engine_poll(e, d, 4);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.refused.OVER_BUDGET"), 1);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_first_msg_id_is_random)
{
    uint16_t ids[8];
    bool differ = false;
    const uint8_t x[1] = {1};
    for (int i = 0; i < 8; i++) {
        lora_engine_t *e = _engine(ALLOW, true);
        ck_assert(lora_engine_on_unicast(e, A, "identity", "group_key_update", x, 1));
        lora_frame_t f;
        ck_assert_int_eq(lora_frame_decode(L.sent[0].frags.buf[0], L.sent[0].frags.len[0], &f), 0);
        ids[i] = f.msg_id;
        differ = differ || ids[i] != ids[0];
        _done(e);
    }
    ck_assert(differ);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_an_unverified_announcement_is_refused)
{
    lora_engine_t *e = _engine(ALLOW, true);
    json_t *a = json_pack("{s:i,s:i}", "lora_addr", 0xD04, "issued_ms", 9);
    ck_assert_int_eq(lora_engine_on_announce(e, A, a, false), LORA_ANN_REFUSED);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.unverified"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.accepted"), 2);
    json_decref(a);
    _done(e);
}
END_TEST_DEFINITION()

static void _filler(int i, char out[37])
{
    snprintf(out, 37, "%08x-0000-4000-8000-%012x", 0x10000000 + i, i);
}

/* Fill the map file with @p n entries off the roster, issued_ms 100 + i. */
static void _fill_file(int n)
{
    lora_addrmap_t *m = lora_addrmap_new();
    ck_assert_int_eq(lora_addrmap_announce(m, A, 0xB02, 5, NULL, NULL), LORA_ANN_ACCEPTED);
    ck_assert_int_eq(lora_addrmap_announce(m, B, 0xC03, 5, NULL, NULL), LORA_ANN_ACCEPTED);
    for (int i = 0; i < n; i++) {
        char u[37];
        _filler(i, u);
        ck_assert_int_eq(lora_addrmap_announce(m, u, 0x1000 + (uint32_t)i, 100 + (uint64_t)i,
                                               NULL, NULL), LORA_ANN_ACCEPTED);
    }
    ck_assert_int_eq(lora_addrmap_save(m, g_path, 0), 0);
    lora_addrmap_free(m);
}

DEFINE_TEST(test_a_full_map_makes_room_by_dropping_the_oldest_non_member)
{
    _fill_file(LORA_ADDRMAP_MAX - 2);
    lora_engine_t *e = _engine(ALLOW, false);   /* loads A, B and the fillers */
    strcpy(E.roster[E.n_roster++], C);
    json_t *c = json_pack("{s:i,s:i}", "lora_addr", 0xE05, "issued_ms", 9);
    ck_assert_int_eq(lora_engine_on_announce(e, C, c, true), LORA_ANN_ACCEPTED);
    json_decref(c);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.full"), 0);
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self;
    uint32_t addr = 0;
    char u0[37], u1[37];
    _filler(0, u0);
    _filler(1, u1);
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 0);
    ck_assert(!lora_addrmap_addr_of(back, u0, &addr));
    ck_assert(lora_addrmap_addr_of(back, u1, &addr) && addr == 0x1001);
    ck_assert(lora_addrmap_addr_of(back, A, &addr) && addr == 0xB02);
    ck_assert(lora_addrmap_addr_of(back, C, &addr) && addr == 0xE05);
    lora_addrmap_free(back);
    _done(e);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_a_bad_entry_in_the_file_is_skipped_with_one_warning)
{
    FILE *f = fopen(g_path, "w");
    fprintf(f, "{\"version\":1,\"self_issued_ms\":42,\"peers\":{"
               "\"%s\":{\"addr\":2818,\"issued_ms\":5},"
               "\"%s\":{\"addr\":0,\"issued_ms\":5}}}", A, B);
    fclose(f);
    _link_up(0xA01);
    memset(&E, 0, sizeof(E));
    E.t = 1000.0;
    E.wall = 1;
    strcpy(E.roster[0], A);
    E.n_roster = 1;
    E.proto = E.have_tag = true;
    lora_cfg_t cfg;
    char fl[64], w[160];
    ck_assert_int_eq(lora_config_parse("{\"port\": 10}", &cfg, fl, sizeof(fl), w, sizeof(w)), 0);
    lora_engine_t *e = lora_engine_new(&cfg, OPS, ENV, g_path);
    ck_assert_int_eq(_logged(1, "skipped"), 1);
    ck_assert_int_eq(_logged(1, B), 1);
    uint32_t addr;
    uint64_t issued;
    ck_assert(lora_engine_announce_due(e, &addr, &issued));
    ck_assert(issued == 43);                     /* self_issued_ms kept */
    lora_addrmap_t *back = lora_addrmap_new();
    uint64_t self;
    ck_assert_int_eq(lora_addrmap_load(back, g_path, &self), 1);
    ck_assert(lora_addrmap_addr_of(back, A, &addr) && addr == 2818);
    lora_addrmap_free(back);
    _done(e);
}
END_TEST_DEFINITION()

int main(void)
{
    int fd = mkstemp(g_path);
    close(fd);
    unlink(g_path);
    test_unicast_to_a_quiet_peer_goes_by_lora_and_is_held();
    test_refused_resend_counts_abandoned();
    test_unicast_is_skipped_when_any_condition_fails();
    test_job_queue_full_counts_abandoned();
    test_fanout_needs_a_quiet_member();
    test_received_frames_are_delivered_once_on_their_channel();
    test_poll_takes_no_more_frames_than_it_can_return();
    test_receive_side_refusals_are_counted();
    test_nack_is_sent_for_a_stalled_unicast();
    test_sent_nack_counts_a_frame();
    test_refused_nack_counts_abandoned();
    test_announce_due_rules();
    test_announce_not_due_before_the_group_is_known();
    test_announce_due_again_after_link_down_up_with_the_same_address();
    test_unsent_announcement_stays_due_and_keeps_issued_ms();
    test_issued_ms_survives_restart_with_clock_behind();
    test_announce_acceptance_rules();
    test_departed_holder_is_replaced_by_a_member();
    test_conflict_warns_once_and_counts_received_frames();
    test_json_group_is_logged_once();
    test_summary_due_every_300_s();
    test_stats_shape();
    test_exclusion_removes_and_persists();
    test_refusals_from_the_link_are_counted_by_reason();
    test_first_msg_id_is_random();
    test_an_unverified_announcement_is_refused();
    test_a_full_map_makes_room_by_dropping_the_oldest_non_member();
    test_a_bad_entry_in_the_file_is_skipped_with_one_warning();
    printf("=== %d checks, %d failures ===\n", _test_count, _test_failures);
    return _test_failures > 0 ? 1 : 0;
}
