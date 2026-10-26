/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>
#include <string.h>

#include "lora/lora_reasm.h"

typedef struct {
    bool used;
    uint32_t src;
    uint16_t msg_id;
    uint8_t type, count;
    uint8_t tag[LORA_TAG_LEN];
    uint16_t have;
    bool nacked;
    double first, last;
    uint64_t seq;                      /* creation order */
    uint8_t *frag[LORA_MAX_FRAGS];
    size_t frag_len[LORA_MAX_FRAGS];
} entry_t;

typedef struct { uint32_t src; uint16_t msg_id; double t; } seen_t;

struct lora_reasm {
    double nack_gap_s, reassembly_s;
    entry_t e[LORA_REASM_MAX];
    seen_t seen[LORA_DEDUPE_MAX];
    size_t n_seen;
    uint64_t next_seq;
    lora_reasm_stats_t stats;
};

lora_reasm_t *lora_reasm_new(double nack_gap_s, double reassembly_s)
{
    lora_reasm_t *r = calloc(1, sizeof(*r));
    if (r != NULL) {
        r->nack_gap_s = nack_gap_s;
        r->reassembly_s = reassembly_s;
    }
    return r;
}

static void _clear(entry_t *e)
{
    for (int i = 0; i < LORA_MAX_FRAGS; i++)
        free(e->frag[i]);
    memset(e, 0, sizeof(*e));
}

void lora_reasm_free(lora_reasm_t *r)
{
    if (r == NULL)
        return;
    for (int i = 0; i < LORA_REASM_MAX; i++)
        _clear(&r->e[i]);
    free(r);
}

static void _forget_old(lora_reasm_t *r, double now)
{
    size_t k = 0;
    for (size_t i = 0; i < r->n_seen; i++)
        if (now - r->seen[i].t < LORA_DEDUPE_S)
            r->seen[k++] = r->seen[i];
    r->n_seen = k;
}

static void _remember(lora_reasm_t *r, uint32_t src, uint16_t msg_id, double now)
{
    if (r->n_seen == LORA_DEDUPE_MAX) {
        memmove(&r->seen[0], &r->seen[1], (LORA_DEDUPE_MAX - 1) * sizeof(seen_t));
        r->n_seen--;
    }
    r->seen[r->n_seen++] = (seen_t){src, msg_id, now};
}

static bool _seen(const lora_reasm_t *r, uint32_t src, uint16_t msg_id)
{
    for (size_t i = 0; i < r->n_seen; i++)
        if (r->seen[i].src == src && r->seen[i].msg_id == msg_id)
            return true;
    return false;
}

static void _drop(lora_reasm_t *r, entry_t *e, double now)
{
    _remember(r, e->src, e->msg_id, now);
    _clear(e);
}

/* The oldest entry by first-seen time, from @p src only when @p by_src. */
static entry_t *_oldest(lora_reasm_t *r, bool by_src, uint32_t src)
{
    entry_t *o = NULL;
    for (int i = 0; i < LORA_REASM_MAX; i++) {
        entry_t *e = &r->e[i];
        if (e->used && (!by_src || e->src == src)
            && (o == NULL || e->first < o->first || (!(o->first < e->first) && e->seq < o->seq)))
            o = e;
    }
    return o;
}

static entry_t *_create(lora_reasm_t *r, uint32_t src, const lora_frame_t *f, double now)
{
    int of_src = 0, total = 0;
    for (int i = 0; i < LORA_REASM_MAX; i++)
        if (r->e[i].used) {
            total++;
            of_src += r->e[i].src == src;
        }
    if (of_src >= LORA_REASM_PER_SRC) {
        _drop(r, _oldest(r, true, src), now);
        r->stats.evicted++;
        total--;
    }
    if (total >= LORA_REASM_MAX) {
        _drop(r, _oldest(r, false, 0), now);
        r->stats.evicted++;
    }
    for (int i = 0; i < LORA_REASM_MAX; i++)
        if (!r->e[i].used) {
            entry_t *e = &r->e[i];
            e->used = true;
            e->src = src;
            e->msg_id = f->msg_id;
            e->type = f->type;
            e->count = f->count;
            memcpy(e->tag, f->tag, LORA_TAG_LEN);
            e->first = e->last = now;
            e->seq = r->next_seq++;
            return e;
        }
    return NULL;
}

lora_rx_status_t lora_reasm_add(lora_reasm_t *r, uint32_t src, const lora_frame_t *f,
                                double now, uint8_t **msg, size_t *len)
{
    *msg = NULL;
    *len = 0;
    _forget_old(r, now);
    if (_seen(r, src, f->msg_id)) {
        r->stats.duplicate++;
        return LORA_RX_DUPLICATE;
    }
    entry_t *e = NULL;
    for (int i = 0; i < LORA_REASM_MAX && e == NULL; i++)
        if (r->e[i].used && r->e[i].src == src && r->e[i].msg_id == f->msg_id)
            e = &r->e[i];
    if (e == NULL) {
        e = _create(r, src, f, now);
        if (e == NULL)
            return LORA_RX_BAD;
    } else if (e->type != f->type || e->count != f->count
               || (f->type == LORA_T_GROUP && memcmp(e->tag, f->tag, LORA_TAG_LEN) != 0)) {
        _drop(r, e, now);
        r->stats.bad++;
        return LORA_RX_BAD;
    }
    uint16_t bit = (uint16_t)(1u << f->idx);
    if (e->have & bit)
        return LORA_RX_PARTIAL;
    e->frag[f->idx] = malloc(f->payload_len);
    if (e->frag[f->idx] == NULL)
        return LORA_RX_PARTIAL;
    memcpy(e->frag[f->idx], f->payload, f->payload_len);
    e->frag_len[f->idx] = f->payload_len;
    e->have |= bit;
    e->last = now;
    uint16_t all = (uint16_t)((1u << e->count) - 1);
    if (e->have != all)
        return LORA_RX_PARTIAL;
    size_t total = 0;
    for (int i = 0; i < e->count; i++)
        total += e->frag_len[i];
    uint8_t *m = malloc(total);
    if (m == NULL) {
        _drop(r, e, now);
        r->stats.bad++;
        return LORA_RX_BAD;
    }
    size_t off = 0;
    for (int i = 0; i < e->count; i++) {
        memcpy(m + off, e->frag[i], e->frag_len[i]);
        off += e->frag_len[i];
    }
    _drop(r, e, now);
    *msg = m;
    *len = total;
    return LORA_RX_COMPLETE;
}

size_t lora_reasm_tick(lora_reasm_t *r, double now, lora_nack_t *out, size_t max)
{
    _forget_old(r, now);
    size_t n = 0;
    /* Creation order, so both runtimes emit NACKs in the same sequence. */
    int order[LORA_REASM_MAX], k = 0;
    for (int i = 0; i < LORA_REASM_MAX; i++) {
        if (!r->e[i].used)
            continue;
        int j = k++;
        while (j > 0 && r->e[order[j - 1]].seq > r->e[i].seq) {
            order[j] = order[j - 1];
            j--;
        }
        order[j] = i;
    }
    for (int q = 0; q < k; q++) {
        entry_t *e = &r->e[order[q]];
        if (now - e->first >= r->reassembly_s) {
            _drop(r, e, now);
            r->stats.timeouts++;
        } else if (e->type == LORA_T_PEER && !e->nacked
                   && now - e->last >= r->nack_gap_s && n < max) {
            uint16_t all = (uint16_t)((1u << e->count) - 1);
            out[n++] = (lora_nack_t){e->src, e->msg_id, (uint16_t)(all & ~e->have)};
            e->nacked = true;
        }
    }
    return n;
}

lora_reasm_stats_t lora_reasm_stats(const lora_reasm_t *r)
{
    return r->stats;
}

typedef struct {
    bool used, served;
    uint32_t dest;
    uint16_t msg_id;
    double put;
    uint64_t seq;
    lora_frags_t frags;
} held_t;

struct lora_hold {
    double hold_s;
    held_t h[LORA_HOLD_MAX];
    uint64_t next_seq, served;
};

lora_hold_t *lora_hold_new(double hold_s)
{
    lora_hold_t *h = calloc(1, sizeof(*h));
    if (h != NULL)
        h->hold_s = hold_s;
    return h;
}

void lora_hold_free(lora_hold_t *h)
{
    if (h == NULL)
        return;
    for (int i = 0; i < LORA_HOLD_MAX; i++)
        lora_frags_free(&h->h[i].frags);
    free(h);
}

void lora_hold_put(lora_hold_t *h, uint32_t dest, uint16_t msg_id, lora_frags_t *frags, double now)
{
    held_t *slot = NULL;
    for (int i = 0; i < LORA_HOLD_MAX && slot == NULL; i++)
        if (!h->h[i].used)
            slot = &h->h[i];
    if (slot == NULL) {
        slot = &h->h[0];
        for (int i = 1; i < LORA_HOLD_MAX; i++)
            if (h->h[i].seq < slot->seq)
                slot = &h->h[i];
        lora_frags_free(&slot->frags);
    }
    *slot = (held_t){.used = true, .dest = dest, .msg_id = msg_id, .put = now,
                     .seq = h->next_seq++, .frags = *frags};
    frags->n = 0;   /* ownership moved */
}

size_t lora_hold_serve(lora_hold_t *h, uint32_t dest, uint16_t msg_id, uint16_t missing,
                       double now, lora_frags_t *out)
{
    out->n = 0;
    /* The newest held message for the key answers. */
    held_t *s = NULL;
    for (int i = 0; i < LORA_HOLD_MAX; i++) {
        held_t *c = &h->h[i];
        if (c->used && c->dest == dest && c->msg_id == msg_id && (s == NULL || c->seq > s->seq))
            s = c;
    }
    if (s != NULL) {
        if (s->served || now - s->put >= h->hold_s)
            return 0;
        s->served = true;
        h->served++;
        for (size_t k = 0; k < s->frags.n; k++)
            if (missing & (1u << k)) {
                uint8_t *b = malloc(s->frags.len[k]);
                if (b == NULL)
                    break;
                memcpy(b, s->frags.buf[k], s->frags.len[k]);
                out->buf[out->n] = b;
                out->len[out->n++] = s->frags.len[k];
            }
        return out->n;
    }
    return 0;
}

uint64_t lora_hold_served(const lora_hold_t *h)
{
    return h->served;
}
