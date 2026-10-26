/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The frames AutonomousTrust traffic rides on its radiod port.
 *
 * Byte 0 is the version (high nibble, 1) and type (low nibble). Then, big-
 * endian: DATA_PEER and DATA_BCAST carry msg_id(2) frag(1) payload; DATA_GROUP
 * carries msg_id(2) frag(1) group_tag(4) payload; NACK carries msg_id(2)
 * missing(2) and nothing else. frag is the fragment index (high nibble) and
 * the fragment count less one (low nibble), so a message has at most 16
 * fragments. missing is a bitmap of fragment indexes a receiver lacks. */
#ifndef LORA_FRAME_H
#define LORA_FRAME_H

#include <stddef.h>
#include <stdint.h>

#define LORA_VERSION   1
#define LORA_T_PEER    1
#define LORA_T_GROUP   2
#define LORA_T_BCAST   3
#define LORA_T_NACK    4
#define LORA_MAX_FRAGS 16
#define LORA_MIN_MTU   64   /* below this the extension stays inert */
#define LORA_TAG_LEN   4

typedef struct {
    uint8_t type;
    uint16_t msg_id;
    uint8_t idx;                 /* data types */
    uint8_t count;               /* data types: 1..16 */
    uint8_t tag[LORA_TAG_LEN];   /* DATA_GROUP */
    uint16_t missing;            /* NACK */
    const uint8_t *payload;      /* data types: points into the decoded buffer */
    size_t payload_len;
} lora_frame_t;

typedef struct {
    size_t n;
    uint8_t *buf[LORA_MAX_FRAGS];
    size_t len[LORA_MAX_FRAGS];
} lora_frags_t;

/** Header length of @p type; 0 for an unknown type. */
size_t lora_header_len(uint8_t type);
/** Parse one frame. 0, or -1 when malformed (unknown version or type, wrong
 *  length, an index not below the count, an empty data payload, a NACK
 *  naming no fragment). */
int lora_frame_decode(const uint8_t *buf, size_t len, lora_frame_t *out);
/** Encode a NACK into @p out. @return 5, or -1 (missing == 0, cap < 5). */
int lora_encode_nack(uint16_t msg_id, uint16_t missing, uint8_t *out, size_t cap);
/** Payload bytes per fragment of a data @p type at @p mtu; 0 when the MTU is
 *  below LORA_MIN_MTU or the type carries no payload. */
size_t lora_payload_per_frag(uint8_t type, uint32_t mtu);
/** Largest message @p max_frags fragments carry (max_frags clamped 1..16). */
size_t lora_max_message(uint8_t type, uint32_t mtu, unsigned max_frags);
/** Split @p msg into data frames. @p tag is required for DATA_GROUP and
 *  ignored otherwise. @return 0 with @p out filled (free with
 *  lora_frags_free), or -1: empty message, too big, MTU too small, a type
 *  that carries no payload, or a GROUP without a tag. */
int lora_fragment(const uint8_t *msg, size_t len, uint8_t type, uint16_t msg_id,
                  const uint8_t *tag, uint32_t mtu, unsigned max_frags,
                  lora_frags_t *out);
void lora_frags_free(lora_frags_t *f);

#endif
