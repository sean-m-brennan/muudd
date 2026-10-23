/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <math.h>

#include "mr_airtime.h"

/* Semtech time-on-air formula. The symbol time is 2^SF / BW; the preamble
 * takes (n + 4.25) symbols; the payload takes 8 plus whole coding blocks of
 * (CR + 4) symbols each. */
uint64_t mr_airtime_lora_us(const Muudd__Radio__LoraParams *p, size_t payload_len)
{
    int sf = (int)p->spreading_factor;
    double tsym = (double)(1u << sf) / (double)p->bandwidth_hz * 1e6;
    int de = p->ldro == 1 ? 1 : p->ldro == 2 ? 0 : (tsym > 16000.0);
    int ih = p->implicit_header ? 1 : 0;
    int crc = p->crc_off ? 0 : 1;
    double tpre = ((double)p->preamble_symbols + 4.25) * tsym;
    double num = 8.0 * (double)payload_len - 4.0 * sf + 28.0 + 16.0 * crc - 20.0 * ih;
    double den = 4.0 * (double)(sf - 2 * de);
    double body = ceil(num / den) * (double)(p->coding_rate + 4);
    double nsym = 8.0 + (body > 0 ? body : 0);
    return (uint64_t)llround(tpre + nsym * tsym);
}

uint64_t mr_airtime_bitrate_us(uint32_t bps, size_t n)
{
    if (bps == 0)
        return 0;
    return ((uint64_t)n * 8u * 1000000u + bps - 1) / bps;
}

bool mr_lora_params_valid(const Muudd__Radio__LoraParams *p)
{
    return p && p->spreading_factor >= 6 && p->spreading_factor <= 12 && p->bandwidth_hz > 0 &&
           p->coding_rate >= 1 && p->coding_rate <= 4;
}

uint64_t mr_airtime_caps_us(const MrCaps *caps, size_t payload_len)
{
    size_t n = payload_len + caps->overhead_bytes;
    if (mr_lora_params_valid(caps->lora))
        return mr_airtime_lora_us(caps->lora, n);
    return mr_airtime_bitrate_us(caps->bitrate_bps, n);
}
