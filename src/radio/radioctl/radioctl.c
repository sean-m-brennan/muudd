/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* radioctl: shows what radiod is doing. */
#include <errno.h>
#include <getopt.h>
#include <jansson.h>
#include <stdio.h>
#include <string.h>

#include "mr_client.h"

static double ms(uint64_t us) { return (double)us / 1000.0; }

static const char *reach_name(int r) { return r == MUUDD__RADIO__REACH__LINK ? "link" : "mesh"; }

static void print_human(const MrStats *st)
{
    bool up = st->state == MUUDD__RADIO__RADIO_STATE__STATE_UP;
    if (up)
        printf("radio: up\n");
    else
        printf("radio: down (%s)\n", st->reason);
    if (st->caps) {
        printf("driver: %s %s, node 0x%08x, mtu %u, reach %s\n", st->caps->name, st->caps->version,
               st->caps->node_addr, st->caps->mtu, reach_name(st->caps->reach));
        if (st->caps->radio_settings)
            printf("settings: region %s, preset %s, bandwidth %u kHz\n", st->caps->radio_settings->region,
                   st->caps->radio_settings->preset, st->caps->radio_settings->bandwidth_khz);
    }
    printf("airtime: %.1f / %.1f ms this hour\n", ms(st->airtime_us), ms(st->cap_us));
    for (size_t i = 0; i < st->n_ports; i++) {
        const Muudd__Radio__PortStats *p = st->ports[i];
        printf("port %u %s: %.1f / %.1f ms (relay %.1f ms)\n", p->port, p->name, ms(p->airtime_us), ms(p->cap_us),
               ms(p->relay_airtime_us));
    }
    for (size_t i = 0; i < st->n_peers; i++) {
        const Muudd__Radio__PeerStats *p = st->peers[i];
        printf("peer %s: node 0x%08x, region %s, %s%s\n", p->addr, p->node_addr, p->region[0] ? p->region : "-",
               p->live ? "live" : "dead", p->gateway ? ", gateway" : "");
    }
    const Muudd__Radio__DropCounters *d = st->drops;
    if (d)
        printf("drops: unclaimed %llu, queue_overflow %llu, duplicate %llu, bridge_auth %llu, relay_auth %llu, "
               "relay_window %llu, relay_no_gateway %llu, over_budget %llu, relay_too_big %llu, dedupe_evicted %llu\n",
               (unsigned long long)d->unclaimed, (unsigned long long)d->queue_overflow,
               (unsigned long long)d->duplicate, (unsigned long long)d->bridge_auth,
               (unsigned long long)d->relay_auth, (unsigned long long)d->relay_window,
               (unsigned long long)d->relay_no_gateway, (unsigned long long)d->over_budget,
               (unsigned long long)d->relay_too_big, (unsigned long long)d->dedupe_evicted);
    printf("relay: %s\n", st->relay_active ? "active" : "inactive");
}

static void print_json(const MrStats *st)
{
    json_t *root = json_object();
    bool up = st->state == MUUDD__RADIO__RADIO_STATE__STATE_UP;
    json_object_set_new(root, "state", json_string(up ? "up" : "down"));
    json_object_set_new(root, "reason", json_string(st->reason));
    if (st->caps) {
        json_t *drv = json_object();
        json_object_set_new(drv, "name", json_string(st->caps->name));
        json_object_set_new(drv, "version", json_string(st->caps->version));
        json_object_set_new(drv, "node_addr", json_integer(st->caps->node_addr));
        json_object_set_new(drv, "mtu", json_integer(st->caps->mtu));
        json_object_set_new(drv, "reach", json_string(reach_name(st->caps->reach)));
        if (st->caps->radio_settings) {
            json_object_set_new(drv, "region", json_string(st->caps->radio_settings->region));
            json_object_set_new(drv, "preset", json_string(st->caps->radio_settings->preset));
            json_object_set_new(drv, "bandwidth_khz", json_integer(st->caps->radio_settings->bandwidth_khz));
        }
        json_object_set_new(root, "driver", drv);
    } else {
        json_object_set_new(root, "driver", json_null());
    }
    json_object_set_new(root, "airtime_us", json_integer((json_int_t)st->airtime_us));
    json_object_set_new(root, "cap_us", json_integer((json_int_t)st->cap_us));
    json_t *ports = json_array();
    for (size_t i = 0; i < st->n_ports; i++) {
        const Muudd__Radio__PortStats *p = st->ports[i];
        json_t *o = json_object();
        json_object_set_new(o, "port", json_integer(p->port));
        json_object_set_new(o, "name", json_string(p->name));
        json_object_set_new(o, "airtime_us", json_integer((json_int_t)p->airtime_us));
        json_object_set_new(o, "relay_airtime_us", json_integer((json_int_t)p->relay_airtime_us));
        json_object_set_new(o, "cap_us", json_integer((json_int_t)p->cap_us));
        json_array_append_new(ports, o);
    }
    json_object_set_new(root, "ports", ports);
    json_t *peers = json_array();
    for (size_t i = 0; i < st->n_peers; i++) {
        const Muudd__Radio__PeerStats *p = st->peers[i];
        json_t *o = json_object();
        json_object_set_new(o, "addr", json_string(p->addr));
        json_object_set_new(o, "node_addr", json_integer(p->node_addr));
        json_object_set_new(o, "region", json_string(p->region));
        json_object_set_new(o, "live", json_boolean(p->live));
        json_object_set_new(o, "gateway", json_boolean(p->gateway));
        json_array_append_new(peers, o);
    }
    json_object_set_new(root, "peers", peers);
    json_t *drops = json_object();
    const Muudd__Radio__DropCounters *d = st->drops;
    if (d) {
        json_object_set_new(drops, "unclaimed", json_integer((json_int_t)d->unclaimed));
        json_object_set_new(drops, "queue_overflow", json_integer((json_int_t)d->queue_overflow));
        json_object_set_new(drops, "duplicate", json_integer((json_int_t)d->duplicate));
        json_object_set_new(drops, "bridge_auth", json_integer((json_int_t)d->bridge_auth));
        json_object_set_new(drops, "relay_auth", json_integer((json_int_t)d->relay_auth));
        json_object_set_new(drops, "relay_window", json_integer((json_int_t)d->relay_window));
        json_object_set_new(drops, "relay_no_gateway", json_integer((json_int_t)d->relay_no_gateway));
        json_object_set_new(drops, "over_budget", json_integer((json_int_t)d->over_budget));
        json_object_set_new(drops, "relay_too_big", json_integer((json_int_t)d->relay_too_big));
        json_object_set_new(drops, "dedupe_evicted", json_integer((json_int_t)d->dedupe_evicted));
    }
    json_object_set_new(root, "drops", drops);
    json_object_set_new(root, "relay_active", json_boolean(st->relay_active));
    json_dumpf(root, stdout, JSON_INDENT(2));
    putchar('\n');
    json_decref(root);
}

static int usage(void)
{
    fprintf(stderr, "usage: radioctl [--socket PATH] status | stats [--json]\n");
    return 2;
}

int main(int argc, char **argv)
{
    static const struct option opts[] = {
        {"socket", required_argument, 0, 's'},
        {"json", no_argument, 0, 'j'},
        {0, 0, 0, 0},
    };
    const char *sock = "/run/muudd/radio.sock";
    bool json = false;
    int ch;
    while ((ch = getopt_long(argc, argv, "", opts, NULL)) != -1) {
        if (ch == 's')
            sock = optarg;
        else if (ch == 'j')
            json = true;
        else
            return usage();
    }
    if (optind != argc - 1)
        return usage();
    const char *cmd = argv[optind];
    if (strcmp(cmd, "status") != 0 && strcmp(cmd, "stats") != 0)
        return usage();
    mr_client_t *c = NULL;
    if (mr_client_connect(sock, &c) != MR_OK) {
        fprintf(stderr, "radioctl: cannot connect to %s: %s\n", sock, strerror(errno));
        return 1;
    }
    MrR2C *m = mr_client_get_stats(c, 3000);
    if (!m) {
        fprintf(stderr, "radioctl: no reply from radiod on %s\n", sock);
        mr_client_close(c);
        return 1;
    }
    if (json && strcmp(cmd, "stats") == 0)
        print_json(m->stats);
    else
        print_human(m->stats);
    muudd__radio__radiod_to_consumer__free_unpacked(m, NULL);
    mr_client_close(c);
    return 0;
}
