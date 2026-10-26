/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef LORA_POLICY_H
#define LORA_POLICY_H

#include <stdbool.h>
#include <stddef.h>

/* Whether a frame also goes by LoRa. Pure functions of their inputs. */

typedef enum { LORA_CH_GROUP = 0, LORA_CH_BCAST = 1 } lora_fan_ch_t;

typedef struct {
    bool link_up;      /* the radio link is connected and claimed */
    bool have_addr;    /* the peer has a LoRa address in the map */
    double quiet_s;    /* seconds since the peer was last heard on IP */
    double ip_quiet_s; /* quiet time that counts as IP down */
    bool allowed;      /* the verb is on the allow list */
    bool proto;        /* the frame is in AT's compact wire format */
    size_t frame_len;
    size_t max_len;    /* largest message the fragments can carry */
} lora_unicast_in_t;

typedef struct {
    lora_fan_ch_t channel;
    bool link_up;
    size_t n_quiet;    /* members that are quiet on IP and have addresses */
    bool allowed;
    bool proto;
    size_t frame_len;
    size_t max_len;
} lora_fanout_in_t;

/* A unicast also goes by LoRa when the link is up, the peer has an address
 * and has been quiet for at least ip_quiet_s, the verb is allowed, the frame
 * is in the wire format, and 0 < frame_len <= max_len. */
bool lora_policy_unicast(const lora_unicast_in_t *in);

/* A group or broadcast frame also goes by LoRa when the link is up, at least
 * one member is quiet, the verb is allowed, the frame is in the wire format
 * (broadcast frames need no format), and 0 < frame_len <= max_len. */
bool lora_policy_fanout(const lora_fanout_in_t *in);

#endif
