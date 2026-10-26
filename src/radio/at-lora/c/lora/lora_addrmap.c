/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <jansson.h>
#include <uuid/uuid.h>

#include "lora/lora_addrmap.h"

typedef struct {
    bool used;
    char uuid[LORA_UUID_LEN + 1];
    uint32_t addr;
    uint64_t issued_ms;
} slot_t;

struct lora_addrmap {
    slot_t s[LORA_ADDRMAP_MAX];
    char skipped[400];   /* the keys the last load skipped, comma-separated */
};

lora_addrmap_t *lora_addrmap_new(void) { return calloc(1, sizeof(lora_addrmap_t)); }
void lora_addrmap_free(lora_addrmap_t *m) { free(m); }

/* Lower-case, canonical form of @p in into @p out; false if not a UUID. */
static bool _norm(const char *in, char *out)
{
    uuid_t u;
    if (in == NULL || uuid_parse(in, u) != 0)
        return false;
    uuid_unparse_lower(u, out);
    return true;
}

static slot_t *_find(const lora_addrmap_t *m, const char *uuid)
{
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++)
        if (m->s[i].used && strcmp(m->s[i].uuid, uuid) == 0)
            return (slot_t *)&m->s[i];
    return NULL;
}

static int _holders(const lora_addrmap_t *m, uint32_t addr)
{
    int n = 0;
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++)
        n += m->s[i].used && m->s[i].addr == addr;
    return n;
}

lora_ann_t lora_addrmap_announce(lora_addrmap_t *m, const char *uuid, uint32_t addr,
                                 uint64_t issued_ms, lora_is_member_fn is_member, void *arg)
{
    char u[LORA_UUID_LEN + 1];
    if (!_norm(uuid, u) || addr == 0 || addr == 0xFFFFFFFFu)
        return LORA_ANN_BAD;
    slot_t *s = _find(m, u);
    if (s != NULL && issued_ms <= s->issued_ms)
        return LORA_ANN_STALE;
    if (s == NULL) {
        for (int i = 0; i < LORA_ADDRMAP_MAX && s == NULL; i++)
            if (!m->s[i].used)
                s = &m->s[i];
        if (s == NULL)
            return LORA_ANN_FULL;
        s->used = true;
        memcpy(s->uuid, u, sizeof(u));
    }
    s->addr = addr;
    s->issued_ms = issued_ms;
    /* A radio moved from a node no longer on the roster: its old entry goes,
     * rather than conflicting forever with a holder that will never speak. */
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++) {
        slot_t *o = &m->s[i];
        if (o->used && o != s && o->addr == addr && is_member != NULL && !is_member(o->uuid, arg))
            memset(o, 0, sizeof(*o));
    }
    return LORA_ANN_ACCEPTED;
}

bool lora_addrmap_addr_of(const lora_addrmap_t *m, const char *uuid, uint32_t *addr)
{
    char u[LORA_UUID_LEN + 1];
    if (!_norm(uuid, u))
        return false;
    slot_t *s = _find(m, u);
    if (s == NULL || _holders(m, s->addr) != 1)
        return false;
    *addr = s->addr;
    return true;
}

bool lora_addrmap_uuid_of(const lora_addrmap_t *m, uint32_t addr, char *uuid_out)
{
    if (_holders(m, addr) != 1)
        return false;
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++)
        if (m->s[i].used && m->s[i].addr == addr) {
            memcpy(uuid_out, m->s[i].uuid, LORA_UUID_LEN + 1);
            return true;
        }
    return false;
}

bool lora_addrmap_addr_conflicted(const lora_addrmap_t *m, uint32_t addr)
{
    return _holders(m, addr) > 1;
}

bool lora_addrmap_remove(lora_addrmap_t *m, const char *uuid)
{
    char u[LORA_UUID_LEN + 1];
    slot_t *s = _norm(uuid, u) ? _find(m, u) : NULL;
    if (s == NULL)
        return false;
    memset(s, 0, sizeof(*s));
    return true;
}

size_t lora_addrmap_snapshot(const lora_addrmap_t *m, lora_addr_entry_t *out, size_t max)
{
    size_t n = 0;
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++) {
        if (!m->s[i].used)
            continue;
        if (out != NULL && n < max) {
            memcpy(out[n].uuid, m->s[i].uuid, sizeof(out[n].uuid));
            out[n].addr = m->s[i].addr;
            out[n].issued_ms = m->s[i].issued_ms;
            out[n].conflicted = _holders(m, m->s[i].addr) > 1;
        }
        n++;
    }
    return n;
}

int lora_addrmap_save(const lora_addrmap_t *m, const char *path, uint64_t self_issued_ms)
{
    json_t *peers = json_object();
    for (int i = 0; i < LORA_ADDRMAP_MAX; i++)
        if (m->s[i].used)
            json_object_set_new(peers, m->s[i].uuid,
                                json_pack("{s:I,s:I}", "addr", (json_int_t)m->s[i].addr,
                                          "issued_ms", (json_int_t)m->s[i].issued_ms));
    json_t *root = json_pack("{s:i,s:I,s:o}", "version", 1, "self_issued_ms",
                             (json_int_t)self_issued_ms, "peers", peers);
    char tmp[4096];
    snprintf(tmp, sizeof(tmp), "%s.tmp", path);
    FILE *f = fopen(tmp, "w");
    int rc = -1;
    if (f != NULL) {
        bool ok = json_dumpf(root, f, JSON_COMPACT) == 0 && fflush(f) == 0
                  && fsync(fileno(f)) == 0;
        ok = fclose(f) == 0 && ok;
        rc = ok && rename(tmp, path) == 0 ? 0 : -1;
    }
    json_decref(root);
    if (rc != 0) {
        int saved = errno;
        remove(tmp);
        errno = saved;
        return rc;
    }
    /* The rename itself is durable once the directory is synced; not every
     * file system lets a directory be opened for that. */
    char dir[4096];
    snprintf(dir, sizeof(dir), "%s", path);
    int dfd = open(dirname(dir), O_RDONLY | O_DIRECTORY);
    if (dfd >= 0) {
        (void)fsync(dfd);
        close(dfd);
    }
    return 0;
}

const char *lora_addrmap_skipped(const lora_addrmap_t *m)
{
    return m->skipped;
}

/* Note @p key as skipped in m->skipped, while it fits. */
static void _skip(lora_addrmap_t *m, const char *key)
{
    size_t used = strlen(m->skipped);
    snprintf(m->skipped + used, sizeof(m->skipped) - used, "%s%s", used > 0 ? ", " : "", key);
}

int lora_addrmap_load(lora_addrmap_t *m, const char *path, uint64_t *self_issued_ms)
{
    memset(m, 0, sizeof(*m));
    *self_issued_ms = 0;
    FILE *f = fopen(path, "r");
    if (f == NULL)
        return errno == ENOENT ? 0 : -1;
    fclose(f);
    json_error_t err;
    json_t *root = json_load_file(path, 0, &err);
    json_t *peers = root ? json_object_get(root, "peers") : NULL;
    json_t *self = root ? json_object_get(root, "self_issued_ms") : NULL;
    if (!json_is_object(root) || !json_is_object(peers) || !json_is_integer(self)
        || json_integer_value(self) < 0
        || json_integer_value(json_object_get(root, "version")) != 1) {
        json_decref(root);
        return -1;
    }
    const char *key;
    json_t *v;
    int skipped = 0;
    json_object_foreach(peers, key, v) {
        json_t *a = json_object_get(v, "addr"), *t = json_object_get(v, "issued_ms");
        if (!json_is_integer(a) || !json_is_integer(t)
            || json_integer_value(a) < 1 || json_integer_value(a) > 0xFFFFFFFELL
            || json_integer_value(t) < 0
            || lora_addrmap_announce(m, key, (uint32_t)json_integer_value(a),
                                     (uint64_t)json_integer_value(t), NULL, NULL) != LORA_ANN_ACCEPTED) {
            _skip(m, key);
            skipped++;
        }
    }
    *self_issued_ms = (uint64_t)json_integer_value(self);
    json_decref(root);
    return skipped;
}
