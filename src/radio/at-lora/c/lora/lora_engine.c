/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The at-lora engine: decides what also goes by LoRa, frames and sends it,
 * reassembles what arrives, answers and sends NACKs, and keeps the address
 * map. It knows nothing of AT's processes: the glue hands it an environment
 * (roster, transport quiet time, group format and tag, clocks). Twin of the
 * Python engine; both follow the same rules, keep the same counters and log
 * the same lines. Single-threaded: every call comes from the network
 * process's main loop. */
#include <errno.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <sodium.h>
#include <uuid/uuid.h>

#include "lora/lora_engine.h"
#include "lora/lora_frame.h"
#include "lora/lora_policy.h"
#include "lora/lora_reasm.h"

#define MEMBER_ANNOUNCE_S 60.0   /* a new member hears us this soon after the last announcement */
#define SUMMARY_S 300.0          /* the one-line summary's period */
#define WARN_EVERY_S 60.0        /* a repeated WARN, at most this often */
#define ROSTER_MAX LORA_ADDRMAP_MAX
#define COUNTERS_MAX 64

typedef struct { char name[48]; uint64_t v; } counter_t;

/* Why an arriving frame was dropped: its counter (lora.rx.<key>), the log
 * text and level. Duplicates are expected (NACK resends, multipath), so they
 * log at DEBUG; the rest WARN. */
typedef enum { DROP_BAD, DROP_UNKNOWN, DROP_CONFLICT, DROP_FOREIGN, DROP_DUPLICATE, DROP_N } drop_t;
static const struct { const char *key, *what; int level; } DROPS[DROP_N] = {
    [DROP_BAD] = {"bad_frame", "a malformed frame", 1},
    [DROP_UNKNOWN] = {"unknown_src", "a frame from an unmapped address", 1},
    [DROP_CONFLICT] = {"conflict", "a frame from a conflicted address", 1},
    [DROP_FOREIGN] = {"foreign_tag", "a frame with another group's tag", 1},
    [DROP_DUPLICATE] = {"duplicate", "a duplicate frame", -1},
};

struct lora_engine {
    lora_cfg_t cfg;
    lora_link_ops_t link;
    lora_env_t env;
    char path[600];
    lora_addrmap_t *map;
    uint64_t self_issued;          /* issued_ms of our last announcement sent */
    lora_reasm_t *reasm;
    lora_hold_t *hold;
    uint16_t msg_id;
    uint32_t announced_addr;       /* 0 before the first announcement */
    double last_announce, last_summary;
    bool was_down;                 /* the link was seen down since the last announcement */
    char announced_to[ROSTER_MAX][37];
    size_t n_announced_to;
    bool json_logged;
    double full_warned_at;         /* the address-map-full WARN */
    double drop_warned_at[DROP_N]; /* the dropped-frame logs, one per reason */
    counter_t c[COUNTERS_MAX];
    size_t n_c;
    /* Scratch: the environment's roster, and it without us. */
    char all[ROSTER_MAX][37];
    char roster[ROSTER_MAX][37];
    size_t n_roster;
};

static void _logf(lora_engine_t *e, int level, const char *fmt, ...)
    __attribute__((format(printf, 3, 4)));

static void _logf(lora_engine_t *e, int level, const char *fmt, ...)
{
    char msg[512];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(msg, sizeof(msg), fmt, ap);
    va_end(ap);
    if (e->env.log != NULL)
        e->env.log(e->env.arg, level, msg);
}

static counter_t *_ctr(lora_engine_t *e, const char *name)
{
    for (size_t i = 0; i < e->n_c; i++)
        if (strcmp(e->c[i].name, name) == 0)
            return &e->c[i];
    if (e->n_c == COUNTERS_MAX)
        return NULL;
    counter_t *c = &e->c[e->n_c++];
    snprintf(c->name, sizeof(c->name), "%s", name);
    c->v = 0;
    return c;
}

static void _count(lora_engine_t *e, const char *name, uint64_t d)
{
    counter_t *c = _ctr(e, name);
    if (c != NULL)
        c->v += d;
}

static void _set(lora_engine_t *e, const char *name, uint64_t v)
{
    counter_t *c = _ctr(e, name);
    if (c != NULL)
        c->v = v;
}

uint64_t lora_engine_counter(const lora_engine_t *e, const char *name)
{
    for (size_t i = 0; i < e->n_c; i++)
        if (strcmp(e->c[i].name, name) == 0)
            return e->c[i].v;
    return 0;
}

static void _save(lora_engine_t *e)
{
    errno = 0;
    if (lora_addrmap_save(e->map, e->path, e->self_issued) != 0)
        _logf(e, 2, "lora: could not write %s: %s", e->path,
              errno != 0 ? strerror(errno) : "failed");
}

lora_engine_t *lora_engine_new(const lora_cfg_t *cfg, lora_link_ops_t link, lora_env_t env,
                               const char *addrmap_path)
{
    if (sodium_init() < 0)
        return NULL;
    lora_engine_t *e = calloc(1, sizeof(*e));
    if (e == NULL)
        return NULL;
    e->cfg = *cfg;
    e->link = link;
    e->env = env;
    snprintf(e->path, sizeof(e->path), "%s", addrmap_path);
    e->map = lora_addrmap_new();
    e->reasm = lora_reasm_new(cfg->nack_gap_s, cfg->reassembly_s);
    e->hold = lora_hold_new(cfg->nack_hold_s);
    if (e->map == NULL || e->reasm == NULL || e->hold == NULL) {
        lora_engine_free(e);
        return NULL;
    }
    int loaded = lora_addrmap_load(e->map, e->path, &e->self_issued);
    if (loaded < 0)
        _logf(e, 1, "lora: %s unreadable (%s); starting with an empty "
              "address map", e->path, "not a readable address map");
    else if (loaded > 0)
        _logf(e, 1, "lora: %s: skipped %d bad entr%s (%s); kept the rest", e->path, loaded,
              loaded == 1 ? "y" : "ies", lora_addrmap_skipped(e->map));
    e->full_warned_at = -1e300;
    for (size_t i = 0; i < DROP_N; i++)
        e->drop_warned_at[i] = -1e300;
    e->msg_id = (uint16_t)randombytes_uniform(65536);
    e->last_announce = -1e300;
    e->last_summary = env.now(env.arg);
    return e;
}

void lora_engine_free(lora_engine_t *e)
{
    if (e == NULL)
        return;
    lora_addrmap_free(e->map);
    lora_reasm_free(e->reasm);
    lora_hold_free(e->hold);
    free(e);
}

static uint16_t _next_id(lora_engine_t *e)
{
    return e->msg_id++;
}

static bool _proto(lora_engine_t *e)
{
    bool ok = e->env.group_proto(e->env.arg);
    if (!ok && !e->json_logged) {
        e->json_logged = true;
        _logf(e, 0, "lora: this cohort uses the JSON wire format; LoRa carries only "
              "proto-format cohorts, so nothing goes on the air");
    }
    return ok;
}

/* The roster without us, into e->roster / e->n_roster. */
static void _members(lora_engine_t *e)
{
    char me[37];
    e->env.my_uuid(e->env.arg, me);
    size_t n = e->env.members(e->env.arg, e->all, ROSTER_MAX), k = 0;
    for (size_t i = 0; i < n && i < ROSTER_MAX; i++)
        if (strcmp(e->all[i], me) != 0)
            memcpy(e->roster[k++], e->all[i], 37);
    e->n_roster = k;
}

static bool _in_roster(const lora_engine_t *e, const char *uuid)
{
    for (size_t i = 0; i < e->n_roster; i++)
        if (strcmp(e->roster[i], uuid) == 0)
            return true;
    return false;
}

static bool _is_member(const char *uuid, void *arg)
{
    return _in_roster(arg, uuid);
}

/* Members with an address that are quiet on IP. */
static size_t _quiet_count(lora_engine_t *e)
{
    _members(e);
    size_t q = 0;
    uint32_t addr;
    for (size_t i = 0; i < e->n_roster; i++)
        if (lora_addrmap_addr_of(e->map, e->roster[i], &addr)
            && e->env.quiet_for(e->env.arg, e->roster[i]) >= e->cfg.ip_quiet_s)
            q++;
    return q;
}

/* Copy @p f (the link takes ownership of what it is given). 0, or -1 with
 * nothing allocated. */
static int _copy(const lora_frags_t *f, lora_frags_t *out)
{
    out->n = 0;
    for (size_t i = 0; i < f->n; i++) {
        out->buf[i] = malloc(f->len[i]);
        if (out->buf[i] == NULL) {
            lora_frags_free(out);
            return -1;
        }
        memcpy(out->buf[i], f->buf[i], f->len[i]);
        out->len[i] = f->len[i];
        out->n = i + 1;
    }
    return 0;
}

/* Hand @p f to the link (it owns it after), counting one message and its
 * frames; a refusal frees @p f and counts lora.tx.abandoned. */
static bool _submit(lora_engine_t *e, uint32_t dest, lora_frags_t *f, bool ack)
{
    size_t n = f->n;
    if (e->link.submit(e->link.ctx, dest, f, ack) != 0) {
        lora_frags_free(f);
        _count(e, "lora.tx.abandoned", 1);
        return false;
    }
    _count(e, "lora.tx.msgs", 1);
    _count(e, "lora.tx.frames", n);
    return true;
}

bool lora_engine_on_unicast(lora_engine_t *e, const char *peer_uuid, const char *process,
                            const char *function, const uint8_t *frame, size_t len)
{
    char u[37];
    uuid_t parsed;
    if (peer_uuid == NULL || uuid_parse(peer_uuid, parsed) != 0)
        return false;
    uuid_unparse_lower(parsed, u);
    lora_link_state_t st;
    e->link.state(e->link.ctx, &st);
    uint32_t mtu = st.up ? st.mtu : 0, addr = 0;
    bool have = lora_addrmap_addr_of(e->map, u, &addr);
    lora_unicast_in_t in = {st.up, have, e->env.quiet_for(e->env.arg, u),
                            e->cfg.ip_quiet_s, lora_config_allows(&e->cfg, process, function),
                            _proto(e), len,
                            lora_max_message(LORA_T_PEER, mtu, e->cfg.max_fragments)};
    if (!lora_policy_unicast(&in))
        return false;
    uint16_t id = _next_id(e);
    lora_frags_t f, held;
    if (lora_fragment(frame, len, LORA_T_PEER, id, NULL, mtu, e->cfg.max_fragments, &f) != 0)
        return false;
    if (_copy(&f, &held) != 0) {
        lora_frags_free(&f);
        return false;
    }
    if (!_submit(e, addr, &f, true)) {
        lora_frags_free(&held);
        return false;
    }
    lora_hold_put(e->hold, addr, id, &held, e->env.now(e->env.arg));
    return true;
}

bool lora_engine_on_fanout(lora_engine_t *e, bool broadcast, const char *process,
                           const char *function, const uint8_t *frame, size_t len)
{
    lora_link_state_t st;
    e->link.state(e->link.ctx, &st);
    uint32_t mtu = st.up ? st.mtu : 0;
    uint8_t type = broadcast ? LORA_T_BCAST : LORA_T_GROUP;
    uint8_t tag[4];
    bool have_tag = e->env.group_tag(e->env.arg, tag);
    if (!broadcast && !have_tag)
        return false;
    bool proto = broadcast ? true : _proto(e);
    lora_fanout_in_t in = {broadcast ? LORA_CH_BCAST : LORA_CH_GROUP, st.up, _quiet_count(e),
                           lora_config_allows(&e->cfg, process, function), proto, len,
                           lora_max_message(type, mtu, e->cfg.max_fragments)};
    if (!lora_policy_fanout(&in))
        return false;
    lora_frags_t f;
    if (lora_fragment(frame, len, type, _next_id(e), broadcast ? NULL : tag, mtu,
                      e->cfg.max_fragments, &f) != 0)
        return false;
    return _submit(e, LORA_BROADCAST, &f, false);
}

/* The map is full: drop the entry off the roster with the oldest issued_ms
 * (the lowest uuid on a tie) and try @p u once more. With every entry a
 * member, count it and WARN at most once a minute. */
static lora_ann_t _make_room(lora_engine_t *e, const char *u, uint32_t addr, uint64_t issued)
{
    lora_addr_entry_t snap[LORA_ADDRMAP_MAX];
    size_t k = lora_addrmap_snapshot(e->map, snap, LORA_ADDRMAP_MAX);
    const lora_addr_entry_t *oldest = NULL;
    for (size_t i = 0; i < k && i < LORA_ADDRMAP_MAX; i++) {
        if (_in_roster(e, snap[i].uuid))
            continue;
        if (oldest == NULL || snap[i].issued_ms < oldest->issued_ms
            || (snap[i].issued_ms == oldest->issued_ms && strcmp(snap[i].uuid, oldest->uuid) < 0))
            oldest = &snap[i];
    }
    if (oldest != NULL) {
        lora_addrmap_remove(e->map, oldest->uuid);
        return lora_addrmap_announce(e->map, u, addr, issued, _is_member, e);
    }
    _count(e, "lora.announce.full", 1);
    double now = e->env.now(e->env.arg);
    if (now - e->full_warned_at >= WARN_EVERY_S) {
        e->full_warned_at = now;
        _logf(e, 1, "lora: the address map is full (%d entries, all members); %s is not mapped",
              LORA_ADDRMAP_MAX, u);
    }
    return LORA_ANN_FULL;
}

int lora_engine_on_announce(lora_engine_t *e, const char *sender_uuid, json_t *obj,
                            bool verified)
{
    char me[37], u[37];
    uuid_t parsed;
    if (sender_uuid == NULL || uuid_parse(sender_uuid, parsed) != 0)
        return LORA_ANN_BAD;
    uuid_unparse_lower(parsed, u);
    e->env.my_uuid(e->env.arg, me);
    if (strcmp(u, me) == 0)
        return LORA_ANN_SELF;
    if (!verified) {
        _count(e, "lora.announce.unverified", 1);
        return LORA_ANN_REFUSED;
    }
    _members(e);
    if (!_in_roster(e, u))
        return LORA_ANN_REFUSED;
    json_t *a = json_object_get(obj, "lora_addr"), *t = json_object_get(obj, "issued_ms");
    if (!json_is_object(obj) || !json_is_integer(a) || !json_is_integer(t))
        return LORA_ANN_BAD;
    json_int_t av = json_integer_value(a), tv = json_integer_value(t);
    if (av <= 0 || av >= (json_int_t)0xFFFFFFFF || tv < 0)
        return LORA_ANN_BAD;
    uint32_t addr = (uint32_t)av;
    lora_ann_t r = lora_addrmap_announce(e->map, u, addr, (uint64_t)tv, _is_member, e);
    if (r == LORA_ANN_FULL)
        r = _make_room(e, u, addr, (uint64_t)tv);
    if (r == LORA_ANN_ACCEPTED) {
        _count(e, "lora.announce.accepted", 1);
        _save(e);
        if (lora_addrmap_addr_conflicted(e->map, addr)) {
            lora_addr_entry_t snap[LORA_ADDRMAP_MAX];
            size_t k = lora_addrmap_snapshot(e->map, snap, LORA_ADDRMAP_MAX);
            char others[400] = "";
            size_t used = 0;
            for (size_t i = 0; i < k && i < LORA_ADDRMAP_MAX; i++) {
                if (snap[i].addr != addr || strcmp(snap[i].uuid, u) == 0)
                    continue;
                size_t ul = strlen(snap[i].uuid);
                if (used + 2 + ul + 1 > sizeof(others))
                    break;
                if (used > 0) {
                    memcpy(others + used, ", ", 2);
                    used += 2;
                }
                memcpy(others + used, snap[i].uuid, ul + 1);
                used += ul;
            }
            _logf(e, 1, "lora: %s and %s both claim LoRa address %#x; LoRa to and from either "
                  "is off until one announces another", u, others, addr);
        }
    } else if (r == LORA_ANN_STALE) {
        _count(e, "lora.announce.stale", 1);
    }
    return (int)r;
}

void lora_engine_on_exclusion(lora_engine_t *e, const char *uuid, bool excluded)
{
    char u[37];
    uuid_t parsed;
    if (!excluded || uuid == NULL || uuid_parse(uuid, parsed) != 0)
        return;
    uuid_unparse_lower(parsed, u);
    if (lora_addrmap_remove(e->map, u))
        _save(e);
}

bool lora_engine_announce_due(lora_engine_t *e, uint32_t *addr, uint64_t *issued_ms)
{
    lora_link_state_t st;
    e->link.state(e->link.ctx, &st);
    if (!st.up) {
        e->was_down = true;
        return false;
    }
    uint8_t tag[4];
    if (st.node_addr == 0 || !e->env.group_tag(e->env.arg, tag))
        return false;
    double now = e->env.now(e->env.arg);
    _members(e);
    bool new_member = false;
    for (size_t i = 0; i < e->n_roster && !new_member; i++) {
        bool known = false;
        for (size_t k = 0; k < e->n_announced_to && !known; k++)
            known = strcmp(e->roster[i], e->announced_to[k]) == 0;
        new_member = !known;
    }
    bool due = st.node_addr != e->announced_addr
               || e->was_down
               || now - e->last_announce >= e->cfg.announce_s
               || (new_member && now - e->last_announce >= MEMBER_ANNOUNCE_S);
    if (!due)
        return false;
    uint64_t wall = e->env.wall_ms(e->env.arg);
    *addr = st.node_addr;
    *issued_ms = wall > e->self_issued ? wall : e->self_issued + 1;
    return true;
}

void lora_engine_announce_sent(lora_engine_t *e, uint32_t addr, uint64_t issued_ms)
{
    e->self_issued = issued_ms;
    e->announced_addr = addr;
    e->last_announce = e->env.now(e->env.arg);
    _members(e);
    memcpy(e->announced_to, e->roster, e->n_roster * 37);
    e->n_announced_to = e->n_roster;
    e->was_down = false;
    _save(e);
    _count(e, "lora.announce.sent", 1);
}

/* Count a frame from @p src dropped for @p why, and log it unless that reason
 * was logged in the last WARN_EVERY_S. */
static void _drop(lora_engine_t *e, drop_t why, uint32_t src, double now)
{
    char name[48];
    snprintf(name, sizeof(name), "lora.rx.%s", DROPS[why].key);
    _count(e, name, 1);
    if (now - e->drop_warned_at[why] < WARN_EVERY_S)
        return;
    e->drop_warned_at[why] = now;
    _logf(e, DROPS[why].level,
          "lora: dropped %s from 0x%x (counted in %s; logged at most once a minute)",
          DROPS[why].what, src, name);
}

/* Answer a NACK from @p src with the held fragments it is missing. */
static void _serve_nack(lora_engine_t *e, uint32_t src, const lora_frame_t *f, double now)
{
    lora_frags_t frames;
    size_t n = lora_hold_serve(e->hold, src, f->msg_id, f->missing, now, &frames);
    if (n == 0)
        return;
    if (e->link.submit(e->link.ctx, src, &frames, true) == 0) {
        _count(e, "lora.nack.served", 1);
        _count(e, "lora.tx.frames", n);
    } else {
        _count(e, "lora.tx.abandoned", 1);
    }
    lora_frags_free(&frames);
}

/* Reassemble one data frame from @p src (@p uuid); true with a delivery in @p out. */
static bool _data(lora_engine_t *e, uint32_t src, const char *uuid, const lora_frame_t *f,
                  double now, lora_delivery_t *out)
{
    uint8_t tag[4];
    if (f->type == LORA_T_GROUP
        && (!e->env.group_tag(e->env.arg, tag) || memcmp(f->tag, tag, 4) != 0)) {
        _drop(e, DROP_FOREIGN, src, now);
        return false;
    }
    uint8_t *msg = NULL;
    size_t len = 0;
    lora_rx_status_t st = lora_reasm_add(e->reasm, src, f, now, &msg, &len);
    if (st == LORA_RX_DUPLICATE) {
        _drop(e, DROP_DUPLICATE, src, now);
    } else if (st == LORA_RX_BAD) {
        _drop(e, DROP_BAD, src, now);
    } else if (st == LORA_RX_COMPLETE) {
        _count(e, "lora.rx.delivered", 1);
        out->channel = f->type == LORA_T_PEER    ? LORA_DELIVER_PEER
                       : f->type == LORA_T_GROUP ? LORA_DELIVER_GROUP
                                                 : LORA_DELIVER_BCAST;
        memcpy(out->uuid, uuid, 37);
        out->msg = msg;
        out->len = len;
        return true;
    }
    return false;
}

size_t lora_engine_poll(lora_engine_t *e, lora_delivery_t *out, size_t max)
{
    double now = e->env.now(e->env.arg);
    lora_link_result_t res[16];
    size_t nr;
    while ((nr = e->link.poll_results(e->link.ctx, res, 16)) > 0)
        for (size_t i = 0; i < nr; i++)
            if (!res[i].ok) {
                char name[48];
                snprintf(name, sizeof(name), "lora.tx.refused.%.*s",
                         (int)sizeof(res[i].reason), res[i].reason);
                _count(e, name, 1);
            }
    /* Results the link had no room to queue: refusals the counters above miss. */
    _set(e, "lora.tx.result_dropped",
         e->link.results_dropped != NULL ? e->link.results_dropped(e->link.ctx) : 0);
    size_t n = 0;
    lora_link_rx_t rx[LORA_LINK_RX_MAX];
    /* At most @p max frames: each completes at most one message, so every
     * completed message is returned; the rest stay queued in the link. */
    size_t k = e->link.poll_rx(e->link.ctx, rx, max < LORA_LINK_RX_MAX ? max : LORA_LINK_RX_MAX);
    for (size_t i = 0; i < k; i++) {
        lora_link_rx_t *r = &rx[i];
        lora_frame_t f;
        char uuid[37];
        _count(e, "lora.rx.frames", 1);
        if (r->via != 0 || lora_frame_decode(r->payload, r->len, &f) != 0) {
            _drop(e, DROP_BAD, r->src, now);
        } else if (!lora_addrmap_uuid_of(e->map, r->src, uuid)) {
            _drop(e, lora_addrmap_addr_conflicted(e->map, r->src) ? DROP_CONFLICT : DROP_UNKNOWN,
                  r->src, now);
        } else if (f.type == LORA_T_NACK) {
            _serve_nack(e, r->src, &f, now);
        } else {
            lora_delivery_t d;
            if (_data(e, r->src, uuid, &f, now, &d))
                out[n++] = d;   /* n < max: one frame completes at most one message */
        }
        free(r->payload);
    }
    lora_nack_t nk[LORA_REASM_MAX];
    size_t nn = lora_reasm_tick(e->reasm, now, nk, LORA_REASM_MAX);
    for (size_t i = 0; i < nn; i++) {
        lora_frags_t one = {0};
        one.buf[0] = malloc(5);
        if (one.buf[0] == NULL) {
            _count(e, "lora.tx.abandoned", 1);
            continue;
        }
        lora_encode_nack(nk[i].msg_id, nk[i].missing, one.buf[0], 5);
        one.len[0] = 5;
        one.n = 1;
        if (e->link.submit(e->link.ctx, nk[i].src, &one, false) == 0) {
            _count(e, "lora.nack.sent", 1);
            _count(e, "lora.tx.frames", 1);
        } else {
            _count(e, "lora.tx.abandoned", 1);
        }
        lora_frags_free(&one);
    }
    lora_reasm_stats_t rs = lora_reasm_stats(e->reasm);
    _set(e, "lora.rx.reasm_timeout", rs.timeouts);
    _set(e, "lora.rx.evicted", rs.evicted);
    return n;
}

json_t *lora_engine_stats(lora_engine_t *e)
{
    json_t *c = json_object();
    for (size_t i = 0; i < e->n_c; i++)
        json_object_set_new(c, e->c[i].name, json_integer((json_int_t)e->c[i].v));
    lora_link_state_t st;
    e->link.state(e->link.ctx, &st);
    json_t *link = json_pack("{s:b,s:b,s:b,s:I,s:I}", "up", st.up, "connected", st.connected,
                             "claimed", st.claimed, "node_addr", (json_int_t)st.node_addr,
                             "mtu", (json_int_t)st.mtu);
    json_t *map = json_array();
    lora_addr_entry_t snap[LORA_ADDRMAP_MAX];
    size_t k = lora_addrmap_snapshot(e->map, snap, LORA_ADDRMAP_MAX);
    for (size_t i = 0; i < k && i < LORA_ADDRMAP_MAX; i++)
        json_array_append_new(map, json_pack("{s:s,s:I,s:I,s:b}", "uuid", snap[i].uuid,
                                             "addr", (json_int_t)snap[i].addr,
                                             "issued_ms", (json_int_t)snap[i].issued_ms,
                                             "conflicted", snap[i].conflicted));
    return json_pack("{s:o,s:o,s:o}", "counters", c, "link", link, "addrmap", map);
}

bool lora_engine_summary_due(lora_engine_t *e, char *line, size_t cap)
{
    double now = e->env.now(e->env.arg);
    if (now - e->last_summary < SUMMARY_S)
        return false;
    e->last_summary = now;
    uint64_t refused = 0;
    for (size_t i = 0; i < e->n_c; i++)
        if (strncmp(e->c[i].name, "lora.tx.refused.", 16) == 0)
            refused += e->c[i].v;
    lora_link_state_t st;
    e->link.state(e->link.ctx, &st);
    snprintf(line, cap, "lora: link %s; sent %llu msgs (%llu frames, %llu refused, %llu "
             "abandoned); delivered %llu; nacks sent %llu served %llu; %zu peers mapped",
             st.up ? "up" : "down",
             (unsigned long long)lora_engine_counter(e, "lora.tx.msgs"),
             (unsigned long long)lora_engine_counter(e, "lora.tx.frames"),
             (unsigned long long)refused,
             (unsigned long long)lora_engine_counter(e, "lora.tx.abandoned"),
             (unsigned long long)lora_engine_counter(e, "lora.rx.delivered"),
             (unsigned long long)lora_engine_counter(e, "lora.nack.sent"),
             (unsigned long long)lora_engine_counter(e, "lora.nack.served"),
             lora_addrmap_snapshot(e->map, NULL, 0));
    return true;
}
