/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* muudd-radiod: owns the node's long-range radio and shares it. */
#include <errno.h>
#include <getopt.h>
#include <signal.h>
#include <sodium.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "mr_bootcount.h"
#include "mr_bridge_io.h"
#include "mr_config.h"
#include "mr_core.h"
#include "mr_crypto.h"
#include "mr_ipc.h"
#include "mr_supervisor.h"

#define DEFAULT_CONFIG "/etc/muudd/radio/radiod.json"
#define DEFAULT_KEY "/etc/muudd/radio/radiod.key"

typedef struct {
    mr_config_t cfg;
    mr_keypair_t kp;
    uint8_t relay_key[32];
    char boot_path[600];
    uint64_t boot_count;
    bool has_boot_count;
    bool has_relay_key;
    mr_core_t *core;
    mr_ipc_t ipc;
    mr_bridge_io_t bridge;
    bool has_bridge_io;
    mr_sup_t *sup;
    char *argv[MR_MAX_DRIVER_ARGS + 2];
} app_t;

static volatile sig_atomic_t g_stop, g_hup;

static void on_signal(int sig)
{
    if (sig == SIGHUP)
        g_hup = 1;
    else
        g_stop = 1;
}

static void logm(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    fputs("radiod: ", stderr);
    vfprintf(stderr, fmt, ap);
    fputc('\n', stderr);
    va_end(ap);
}

static void io_to_consumer(void *ctx, int cid, const MrR2C *m) { mr_ipc_send(&((app_t *)ctx)->ipc, cid, m); }

static void io_to_driver(void *ctx, const MrR2D *m)
{
    app_t *a = ctx;
    if (a->sup)
        mr_sup_send(a->sup, m);
}

static void io_to_peer(void *ctx, int peer, const MrDatagram *d)
{
    app_t *a = ctx;
    if (a->has_bridge_io)
        mr_bridge_io_send(&a->bridge, peer, d);
}

static uint64_t io_wall(void *ctx)
{
    (void)ctx;
    return mr_wall_ms();
}

static void io_log(void *ctx, const char *msg)
{
    (void)ctx;
    logm("%s", msg);
}

static int sup_hello(void *ctx, const MrCaps *caps)
{
    app_t *a = ctx;
    int rc = mr_core_driver_hello(a->core, caps, mr_now_ms());
    if (rc == MR_OK)
        logm("driver %s %s up: node 0x%08x, mtu %u", caps->name, caps->version, caps->node_addr, caps->mtu);
    else
        logm("driver refused: %s", mr_core_reason(a->core));
    return rc;
}

static void sup_msg(void *ctx, const MrD2R *m)
{
    app_t *a = ctx;
    uint64_t now = mr_now_ms();
    if (mr_core_driver_msg(a->core, m, now) == MR_EFATAL && mr_sup_state(a->sup) != MR_SUP_KILLING) {
        logm("driver refused: %s", mr_core_reason(a->core));
        mr_sup_fatal(a->sup, mr_core_reason(a->core), now);
    }
}

static void sup_down(void *ctx, const char *reason)
{
    app_t *a = ctx;
    logm("driver down: %s", reason);
    mr_core_driver_down(a->core, reason, mr_now_ms());
}

static void sup_stderr(void *ctx, const char *line)
{
    (void)ctx;
    logm("driver: %s", line);
}

/* Why a key file was refused, for the log. */
static const char *key_error(int rc)
{
    if (rc == MR_EPERM)
        return "group or others may access it; it must be mode 0600 or stricter";
    if (rc == MR_EINVAL)
        return "it must be exactly 32 bytes";
    if (errno == ELOOP)
        return "it is a symbolic link";
    return strerror(errno);
}

static int print_pubkey(const char *key_file)
{
    mr_keypair_t kp;
    int rc = mr_keypair_load_or_create(key_file, &kp);
    if (rc != MR_OK) {
        logm("cannot load or create %s: %s", key_file, key_error(rc));
        return 2;
    }
    char b64[sodium_base64_ENCODED_LEN(32, sodium_base64_VARIANT_ORIGINAL)];
    sodium_bin2base64(b64, sizeof b64, kp.pk, 32, sodium_base64_VARIANT_ORIGINAL);
    printf("%s\n", b64);
    return 0;
}

static void usage(FILE *f)
{
    fprintf(f, "usage: muudd-radiod [--config PATH] [--print-pubkey] [--key-file PATH]\n");
}

int main(int argc, char **argv)
{
    static const struct option opts[] = {
        {"config", required_argument, 0, 'c'},
        {"print-pubkey", no_argument, 0, 'p'},
        {"key-file", required_argument, 0, 'k'},
        {"help", no_argument, 0, 'h'},
        {0, 0, 0, 0},
    };
    const char *config_path = DEFAULT_CONFIG, *key_override = NULL;
    bool want_pubkey = false;
    int ch;
    while ((ch = getopt_long(argc, argv, "", opts, NULL)) != -1) {
        switch (ch) {
        case 'c': config_path = optarg; break;
        case 'p': want_pubkey = true; break;
        case 'k': key_override = optarg; break;
        case 'h': usage(stdout); return 0;
        default: usage(stderr); return 2;
        }
    }
    if (sodium_init() < 0)
        return 1;

    static app_t app;
    char err[256];
    if (want_pubkey) {
        const char *kf = key_override;
        if (!kf)
            kf = mr_config_load(config_path, &app.cfg, err, sizeof err) == MR_OK ? app.cfg.key_file : DEFAULT_KEY;
        return print_pubkey(kf);
    }
    if (mr_config_load(config_path, &app.cfg, err, sizeof err) != MR_OK) {
        logm("config: %s", err);
        return 2;
    }
    if (app.cfg.has_bridge) {
        const char *kf = key_override ? key_override : app.cfg.key_file;
        int krc = mr_keypair_load_or_create(kf, &app.kp);
        if (krc != MR_OK) {
            logm("cannot load or create bridge key %s: %s", kf, key_error(krc));
            return 2;
        }
        snprintf(app.boot_path, sizeof app.boot_path, "%s.boot", kf);
        char why[160];
        if (mr_bootcount_next(app.boot_path, &app.boot_count, why, sizeof why) == MR_OK)
            app.has_boot_count = true;
        else
            logm("cannot use boot counter %s: %s; using wall-clock start time", app.boot_path, why);
        size_t self = mr_config_mark_self(&app.cfg, app.kp.pk);
        if (self)
            logm("ignoring %zu peer entr%s carrying this node's own key", self, self == 1 ? "y" : "ies");
        if (app.cfg.relay_key_file[0]) {
            int rrc = mr_relay_key_load(app.cfg.relay_key_file, app.relay_key);
            if (rrc != MR_OK) {
                logm("cannot load relay key %s: %s", app.cfg.relay_key_file, key_error(rrc));
                return 2;
            }
            app.has_relay_key = true;
        }
    }

    signal(SIGPIPE, SIG_IGN);
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_signal;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);

    mr_core_io_t io = {&app, io_to_consumer, io_to_driver, io_to_peer, io_wall, io_log};
    if (mr_core_create(&app.cfg, &app.kp, app.has_relay_key ? app.relay_key : NULL, &io, &app.core) != MR_OK) {
        logm("out of memory");
        return 1;
    }
    if (app.has_boot_count)
        mr_core_set_boot_id(app.core, app.boot_count);
    if (mr_ipc_open(&app.ipc, app.cfg.socket_path, app.core) != MR_OK) {
        logm("cannot listen on %s: %s", app.cfg.socket_path, strerror(errno));
        return 1;
    }
    if (app.cfg.has_bridge) {
        if (mr_bridge_io_open(&app.bridge, app.cfg.listen, &app.kp, app.core) != MR_OK) {
            logm("cannot open the bridge socket on %s", app.cfg.listen);
            return 1;
        }
        app.has_bridge_io = true;
    }
    if (app.cfg.has_driver) {
        app.argv[0] = app.cfg.driver_exec;
        for (size_t i = 0; i < app.cfg.n_driver_args; i++)
            app.argv[i + 1] = app.cfg.driver_args[i];
        app.argv[app.cfg.n_driver_args + 1] = NULL;
        mr_sup_cfg_t sc;
        mr_sup_default_cfg(&sc, app.argv);
        mr_sup_io_t sio = {&app, sup_hello, sup_msg, sup_down, sup_stderr};
        if (mr_sup_create(&sc, &sio, &app.sup) != MR_OK) {
            logm("out of memory");
            return 1;
        }
    }
    logm("listening on %s", app.cfg.socket_path);

    while (!g_stop) {
        struct pollfd fds[1 + MR_MAX_CONSUMERS + 3];
        int n = mr_ipc_poll_fds(&app.ipc, fds, 1 + MR_MAX_CONSUMERS);
        int ipc_n = n, bridge_i = -1, sup_i;
        if (app.has_bridge_io) {
            bridge_i = n;
            fds[n++] = (struct pollfd){app.bridge.fd, POLLIN, 0};
        }
        sup_i = n;
        if (app.sup)
            n += mr_sup_poll_fds(app.sup, fds + n, 2);
        poll(fds, (nfds_t)n, 50);
        uint64_t now = mr_now_ms();
        mr_ipc_handle(&app.ipc, fds, ipc_n, now);
        if (bridge_i >= 0 && fds[bridge_i].revents)
            mr_bridge_io_handle(&app.bridge, now);
        if (app.sup) {
            mr_sup_handle(app.sup, fds + sup_i, n - sup_i, now);
            if (g_hup) {
                g_hup = 0;
                logm("SIGHUP: the driver may start again");
                mr_sup_reset_fatal(app.sup, now);
            }
            mr_sup_tick(app.sup, now);
        }
        mr_core_tick(app.core, now);
        if (app.has_bridge_io)
            mr_bridge_io_tick(&app.bridge, now);
    }

    logm("stopping");
    mr_sup_free(app.sup);
    mr_ipc_close(&app.ipc);
    if (app.has_bridge_io)
        mr_bridge_io_close(&app.bridge);
    mr_core_free(app.core);
    return 0;
}
