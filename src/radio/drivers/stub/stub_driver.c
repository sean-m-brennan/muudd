/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* stub-driver: a radiod driver for tests. On its own it pretends every send
 * was delivered. With --medium DIR, several stub drivers share a directory
 * of Unix datagram sockets that stands in for one LoRa channel.
 *
 * Medium datagram, big-endian: src(4) dest(4) port(2) packet_id(4) payload. */
#include <dirent.h>
#include <getopt.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "mr_common.h"
#include "mr_framing.h"
#include "mr_proto.h"

#define MEDIUM_HDR 14

static struct {
    uint32_t node, mtu, bw_khz, status_ms, loss_pct, delay_ms;
    long long exit_after_ms;
    int exit_code;
    bool no_hello, no_status, garbage, ignore_term;
    const char *fatal, *region, *preset, *medium;
} O = {
    .node = 0xA01, .mtu = 233, .bw_khz = 500, .status_ms = 5000, .exit_after_ms = -1, .exit_code = 3,
    .fatal = "", .region = "US", .preset = "STUB",
};

static int g_sock = -1;
static char g_path[108];
static uint32_t g_next_pid;

static void put32(uint8_t *p, uint32_t v) { p[0] = v >> 24; p[1] = v >> 16; p[2] = v >> 8; p[3] = v; }
static void put16(uint8_t *p, uint16_t v) { p[0] = v >> 8; p[1] = v; }
static uint32_t get32(const uint8_t *p) { return (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3]; }
static uint16_t get16(const uint8_t *p) { return (uint16_t)(p[0] << 8 | p[1]); }

static void emit(const MrD2R *m)
{
    uint8_t buf[MR_FRAME_MAX + 2];
    int n = mr_frame_pack(&m->base, buf, sizeof buf);
    if (n < 0 || mr_frame_write_fd(1, buf, (size_t)n) != MR_OK)
        exit(1);
}

static Muudd__Radio__RadioSettings settings(void)
{
    Muudd__Radio__RadioSettings rs = MUUDD__RADIO__RADIO_SETTINGS__INIT;
    rs.region = (char *)O.region;
    rs.preset = (char *)O.preset;
    rs.bandwidth_khz = O.bw_khz;
    return rs;
}

static void send_hello(void)
{
    Muudd__Radio__LoraParams lp = MUUDD__RADIO__LORA_PARAMS__INIT;
    lp.spreading_factor = 7;
    lp.bandwidth_hz = O.bw_khz * 1000u;
    lp.coding_rate = 1;
    lp.preamble_symbols = 16;
    Muudd__Radio__RadioSettings rs = settings();
    MrCaps caps = MUUDD__RADIO__CAPS__INIT;
    caps.name = (char *)"stub";
    caps.version = (char *)"1.0";
    caps.reach = MUUDD__RADIO__REACH__MESH;
    caps.mtu = O.mtu;
    /* SF7, coding rate 4/5; the stub's frames carry no on-air overhead */
    caps.bitrate_bps = 7 * (O.bw_khz * 1000u) / 128 * 4 / 5;
    caps.overhead_bytes = 0;
    caps.lora = &lp;
    caps.reports_link_quality = true;
    caps.supports_ack = true;
    caps.node_addr = O.node;
    caps.radio_settings = &rs;
    caps.fatal = (char *)O.fatal;
    MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_HELLO;
    m.hello = &caps;
    emit(&m);
}

static void send_status(void)
{
    Muudd__Radio__RadioSettings rs = settings();
    Muudd__Radio__DriverStatus st = MUUDD__RADIO__DRIVER_STATUS__INIT;
    st.state = MUUDD__RADIO__RADIO_STATE__STATE_UP;
    st.reason = (char *)"";
    st.radio_settings = &rs;
    MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_STATUS;
    m.status = &st;
    emit(&m);
}

static void send_result(uint32_t req, int status, int reason, uint32_t pid)
{
    MrSendResult r = MUUDD__RADIO__SEND_RESULT__INIT;
    r.req_id = req;
    r.status = status;
    r.reason = reason;
    r.packet_id = pid;
    MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
    m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_SEND_RESULT;
    m.send_result = &r;
    emit(&m);
}

static bool lost(void) { return O.loss_pct && (uint32_t)(rand() % 100) < O.loss_pct; }

static void medium_transmit(uint32_t dest, uint32_t port, uint32_t pid, const uint8_t *data, size_t n)
{
    uint8_t buf[MEDIUM_HDR + MR_FRAME_MAX];
    put32(buf, O.node);
    put32(buf + 4, dest);
    put16(buf + 8, (uint16_t)port);
    put32(buf + 10, pid);
    memcpy(buf + MEDIUM_HDR, data, n);
    if (O.delay_ms)
        usleep(O.delay_ms * 1000u);
    DIR *d = opendir(O.medium);
    if (!d)
        return;
    struct dirent *e;
    while ((e = readdir(d))) {
        size_t len = strlen(e->d_name);
        if (len < 5 || strcmp(e->d_name + len - 5, ".sock") != 0)
            continue;
        if (strlen(O.medium) + 1 + len >= sizeof(((struct sockaddr_un *)0)->sun_path))
            continue;
        struct sockaddr_un sa = {.sun_family = AF_UNIX};
        char full[sizeof sa.sun_path + 256];   /* length checked above */
        snprintf(full, sizeof full, "%s/%s", O.medium, e->d_name);
        memcpy(sa.sun_path, full, strlen(full) + 1);
        if (strcmp(sa.sun_path, g_path) == 0 || lost())
            continue;
        sendto(g_sock, buf, MEDIUM_HDR + n, MSG_DONTWAIT, (struct sockaddr *)&sa, sizeof sa);
    }
    closedir(d);
}

static bool dest_reachable(uint32_t dest)
{
    char path[sizeof g_path];
    snprintf(path, sizeof path, "%s/%08x.sock", O.medium, dest);
    return access(path, F_OK) == 0 && !lost();
}

static int on_frame(void *ctx, const uint8_t *buf, size_t n)
{
    (void)ctx;
    MrR2D *m = muudd__radio__radiod_to_driver__unpack(NULL, n, buf);
    if (!m)
        return 0;
    if (m->msg_case == MUUDD__RADIO__RADIOD_TO_DRIVER__MSG_SEND) {
        const MrSend *s = m->send;
        if (s->payload.len > O.mtu) {
            send_result(s->req_id, MUUDD__RADIO__SEND_STATUS__REFUSED, MUUDD__RADIO__REFUSE_REASON__TOO_BIG, 0);
        } else {
            uint32_t pid = ++g_next_pid;
            if (O.medium)
                medium_transmit(s->dest, s->port, pid, s->payload.data, s->payload.len);
            send_result(s->req_id, MUUDD__RADIO__SEND_STATUS__SENT, 0, pid);
            if (s->want_ack) {
                bool ok = s->dest != MR_BROADCAST && (!O.medium || dest_reachable(s->dest));
                send_result(s->req_id, ok ? MUUDD__RADIO__SEND_STATUS__ACKED : MUUDD__RADIO__SEND_STATUS__NO_ACK,
                            0, pid);
            }
        }
    }
    muudd__radio__radiod_to_driver__free_unpacked(m, NULL);
    return 0;
}

static void medium_receive(void)
{
    uint8_t buf[MEDIUM_HDR + MR_FRAME_MAX];
    ssize_t n;
    while ((n = recv(g_sock, buf, sizeof buf, MSG_DONTWAIT)) >= MEDIUM_HDR) {
        uint32_t dest = get32(buf + 4);
        if (dest != MR_BROADCAST && dest != O.node)
            continue;
        MrMeta meta = MUUDD__RADIO__META__INIT;
        meta.rssi_dbm = -80;
        meta.snr_db = 5.0f;
        meta.has_link_quality = true;
        meta.has_hops = true;
        meta.rx_time_ms = mr_wall_ms();
        meta.protection = dest == MR_BROADCAST ? MUUDD__RADIO__PROTECTION__CHANNEL_KEY
                                               : MUUDD__RADIO__PROTECTION__PUBLIC_KEY_DM;
        meta.packet_id = get32(buf + 10);
        MrReceived rv = MUUDD__RADIO__RECEIVED__INIT;
        rv.src = get32(buf);
        rv.dest = dest;
        rv.port = get16(buf + 8);
        rv.payload.data = buf + MEDIUM_HDR;
        rv.payload.len = (size_t)n - MEDIUM_HDR;
        rv.meta = &meta;
        MrD2R m = MUUDD__RADIO__DRIVER_TO_RADIOD__INIT;
        m.msg_case = MUUDD__RADIO__DRIVER_TO_RADIOD__MSG_RECEIVED;
        m.received = &rv;
        emit(&m);
    }
}

static void cleanup(void)
{
    if (g_sock >= 0)
        unlink(g_path);
}

int main(int argc, char **argv)
{
    static const struct option opts[] = {
        {"node-addr", required_argument, 0, 'n'}, {"mtu", required_argument, 0, 'm'},
        {"bandwidth-khz", required_argument, 0, 'b'}, {"region", required_argument, 0, 'r'},
        {"preset", required_argument, 0, 'p'}, {"status-ms", required_argument, 0, 's'},
        {"exit-after-ms", required_argument, 0, 'x'}, {"exit-code", required_argument, 0, 'c'},
        {"no-hello", no_argument, 0, 'H'}, {"no-status", no_argument, 0, 'S'},
        {"garbage", no_argument, 0, 'g'}, {"fatal", required_argument, 0, 'f'},
        {"ignore-sigterm", no_argument, 0, 'i'}, {"medium", required_argument, 0, 'M'},
        {"loss", required_argument, 0, 'l'}, {"delay-ms", required_argument, 0, 'd'},
        {0, 0, 0, 0},
    };
    int ch;
    while ((ch = getopt_long(argc, argv, "", opts, NULL)) != -1) {
        switch (ch) {
        case 'n': O.node = (uint32_t)strtoul(optarg, NULL, 0); break;
        case 'm': O.mtu = (uint32_t)strtoul(optarg, NULL, 0); break;
        case 'b': O.bw_khz = (uint32_t)strtoul(optarg, NULL, 0); break;
        case 'r': O.region = optarg; break;
        case 'p': O.preset = optarg; break;
        case 's': O.status_ms = (uint32_t)strtoul(optarg, NULL, 0); break;
        case 'x': O.exit_after_ms = strtoll(optarg, NULL, 0); break;
        case 'c': O.exit_code = atoi(optarg); break;
        case 'H': O.no_hello = true; break;
        case 'S': O.no_status = true; break;
        case 'g': O.garbage = true; break;
        case 'f': O.fatal = optarg; break;
        case 'i': O.ignore_term = true; break;
        case 'M': O.medium = optarg; break;
        case 'l': O.loss_pct = (uint32_t)strtoul(optarg, NULL, 0); break;
        case 'd': O.delay_ms = (uint32_t)strtoul(optarg, NULL, 0); break;
        default: fprintf(stderr, "stub-driver: unknown option\n"); return 2;
        }
    }
    signal(SIGPIPE, SIG_IGN);
    if (O.ignore_term)
        signal(SIGTERM, SIG_IGN);
    srand(O.node ^ (unsigned)getpid());
    g_next_pid = (uint32_t)rand();
    if (O.medium) {
        g_sock = socket(AF_UNIX, SOCK_DGRAM | SOCK_CLOEXEC, 0);
        struct sockaddr_un sa = {.sun_family = AF_UNIX};
        snprintf(g_path, sizeof g_path, "%s/%08x.sock", O.medium, O.node);
        unlink(g_path);
        snprintf(sa.sun_path, sizeof sa.sun_path, "%s", g_path);
        if (g_sock < 0 || bind(g_sock, (struct sockaddr *)&sa, sizeof sa) != 0) {
            fprintf(stderr, "stub-driver: cannot bind %s\n", g_path);
            return 1;
        }
    }
    if (!O.no_hello)
        send_hello();
    if (O.garbage) {
        static const uint8_t junk[] = {0xFF, 0xFF, 0x00, 0x01};
        mr_frame_write_fd(1, junk, sizeof junk);
    }
    uint64_t start = mr_now_ms(), last_status = start;
    mr_decoder_t dec;
    mr_decoder_init(&dec);
    for (;;) {
        struct pollfd fds[2] = {{0, POLLIN, 0}, {g_sock, POLLIN, 0}};
        poll(fds, g_sock >= 0 ? 2 : 1, 20);
        if (fds[0].revents & (POLLIN | POLLHUP)) {
            uint8_t buf[4096];
            ssize_t n = read(0, buf, sizeof buf);
            if (n <= 0 || mr_decoder_feed(&dec, buf, (size_t)n, on_frame, NULL) != MR_OK)
                break;
        }
        if (g_sock >= 0 && (fds[1].revents & POLLIN))
            medium_receive();
        uint64_t now = mr_now_ms();
        if (!O.no_status && !O.no_hello && now - last_status >= O.status_ms) {
            send_status();
            last_status = now;
        }
        if (O.exit_after_ms >= 0 && now - start >= (uint64_t)O.exit_after_ms) {
            cleanup();
            return O.exit_code;
        }
    }
    cleanup();
    return 0;
}
