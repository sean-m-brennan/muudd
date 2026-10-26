/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef LORA_ENGINE_H
#define LORA_ENGINE_H

/* The at-lora engine: decides what also goes by LoRa, frames and sends it,
 * reassembles what arrives, answers and sends NACKs, and keeps the address
 * map. It knows nothing of AT's processes: the glue hands it an environment.
 * Single-threaded by contract: every call comes from the network process's
 * main loop. Twin of the Python engine. */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <jansson.h>

#include "lora/lora_addrmap.h"
#include "lora/lora_config.h"
#include "lora/lora_link.h"

/* What the engine asks of the network process. */
typedef struct {
    void *arg;
    /* This node's uuid, lower case. */
    void (*my_uuid)(void *arg, char out[37]);
    /* The roster (lower-case uuids, may include us), up to @p max; the count. */
    size_t (*members)(void *arg, char (*out)[37], size_t max);
    /* Seconds since @p uuid was last heard through the IP transport. */
    double (*quiet_for)(void *arg, const char *uuid);
    /* True iff the group speaks the proto wire format. */
    bool (*group_proto)(void *arg);
    /* The group's 4-byte tag (the first bytes of its uuid); false with no group. */
    bool (*group_tag)(void *arg, uint8_t tag[4]);
    /* Wall clock, ms since the epoch. */
    uint64_t (*wall_ms)(void *arg);
    /* Monotonic clock, seconds. */
    double (*now)(void *arg);
    void (*log)(void *arg, int level, const char *msg);   /* 0 info, 1 warn, 2 error */
} lora_env_t;

#define LORA_DELIVER_PEER  0   /* the same values as AT's NET_CHAN_PEER/BROADCAST/GROUP */
#define LORA_DELIVER_BCAST 1
#define LORA_DELIVER_GROUP 2

/* One reassembled message: its channel, the sender's uuid, and the message
 * (malloc'd; the caller of lora_engine_poll frees it). */
typedef struct { int channel; char uuid[37]; uint8_t *msg; size_t len; } lora_delivery_t;

#define LORA_ANN_REFUSED (-1)   /* the sender is not on the roster */
#define LORA_ANN_SELF    (-2)   /* the sender is this node */

typedef struct lora_engine lora_engine_t;

/* An engine over @p link and @p env, its address map persisted at
 * @p addrmap_path (an unreadable file logs a WARN and starts empty). NULL on
 * allocation failure. */
lora_engine_t *lora_engine_new(const lora_cfg_t *cfg, lora_link_ops_t link, lora_env_t env,
                               const char *addrmap_path);
/* Frees the engine; the address map file stays. NULL-safe. */
void lora_engine_free(lora_engine_t *e);

/* A unicast the transport is about to send, sealed for @p peer_uuid: it also
 * goes by LoRa (acknowledged, and held for NACKs) when the link is up, the
 * peer has an address, has been quiet on IP for ip_quiet_s, the verb is
 * allowed, the group is proto-format and the frame fits. @return true if it
 * was handed to the link. A refused hand-off counts lora.tx.abandoned. */
bool lora_engine_on_unicast(lora_engine_t *e, const char *peer_uuid, const char *process,
                            const char *function, const uint8_t *frame, size_t len);

/* A GROUP (@p broadcast false; needs the group tag and the proto format) or
 * BROADCAST frame the transport just sent: it also goes to LoRa's broadcast
 * address, unacknowledged, when the link is up, some member with an address
 * is quiet on IP, the verb is allowed and the frame fits. @return true if it
 * was handed to the link. */
bool lora_engine_on_fanout(lora_engine_t *e, bool broadcast, const char *process,
                           const char *function, const uint8_t *frame, size_t len);

/* A member's announcement {lora_addr, issued_ms}. @p verified: its envelope's
 * signature checked out against the key held for the sender; one that did not
 * is refused and counted (lora.announce.unverified), since anyone holding the
 * group key could forge it. @return a lora_ann_t (LORA_ANN_BAD for a
 * malformed @p obj), LORA_ANN_REFUSED (unverified, or not on the roster) or
 * LORA_ANN_SELF. Accepted: persisted, counted, and a WARN when the address is
 * now claimed twice. Stale: counted. Full: the entry off the roster with the
 * oldest issued_ms (lowest uuid on a tie) makes room and it is tried once
 * more; with every entry a member, counted (lora.announce.full) with a WARN at
 * most once a minute. */
int lora_engine_on_announce(lora_engine_t *e, const char *sender_uuid, json_t *obj,
                            bool verified);

/* Reputation cut @p uuid off (@p excluded): its entry goes, persisted. */
void lora_engine_on_exclusion(lora_engine_t *e, const char *uuid, bool excluded);

/* Drain the link: count refusals by reason, answer NACKs from the hold,
 * reassemble data frames, send NACKs for stalled messages. Takes at most
 * @p max received frames (each completes at most one message), so every
 * completed message is written to @p out; frames not taken stay queued in
 * the link for the next call. @return the number written; the caller frees
 * each msg. */
size_t lora_engine_poll(lora_engine_t *e, lora_delivery_t *out, size_t max);

/* Whether our announcement is due, and what it says: our node address and
 * issued_ms = max(wall_ms, last issued + 1). Changes no announcement state;
 * report a successful send with lora_engine_announce_sent. False while there
 * is no group tag, the link is not up or the node address is 0. Due on a new
 * address, after a link down-to-up since the last one, every announce_s, and
 * MEMBER_ANNOUNCE_S after the last when the roster gained a member. */
bool lora_engine_announce_due(lora_engine_t *e, uint32_t *addr, uint64_t *issued_ms);

/* The announcement from lora_engine_announce_due went out: remember its
 * address, time and roster, persist issued_ms, count lora.announce.sent. */
void lora_engine_announce_sent(lora_engine_t *e, uint32_t addr, uint64_t issued_ms);

/* {counters, link, addrmap}; the caller owns the reference. */
json_t *lora_engine_stats(lora_engine_t *e);

/* Every SUMMARY_S (300 s): writes the one-line summary to @p line and
 * returns true. */
bool lora_engine_summary_due(lora_engine_t *e, char *line, size_t cap);

/* The counter @p name (the Python twin's names), 0 if never counted. */
uint64_t lora_engine_counter(const lora_engine_t *e, const char *name);

#endif /* LORA_ENGINE_H */
