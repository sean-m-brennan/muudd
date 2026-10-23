/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <jansson.h>
#include <sodium.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "mr_config.h"

#define MR_CONFIG_MAX_BYTES (1024 * 1024)

static int fail(char *err, size_t len, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(err, len, fmt, ap);
    va_end(ap);
    return MR_EINVAL;
}

static int only_keys(json_t *obj, const char *prefix, const char *const *allowed, char *err, size_t len)
{
    const char *k;
    json_t *v;
    json_object_foreach(obj, k, v) {
        bool ok = false;
        for (size_t i = 0; allowed[i]; i++)
            if (strcmp(k, allowed[i]) == 0)
                ok = true;
        if (!ok)
            return fail(err, len, "%s%s: unknown field", prefix, k);
    }
    return MR_OK;
}

static int copy_str(json_t *v, char *dst, size_t cap, const char *path, char *err, size_t len)
{
    if (!json_is_string(v))
        return fail(err, len, "%s: must be a string", path);
    const char *s = json_string_value(v);
    size_t n = strlen(s);
    if (n >= cap)
        return fail(err, len, "%s: longer than %zu characters", path, cap - 1);
    memcpy(dst, s, n + 1);
    return MR_OK;
}

static int opt_bool(json_t *obj, const char *key, bool def, bool *out, const char *prefix, char *err, size_t len)
{
    json_t *v = json_object_get(obj, key);
    if (!v) {
        *out = def;
        return MR_OK;
    }
    if (!json_is_boolean(v))
        return fail(err, len, "%s%s: must be true or false", prefix, key);
    *out = json_is_true(v);
    return MR_OK;
}

static int opt_int(json_t *obj, const char *key, long long def, long long lo, long long hi, long long *out,
                   const char *prefix, char *err, size_t len)
{
    json_t *v = json_object_get(obj, key);
    if (!v) {
        *out = def;
        return MR_OK;
    }
    if (!json_is_integer(v))
        return fail(err, len, "%s%s: must be an integer", prefix, key);
    long long x = json_integer_value(v);
    if (x < lo || x > hi)
        return fail(err, len, "%s%s: must be between %lld and %lld", prefix, key, lo, hi);
    *out = x;
    return MR_OK;
}

static int opt_num(json_t *obj, const char *key, double def, double lo, double hi, double *out, const char *prefix,
                   char *err, size_t len)
{
    json_t *v = json_object_get(obj, key);
    if (!v) {
        *out = def;
        return MR_OK;
    }
    if (!json_is_number(v))
        return fail(err, len, "%s%s: must be a number", prefix, key);
    double x = json_number_value(v);
    if (x < lo || x > hi)
        return fail(err, len, "%s%s: must be between %g and %g", prefix, key, lo, hi);
    *out = x;
    return MR_OK;
}

static int parse_driver(json_t *d, mr_config_t *c, char *err, size_t len)
{
    static const char *const keys[] = {"exec", "args", "settings", "apply_radio_config", NULL};
    int rc;
    if (!json_is_object(d))
        return fail(err, len, "driver: must be an object or null");
    if ((rc = only_keys(d, "driver.", keys, err, len)))
        return rc;
    json_t *exec = json_object_get(d, "exec");
    if (!exec)
        return fail(err, len, "driver.exec: required");
    if ((rc = copy_str(exec, c->driver_exec, sizeof c->driver_exec, "driver.exec", err, len)))
        return rc;
    json_t *args = json_object_get(d, "args");
    if (args) {
        if (!json_is_array(args))
            return fail(err, len, "driver.args: must be an array of strings");
        if (json_array_size(args) > MR_MAX_DRIVER_ARGS)
            return fail(err, len, "driver.args: at most %d entries", MR_MAX_DRIVER_ARGS);
        size_t i;
        json_t *a;
        json_array_foreach(args, i, a) {
            char path[48];
            snprintf(path, sizeof path, "driver.args[%zu]", i);
            if ((rc = copy_str(a, c->driver_args[i], sizeof c->driver_args[i], path, err, len)))
                return rc;
        }
        c->n_driver_args = json_array_size(args);
    }
    json_t *settings = json_object_get(d, "settings");
    if (settings) {
        if (!json_is_object(settings))
            return fail(err, len, "driver.settings: must be an object of strings");
        if (json_object_size(settings) > MR_MAX_SETTINGS)
            return fail(err, len, "driver.settings: at most %d entries", MR_MAX_SETTINGS);
        const char *k;
        json_t *v;
        size_t n = 0;
        json_object_foreach(settings, k, v) {
            char path[96];
            snprintf(path, sizeof path, "driver.settings.%s", k);
            if (strlen(k) >= sizeof c->setting_keys[0])
                return fail(err, len, "%s: key too long", path);
            memcpy(c->setting_keys[n], k, strlen(k) + 1);
            if ((rc = copy_str(v, c->setting_vals[n], sizeof c->setting_vals[n], path, err, len)))
                return rc;
            n++;
        }
        c->n_settings = n;
    }
    return opt_bool(d, "apply_radio_config", false, &c->apply_radio_config, "driver.", err, len);
}

static int parse_ports(json_t *ports, mr_config_t *c, char *err, size_t len)
{
    static const char *const keys[] = {"name", "share_pct", "policy", "cross_region", "ip_fallback", NULL};
    if (!json_is_object(ports))
        return fail(err, len, "ports: must be an object");
    double sum = 0;
    const char *k;
    json_t *v;
    int rc;
    json_object_foreach(ports, k, v) {
        char *end;
        long port = strtol(k, &end, 10);
        if (!*k || *end || port < 0 || port > 255)
            return fail(err, len, "ports.%s: key must be a port number from 0 to 255", k);
        if (port == 0)
            return fail(err, len, "ports.0: reserved");
        if (port == (long)MR_PORT_RELAY)
            return fail(err, len, "ports.2: reserved for relay frames");
        char prefix[16], path[32];
        snprintf(prefix, sizeof prefix, "ports.%ld.", port);
        if (!json_is_object(v))
            return fail(err, len, "ports.%ld: must be an object", port);
        if ((rc = only_keys(v, prefix, keys, err, len)))
            return rc;
        mr_port_cfg_t *pc = &c->ports[port];
        json_t *name = json_object_get(v, "name");
        if (!name)
            return fail(err, len, "%sname: required", prefix);
        snprintf(path, sizeof path, "%sname", prefix);
        if ((rc = copy_str(name, pc->name, sizeof pc->name, path, err, len)))
            return rc;
        if ((rc = opt_num(v, "share_pct", 0, 0, 100, &pc->share_pct, prefix, err, len)))
            return rc;
        json_t *pol = json_object_get(v, "policy");
        const char *ps = json_is_string(pol) ? json_string_value(pol) : "";
        if (strcmp(ps, "radiod") == 0)
            pc->policy = MUUDD__RADIO__POLICY__RADIOD_BRIDGED;
        else if (strcmp(ps, "consumer") == 0)
            pc->policy = MUUDD__RADIO__POLICY__CONSUMER_BRIDGED;
        else
            return fail(err, len, "%spolicy: must be \"radiod\" or \"consumer\"", prefix);
        if ((rc = opt_bool(v, "cross_region", false, &pc->cross_region, prefix, err, len)))
            return rc;
        if ((rc = opt_bool(v, "ip_fallback", false, &pc->ip_fallback, prefix, err, len)))
            return rc;
        if (pc->policy == MUUDD__RADIO__POLICY__CONSUMER_BRIDGED && (pc->cross_region || pc->ip_fallback))
            return fail(err, len, "%s%s: only valid with policy radiod", prefix,
                        pc->ip_fallback ? "ip_fallback" : "cross_region");
        pc->configured = true;
        sum += pc->share_pct;
    }
    if (sum > 100.0 + 1e-9)
        return fail(err, len, "ports: share_pct values sum to %.1f, more than 100", sum);
    return MR_OK;
}

/* host:port with a non-empty host (an IPv6 literal goes in brackets) and a
 * port from 1 to 65535. */
static int check_hostport(const char *v, const char *path, char *err, size_t len)
{
    const char *colon = strrchr(v, ':');
    if (colon && colon > v && colon[1]) {
        size_t hl = (size_t)(colon - v);
        bool bracket_ok = v[0] != '[' || (hl >= 3 && v[hl - 1] == ']');
        bool port_ok = true;
        long port = 0;
        for (const char *q = colon + 1; *q; q++) {
            if (*q < '0' || *q > '9' || port > 65535) {
                port_ok = false;
                break;
            }
            port = port * 10 + (*q - '0');
        }
        if (bracket_ok && port_ok && port >= 1 && port <= 65535)
            return MR_OK;
    }
    return fail(err, len, "%s: must be host:port", path);
}

static int parse_peer(json_t *p, size_t i, mr_peer_cfg_t *pc, char *err, size_t len)
{
    static const char *const keys[] = {"addr", "pubkey", "region", "priority", NULL};
    char prefix[32], path[64];
    int rc;
    long long x;
    snprintf(prefix, sizeof prefix, "bridge.peers[%zu].", i);
    if (!json_is_object(p))
        return fail(err, len, "bridge.peers[%zu]: must be an object", i);
    if ((rc = only_keys(p, prefix, keys, err, len)))
        return rc;
    json_t *addr = json_object_get(p, "addr");
    if (!addr)
        return fail(err, len, "%saddr: required", prefix);
    snprintf(path, sizeof path, "%saddr", prefix);
    if ((rc = copy_str(addr, pc->addr, sizeof pc->addr, path, err, len)))
        return rc;
    if ((rc = check_hostport(pc->addr, path, err, len)))
        return rc;
    json_t *pk = json_object_get(p, "pubkey");
    snprintf(path, sizeof path, "%spubkey", prefix);
    if (!json_is_string(pk))
        return fail(err, len, "%s: required, base64 of 32 bytes", path);
    const char *b64 = json_string_value(pk);
    size_t bin_len = 0;
    if (sodium_base642bin(pc->pubkey, sizeof pc->pubkey, b64, strlen(b64), NULL, &bin_len, NULL,
                          sodium_base64_VARIANT_ORIGINAL) != 0 || bin_len != 32)
        return fail(err, len, "%s: must be base64 of exactly 32 bytes", path);
    json_t *region = json_object_get(p, "region");
    snprintf(path, sizeof path, "%sregion", prefix);
    if (region && (rc = copy_str(region, pc->region, sizeof pc->region, path, err, len)))
        return rc;
    if ((rc = opt_int(p, "priority", 100, 0, 1000000, &x, prefix, err, len)))
        return rc;
    pc->priority = (int)x;
    return MR_OK;
}

static int parse_bridge(json_t *b, mr_config_t *c, char *err, size_t len)
{
    static const char *const keys[] = {"listen", "gateway", "region", "priority", "key_file", "relay_key_file",
                                       "relay_window_s", "keepalive_ms", "dead_after_ms", "peers", NULL};
    int rc;
    long long x;
    if (!json_is_object(b))
        return fail(err, len, "bridge: must be an object or null");
    if ((rc = only_keys(b, "bridge.", keys, err, len)))
        return rc;
    json_t *listen = json_object_get(b, "listen");
    if (!listen)
        return fail(err, len, "bridge.listen: required");
    if ((rc = copy_str(listen, c->listen, sizeof c->listen, "bridge.listen", err, len)))
        return rc;
    if ((rc = check_hostport(c->listen, "bridge.listen", err, len)))
        return rc;
    if ((rc = opt_bool(b, "gateway", false, &c->gateway, "bridge.", err, len)))
        return rc;
    json_t *region = json_object_get(b, "region");
    if (region && (rc = copy_str(region, c->region, sizeof c->region, "bridge.region", err, len)))
        return rc;
    if (c->gateway && !c->region[0])
        return fail(err, len, "bridge.region: required when bridge.gateway is true");
    if ((rc = opt_int(b, "priority", 100, 0, 1000000, &x, "bridge.", err, len)))
        return rc;
    c->priority = (int)x;
    json_t *kf = json_object_get(b, "key_file");
    if (kf && (rc = copy_str(kf, c->key_file, sizeof c->key_file, "bridge.key_file", err, len)))
        return rc;
    json_t *rk = json_object_get(b, "relay_key_file");
    if (rk && (rc = copy_str(rk, c->relay_key_file, sizeof c->relay_key_file, "bridge.relay_key_file", err, len)))
        return rc;
    if ((rc = opt_int(b, "relay_window_s", 600, 0, 604800, &x, "bridge.", err, len)))
        return rc;
    c->relay_window_s = (uint32_t)x;
    if ((rc = opt_int(b, "keepalive_ms", 5000, 100, 3600000, &x, "bridge.", err, len)))
        return rc;
    c->keepalive_ms = (uint32_t)x;
    if ((rc = opt_int(b, "dead_after_ms", 15000, 200, 36000000, &x, "bridge.", err, len)))
        return rc;
    c->dead_after_ms = (uint32_t)x;
    if (c->dead_after_ms <= c->keepalive_ms)
        return fail(err, len, "bridge.dead_after_ms: must exceed bridge.keepalive_ms");
    json_t *peers = json_object_get(b, "peers");
    if (peers) {
        if (!json_is_array(peers))
            return fail(err, len, "bridge.peers: must be an array");
        if (json_array_size(peers) > MR_MAX_PEERS)
            return fail(err, len, "bridge.peers: at most %d entries", MR_MAX_PEERS);
        size_t i;
        json_t *p;
        json_array_foreach(peers, i, p) {
            if ((rc = parse_peer(p, i, &c->peers[i], err, len)))
                return rc;
        }
        c->n_peers = json_array_size(peers);
    }
    return MR_OK;
}

static int parse_root(json_t *root, mr_config_t *c, char *err, size_t len)
{
    static const char *const keys[] = {"node_addr", "driver", "duty_cycle_pct", "min_bandwidth_khz",
                                       "socket", "ports", "bridge", NULL};
    int rc;
    long long x;
    if (!json_is_object(root))
        return fail(err, len, "top level: must be an object");
    if ((rc = only_keys(root, "", keys, err, len)))
        return rc;
    json_t *na = json_object_get(root, "node_addr");
    if (na && !(json_is_string(na) && strcmp(json_string_value(na), "auto") == 0)) {
        if (!json_is_integer(na) || json_integer_value(na) < 1 || json_integer_value(na) >= 0xFFFFFFFFLL)
            return fail(err, len, "node_addr: must be \"auto\" or an integer from 1 to 4294967294");
        c->node_addr_auto = false;
        c->node_addr = (uint32_t)json_integer_value(na);
    }
    json_t *drv = json_object_get(root, "driver");
    if (drv && !json_is_null(drv)) {
        if ((rc = parse_driver(drv, c, err, len)))
            return rc;
        c->has_driver = true;
    }
    if ((rc = opt_num(root, "duty_cycle_pct", 10, 0, 100, &c->duty_cycle_pct, "", err, len)))
        return rc;
    if ((rc = opt_int(root, "min_bandwidth_khz", 500, 0, 100000, &x, "", err, len)))
        return rc;
    c->min_bandwidth_khz = (uint32_t)x;
    json_t *sock = json_object_get(root, "socket");
    if (sock && (rc = copy_str(sock, c->socket_path, sizeof c->socket_path, "socket", err, len)))
        return rc;
    json_t *ports = json_object_get(root, "ports");
    if (ports && (rc = parse_ports(ports, c, err, len)))
        return rc;
    json_t *br = json_object_get(root, "bridge");
    if (br && !json_is_null(br)) {
        if ((rc = parse_bridge(br, c, err, len)))
            return rc;
        c->has_bridge = true;
    }
    if (!c->has_driver) {
        if (c->node_addr_auto)
            return fail(err, len, "node_addr: required when driver is null");
        if (!c->has_bridge)
            return fail(err, len, "bridge: required when driver is null");
        if (c->gateway)
            return fail(err, len, "bridge.gateway: a node without a driver cannot be a gateway");
    }
    for (unsigned p = 0; p < MR_NUM_PORTS; p++)
        if (c->ports[p].ip_fallback && !c->relay_key_file[0])
            return fail(err, len, "bridge.relay_key_file: required when a port sets ip_fallback");
    return MR_OK;
}

int mr_config_parse(const char *text, mr_config_t *c, char *err, size_t len)
{
    memset(c, 0, sizeof *c);
    c->node_addr_auto = true;
    c->duty_cycle_pct = 10;
    c->min_bandwidth_khz = 500;
    snprintf(c->socket_path, sizeof c->socket_path, "/run/muudd/radio.sock");
    snprintf(c->key_file, sizeof c->key_file, "/etc/muudd/radio/radiod.key");
    c->priority = 100;
    c->relay_window_s = 600;
    c->keepalive_ms = 5000;
    c->dead_after_ms = 15000;
    json_error_t je;
    json_t *root = json_loads(text, 0, &je);
    if (!root)
        return fail(err, len, "line %d: %s", je.line, je.text);
    int rc = parse_root(root, c, err, len);
    json_decref(root);
    return rc;
}

int mr_config_load(const char *path, mr_config_t *c, char *err, size_t len)
{
    FILE *f = fopen(path, "rb");
    if (!f) {
        snprintf(err, len, "cannot read %s: %s", path, strerror(errno));
        return MR_EIO;
    }
    struct stat st;
    if (fstat(fileno(f), &st) != 0 || !S_ISREG(st.st_mode)) {
        fclose(f);
        snprintf(err, len, "cannot read %s: not a regular file", path);
        return MR_EIO;
    }
    if (st.st_size > MR_CONFIG_MAX_BYTES) {
        fclose(f);
        snprintf(err, len, "cannot read %s: larger than %d bytes", path, MR_CONFIG_MAX_BYTES);
        return MR_EIO;
    }
    size_t n = (size_t)st.st_size;
    char *buf = malloc(n + 1);
    if (!buf || fread(buf, 1, n, f) != n) {
        fclose(f);
        free(buf);
        snprintf(err, len, "cannot read %s", path);
        return MR_EIO;
    }
    fclose(f);
    buf[n] = 0;
    int rc = mr_config_parse(buf, c, err, len);
    free(buf);
    return rc;
}

size_t mr_config_mark_self(mr_config_t *c, const uint8_t own[32])
{
    size_t n = 0;
    for (size_t i = 0; i < c->n_peers; i++) {
        c->peers[i].is_self = memcmp(c->peers[i].pubkey, own, 32) == 0;
        n += c->peers[i].is_self;
    }
    return n;
}
