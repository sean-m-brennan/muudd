/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The lora protocol adapter (kind: scenario). Twin of the Python LoraAdapter
 * (python/lora_adapter.py). Reads fixtures.lora.op, runs libat_lora's pure
 * units, and asserts expected_state.host. */
#include "lora_adapter.h"

#include <ctype.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <jansson.h>
#include <unistd.h>
#include <uuid/uuid.h>

#include "lora/lora_addrmap.h"
#include "lora/lora_config.h"
#include "lora/lora_frame.h"
#include "lora/lora_policy.h"
#include "lora/lora_reasm.h"

/* The adapter's buffers all derive from these. A frame is at most
 * ADAPTER_MTU_MAX bytes (the largest port MTU a scenario uses) and a message
 * is at most LORA_MAX_FRAGS of them; a trace output row holds ROW_LEN bytes. */
#define ADAPTER_MTU_MAX 512
#define ADAPTER_HDR_MAX 8                                    /* longest header: DATA_GROUP */
#define ADAPTER_MSG_MAX (LORA_MAX_FRAGS * ADAPTER_MTU_MAX)
#define ROW_LEN         512
#define GOT_ROWS        512  /* output rows a trace may produce */

#define FAILF(...)                                                        \
    do {                                                                  \
        char _d[320];                                                     \
        snprintf(_d, sizeof(_d), __VA_ARGS__);                            \
        at_case_result_set_fail(out, 0, "AssertionError", _d);            \
        return;                                                           \
    } while (0)

/* A string member, NULL when absent or not a string. */
static const char *_s(json_t *o, const char *k) { return json_string_value(json_object_get(o, k)); }
/* The same, "" when absent: a malformed scenario then fails its comparison
 * rather than handing NULL to strcmp. */
static const char *_ss(json_t *o, const char *k) { const char *v = _s(o, k); return v ? v : ""; }
static const char *_js(json_t *v) { const char *s = json_string_value(v); return s ? s : ""; }
static long long _i(json_t *o, const char *k) { return json_integer_value(json_object_get(o, k)); }
static bool _want_error(json_t *exp) { return json_is_true(json_object_get(exp, "error")); }

static int _type(const char *name)
{
    if (name == NULL) return -1;
    if (strcmp(name, "peer") == 0) return LORA_T_PEER;
    if (strcmp(name, "group") == 0) return LORA_T_GROUP;
    if (strcmp(name, "bcast") == 0) return LORA_T_BCAST;
    if (strcmp(name, "nack") == 0) return LORA_T_NACK;
    return -1;
}

static const char *_type_name(uint8_t t)
{
    switch (t) {
    case LORA_T_PEER: return "peer";
    case LORA_T_GROUP: return "group";
    case LORA_T_BCAST: return "bcast";
    default: return "nack";
    }
}

/* Hex into @p out; @return the byte count, or -1. */
static long _unhex(const char *h, uint8_t *out, size_t cap)
{
    size_t n = h ? strlen(h) : 0;
    if (n % 2 != 0 || n / 2 > cap)
        return -1;
    for (size_t i = 0; i < n / 2; i++) {
        /* sscanf would take a sign or leading space as part of a number. */
        if (!isxdigit((unsigned char)h[2 * i]) || !isxdigit((unsigned char)h[2 * i + 1]))
            return -1;
        unsigned v;
        sscanf(h + 2 * i, "%2x", &v);
        out[i] = (uint8_t)v;
    }
    return (long)(n / 2);
}

static void _hex(const uint8_t *b, size_t n, char *out)
{
    for (size_t i = 0; i < n; i++)
        sprintf(out + 2 * i, "%02x", b[i]);
    out[2 * n] = '\0';
}

static void _op_fragment(json_t *fx, json_t *exp, at_case_result_t *out)
{
    size_t n = (size_t)_i(fx, "msg_len");
    uint8_t *msg = malloc(n ? n : 1);
    if (msg == NULL)
        FAILF("fragment: out of memory");
    for (size_t i = 0; i < n; i++)
        msg[i] = (uint8_t)(i % 256);
    uint8_t tag[LORA_TAG_LEN];
    const uint8_t *tagp = NULL;
    if (_s(fx, "tag_hex") != NULL && _unhex(_s(fx, "tag_hex"), tag, sizeof(tag)) == LORA_TAG_LEN)
        tagp = tag;
    int type = _type(_s(fx, "type"));
    lora_frags_t f;
    /* The API takes a uint16_t; a scenario's id outside that range is refused, as Python does. */
    long long id = _i(fx, "msg_id");
    int rc = -1;
    if (id >= 0 && id <= 0xFFFF)
        rc = lora_fragment(msg, n, (uint8_t)type, (uint16_t)id, tagp, (uint32_t)_i(fx, "mtu"),
                           (int)_i(fx, "max_fragments"), &f);
    if (rc != 0) {
        free(msg);
        if (!_want_error(exp))
            FAILF("fragment: refused, but the scenario expects frames");
        at_case_result_set_pass(out, 0);
        return;
    }
    json_t *lens = json_object_get(exp, "lengths"), *hdrs = json_object_get(exp, "headers_hex");
    size_t hdr = lora_header_len((uint8_t)type), off = 0;
    bool ok = !_want_error(exp) && json_array_size(lens) == f.n && json_array_size(hdrs) == f.n;
    for (size_t i = 0; ok && i < f.n; i++) {
        char h[2 * ADAPTER_HDR_MAX + 1];
        _hex(f.buf[i], hdr, h);
        ok = (size_t)json_integer_value(json_array_get(lens, i)) == f.len[i]
             && strcmp(h, _js(json_array_get(hdrs, i))) == 0
             && memcmp(f.buf[i] + hdr, msg + off, f.len[i] - hdr) == 0;
        off += f.len[i] - hdr;
    }
    ok = ok && off == n;
    lora_frags_free(&f);
    free(msg);
    if (!ok)
        FAILF("fragment: frames differ from the scenario");
    at_case_result_set_pass(out, 0);
}

static void _op_decode(json_t *fx, json_t *exp, at_case_result_t *out)
{
    uint8_t buf[ADAPTER_MTU_MAX];
    long n = _unhex(_s(fx, "hex"), buf, sizeof(buf));
    lora_frame_t f;
    if (n < 0 || lora_frame_decode(buf, (size_t)n, &f) != 0)
        FAILF("decode: refused %s", _ss(fx, "hex"));
    if (strcmp(_type_name(f.type), _ss(exp, "type")) != 0 || f.msg_id != _i(exp, "msg_id"))
        FAILF("decode: type or msg_id differs");
    if (f.type == LORA_T_NACK) {
        if (f.missing != _i(exp, "missing"))
            FAILF("decode: missing %u", f.missing);
        at_case_result_set_pass(out, 0);
        return;
    }
    char p[2 * ADAPTER_MTU_MAX + 1];
    _hex(f.payload, f.payload_len, p);
    if (f.idx != _i(exp, "idx") || f.count != _i(exp, "count") || strcmp(p, _ss(exp, "payload_hex")) != 0)
        FAILF("decode: idx, count or payload differs");
    if (_s(exp, "tag_hex") != NULL) {
        char t[9];
        _hex(f.tag, LORA_TAG_LEN, t);
        if (strcmp(t, _s(exp, "tag_hex")) != 0)
            FAILF("decode: tag %s", t);
    }
    at_case_result_set_pass(out, 0);
}

static void _op_decode_all(json_t *fx, json_t *exp, at_case_result_t *out)
{
    json_t *hexes = json_object_get(fx, "hexes"), *h;
    size_t i, refused = 0;
    json_array_foreach(hexes, i, h) {
        uint8_t buf[ADAPTER_MTU_MAX];
        lora_frame_t f;
        long n = _unhex(json_string_value(h), buf, sizeof(buf));
        if (n < 0 || lora_frame_decode(buf, (size_t)n, &f) != 0)
            refused++;
    }
    if ((refused == json_array_size(hexes)) != json_is_true(json_object_get(exp, "all_refused")))
        FAILF("decode_all: %zu of %zu refused", refused, json_array_size(hexes));
    at_case_result_set_pass(out, 0);
}

static void _op_nack(json_t *fx, json_t *exp, at_case_result_t *out)
{
    uint8_t b[5];
    int n = lora_encode_nack((uint16_t)_i(fx, "msg_id"), (uint16_t)_i(fx, "missing"), b, sizeof(b));
    if (n < 0) {
        if (!_want_error(exp))
            FAILF("nack: refused");
        at_case_result_set_pass(out, 0);
        return;
    }
    char h[11];
    _hex(b, 5, h);
    if (_want_error(exp) || strcmp(h, _ss(exp, "hex")) != 0)
        FAILF("nack: %s", h);
    at_case_result_set_pass(out, 0);
}

static bool _outputs_match(json_t *exp, char got[][ROW_LEN], size_t n)
{
    json_t *want = json_object_get(exp, "outputs");
    if (json_array_size(want) != n)
        return false;
    for (size_t i = 0; i < n; i++)
        if (strcmp(got[i], _js(json_array_get(want, i))) != 0) {
            fprintf(stderr, "lora: output %zu got '%s' want '%s'\n", i, got[i],
                    _js(json_array_get(want, i)));
            return false;
        }
    return true;
}

static void _op_reasm_trace(json_t *fx, json_t *exp, at_case_result_t *out)
{
    lora_reasm_t *r = lora_reasm_new(json_number_value(json_object_get(fx, "nack_gap_s")),
                                     json_number_value(json_object_get(fx, "reassembly_s")));
    json_t *events = json_object_get(fx, "events"), *ev;
    static char got[GOT_ROWS][ROW_LEN];
    size_t i, n = 0;
    json_array_foreach(events, i, ev) {
        double at = json_number_value(json_object_get(ev, "at"));
        if (n >= GOT_ROWS) {
            lora_reasm_free(r);
            FAILF("reasm_trace: more than %d events", GOT_ROWS);
        }
        if (json_is_true(json_object_get(ev, "tick"))) {
            lora_nack_t nk[16];
            size_t k = lora_reasm_tick(r, at, nk, 16);
            int off = snprintf(got[n], sizeof(got[n]), "tick");
            for (size_t j = 0; j < k; j++)
                if ((size_t)off < sizeof(got[n]))
                    off += snprintf(got[n] + off, sizeof(got[n]) - (size_t)off, " nack:%u:%u:%u",
                                    nk[j].src, nk[j].msg_id, nk[j].missing);
            n++;
            continue;
        }
        uint8_t b[ADAPTER_MTU_MAX];
        lora_frame_t f;
        long len = _unhex(_s(ev, "hex"), b, sizeof(b));
        if (len < 0 || lora_frame_decode(b, (size_t)len, &f) != 0) {
            lora_reasm_free(r);
            FAILF("reasm_trace: event %zu does not decode", i);
        }
        uint8_t *m = NULL;
        size_t mlen = 0;
        lora_rx_status_t st = lora_reasm_add(r, (uint32_t)_i(ev, "src"), &f, at, &m, &mlen);
        static const char *names[] = {"partial", "complete", "duplicate", "bad"};
        if (st == LORA_RX_COMPLETE) {
            static char hex[2 * ADAPTER_MSG_MAX + 1];
            _hex(m, mlen, hex);
            /* The precision bounds the row to ROW_LEN bytes with "complete " and the NUL. */
            snprintf(got[n], sizeof(got[n]), "complete %.*s", ROW_LEN - 10, hex);
        } else {
            snprintf(got[n], sizeof(got[n]), "%s", names[st]);
        }
        free(m);
        n++;
    }
    bool ok = _outputs_match(exp, got, n);
    lora_reasm_stats_t s = lora_reasm_stats(r);
    json_t *stats = json_object_get(exp, "stats");
    if (stats != NULL) {
        json_t *v;
        const char *key;
        json_object_foreach(stats, key, v) {
            /* An unknown key fails, as KeyError does in Python. */
            uint64_t got_v;
            if (strcmp(key, "timeouts") == 0) got_v = s.timeouts;
            else if (strcmp(key, "evicted") == 0) got_v = s.evicted;
            else if (strcmp(key, "duplicate") == 0) got_v = s.duplicate;
            else if (strcmp(key, "bad") == 0) got_v = s.bad;
            else {
                fprintf(stderr, "lora: unknown stats key '%s'\n", key);
                ok = false;
                continue;
            }
            if (!json_is_integer(v) || (uint64_t)json_integer_value(v) != got_v)
                ok = false;
        }
    }
    lora_reasm_free(r);
    if (!ok)
        FAILF("reasm_trace: outputs or stats differ");
    at_case_result_set_pass(out, 0);
}

static void _op_hold_trace(json_t *fx, json_t *exp, at_case_result_t *out)
{
    lora_hold_t *h = lora_hold_new(json_number_value(json_object_get(fx, "hold_s")));
    json_t *events = json_object_get(fx, "events"), *ev;
    static char got[GOT_ROWS][ROW_LEN];
    size_t i, n = 0;
    json_array_foreach(events, i, ev) {
        double at = json_number_value(json_object_get(ev, "at"));
        if (n >= GOT_ROWS) {
            lora_hold_free(h);
            FAILF("hold_trace: more than %d events", GOT_ROWS);
        }
        json_t *p = json_object_get(ev, "put"), *s = json_object_get(ev, "serve");
        if (p != NULL) {
            lora_frags_t f = {0};
            json_t *fh;
            size_t k;
            json_array_foreach(json_object_get(p, "frames_hex"), k, fh) {
                uint8_t b[ADAPTER_MTU_MAX];
                long len = _unhex(json_string_value(fh), b, sizeof(b));
                if (len < 0 || f.n >= LORA_MAX_FRAGS
                    || (f.buf[f.n] = malloc((size_t)len ? (size_t)len : 1)) == NULL) {
                    lora_frags_free(&f);
                    lora_hold_free(h);
                    FAILF("hold_trace: event %zu has a bad or surplus frame, or no memory", i);
                }
                memcpy(f.buf[f.n], b, (size_t)len);
                f.len[f.n++] = (size_t)len;
            }
            lora_hold_put(h, (uint32_t)_i(p, "dest"), (uint16_t)_i(p, "msg_id"), &f, at);
            snprintf(got[n++], sizeof(got[0]), "held");
        } else {
            lora_frags_t f;
            size_t k = lora_hold_serve(h, (uint32_t)_i(s, "dest"), (uint16_t)_i(s, "msg_id"),
                                       (uint16_t)_i(s, "missing"), at, &f);
            int off = snprintf(got[n], sizeof(got[0]), "served ");
            if (k == 0)
                snprintf(got[n] + off, sizeof(got[0]) - (size_t)off, "none");
            for (size_t j = 0; j < k; j++) {
                char hex[2 * ADAPTER_MTU_MAX + 1];
                _hex(f.buf[j], f.len[j], hex);
                if ((size_t)off < sizeof(got[0]))
                    off += snprintf(got[n] + off, sizeof(got[0]) - (size_t)off, "%s%s",
                                    j ? "," : "", hex);
            }
            lora_frags_free(&f);
            n++;
        }
    }
    bool ok = _outputs_match(exp, got, n);
    lora_hold_free(h);
    if (!ok)
        FAILF("hold_trace: outputs differ");
    at_case_result_set_pass(out, 0);
}

static json_t *_roster;   /* the trace's current members; NULL: everyone */

static bool _in_roster(const char *uuid, void *arg)
{
    (void)arg;
    size_t i;
    json_t *u;
    json_array_foreach(_roster, i, u) {
        char norm[LORA_UUID_LEN + 1];
        uuid_t parsed;
        if (uuid_parse(_js(u), parsed) == 0) {
            uuid_unparse_lower(parsed, norm);
            if (strcmp(norm, uuid) == 0)
                return true;
        }
    }
    return false;
}

/* The file a save event writes and a load event reads, in the trace's own directory. */
static char _map_dir[64], _map_path[128];

static void _map_cleanup(void)
{
    char tmp[sizeof(_map_path) + 8];
    snprintf(tmp, sizeof(tmp), "%s.tmp", _map_path);
    unlink(tmp);
    unlink(_map_path);
    rmdir(_map_dir);
}

/* The file's JSON, keys sorted and no spaces, into @p out; false when it cannot be read. */
static bool _canonical_file(char *out, size_t cap)
{
    json_t *doc = json_load_file(_map_path, 0, NULL);
    char *text = doc ? json_dumps(doc, JSON_COMPACT | JSON_SORT_KEYS) : NULL;
    bool ok = text != NULL && strlen(text) < cap;
    if (ok)
        snprintf(out, cap, "%s", text);
    free(text);
    json_decref(doc);
    return ok;
}

/* Replace @p m from @p load: {"text": file contents} or {"absent": true}. */
static void _map_load(lora_addrmap_t *m, json_t *load, char *row, size_t cap)
{
    unlink(_map_path);
    const char *text = _s(load, "text");
    if (text != NULL) {
        FILE *f = fopen(_map_path, "w");
        if (f == NULL || fputs(text, f) == EOF) {
            if (f != NULL)
                fclose(f);
            snprintf(row, cap, "load failed");
            return;
        }
        fclose(f);
    }
    uint64_t self_ms;
    int rc = lora_addrmap_load(m, _map_path, &self_ms);
    if (rc < 0)
        snprintf(row, cap, "load failed");
    else
        snprintf(row, cap, "loaded self=%llu skipped=%d", (unsigned long long)self_ms, rc);
}

static void _op_addrmap_trace(json_t *fx, json_t *exp, at_case_result_t *out)
{
    static const char *ann[] = {"accepted", "stale", "bad", "full"};
    static char got[GOT_ROWS][ROW_LEN];
    snprintf(_map_dir, sizeof(_map_dir), "/tmp/lora-adapter-XXXXXX");
    if (mkdtemp(_map_dir) == NULL)
        FAILF("addrmap_trace: no temporary directory");
    snprintf(_map_path, sizeof(_map_path), "%s/addrmap.json", _map_dir);
    lora_addrmap_t *m = lora_addrmap_new();
    _roster = NULL;
    size_t i, n = 0;
    json_t *ev;
    json_array_foreach(json_object_get(fx, "events"), i, ev) {
        json_t *v;
        if (n >= GOT_ROWS) {
            lora_addrmap_free(m);
            _roster = NULL;
            _map_cleanup();
            FAILF("addrmap_trace: more than %d events", GOT_ROWS);
        }
        if ((v = json_object_get(ev, "members")) != NULL) {
            _roster = v;
            snprintf(got[n++], sizeof(got[0]), "ok");
        } else if ((v = json_object_get(ev, "announce")) != NULL) {
            lora_ann_t r = lora_addrmap_announce(
                m, _js(json_array_get(v, 0)),
                (uint32_t)json_integer_value(json_array_get(v, 1)),
                (uint64_t)json_integer_value(json_array_get(v, 2)),
                _roster != NULL ? _in_roster : NULL, NULL);
            snprintf(got[n++], sizeof(got[0]), "%s", ann[r]);
        } else if ((v = json_object_get(ev, "addr_of")) != NULL) {
            uint32_t a;
            if (lora_addrmap_addr_of(m, _js(v), &a))
                snprintf(got[n++], sizeof(got[0]), "addr %u", a);
            else
                snprintf(got[n++], sizeof(got[0]), "none");
        } else if ((v = json_object_get(ev, "uuid_of")) != NULL) {
            char u[LORA_UUID_LEN + 1];
            if (lora_addrmap_uuid_of(m, (uint32_t)json_integer_value(v), u))
                snprintf(got[n++], sizeof(got[0]), "uuid %s", u);
            else
                snprintf(got[n++], sizeof(got[0]), "none");
        } else if ((v = json_object_get(ev, "save")) != NULL) {
            snprintf(got[n++], sizeof(got[0]), "%s",
                     lora_addrmap_save(m, _map_path, (uint64_t)json_integer_value(v)) == 0
                         ? "saved" : "save failed");
        } else if (json_object_get(ev, "saved_file") != NULL) {
            char text[ROW_LEN - 8];
            int off = snprintf(got[n], sizeof(got[0]), "file ");
            if (_canonical_file(text, sizeof(text)))
                snprintf(got[n] + off, sizeof(got[0]) - (size_t)off, "%s", text);
            else
                snprintf(got[n], sizeof(got[0]), "no file");
            n++;
        } else if ((v = json_object_get(ev, "load")) != NULL) {
            _map_load(m, v, got[n], sizeof(got[0]));
            n++;
        } else {
            v = json_object_get(ev, "remove");
            snprintf(got[n++], sizeof(got[0]), "%s",
                     lora_addrmap_remove(m, _js(v)) ? "removed" : "absent");
        }
    }
    bool ok = _outputs_match(exp, got, n);
    lora_addrmap_free(m);
    _roster = NULL;
    _map_cleanup();
    if (!ok)
        FAILF("addrmap_trace: outputs differ");
    at_case_result_set_pass(out, 0);
}

static bool _b(json_t *o, const char *k) { return json_is_true(json_object_get(o, k)); }
static double _d(json_t *o, const char *k) { return json_number_value(json_object_get(o, k)); }

static void _op_policy(json_t *fx, json_t *exp, at_case_result_t *out)
{
    size_t i, bad = 0;
    json_t *row;
    json_array_foreach(json_object_get(fx, "rows"), i, row) {
        bool got;
        if (strcmp(_ss(row, "kind"), "unicast") == 0) {
            lora_unicast_in_t in = {_b(row, "link_up"), _b(row, "have_addr"), _d(row, "quiet_s"),
                                    _d(row, "ip_quiet_s"), _b(row, "allowed"), _b(row, "proto"),
                                    (size_t)_i(row, "frame_len"), (size_t)_i(row, "max_len")};
            got = lora_policy_unicast(&in);
        } else {
            lora_fanout_in_t in = {strcmp(_ss(row, "channel"), "broadcast") == 0 ? LORA_CH_BCAST
                                                                                : LORA_CH_GROUP,
                                   _b(row, "link_up"), (size_t)_i(row, "n_quiet"), _b(row, "allowed"),
                                   _b(row, "proto"), (size_t)_i(row, "frame_len"),
                                   (size_t)_i(row, "max_len")};
            got = lora_policy_fanout(&in);
        }
        if (got != _b(row, "send")) {
            fprintf(stderr, "lora policy: row %zu got %d\n", i, got);
            bad++;
        }
    }
    if ((bad == 0) != _b(exp, "all_rows_match"))
        FAILF("policy: %zu rows differ", bad);
    at_case_result_set_pass(out, 0);
}

/* One accepted case's `values` against the parsed config. */
static bool _config_values_match(json_t *values, const lora_cfg_t *c)
{
    const char *key;
    json_t *v;
    json_object_foreach(values, key, v) {
        if (strcmp(key, "socket") == 0) {
            if (strcmp(c->socket, _js(v)) != 0) return false;
        } else if (strcmp(key, "allow") == 0) {
            if (json_array_size(v) != c->n_allow) return false;
            for (size_t k = 0; k < c->n_allow; k++)
                if (strcmp(c->allow[k], _js(json_array_get(v, k))) != 0) return false;
        } else {
            long long want = json_integer_value(v), got =
                strcmp(key, "ip_quiet_s") == 0 ? c->ip_quiet_s :
                strcmp(key, "max_fragments") == 0 ? c->max_fragments :
                strcmp(key, "nack_gap_s") == 0 ? c->nack_gap_s :
                strcmp(key, "nack_hold_s") == 0 ? c->nack_hold_s :
                strcmp(key, "reassembly_s") == 0 ? c->reassembly_s :
                strcmp(key, "announce_s") == 0 ? c->announce_s :
                strcmp(key, "port") == 0 ? (long long)c->port : -1;
            if (got != want) return false;
        }
    }
    return true;
}

static void _op_config(json_t *fx, json_t *exp, at_case_result_t *out)
{
    size_t i, bad = 0;
    json_t *cs;
    json_array_foreach(json_object_get(fx, "cases"), i, cs) {
        lora_cfg_t c;
        char field[64], why[160];
        int rc = lora_config_parse(_ss(cs, "json"), &c, field, sizeof(field), why, sizeof(why));
        bool ok;
        if (rc != 0)
            ok = _s(cs, "field") != NULL && strcmp(_s(cs, "field"), field) == 0;
        else
            ok = _b(cs, "ok") && (json_object_get(cs, "values") == NULL
                                  || _config_values_match(json_object_get(cs, "values"), &c));
        if (!ok) {
            fprintf(stderr, "lora config: case %zu (%s) gave %s\n", i, _ss(cs, "json"),
                    rc != 0 ? field : "accepted");
            bad++;
        }
    }
    if ((bad == 0) != _b(exp, "all_cases_match"))
        FAILF("config: %zu cases differ", bad);
    at_case_result_set_pass(out, 0);
}

void at_lora_conf_run(const at_case_t *c, at_case_result_t *out)
{
    if (strcmp(c->kind, "scenario") != 0) {
        at_case_result_set_skip(out, "C lora adapter only handles kind:scenario");
        return;
    }
    json_t *fixtures = json_object_get(c->data, "fixtures");
    json_t *fx = json_is_object(fixtures) ? json_object_get(fixtures, "lora") : NULL;
    json_t *state = json_object_get(c->data, "expected_state");
    json_t *exp = json_is_object(state) ? json_object_get(state, "host") : NULL;
    const char *op = fx ? _s(fx, "op") : NULL;
    if (op == NULL || exp == NULL) {
        at_case_result_set_fail(out, 0, "AssertionError", "missing fixtures.lora.op or expected_state.host");
        return;
    }
    if (strcmp(op, "fragment") == 0) _op_fragment(fx, exp, out);
    else if (strcmp(op, "decode") == 0) _op_decode(fx, exp, out);
    else if (strcmp(op, "decode_all") == 0) _op_decode_all(fx, exp, out);
    else if (strcmp(op, "nack") == 0) _op_nack(fx, exp, out);
    else if (strcmp(op, "reasm_trace") == 0) _op_reasm_trace(fx, exp, out);
    else if (strcmp(op, "hold_trace") == 0) _op_hold_trace(fx, exp, out);
    else if (strcmp(op, "addrmap_trace") == 0) _op_addrmap_trace(fx, exp, out);
    else if (strcmp(op, "policy") == 0) _op_policy(fx, exp, out);
    else if (strcmp(op, "config") == 0) _op_config(fx, exp, out);
    else {
        char d[160];
        snprintf(d, sizeof(d), "unknown lora op %s", op);
        at_case_result_set_fail(out, 0, "AssertionError", d);
    }
}
