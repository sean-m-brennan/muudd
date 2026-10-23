/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <time.h>

#include "mr_common.h"

static uint64_t clock_ms(clockid_t id)
{
    struct timespec ts;
    clock_gettime(id, &ts);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}

uint64_t mr_now_ms(void) { return clock_ms(CLOCK_MONOTONIC); }
uint64_t mr_wall_ms(void) { return clock_ms(CLOCK_REALTIME); }
