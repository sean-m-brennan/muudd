/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <jansson.h>

#include "lora/lora_config.h"

#define CFG_MAX_BYTES 16384

static const char *const DEFAULT_ALLOW[] = {
    "identity/group_key_update", "identity/group_partition_probe",
    "identity/group_partition_response", "identity/peer_dm",
};

void lora_config_defaults(lora_cfg_t *c)
{
    memset(c, 0, sizeof(*c));
    snprintf(c->socket, sizeof(c->socket), "%s", "/run/muudd/radio.sock");
    c->ip_quiet_s = 65;
    c->max_fragments = 8;
    c->nack_gap_s = 20;
    c->nack_hold_s = 120;
    c->reassembly_s = 90;
    c->announce_s = 600;
    c->n_allow = sizeof(DEFAULT_ALLOW) / sizeof(DEFAULT_ALLOW[0]);
    for (size_t i = 0; i < c->n_allow; i++)
        snprintf(c->allow[i], LORA_ALLOW_LEN, "%s", DEFAULT_ALLOW[i]);
}

static int _fail(char *field, size_t fl, char *why, size_t wl, const char *f, const char *w)
{
    snprintf(field, fl, "%s", f);
    snprintf(why, wl, "%s", w);
    return -1;
}

/* One part of process/function: 1..64 of [a-z0-9_]. */
static bool _part_ok(const char *s, size_t n)
{
    if (n < 1 || n > 64)
        return false;
    for (size_t i = 0; i < n; i++)
        if (!((s[i] >= 'a' && s[i] <= 'z') || (s[i] >= '0' && s[i] <= '9') || s[i] == '_'))
            return false;
    return true;
}

static bool _verb_ok(const char *v)
{
    const char *slash = v ? strchr(v, '/') : NULL;
    return slash != NULL && _part_ok(v, (size_t)(slash - v))
           && _part_ok(slash + 1, strlen(slash + 1));
}

static int _uint(json_t *root, const char *key, unsigned lo, unsigned hi, unsigned *out,
                 char *field, size_t fl, char *why, size_t wl)
{
    json_t *v = json_object_get(root, key);
    if (v == NULL)
        return 0;
    if (!json_is_integer(v) || json_integer_value(v) < (json_int_t)lo
        || json_integer_value(v) > (json_int_t)hi) {
        char w[96];
        snprintf(w, sizeof(w), "must be an integer from %u to %u", lo, hi);
        return _fail(field, fl, why, wl, key, w);
    }
    *out = (unsigned)json_integer_value(v);
    return 0;
}

static int _parse(const char *text, size_t len, lora_cfg_t *out, char *field, size_t fl,
                  char *why, size_t wl)
{
    static const char *const known[] = {"socket", "port", "ip_quiet_s", "max_fragments",
                                        "nack_gap_s", "nack_hold_s", "reassembly_s",
                                        "announce_s", "allow"};
    lora_config_defaults(out);
    json_error_t err;
    json_t *root = json_loadb(text, len, 0, &err);
    if (!json_is_object(root)) {
        json_decref(root);
        return _fail(field, fl, why, wl, "(file)", "not a JSON object");
    }
    int rc = -1;
    const char *key;
    json_t *v;
    json_object_foreach(root, key, v) {
        bool ok = false;
        for (size_t i = 0; i < sizeof(known) / sizeof(known[0]); i++)
            ok = ok || strcmp(key, known[i]) == 0;
        if (!ok) {
            _fail(field, fl, why, wl, key, "unknown field");
            goto out;
        }
    }
    unsigned port = 0;
    if (json_object_get(root, "port") == NULL) {
        _fail(field, fl, why, wl, "port", "required");
        goto out;
    }
    if (_uint(root, "port", 3, 255, &port, field, fl, why, wl)
        || _uint(root, "ip_quiet_s", 10, 86400, &out->ip_quiet_s, field, fl, why, wl)
        || _uint(root, "max_fragments", 1, 16, &out->max_fragments, field, fl, why, wl)
        || _uint(root, "nack_gap_s", 1, 600, &out->nack_gap_s, field, fl, why, wl)
        || _uint(root, "nack_hold_s", 1, 3600, &out->nack_hold_s, field, fl, why, wl)
        || _uint(root, "reassembly_s", 2, 3600, &out->reassembly_s, field, fl, why, wl)
        || _uint(root, "announce_s", 30, 86400, &out->announce_s, field, fl, why, wl))
        goto out;
    out->port = port;
    if (out->nack_gap_s >= out->reassembly_s) {
        _fail(field, fl, why, wl, "nack_gap_s", "must be less than reassembly_s");
        goto out;
    }
    v = json_object_get(root, "socket");
    if (v != NULL) {
        const char *s = json_string_value(v);
        if (s == NULL || s[0] == '\0' || strlen(s) >= sizeof(out->socket)) {
            _fail(field, fl, why, wl, "socket", "must be a non-empty path of at most 107 bytes");
            goto out;
        }
        snprintf(out->socket, sizeof(out->socket), "%s", s);
    }
    v = json_object_get(root, "allow");
    if (v != NULL) {
        if (!json_is_array(v) || json_array_size(v) > LORA_ALLOW_MAX) {
            _fail(field, fl, why, wl, "allow", "must be a list of at most 32 process/function names");
            goto out;
        }
        out->n_allow = 0;
        size_t i;
        json_t *e;
        json_array_foreach(v, i, e) {
            const char *s = json_string_value(e);
            if (!_verb_ok(s) || strcmp(s, "network/lora_addr") == 0) {
                _fail(field, fl, why, wl, "allow",
                      "entries are process/function of [a-z0-9_], never network/lora_addr");
                goto out;
            }
            for (size_t k = 0; k < out->n_allow; k++)
                if (strcmp(out->allow[k], s) == 0) {
                    _fail(field, fl, why, wl, "allow", "entries must be distinct");
                    goto out;
                }
            snprintf(out->allow[out->n_allow++], LORA_ALLOW_LEN, "%s", s);
        }
    }
    rc = 0;
out:
    json_decref(root);
    return rc;
}

int lora_config_parse(const char *text, lora_cfg_t *out, char *field, size_t fl, char *why, size_t wl)
{
    return _parse(text, strlen(text), out, field, fl, why, wl);
}

int lora_config_load(const char *path, lora_cfg_t *out, char *field, size_t fl, char *why, size_t wl)
{
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        if (errno == ENOENT)
            return 1;
        return _fail(field, fl, why, wl, "(file)", "unreadable");
    }
    char buf[CFG_MAX_BYTES + 1];
    size_t n = fread(buf, 1, sizeof(buf), f);
    bool bad = ferror(f) != 0;
    fclose(f);
    if (bad)
        return _fail(field, fl, why, wl, "(file)", "unreadable");
    if (n > CFG_MAX_BYTES)
        return _fail(field, fl, why, wl, "(file)", "larger than 16 KiB");
    return _parse(buf, n, out, field, fl, why, wl);
}

bool lora_config_allows(const lora_cfg_t *c, const char *process, const char *function)
{
    char v[LORA_ALLOW_LEN];
    if (process == NULL || function == NULL)
        return false;
    /* A name too long for v cannot equal an allow entry; a truncated copy might. */
    if (snprintf(v, sizeof(v), "%s/%s", process, function) >= (int)sizeof(v))
        return false;
    for (size_t i = 0; i < c->n_allow; i++)
        if (strcmp(c->allow[i], v) == 0)
            return true;
    return false;
}
