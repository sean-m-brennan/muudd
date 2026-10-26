/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef LORA_ADDRMAP_H
#define LORA_ADDRMAP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* AT peer UUID <-> 32-bit LoRa address, learned from announcements on AT's
 * group channel. Conflicts fail closed; the map persists across restarts. */

#define LORA_ADDRMAP_MAX 256   /* most peers the map holds */
#define LORA_UUID_LEN 36       /* canonical lower-case text form, no NUL */

typedef enum {
    LORA_ANN_ACCEPTED = 0,   /* entry added or updated */
    LORA_ANN_STALE,          /* issued_ms not newer than the stored entry */
    LORA_ANN_BAD,            /* unparseable UUID, or address 0 / 0xFFFFFFFF */
    LORA_ANN_FULL            /* new UUID, map at LORA_ADDRMAP_MAX */
} lora_ann_t;

typedef struct {
    char uuid[LORA_UUID_LEN + 1];
    uint32_t addr;
    uint64_t issued_ms;
    bool conflicted;   /* another entry holds this address */
} lora_addr_entry_t;

/* Reports whether @p uuid is still on the group roster. */
typedef bool (*lora_is_member_fn)(const char *uuid, void *arg);

typedef struct lora_addrmap lora_addrmap_t;

lora_addrmap_t *lora_addrmap_new(void);
void lora_addrmap_free(lora_addrmap_t *m);

/* Apply an announcement. UUIDs are lower-cased. An existing entry with
 * issued_ms <= stored is STALE. After an accept, every other entry holding the
 * same address whose UUID @p is_member reports false is removed; a NULL
 * @p is_member counts everyone as a member. */
lora_ann_t lora_addrmap_announce(lora_addrmap_t *m, const char *uuid, uint32_t addr, uint64_t issued_ms,
                                 lora_is_member_fn is_member, void *arg);

/* False for an absent or conflicted entry. */
bool lora_addrmap_addr_of(const lora_addrmap_t *m, const char *uuid, uint32_t *addr);

/* False if no entry holds @p addr or more than one does. @p uuid_out needs
 * LORA_UUID_LEN + 1 bytes. */
bool lora_addrmap_uuid_of(const lora_addrmap_t *m, uint32_t addr, char *uuid_out);

/* True when more than one entry holds @p addr. */
bool lora_addrmap_addr_conflicted(const lora_addrmap_t *m, uint32_t addr);

/* True if an entry was removed. */
bool lora_addrmap_remove(lora_addrmap_t *m, const char *uuid);

/* Copy up to @p max entries into @p out; returns the entry count. With
 * @p out NULL, only counts. */
size_t lora_addrmap_snapshot(const lora_addrmap_t *m, lora_addr_entry_t *out, size_t max);

/* Write {"version":1,"self_issued_ms":N,"peers":{uuid:{"addr":N,"issued_ms":N}}}
 * to <path>.tmp, sync it, rename it into place and sync the directory.
 * 0 on success, -1 on failure. */
int lora_addrmap_save(const lora_addrmap_t *m, const char *path, uint64_t self_issued_ms);

/* Replace the map from @p path. An absent file gives 0 with an empty map and
 * *self_issued_ms 0; an unreadable or malformed one gives -1 likewise. A bad
 * entry in an otherwise good file is skipped: the return is then the number
 * skipped, and lora_addrmap_skipped names them. */
int lora_addrmap_load(lora_addrmap_t *m, const char *path, uint64_t *self_issued_ms);

/* The keys the last lora_addrmap_load skipped, comma-separated (truncated
 * past a few hundred bytes); "" when none. */
const char *lora_addrmap_skipped(const lora_addrmap_t *m);

#endif
