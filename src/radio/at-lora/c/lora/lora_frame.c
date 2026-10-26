/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <stdlib.h>
#include <string.h>

#include "lora/lora_frame.h"

size_t lora_header_len(uint8_t type)
{
    switch (type) {
    case LORA_T_PEER:
    case LORA_T_BCAST: return 4;
    case LORA_T_GROUP: return 8;
    case LORA_T_NACK:  return 5;
    default:           return 0;
    }
}

int lora_frame_decode(const uint8_t *buf, size_t len, lora_frame_t *out)
{
    if (buf == NULL || out == NULL || len == 0 || (buf[0] >> 4) != LORA_VERSION)
        return -1;
    uint8_t type = buf[0] & 0x0F;
    size_t hdr = lora_header_len(type);
    if (hdr == 0 || len < hdr)
        return -1;
    memset(out, 0, sizeof(*out));
    out->type = type;
    out->msg_id = (uint16_t)((buf[1] << 8) | buf[2]);
    if (type == LORA_T_NACK) {
        if (len != hdr)
            return -1;
        out->missing = (uint16_t)((buf[3] << 8) | buf[4]);
        return out->missing == 0 ? -1 : 0;
    }
    if (len == hdr)
        return -1;
    out->idx = buf[3] >> 4;
    out->count = (uint8_t)((buf[3] & 0x0F) + 1);
    if (out->idx >= out->count)
        return -1;
    if (type == LORA_T_GROUP)
        memcpy(out->tag, buf + 4, LORA_TAG_LEN);
    out->payload = buf + hdr;
    out->payload_len = len - hdr;
    return 0;
}

int lora_encode_nack(uint16_t msg_id, uint16_t missing, uint8_t *out, size_t cap)
{
    if (out == NULL || cap < 5 || missing == 0)
        return -1;
    out[0] = (LORA_VERSION << 4) | LORA_T_NACK;
    out[1] = (uint8_t)(msg_id >> 8);
    out[2] = (uint8_t)msg_id;
    out[3] = (uint8_t)(missing >> 8);
    out[4] = (uint8_t)missing;
    return 5;
}

size_t lora_payload_per_frag(uint8_t type, uint32_t mtu)
{
    if (type == LORA_T_NACK || mtu < LORA_MIN_MTU)
        return 0;
    size_t hdr = lora_header_len(type);
    return hdr == 0 ? 0 : (size_t)mtu - hdr;
}

static unsigned _clamp_frags(unsigned max_frags)
{
    return max_frags < 1 ? 1 : (max_frags > LORA_MAX_FRAGS ? LORA_MAX_FRAGS : max_frags);
}

size_t lora_max_message(uint8_t type, uint32_t mtu, unsigned max_frags)
{
    return lora_payload_per_frag(type, mtu) * _clamp_frags(max_frags);
}

void lora_frags_free(lora_frags_t *f)
{
    if (f == NULL)
        return;
    for (size_t i = 0; i < f->n; i++)
        free(f->buf[i]);
    f->n = 0;
}

int lora_fragment(const uint8_t *msg, size_t len, uint8_t type, uint16_t msg_id,
                  const uint8_t *tag, uint32_t mtu, unsigned max_frags,
                  lora_frags_t *out)
{
    if (out == NULL)
        return -1;
    out->n = 0;
    size_t per = lora_payload_per_frag(type, mtu);
    if (msg == NULL || len == 0 || per == 0 || (type == LORA_T_GROUP && tag == NULL)
        || len > lora_max_message(type, mtu, max_frags))
        return -1;
    size_t hdr = lora_header_len(type);
    size_t count = (len + per - 1) / per;
    for (size_t i = 0; i < count; i++) {
        size_t off = i * per;
        size_t n = len - off < per ? len - off : per;
        uint8_t *b = malloc(hdr + n);
        if (b == NULL) {
            lora_frags_free(out);
            return -1;
        }
        b[0] = (LORA_VERSION << 4) | type;
        b[1] = (uint8_t)(msg_id >> 8);
        b[2] = (uint8_t)msg_id;
        b[3] = (uint8_t)((i << 4) | (count - 1));
        if (type == LORA_T_GROUP)
            memcpy(b + 4, tag, LORA_TAG_LEN);
        memcpy(b + hdr, msg + off, n);
        out->buf[i] = b;
        out->len[i] = hdr + n;
        out->n = i + 1;
    }
    return 0;
}
