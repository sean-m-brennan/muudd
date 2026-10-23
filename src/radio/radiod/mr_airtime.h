/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Time on air per frame, in microseconds, for the airtime ledger. */
#ifndef MR_AIRTIME_H
#define MR_AIRTIME_H

#include "mr_common.h"
#include "mr_proto.h"

uint64_t mr_airtime_lora_us(const Muudd__Radio__LoraParams *p, size_t payload_len);
uint64_t mr_airtime_bitrate_us(uint32_t bitrate_bps, size_t payload_len);
bool mr_lora_params_valid(const Muudd__Radio__LoraParams *p);
uint64_t mr_airtime_caps_us(const MrCaps *caps, size_t payload_len);

#endif
