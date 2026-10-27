/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* libat_lora's attachment to AutonomousTrust's network process: a config
 * gate, the announce and stats verbs, and the network hooks that feed the
 * engine. Present means linked; without <cfg_dir>/lora.cfg.json it is inert.
 * Twin of the Python extension. Every hook runs on the network process's
 * main loop, which is what keeps the engine single-threaded. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include <jansson.h>
#include <uuid/uuid.h>

#include "config/configuration.h"
#include "identity/identity.h"
#include "identity/identity_priv.h"
#include "network/net_ext.h"
#include "network/net_liveness.h"
#include "network/net_proc_priv.h"
#include "processes/processes.h"
#include "utilities/logger.h"
#include "utilities/message.h"
#include "utilities/msg_types_priv.h"

#include "lora/at_lora.h"
#include "lora/lora_config.h"
#include "lora/lora_engine.h"
#include "lora/lora_ext_priv.h"
#include "lora/lora_link.h"

static const char LORA_ADDR[] = "lora_addr";
static const char LORA_STATS[] = "lora_stats";
static const char LORA_STATS_RESP[] = "lora_stats_resp";
static const char ADDRMAP_FILE[] = "lora_peers.json";
#define DELIVERIES_MAX LORA_LINK_RX_MAX   /* the engine never completes more than it returns */
#define WARN_EVERY_S 60.0

static struct {
    net_thread_ctx_t *ctx;
    lora_cfg_t cfg;
    lora_link_t *link;
    lora_engine_t *eng;
    double pack_warned_at, queue_warned_at;   /* the announcement WARNs, once per WARN_EVERY_S */
} g = {.pack_warned_at = -1e300, .queue_warned_at = -1e300};

void at_lora_link(void)
{
}

/* ---- the engine's environment ---- */

static void _log(void *arg, int level, const char *msg)
{
    logger_t *lg = arg;
    if (level >= 2)
        log_error(lg, "%s\n", msg);
    else if (level == 1)
        log_warn(lg, "%s\n", msg);
    else if (level == 0)
        log_info(lg, "%s\n", msg);
    else
        log_debug(lg, "%s\n", msg);
}

static void _my_uuid(void *arg, char out[37])
{
    (void)arg;
    if (g.ctx != NULL && g.ctx->myself != NULL)
        uuid_unparse_lower(g.ctx->myself->uuid, out);
    else
        out[0] = '\0';
}

static size_t _members(void *arg, char (*out)[37], size_t max)
{
    (void)arg;
    process_t *proc = g.ctx->proc;
    peers_read_lock(proc);
    size_t n = 0;
    for (size_t i = 0; i < proc->protocol.num_peers && n < max; i++)
        uuid_unparse_lower(proc->protocol.peers[i].uuid, out[n++]);
    peers_read_unlock(proc);
    return n;
}

static double _quiet_for(void *arg, const char *uuid)
{
    (void)arg;
    uuid_t u;
    if (uuid_parse(uuid, u) != 0)
        return 0.0;
    return net_liveness_quiet_for(u, net_liveness_now());
}

static bool _group_proto(void *arg)
{
    (void)arg;
    return g.ctx->proc->protocol.group.wire_format == NET_WIRE_PROTO;
}

/* The group's tag: the first 4 bytes of its uuid, once the node has a group. */
static bool _group_tag(void *arg, uint8_t tag[4])
{
    (void)arg;
    const group_t *grp = &g.ctx->proc->protocol.group;
    if (grp->address[0] == '\0')
        return false;
    memcpy(tag, grp->uuid, 4);
    return true;
}

static uint64_t _wall_ms(void *arg)
{
    (void)arg;
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}

static double _now(void *arg)
{
    (void)arg;
    return net_liveness_now();
}

/* ---- the hooks ---- */

/* <cfg_dir>/lora.cfg.json into @p path; false when the directory is unknown. */
static bool _cfg_path(char *path, size_t cap)
{
    char dir[512];
    if (get_cfg_dir(dir, sizeof(dir)) <= 0)
        return false;
    return snprintf(path, cap, "%s/%s", dir, LORA_CFG_FILENAME) < (int)cap;
}

static int _check_config(logger_t *logger)
{
    char path[600], field[64], why[160];
    lora_cfg_t cfg;
    if (!_cfg_path(path, sizeof(path)))
        return 0;
    if (lora_config_load(path, &cfg, field, sizeof(field), why, sizeof(why)) < 0) {
        log_error(logger, "lora: %s refused: %s: %s\n", path, field, why);
        return -1;
    }
    return 0;
}

static void _start(net_thread_ctx_t *ctx)
{
    char path[600], field[64], why[160], dir[512];
    g.ctx = ctx;
    int rc = _cfg_path(path, sizeof(path))
                 ? lora_config_load(path, &g.cfg, field, sizeof(field), why, sizeof(why))
                 : 1;
    if (rc == 1) {
        log_info(ctx->logger, "lora: no %s; the extension is inert\n", LORA_CFG_FILENAME);
        return;
    }
    if (rc != 0) {   /* check_config refuses this before start; kept inert if it got here */
        log_error(ctx->logger, "lora: %s refused: %s: %s\n", path, field, why);
        return;
    }
    if (get_data_dir(dir, sizeof(dir)) <= 0) {
        log_error(ctx->logger, "lora: no data directory for %s; the extension is inert\n",
                  ADDRMAP_FILE);
        return;
    }
    snprintf(path, sizeof(path), "%s/%s", dir, ADDRMAP_FILE);
    g.link = lora_link_start(g.cfg.socket, g.cfg.port, _log, ctx->logger);
    if (g.link == NULL) {
        log_error(ctx->logger, "lora: could not start the radio link; the extension is inert\n");
        return;
    }
    if (lora_ext_attach(ctx, &g.cfg, lora_link_ops(g.link), path) == NULL) {
        log_error(ctx->logger, "lora: could not start the engine; the extension is inert\n");
        lora_link_stop(g.link);
        g.link = NULL;
    }
}

lora_engine_t *lora_ext_attach(net_thread_ctx_t *ctx, const lora_cfg_t *cfg,
                               lora_link_ops_t link, const char *addrmap_path)
{
    g.ctx = ctx;
    lora_env_t env = {
        .arg = ctx->logger, .my_uuid = _my_uuid, .members = _members, .quiet_for = _quiet_for,
        .group_proto = _group_proto, .group_tag = _group_tag, .wall_ms = _wall_ms,
        .now = _now, .log = _log,
    };
    g.eng = lora_engine_new(cfg, link, env, addrmap_path);
    return g.eng;
}

void lora_ext_detach(void)
{
    lora_engine_free(g.eng);
    g.eng = NULL;
    if (g.link != NULL)
        lora_link_stop(g.link);
    g.link = NULL;
    g.ctx = NULL;
    g.pack_warned_at = g.queue_warned_at = -1e300;
}

/* Log @p what as a WARN unless it was logged in the last WARN_EVERY_S. */
static void _warn_limited(net_thread_ctx_t *ctx, double *at, const char *what)
{
    double now = net_liveness_now();
    if (now - *at < WARN_EVERY_S)
        return;
    *at = now;
    log_warn(ctx->logger, "%s\n", what);
}

/* Queue our announcement to our own network process, which sends it to the
 * group like the presence heartbeat. @return 0 queued, -1 the body could not
 * be built, 1 the queue refused it. */
static int _send_announce(net_thread_ctx_t *ctx, uint32_t addr, uint64_t issued_ms)
{
    generic_msg_t m = {0};
    m.type = NET_MESSAGE;
    snprintf(m.info.net_msg.process, sizeof(m.info.net_msg.process), "%s", "network");
    m.info.net_msg.function = (char *)LORA_ADDR;
    m.info.net_msg.encrypt = true;
    m.info.net_msg.group_multicast = true;
    json_t *body = json_pack("{s:I,s:I}", "lora_addr", (json_int_t)addr,
                             "issued_ms", (json_int_t)issued_ms);
    if (body == NULL)
        return -1;
    int packed = net_msg_pack_json(&m.info.net_msg, body);
    json_decref(body);
    if (packed != 0) {
        net_msg_free_obj(&m.info.net_msg);
        return -1;
    }
    int rc = messaging_send(ctx->proc->name, NET_MESSAGE, &m, false) == 0 ? 0 : 1;
    net_msg_free_obj(&m.info.net_msg);
    return rc;
}

static void _periodic(net_thread_ctx_t *ctx)
{
    if (g.eng == NULL || ctx == NULL)
        return;
    lora_delivery_t d[DELIVERIES_MAX];
    size_t n = lora_engine_poll(g.eng, d, DELIVERIES_MAX);
    for (size_t i = 0; i < n; i++) {
        handle_inbound_ext(ctx, (net_channel_t)d[i].channel, d[i].msg, d[i].len, d[i].uuid);
        free(d[i].msg);
    }
    uint32_t addr;
    uint64_t issued;
    if (lora_engine_announce_due(g.eng, &addr, &issued)) {
        int rc = _send_announce(ctx, addr, issued);
        if (rc == 0)
            lora_engine_announce_sent(g.eng, addr, issued);
        else if (rc < 0)
            _warn_limited(ctx, &g.pack_warned_at,
                          "lora: could not build the announcement; not queued");
        else
            _warn_limited(ctx, &g.queue_warned_at,
                          "lora: network queue full; announcement not queued");
    }
    char line[256];
    if (lora_engine_summary_due(g.eng, line, sizeof(line)))
        log_info(ctx->logger, "%s\n", line);
}

/* A peer's announcement (from the wire, so it carries a sender), or our own
 * outbound one (no sender), which is declined so the core sends it. A peer's
 * counts only if its envelope verified against the key we hold for it. */
static bool _lora_addr(net_msg_t *nmsg)
{
    if (uuid_is_null(nmsg->from_whom.uuid))
        return false;
    json_t *body = NULL;
    if (g.eng != NULL && net_msg_unpack_json(nmsg, &body) == 0) {
        char sender[37];
        uuid_unparse_lower(nmsg->from_whom.uuid, sender);
        (void)lora_engine_on_announce(g.eng, sender, body, nmsg->verified);
    }
    json_decref(body);
    return true;
}

/* Local only: the counters, the link and the address map, to return_to. A
 * local caller may name this node as the sender; any other sender came from
 * the wire. */
static bool _lora_stats(net_thread_ctx_t *ctx, net_msg_t *nmsg)
{
    if (!uuid_is_null(nmsg->from_whom.uuid)
        && (ctx == NULL || ctx->myself == NULL
            || uuid_compare(nmsg->from_whom.uuid, ctx->myself->uuid) != 0)) {
        log_warn(ctx != NULL ? ctx->logger : NULL, "lora: refusing lora_stats from the wire\n");
        return true;
    }
    if (nmsg->return_to[0] == '\0')
        return true;
    json_t *body = g.eng != NULL ? lora_engine_stats(g.eng) : json_pack("{s:b}", "inert", 1);
    if (body == NULL)
        return true;
    generic_msg_t m = {0};
    m.type = NET_MESSAGE;
    snprintf(m.info.net_msg.process, sizeof(m.info.net_msg.process), "%s", "network");
    m.info.net_msg.function = (char *)LORA_STATS_RESP;
    int packed = net_msg_pack_json(&m.info.net_msg, body);
    json_decref(body);
    if (packed == 0 && messaging_send(nmsg->return_to, NET_MESSAGE, &m, false) != 0)
        log_warn(ctx != NULL ? ctx->logger : NULL, "lora: could not answer lora_stats to %s\n",
                 nmsg->return_to);
    net_msg_free_obj(&m.info.net_msg);
    return true;
}

static bool _local_verb(net_thread_ctx_t *ctx, net_msg_t *nmsg)
{
    if (nmsg == NULL || nmsg->function == NULL)
        return false;
    if (strcmp(nmsg->function, LORA_ADDR) == 0)
        return _lora_addr(nmsg);
    if (strcmp(nmsg->function, LORA_STATS) == 0)
        return _lora_stats(ctx, nmsg);
    return false;
}

static void _also_unicast(const uuid_t peer, const char *process, const char *function,
                          const uint8_t *buf, size_t len)
{
    if (g.eng == NULL)
        return;
    char u[37];
    uuid_unparse_lower(peer, u);
    (void)lora_engine_on_unicast(g.eng, u, process, function, buf, len);
}

/* The LoRa frame carries the node's own group tag, so a group frame sealed
 * for another cohort (a gateway's send to a child cohort) stays off the air:
 * parent members could not open it and child members would drop it as
 * foreign. No group means the node's own. */
static void _fanout(net_channel_t ch, const char *process, const char *function,
                    const group_t *grp, const uint8_t *buf, size_t len)
{
    if (g.eng == NULL || g.ctx == NULL)
        return;
    if (ch == NET_CHAN_GROUP && grp != NULL
        && uuid_compare(grp->uuid, g.ctx->proc->protocol.group.uuid) != 0)
        return;
    if (ch == NET_CHAN_GROUP || ch == NET_CHAN_BROADCAST)
        (void)lora_engine_on_fanout(g.eng, ch == NET_CHAN_BROADCAST, process, function, buf, len);
}

static void _exclusion(const char *uuid, bool excluded)
{
    if (g.eng != NULL)
        lora_engine_on_exclusion(g.eng, uuid, excluded);
}

static const char *const LORA_WIRE_VERBS[] = {LORA_ADDR, NULL};

static const net_ext_t lora_net_ext = {
    .name = "lora",
    .start = _start,
    .periodic = _periodic,
    .local_verb = _local_verb,
    .exclusion = _exclusion,
    .wire_verbs = LORA_WIRE_VERBS,
    .check_config = _check_config,
    .also_unicast = _also_unicast,
    .fanout = _fanout,
};
NET_EXT_REGISTER(lora, &lora_net_ext)
