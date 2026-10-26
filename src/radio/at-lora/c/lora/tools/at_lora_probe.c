/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* at_lora_probe: the C at-lora engine on a real radiod, for interop tests.
 * The peer is preloaded and always quiet; one optional unicast is sent with
 * one fragment optionally dropped on its first transmission, so the NACK
 * round is exercised deterministically. Prints deliveries and counters. */
#include <getopt.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include <jansson.h>

#include "lora/lora_config.h"
#include "lora/lora_engine.h"
#include "lora/lora_link.h"

static char g_me[37], g_peer[37];
static lora_link_t *g_link;
static int g_drop = -1;
static bool g_dropped;

static void _my_uuid(void *a, char out[37]) { (void)a; memcpy(out, g_me, 37); }
static size_t _members(void *a, char (*out)[37], size_t max)
{
    (void)a;
    if (max == 0) return 0;
    memcpy(out[0], g_peer, 37);
    return 1;
}
static double _quiet(void *a, const char *u) { (void)a; (void)u; return 1e9; }
static bool _proto(void *a) { (void)a; return true; }
static bool _tag(void *a, uint8_t t[4]) { (void)a; memcpy(t, "\xa1\xb2\xc3\xd4", 4); return true; }
static double _now(void *a)
{
    (void)a;
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}
static uint64_t _wall(void *a) { return (uint64_t)(_now(a) * 1000.0); }
static void _log(void *a, int level, const char *msg) { (void)a; fprintf(stderr, "[%d] %s\n", level, msg); }

/* The real link's submit, dropping fragment g_drop from the first message. */
static int _submit(void *ctx, uint32_t dest, lora_frags_t *f, bool ack)
{
    if (g_drop >= 0 && !g_dropped && f->n > (size_t)g_drop && f->n > 1) {
        g_dropped = true;
        free(f->buf[g_drop]);
        memmove(&f->buf[g_drop], &f->buf[g_drop + 1], (f->n - (size_t)g_drop - 1) * sizeof(f->buf[0]));
        memmove(&f->len[g_drop], &f->len[g_drop + 1], (f->n - (size_t)g_drop - 1) * sizeof(f->len[0]));
        f->n--;
    }
    return lora_link_submit(ctx, dest, f, ack);
}

int main(int argc, char **argv)
{
    const char *sock = NULL, *send_hex = NULL;
    unsigned port = 0, peer_addr = 0;
    int seconds = 20;
    static struct option opts[] = {
        {"socket", 1, 0, 's'}, {"port", 1, 0, 'p'}, {"me", 1, 0, 'm'}, {"peer", 1, 0, 'P'},
        {"peer-addr", 1, 0, 'a'}, {"send-hex", 1, 0, 'x'}, {"drop-first-frag", 1, 0, 'd'},
        {"seconds", 1, 0, 'S'}, {0, 0, 0, 0}};
    int c;
    while ((c = getopt_long(argc, argv, "", opts, NULL)) != -1) {
        switch (c) {
        case 's': sock = optarg; break;
        case 'p': port = (unsigned)strtoul(optarg, NULL, 0); break;
        case 'm': snprintf(g_me, sizeof(g_me), "%s", optarg); break;
        case 'P': snprintf(g_peer, sizeof(g_peer), "%s", optarg); break;
        case 'a': peer_addr = (unsigned)strtoul(optarg, NULL, 16); break;
        case 'x': send_hex = optarg; break;
        case 'd': g_drop = atoi(optarg); break;
        case 'S': seconds = atoi(optarg); break;
        default: return 2;
        }
    }
    if (sock == NULL || port == 0 || g_me[0] == '\0' || g_peer[0] == '\0' || peer_addr == 0)
        return 2;
    lora_cfg_t cfg;
    char field[64], why[160], text[256];
    snprintf(text, sizeof(text), "{\"port\": %u, \"nack_gap_s\": 2, \"reassembly_s\": 10, "
             "\"allow\": [\"test/probe\"]}", port);
    if (lora_config_parse(text, &cfg, field, sizeof(field), why, sizeof(why)) != 0)
        return 2;
    char map[] = "/tmp/at_lora_probe_XXXXXX";
    int fd = mkstemp(map);
    close(fd);
    unlink(map);
    g_link = lora_link_start(sock, port, _log, NULL);
    lora_link_ops_t ops = lora_link_ops(g_link);
    ops.submit = _submit;
    lora_env_t env = {NULL, _my_uuid, _members, _quiet, _proto, _tag, _wall, _now, _log};
    lora_engine_t *e = lora_engine_new(&cfg, ops, env, map);
    json_t *ann = json_pack("{s:I,s:I}", "lora_addr", (json_int_t)peer_addr, "issued_ms", (json_int_t)1);
    lora_engine_on_announce(e, g_peer, ann, true);
    json_decref(ann);
    bool sent = send_hex == NULL;
    double end = _now(NULL) + seconds;
    while (_now(NULL) < end) {
        lora_link_state_t st;
        lora_link_state(g_link, &st);
        if (!sent && st.up) {
            size_t n = strlen(send_hex) / 2;
            uint8_t *msg = malloc(n);
            for (size_t i = 0; i < n; i++) {
                unsigned v;
                sscanf(send_hex + 2 * i, "%2x", &v);
                msg[i] = (uint8_t)v;
            }
            sent = lora_engine_on_unicast(e, g_peer, "test", "probe", msg, n);
            free(msg);
        }
        lora_delivery_t d[8];
        size_t k = lora_engine_poll(e, d, 8);
        static const char *ch[] = {"peer", "broadcast", "group"};
        for (size_t i = 0; i < k; i++) {
            printf("deliver %s %s ", ch[d[i].channel], d[i].uuid);
            for (size_t j = 0; j < d[i].len; j++)
                printf("%02x", d[i].msg[j]);
            printf("\n");
            fflush(stdout);
            free(d[i].msg);
        }
        usleep(50000);
    }
    const char *names[] = {"lora.tx.msgs", "lora.nack.sent", "lora.nack.served", "lora.rx.delivered"};
    for (size_t i = 0; i < 4; i++)
        printf("counter %s %llu\n", names[i], (unsigned long long)lora_engine_counter(e, names[i]));
    lora_engine_free(e);
    lora_link_stop(g_link);
    unlink(map);
    return 0;
}
