/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include "lora/lora_policy.h"

bool lora_policy_unicast(const lora_unicast_in_t *in)
{
    return in->link_up && in->have_addr && in->quiet_s >= in->ip_quiet_s && in->allowed
           && in->proto && in->frame_len > 0 && in->frame_len <= in->max_len;
}

bool lora_policy_fanout(const lora_fanout_in_t *in)
{
    return in->link_up && in->n_quiet > 0 && in->allowed
           && (in->channel == LORA_CH_BCAST || in->proto)
           && in->frame_len > 0 && in->frame_len <= in->max_len;
}
