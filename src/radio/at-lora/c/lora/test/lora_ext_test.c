/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The lora extension as AT's network process sees it, without a network
 * process: its registration, its config gate and its verbs. */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include <jansson.h>

#include <pthread.h>

#include "config/configuration.h"
#include "identity/identity.h"
#include "identity/identity_priv.h"
#include "processes/processes.h"
#include "network/net_ext.h"
#include "network/net_liveness.h"
#include "utilities/logger.h"
#include "utilities/message.h"
#include "utilities/msg_types_priv.h"
#include "lora/at_lora.h"
#include "lora/lora_addrmap.h"
#include "lora/lora_config.h"
#include "lora/lora_engine.h"
#include "lora/lora_ext_priv.h"

static char g_root[] = "/tmp/lora_ext_rootXXXXXX";
static char g_cfg[512];

static void _write_cfg(const char *text)
{
    char path[600];
    snprintf(path, sizeof(path), "%s/lora.cfg.json", g_cfg);
    if (text == NULL) {
        unlink(path);
        return;
    }
    FILE *f = fopen(path, "w");
    fputs(text, f);
    fclose(f);
}

/* What messaging_send was asked to send, and what it answers. */
static struct {
    int n;
    int ret;
    char key[64];
    char process[PROC_NAME_LEN + 1];
    char function[64];
    char body[256];
    bool encrypt, group_multicast;
} S;

static int _hook(const char *key, const message_type_t type, generic_msg_t *msg, bool blocking)
{
    (void)type;
    (void)blocking;
    S.n++;
    snprintf(S.key, sizeof(S.key), "%s", key);
    snprintf(S.process, sizeof(S.process), "%s", msg->info.net_msg.process);
    S.encrypt = msg->info.net_msg.encrypt;
    S.group_multicast = msg->info.net_msg.group_multicast;
    snprintf(S.function, sizeof(S.function), "%s",
             msg->info.net_msg.function ? msg->info.net_msg.function : "");
    snprintf(S.body, sizeof(S.body), "%.*s", (int)msg->info.net_msg.len,
             msg->info.net_msg.obj ? (const char *)msg->info.net_msg.obj : "");
    return S.ret;
}

/* ---- a live engine over a fake link and a fake network ctx ---- */

static const lora_link_state_t LINK_UP = {true, true, true, true, 0xA01, 233, true};
static void fl_state(void *c, lora_link_state_t *o) { (void)c; *o = LINK_UP; }
static int fl_submit(void *c, uint32_t d, lora_frags_t *f, bool a) { (void)c; (void)d; (void)f; (void)a; return -1; }
static size_t fl_rx(void *c, lora_link_rx_t *o, size_t m) { (void)c; (void)o; (void)m; return 0; }
static size_t fl_res(void *c, lora_link_result_t *o, size_t m) { (void)c; (void)o; (void)m; return 0; }
static const lora_link_ops_t OPS = {NULL, fl_state, fl_submit, fl_rx, fl_res};

static process_t *g_proc;
static identity_t *g_me;
static net_thread_ctx_t g_ctx;
static char g_map[600];

static lora_engine_t *_live(const char *group_address)
{
    g_proc = calloc(1, sizeof(*g_proc));
    g_me = calloc(1, sizeof(*g_me));
    snprintf(g_proc->name, sizeof(g_proc->name), "%s", "network");
    pthread_rwlock_init(&g_proc->protocol.peers_rwlock, NULL);
    uuid_generate(g_proc->protocol.group.uuid);
    snprintf(g_proc->protocol.group.address, sizeof(g_proc->protocol.group.address), "%s",
             group_address);
    g_proc->protocol.group.wire_format = NET_WIRE_PROTO;
    uuid_generate(g_proc->protocol.peers[0].uuid);
    g_proc->protocol.num_peers = 1;
    uuid_generate(g_me->uuid);
    memset(&g_ctx, 0, sizeof(g_ctx));
    g_ctx.proc = g_proc;
    g_ctx.myself = g_me;
    snprintf(g_map, sizeof(g_map), "%s/lora_peers.json", g_root);
    unlink(g_map);
    lora_cfg_t cfg;
    char f[64], w[160];
    ck_assert_int_eq(lora_config_parse("{\"port\": 10}", &cfg, f, sizeof(f), w, sizeof(w)), 0);
    lora_engine_t *e = lora_ext_attach(&g_ctx, &cfg, OPS, g_map);
    ck_assert_ptr_nonnull(e);
    memset(&S, 0, sizeof(S));
    messaging_set_test_hook(_hook);
    return e;
}

static void _unlive(void)
{
    messaging_set_test_hook(NULL);
    lora_ext_detach();
    pthread_rwlock_destroy(&g_proc->protocol.peers_rwlock);
    free(g_proc);
    free(g_me);
    unlink(g_map);
}

/* self_issued_ms persisted in g_map (0 when the file is absent). */
static uint64_t _persisted_self(void)
{
    lora_addrmap_t *m = lora_addrmap_new();
    uint64_t self = 0;
    ck_assert_int_eq(lora_addrmap_load(m, g_map, &self), 0);
    lora_addrmap_free(m);
    return self;
}

DEFINE_TEST(test_registered_with_its_wire_verb)
{
    at_lora_link();
    ck_assert(net_ext_present("lora"));
    ck_assert(net_ext_wire_verb_declared("lora_addr"));
    ck_assert(!net_ext_wire_verb_declared("lora_stats"));
}
END_TEST_DEFINITION()

DEFINE_TEST(test_config_gate)
{
    _write_cfg(NULL);
    ck_assert_int_eq(net_ext_check_config(NULL), 0);
    _write_cfg("{\"port\": 1}");
    ck_assert_int_eq(net_ext_check_config(NULL), -1);
    _write_cfg("{\"port\": 10}");
    ck_assert_int_eq(net_ext_check_config(NULL), 0);
    _write_cfg(NULL);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_announce_verb_declines_our_own_and_takes_a_peers)
{
    char fn[] = "lora_addr";
    net_msg_t mine = {0}, theirs = {0};
    mine.function = fn;
    theirs.function = fn;
    uuid_generate(theirs.from_whom.uuid);
    ck_assert(!net_ext_local_verb(NULL, &mine));   /* no sender: the core sends it */
    ck_assert(net_ext_local_verb(NULL, &theirs));  /* inert engine: taken, ignored */
}
END_TEST_DEFINITION()

/* A member's announcement counts only from an envelope whose signature
 * checked out against the key held for it. */
DEFINE_TEST(test_announce_verb_refuses_an_unverified_envelope)
{
    lora_engine_t *e = _live("239.192.0.1");
    char fn[] = "lora_addr";
    net_msg_t m = {0};
    m.function = fn;
    uuid_copy(m.from_whom.uuid, g_proc->protocol.peers[0].uuid);
    json_t *body = json_pack("{s:i,s:i}", "lora_addr", 0xB02, "issued_ms", 9);
    ck_assert_int_eq(net_msg_pack_json(&m, body), 0);
    ck_assert(net_ext_local_verb(&g_ctx, &m));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.unverified"), 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.accepted"), 0);
    m.verified = true;
    ck_assert(net_ext_local_verb(&g_ctx, &m));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.accepted"), 1);
    json_decref(body);
    net_msg_free_obj(&m);
    _unlive();
}
END_TEST_DEFINITION()

DEFINE_TEST(test_stats_answer_locally_and_refuse_the_wire)
{
    char fn[] = "lora_stats";
    net_msg_t local = {0}, wire = {0};
    local.function = fn;
    wire.function = fn;
    snprintf(local.return_to, sizeof(local.return_to), "%s", "tool");
    snprintf(wire.return_to, sizeof(wire.return_to), "%s", "tool");
    uuid_generate(wire.from_whom.uuid);
    memset(&S, 0, sizeof(S));
    messaging_set_test_hook(_hook);
    ck_assert(net_ext_local_verb(NULL, &local));
    ck_assert_int_eq(S.n, 1);
    ck_assert_str_eq(S.key, "tool");
    ck_assert_str_eq(S.function, "lora_stats_resp");
    ck_assert_str_eq(S.body, "{\"inert\":true}");
    ck_assert(net_ext_local_verb(NULL, &wire));    /* refused, but owned */
    ck_assert_int_eq(S.n, 1);
    messaging_set_test_hook(NULL);
}
END_TEST_DEFINITION()

DEFINE_TEST(test_stats_answer_a_local_caller_that_names_this_node)
{
    (void)_live("239.192.0.1");
    char fn[] = "lora_stats";
    net_msg_t mine = {0}, other = {0};
    mine.function = fn;
    other.function = fn;
    snprintf(mine.return_to, sizeof(mine.return_to), "%s", "tool");
    snprintf(other.return_to, sizeof(other.return_to), "%s", "tool");
    uuid_copy(mine.from_whom.uuid, g_me->uuid);
    uuid_copy(other.from_whom.uuid, g_proc->protocol.peers[0].uuid);
    ck_assert(net_ext_local_verb(&g_ctx, &mine));
    ck_assert_int_eq(S.n, 1);
    ck_assert_str_eq(S.function, "lora_stats_resp");
    ck_assert(net_ext_local_verb(&g_ctx, &other));   /* refused, but owned */
    ck_assert_int_eq(S.n, 1);
    _unlive();
}
END_TEST_DEFINITION()

DEFINE_TEST(test_periodic_queues_the_announcement_then_reports_it_sent)
{
    lora_engine_t *e = _live("239.192.0.1");
    net_ext_periodic(&g_ctx);
    ck_assert_int_eq(S.n, 1);
    ck_assert_str_eq(S.key, "network");
    ck_assert_str_eq(S.process, "network");
    ck_assert_str_eq(S.function, "lora_addr");
    ck_assert(S.encrypt);
    ck_assert(S.group_multicast);
    json_t *body = json_loads(S.body, 0, NULL);
    ck_assert_ptr_nonnull(body);
    ck_assert_int_eq(json_integer_value(json_object_get(body, "lora_addr")), 0xA01);
    uint64_t issued = (uint64_t)json_integer_value(json_object_get(body, "issued_ms"));
    ck_assert(issued > 0);
    json_decref(body);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 1);
    ck_assert(_persisted_self() == issued);
    net_ext_periodic(&g_ctx);                  /* sent: no longer due */
    ck_assert_int_eq(S.n, 1);
    _unlive();
}
END_TEST_DEFINITION()

DEFINE_TEST(test_periodic_refused_send_stays_due)
{
    lora_engine_t *e = _live("239.192.0.1");
    S.ret = 1;                                 /* the queue refuses it */
    net_ext_periodic(&g_ctx);
    ck_assert_int_eq(S.n, 1);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 0);
    ck_assert(_persisted_self() == 0);
    net_ext_periodic(&g_ctx);                  /* still due: asked again */
    ck_assert_int_eq(S.n, 2);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 0);
    S.ret = 0;
    net_ext_periodic(&g_ctx);
    ck_assert_int_eq(S.n, 3);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 1);
    _unlive();
}
END_TEST_DEFINITION()

/* Lines of @p path containing @p needle. */
static int _lines_with(const char *path, const char *needle)
{
    FILE *f = fopen(path, "r");
    ck_assert_ptr_nonnull(f);
    char line[512];
    int n = 0;
    while (fgets(line, sizeof(line), f) != NULL)
        if (strstr(line, needle) != NULL)
            n++;
    fclose(f);
    return n;
}

DEFINE_TEST(test_a_full_network_queue_warns_once_a_minute)
{
    char log[600];
    snprintf(log, sizeof(log), "%s/lora_ext_test.log", g_root);
    logger_t lg;
    ck_assert_int_eq(logger_init(&lg, DEBUG, log), 0);
    (void)_live("239.192.0.1");
    g_ctx.logger = &lg;
    S.ret = 1;                                 /* the queue refuses it, twice within 60 s */
    net_ext_periodic(&g_ctx);
    net_ext_periodic(&g_ctx);
    ck_assert_int_eq(S.n, 2);
    ck_assert_int_eq(_lines_with(log, "lora: network queue full; announcement not queued"), 1);
    _unlive();
    if (lg.file != NULL)                       /* logger_close has no definition to link */
        fclose(lg.file);
    unlink(log);
}
END_TEST_DEFINITION()

/* Only a group frame sealed for the node's own group goes on the air: its
 * LoRa tag names that group, so a send to a child cohort stays off. */
DEFINE_TEST(test_group_fanout_goes_only_for_the_nodes_own_group)
{
    lora_engine_t *e = _live("239.192.0.1");
    net_liveness_reset(net_liveness_now() - 1000.0);   /* the peer is quiet on IP */
    char peer[37];
    uuid_unparse_lower(g_proc->protocol.peers[0].uuid, peer);
    json_t *a = json_pack("{s:i,s:i}", "lora_addr", 0xB02, "issued_ms", 5);
    ck_assert_int_eq(lora_engine_on_announce(e, peer, a, true), LORA_ANN_ACCEPTED);
    json_decref(a);
    group_t *child = calloc(1, sizeof(*child));
    uuid_generate(child->uuid);
    const uint8_t frame[3] = {1, 2, 3};
    net_ext_fanout(NET_CHAN_GROUP, "identity", "group_key_update", child, frame, sizeof(frame));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 0);
    /* The fake link refuses every submit, so an attempted send counts abandoned. */
    net_ext_fanout(NET_CHAN_GROUP, "identity", "group_key_update", &g_proc->protocol.group,
                   frame, sizeof(frame));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 1);
    /* No group means the node's own. */
    net_ext_fanout(NET_CHAN_GROUP, "identity", "group_key_update", NULL, frame, sizeof(frame));
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.tx.abandoned"), 2);
    free(child);
    _unlive();
}
END_TEST_DEFINITION()

DEFINE_TEST(test_periodic_without_a_group_sends_nothing)
{
    lora_engine_t *e = _live("");
    net_ext_periodic(&g_ctx);
    ck_assert_int_eq(S.n, 0);
    ck_assert_int_eq((int)lora_engine_counter(e, "lora.announce.sent"), 0);
    _unlive();
}
END_TEST_DEFINITION()

DEFINE_TEST(test_other_verbs_are_not_ours)
{
    char fn[] = "lora_something";
    net_msg_t m = {0};
    m.function = fn;
    ck_assert(!net_ext_local_verb(NULL, &m));
}
END_TEST_DEFINITION()

DEFINE_TEST(test_hooks_are_harmless_when_inert)
{
    uuid_t u;
    uuid_generate(u);
    const uint8_t frame[3] = {1, 2, 3};
    net_ext_also_unicast(u, "identity", "peer_dm", frame, sizeof(frame));
    net_ext_fanout(NET_CHAN_GROUP, "identity", "presence", NULL, frame, sizeof(frame));
    net_note_exclusion("12345678-0000-4000-8000-0000000000aa", NULL, true);
    net_ext_periodic(NULL);
    ck_assert(true);
}
END_TEST_DEFINITION()

int main(void)
{
    ck_assert_ptr_nonnull(mkdtemp(g_root));
    setenv("AUTONOMOUS_TRUST_ROOT", g_root, 1);
    ck_assert(get_cfg_dir(g_cfg, sizeof(g_cfg)) > 0);
    char cmd[700];
    snprintf(cmd, sizeof(cmd), "mkdir -p '%s'", g_cfg);
    ck_assert_int_eq(system(cmd), 0);
    test_registered_with_its_wire_verb();
    test_config_gate();
    test_announce_verb_declines_our_own_and_takes_a_peers();
    test_stats_answer_locally_and_refuse_the_wire();
    test_other_verbs_are_not_ours();
    test_hooks_are_harmless_when_inert();
    test_periodic_queues_the_announcement_then_reports_it_sent();
    test_periodic_refused_send_stays_due();
    test_periodic_without_a_group_sends_nothing();
    test_a_full_network_queue_warns_once_a_minute();
    test_stats_answer_a_local_caller_that_names_this_node();
    test_group_fanout_goes_only_for_the_nodes_own_group();
    test_announce_verb_refuses_an_unverified_envelope();
    net_exclusions_reset();
    snprintf(cmd, sizeof(cmd), "rm -rf '%s'", g_root);
    if (system(cmd) != 0)
        fprintf(stderr, "could not remove %s\n", g_root);
    printf("=== %d checks, %d failures ===\n", _test_count, _test_failures);
    return _test_failures > 0 ? 1 : 0;
}
