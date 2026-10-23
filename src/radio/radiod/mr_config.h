/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* radiod.json: loading, defaults and validation. */
#ifndef MR_CONFIG_H
#define MR_CONFIG_H

#include "mr_common.h"
#include "mr_proto.h"

#define MR_MAX_PEERS 64
#define MR_MAX_DRIVER_ARGS 16
#define MR_MAX_SETTINGS 16

typedef struct {
    bool configured;
    char name[32];
    double share_pct;
    int policy;
    bool cross_region;
    bool ip_fallback;
} mr_port_cfg_t;

typedef struct {
    char addr[128];
    uint8_t pubkey[32];
    char region[64];
    int priority;
    bool is_self;
} mr_peer_cfg_t;

typedef struct {
    bool node_addr_auto;
    uint32_t node_addr;
    bool has_driver;
    char driver_exec[512];
    char driver_args[MR_MAX_DRIVER_ARGS][256];
    size_t n_driver_args;
    char setting_keys[MR_MAX_SETTINGS][64];
    char setting_vals[MR_MAX_SETTINGS][256];
    size_t n_settings;
    bool apply_radio_config;
    double duty_cycle_pct;
    uint32_t min_bandwidth_khz;
    char socket_path[108];
    mr_port_cfg_t ports[MR_NUM_PORTS];
    bool has_bridge;
    char listen[128];
    bool gateway;
    char region[64];
    int priority;
    char key_file[512];
    char relay_key_file[512];
    uint32_t relay_window_s;
    uint32_t keepalive_ms;
    uint32_t dead_after_ms;
    mr_peer_cfg_t peers[MR_MAX_PEERS];
    size_t n_peers;
} mr_config_t;

int mr_config_parse(const char *json_text, mr_config_t *cfg, char *err, size_t errlen);
int mr_config_load(const char *path, mr_config_t *cfg, char *err, size_t errlen);
size_t mr_config_mark_self(mr_config_t *cfg, const uint8_t own_pubkey[32]);

#endif
