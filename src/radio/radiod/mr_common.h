/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Error codes, protocol constants and clocks shared by every radiod unit. */
#ifndef MR_COMMON_H
#define MR_COMMON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MR_OK       0
#define MR_ERR     -1
#define MR_EFRAME  -2
#define MR_ENOMEM  -3
#define MR_EINVAL  -4
#define MR_EAUTH   -5
#define MR_EIO     -6
#define MR_EBUDGET -7
#define MR_EFATAL  -8
#define MR_EPERM   -9   /* a key file others may read or write */

#define MR_BROADCAST      0xFFFFFFFFu
#define MR_PORT_TEXT      1u
#define MR_PORT_RELAY     2u
#define MR_NUM_PORTS      256u
#define MR_RELAY_OVERHEAD 57u
#define MR_DEDUPE_TTL_MS  600000u
#define MR_FRAME_MAX      8192u

/* Milliseconds from CLOCK_MONOTONIC; use for every internal timer. */
uint64_t mr_now_ms(void);
/* Milliseconds since the Unix epoch; use only for relay timestamps and boot_id. */
uint64_t mr_wall_ms(void);

#endif
