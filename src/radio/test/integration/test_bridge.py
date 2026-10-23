# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import contextlib
import time

import pytest

from radiod_harness import BROADCAST, Sniffer, inject, pb, wait_until
from topology import UdpProxy, driverless, gateway, peer, radio_node, relay_key

BOGUS_KEY = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="


def all_live(client, n):
    st = client.stats()
    return len(st.peers) == n and all(p.live for p in st.peers)


@contextlib.contextmanager
def sniff_closing(sniff):
    try:
        yield sniff
    finally:
        sniff.close()


def no_more(client, seconds=1.0):
    try:
        client.recv(timeout=seconds)
    except TimeoutError:
        return True
    return False


@pytest.fixture
def one_region(radiods, medium, tmp_path):
    """Gateways G1 (0xA1, priority 10) and G2 (0xA2, priority 20) on one medium,
    and a driverless node D (0x0D01)."""
    key = relay_key(tmp_path / "relay.key")
    g1, g2, d = radiods("g1"), radiods("g2"), radiods("d")
    g1.start(gateway(medium, 0xA1, "north", 10, [peer(g2, 20), peer(d)], key))
    g2.start(gateway(medium, 0xA2, "north", 20, [peer(g1, 10), peer(d)], key))
    d.start(driverless(0x0D01, [peer(g1, 10), peer(g2, 20)], key))
    return g1, g2, d


def test_two_gateways_hear_one_frame_once(one_region, medium):
    _, _, d = one_region
    with d.client() as c:
        c.wait_status(pb.STATE_UP)
        assert wait_until(lambda: all_live(c, 2))
        c.claim(16)
        inject(medium, 0xA1, 0xB001, BROADCAST, 16, b"t=1", 1)
        inject(medium, 0xA2, 0xB001, BROADCAST, 16, b"t=1", 1)
        assert wait_until(lambda: c.stats().drops.duplicate >= 1)
        rx = c.recv()
        assert rx.src == 0xB001 and rx.meta.via == pb.VIA_BRIDGE and rx.meta.region == "north"
        assert no_more(c)


def test_gateway_echo_not_delivered_twice(one_region, medium):
    _, g2, d = one_region
    sniff = Sniffer(medium)
    with sniff_closing(sniff), d.client() as cd, g2.client() as cg:
        cd.wait_status(pb.STATE_UP)
        cg.wait_status(pb.STATE_UP)
        assert wait_until(lambda: all_live(cd, 2) and all_live(cg, 2))
        cg.claim(16)
        cd.claim(16)
        assert cd.result(cd.send(BROADCAST, 16, b"hello")).status == pb.SENT
        rx = cg.recv()
        assert rx.src == 0x0D01 and rx.meta.via == pb.VIA_BRIDGE
        assert no_more(cg)
        assert no_more(cd, 0.2)
        sent = [f for f in sniff.frames(0.3) if f[2] == 16]
        assert [f[0] for f in sent] == [0xA1]


def test_failover_to_second_gateway(one_region, medium):
    g1, _, d = one_region
    sniff = Sniffer(medium)
    try:
        with d.client() as c:
            c.wait_status(pb.STATE_UP)
            assert wait_until(lambda: all_live(c, 2))
            c.claim(16)
            c.send(BROADCAST, 16, b"one")
            assert [f[0] for f in sniff.frames(0.5) if f[2] == 16] == [0xA1]
            g1.kill()
            time.sleep(1.5)
            c.send(BROADCAST, 16, b"two")
            assert [f[0] for f in sniff.frames(0.5) if f[2] == 16] == [0xA2]
    finally:
        sniff.close()


@pytest.fixture
def with_radio_node(radiods, medium, tmp_path):
    """Gateway G (0xA1), non-gateway radio node R (0xE1) on the same medium, and
    driverless D (0x0D01), all bridge peers of each other."""
    key = relay_key(tmp_path / "relay.key")
    g, r, d = radiods("g"), radiods("r"), radiods("d")
    g.start(gateway(medium, 0xA1, "north", 10, [peer(r), peer(d)], key))
    r.start(radio_node(medium, 0xE1, "north", [peer(g, 10), peer(d)], key))
    d.start(driverless(0x0D01, [peer(g, 10), peer(r)], key))
    return g, r, d


def test_radio_node_broadcast_reaches_ip_once(with_radio_node):
    g, r, d = with_radio_node
    with r.client() as cr, d.client() as cd, g.client() as cg:
        cr.wait_status(pb.STATE_UP)
        cd.wait_status(pb.STATE_UP)
        assert wait_until(lambda: all_live(cr, 2) and all_live(cd, 2) and all_live(cg, 2))
        cd.claim(16)
        cr.claim(16)
        assert cr.result(cr.send(BROADCAST, 16, b"from-r")).status == pb.SENT
        rx = cd.recv()
        assert rx.src == 0xE1 and rx.payload == b"from-r" and rx.meta.via == pb.VIA_BRIDGE
        assert no_more(cd)
        assert no_more(cr, 0.2)


def test_ip_broadcast_reaches_radio_node_once(with_radio_node):
    g, r, d = with_radio_node
    with r.client() as cr, d.client() as cd, g.client() as cg:
        cr.wait_status(pb.STATE_UP)
        cd.wait_status(pb.STATE_UP)
        assert wait_until(lambda: all_live(cr, 2) and all_live(cd, 2) and all_live(cg, 2))
        cr.claim(16)
        cd.claim(16)
        assert cd.result(cd.send(BROADCAST, 16, b"from-d")).status == pb.SENT
        rx = cr.recv()
        assert rx.src == 0x0D01 and rx.payload == b"from-d" and rx.meta.via == pb.VIA_BRIDGE
        assert no_more(cr)
        assert wait_until(lambda: cr.stats().drops.duplicate >= 1)


@pytest.fixture
def two_regions(radiods, tmp_path):
    """G1 (0xA1) alone on medium north, G2 (0xB2) alone on medium south, and D."""
    north, south = tmp_path / "north", tmp_path / "south"
    north.mkdir()
    south.mkdir()
    key = relay_key(tmp_path / "relay.key")
    g1, g2, d = radiods("g1"), radiods("g2"), radiods("d")
    g1.start(gateway(north, 0xA1, "north", 10, [peer(g2, 10), peer(d)], key))
    g2.start(gateway(south, 0xB2, "south", 10, [peer(g1, 10), peer(d)], key))
    d.start(driverless(0x0D01, [peer(g1, 10), peer(g2, 10)], key))
    return north, south, g1, g2, d


def test_unicast_goes_to_the_region_last_heard(two_regions):
    north, south, _, _, d = two_regions
    sn, ss = Sniffer(north), Sniffer(south)
    try:
        with d.client() as c:
            c.wait_status(pb.STATE_UP)
            assert wait_until(lambda: all_live(c, 2))
            c.claim(16)
            inject(south, 0xB2, 0xC002, BROADCAST, 16, b"here", 1)
            c.recv()
            req = c.send(0xC002, 16, b"dm")
            assert c.result(req).status == pb.SENT
            assert [(f[0], f[1]) for f in ss.frames(0.5) if f[2] == 16] == [(0xB2, 0xC002)]
            assert [f for f in sn.frames(0.2) if f[2] == 16] == []
    finally:
        sn.close()
        ss.close()


def test_cross_region_only_when_configured(two_regions):
    north, south, _, _, d = two_regions
    ss = Sniffer(south)
    try:
        with d.client() as c:
            c.wait_status(pb.STATE_UP)
            assert wait_until(lambda: all_live(c, 2))
            inject(north, 0xA1, 0xC001, BROADCAST, 16, b"local", 1)
            inject(north, 0xA1, 0xC001, BROADCAST, 17, b"wide", 2)
            frames = ss.frames(0.8)
            assert [(f[0], f[2], f[4]) for f in frames] == [(0xB2, 17, b"wide")]
    finally:
        ss.close()


def test_wrong_key_rejected(radiods, medium, tmp_path):
    key = relay_key(tmp_path / "relay.key")
    g1, d = radiods("g1"), radiods("d")
    g1.start(gateway(medium, 0xA1, "north", 10, [peer(d)], key))
    bad = dict(peer(g1, 10), pubkey=BOGUS_KEY)
    d.start(driverless(0x0D01, [bad], key))
    with d.client() as c:
        time.sleep(1.0)
        st = c.stats()
        assert not st.peers[0].live
        assert st.drops.bridge_auth > 0
        assert c.status.state == pb.STATE_DOWN


def test_over_budget_comes_back_through_the_bridge(radiods, medium, tmp_path):
    key = relay_key(tmp_path / "relay.key")
    g1, d = radiods("g1"), radiods("d")
    g1.start(gateway(medium, 0xA1, "north", 10, [peer(d)], key, duty=0.001))
    d.start(driverless(0x0D01, [peer(g1, 10)], key))
    with d.client() as c:
        c.wait_status(pb.STATE_UP)
        c.claim(16)
        inject(medium, 0xA1, 0xC001, BROADCAST, 16, b"x", 1)
        c.recv()
        req = c.send(0xC001, 16, bytes(150))
        r = c.result(req)
        assert r.status == pb.REFUSED and r.reason == pb.OVER_BUDGET


@pytest.fixture
def islands(radiods, medium, tmp_path):
    """Island A: G1 (0xA1) and D1 (0x0D01). Island B: G2 (0xA2) and D2 (0x0D02).
    Both gateways share one medium; all IP traffic passes through the proxy."""
    key = relay_key(tmp_path / "relay.key")
    g1, g2, d1, d2 = radiods("g1"), radiods("g2"), radiods("d1"), radiods("d2")
    proxy = UdpProxy()
    try:
        for r, group in ((g1, "A"), (d1, "A"), (g2, "B"), (d2, "B")):
            proxy.add(r, group)
        _start_islands(proxy, medium, key, g1, g2, d1, d2)
        yield proxy, d1, d2
    finally:
        proxy.close()


def _start_islands(proxy, medium, key, g1, g2, d1, d2):
    g1.start(gateway(medium, 0xA1, "north", 10, [peer(g2, 20, proxy), peer(d1, proxy=proxy), peer(d2, proxy=proxy)], key))
    g2.start(gateway(medium, 0xA2, "north", 20, [peer(g1, 10, proxy), peer(d1, proxy=proxy), peer(d2, proxy=proxy)], key))
    d1.start(driverless(0x0D01, [peer(g1, 10, proxy), peer(g2, 20, proxy), peer(d2, proxy=proxy)], key))
    d2.start(driverless(0x0D02, [peer(g1, 10, proxy), peer(g2, 20, proxy), peer(d1, proxy=proxy)], key))


def test_partition_crosses_by_relay_then_heals(islands, medium):
    proxy, d1, d2 = islands
    sniff = Sniffer(medium)
    try:
        with d1.client() as c1, d2.client() as c2:
            assert wait_until(lambda: all_live(c1, 3) and all_live(c2, 3))
            for c in (c1, c2):
                c.claim(16)
                c.claim(18)

            proxy.block("A", "B")
            time.sleep(1.5)

            c1.send(BROADCAST, 18, b"fb")
            rx = c2.recv()
            assert rx.src == 0x0D01 and rx.port == 18 and rx.payload == b"fb"
            assert rx.meta.via == pb.VIA_RELAY
            assert no_more(c2, 0.5)
            relays = [f for f in sniff.frames(0.5) if f[2] == 2]
            assert [f[0] for f in relays] == [0xA1]       # relayed once, never relayed back

            sniff.frames(0.1)
            req = c1.send(BROADCAST, 16, b"plain")
            assert c1.result(req).status == pb.SENT
            assert [f[0] for f in sniff.frames(0.3) if f[2] == 16] == [0xA1]
            assert no_more(c2, 1.0)

            c1.send(0x0D02, 18, b"dm")
            rx = c2.recv()
            assert rx.src == 0x0D01 and rx.port == 18
            assert rx.payload == b"dm" and rx.meta.via == pb.VIA_RELAY
            assert no_more(c2, 0.5)

            proxy.heal()
            assert wait_until(lambda: all_live(c1, 3) and all_live(c2, 3))
            sniff.frames(0.3)
            c1.send(BROADCAST, 18, b"healed")
            rx = c2.recv()
            assert rx.payload == b"healed" and rx.meta.via == pb.VIA_BRIDGE
            assert no_more(c2, 1.0)
            assert no_more(c1, 0.2)
            assert [f for f in sniff.frames(0.5) if f[2] == 2] == []
    finally:
        sniff.close()
