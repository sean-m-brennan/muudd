# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Configurations and a partitionable UDP proxy for multi-radiod tests."""
import os
import socket
import threading

from conftest import stub_driver

PORTS = {
    "16": {"name": "telemetry", "share_pct": 20, "policy": "radiod"},
    "17": {"name": "wide", "share_pct": 20, "policy": "radiod", "cross_region": True},
    "18": {"name": "fallback", "share_pct": 20, "policy": "radiod", "ip_fallback": True},
    "19": {"name": "local", "share_pct": 20, "policy": "radiod"},
}
FAST = {"keepalive_ms": 200, "dead_after_ms": 1000}


def relay_key(path):
    with open(path, "wb") as f:
        f.write(os.urandom(32))
    os.chmod(path, 0o600)
    return str(path)


def peer(r, priority=100, proxy=None):
    port = proxy.port_for(r) if proxy else r.port
    return {"addr": f"127.0.0.1:{port}", "pubkey": r.pubkey, "priority": priority}


def gateway(medium, node, region, priority, peers, key, duty=10):
    return {
        "node_addr": "auto",
        "duty_cycle_pct": duty,
        "driver": stub_driver("--medium", medium, "--node-addr", hex(node), "--status-ms", "200"),
        "ports": PORTS,
        "bridge": {"gateway": True, "region": region, "priority": priority, "relay_key_file": key, **FAST,
                   "peers": peers},
    }


def driverless(node, peers, key):
    return {"node_addr": node, "driver": None, "ports": PORTS,
            "bridge": {"relay_key_file": key, **FAST, "peers": peers}}


class UdpProxy:
    """Stands between radiods so a test can cut the IP network between groups."""

    def __init__(self):
        self._group_of_port = {}
        self._socks = {}
        self._blocked = set()
        self._out = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._stop = False

    def add(self, r, group):
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.bind(("127.0.0.1", 0))
        s.settimeout(0.1)
        self._socks[r.name] = (s, r)
        self._group_of_port[r.port] = group
        threading.Thread(target=self._pump, args=(s, r, group), daemon=True).start()

    def port_for(self, r):
        return self._socks[r.name][0].getsockname()[1]

    def block(self, a, b):
        self._blocked |= {(a, b), (b, a)}

    def heal(self):
        self._blocked.clear()

    def _pump(self, s, r, dst_group):
        while not self._stop:
            try:
                data, (_, sport) = s.recvfrom(65536)
            except socket.timeout:
                continue
            except OSError:
                return
            if (self._group_of_port.get(sport), dst_group) in self._blocked:
                continue
            self._out.sendto(data, ("127.0.0.1", r.port))

    def close(self):
        self._stop = True
        for s, _ in self._socks.values():
            s.close()
        self._out.close()
