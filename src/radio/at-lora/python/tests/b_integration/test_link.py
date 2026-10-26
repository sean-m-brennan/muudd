# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import logging
import threading
import time

import pytest
from radiod_harness import Radiod, stub_driver, wait_until

from autonomous_trust.lora._python import frame as fr
from autonomous_trust.lora._python.link import RadioLink

AT_PORT = {"10": {"name": "at", "share_pct": 40, "policy": "consumer"}}


def _cfg(medium, node, ports=AT_PORT):
    return {"node_addr": "auto", "duty_cycle_pct": 100,
            "driver": stub_driver("--medium", medium, "--node-addr", hex(node), "--status-ms", "200"),
            "ports": ports}


@pytest.fixture
def pair(tmp_path):
    medium = tmp_path / "air"
    medium.mkdir()
    a = Radiod(tmp_path, "a").start(_cfg(medium, 0xA01))
    b = Radiod(tmp_path, "b").start(_cfg(medium, 0xB02))
    yield a, b
    a.stop()
    b.stop()


def _results(link, timeout=5.0):
    got = []
    wait_until(lambda: got.extend(link.poll_results()) or got, timeout)
    return got


def test_claims_reports_up_and_frames_cross(pair):
    a, b = pair
    la = RadioLink(a.socket, 10, logging.getLogger('a'))
    lb = RadioLink(b.socket, 10, logging.getLogger('b'))
    try:
        assert wait_until(lambda: la.state().up and lb.state().up, 5)
        assert la.state().node_addr == 0xA01 and la.state().mtu >= 64
        frags = fr.fragment(bytes(range(200)) * 3, fr.PEER, 7, la.state().mtu, 8)
        assert la.submit(0xB02, frags, True)
        assert _results(la) == [(True, '', len(frags), len(frags))]
        rx = []
        assert wait_until(lambda: rx.extend(lb.poll_rx()) or len(rx) == len(frags), 5)
        assert [p for _, p, _ in rx] == frags and all(s == 0xA01 for s, _, _ in rx)
    finally:
        la.close()
        lb.close()


def test_port_unconfigured_stays_inert(tmp_path, caplog):
    medium = tmp_path / "air"
    medium.mkdir()
    r = Radiod(tmp_path, "r").start(_cfg(medium, 0xA01, ports={}))
    link = RadioLink(r.socket, 10, logging.getLogger('lora-test'))
    try:
        with caplog.at_level(logging.ERROR):
            time.sleep(1.0)
        st = link.state()
        assert st.connected and not st.claimed and not st.up
        assert sum('port 10' in m for m in caplog.messages) == 1
        assert not link.submit(0xB02, [b'\x11\x00\x01\x00x'], False)
    finally:
        link.close()
        r.stop()


def test_radiod_killed_mid_message(pair):
    a, b = pair
    la = RadioLink(a.socket, 10, logging.getLogger('a'))
    try:
        assert wait_until(lambda: la.state().up, 5)
        frags = fr.fragment(bytes(3000), fr.PEER, 8, la.state().mtu, 16)
        assert la.submit(0xB02, frags, True)
        a.kill()
        got = _results(la)
        assert len(got) == 1 and (got[0][0] is False or got[0][2] == got[0][3])
        assert wait_until(lambda: not la.state().up, 5)
        a.start()
        assert wait_until(lambda: la.state().up, 10)
    finally:
        la.close()


def test_job_queue_full_refuses(pair):
    a, _ = pair
    la = RadioLink(a.socket, 10, logging.getLogger('a'))
    try:
        assert wait_until(lambda: la.state().up, 5)
        la._pause_sending = True          # test seam: hold the queue, send nothing
        accepted = sum(la.submit(0xB02, [b'\x11\x00\x01\x00x'], False) for _ in range(20))
        assert accepted == RadioLink.JOBS_MAX
    finally:
        la.close()


def test_low_mtu_warns_once_and_stays_off(tmp_path, caplog):
    medium = tmp_path / "air"
    medium.mkdir()
    cfg = _cfg(medium, 0xA01)
    cfg["driver"] = stub_driver("--medium", medium, "--node-addr", "0xa01", "--status-ms", "100",
                                "--mtu", "40")
    r = Radiod(tmp_path, "r").start(cfg)
    link = RadioLink(r.socket, 10, logging.getLogger('lora-test'))
    try:
        with caplog.at_level(logging.WARNING):
            time.sleep(1.0)
        st = link.state()
        assert st.connected and not st.up
        assert sum('below 64' in m for m in caplog.messages) == 1
    finally:
        link.close()
        r.stop()


def test_refused_claim_retried_only_after_reconnect(tmp_path, caplog, monkeypatch):
    from muudd_radio.client import RadioClient
    claims = []
    real_claim = RadioClient.claim

    def spy(self, port, policy="radiod", timeout=5.0):
        claims.append(port)
        return real_claim(self, port, policy, timeout)

    monkeypatch.setattr(RadioClient, 'claim', spy)
    medium = tmp_path / "air"
    medium.mkdir()
    r = Radiod(tmp_path, "r").start(_cfg(medium, 0xA01, ports={}))
    link = RadioLink(r.socket, 10, logging.getLogger('lora-test'))
    try:
        with caplog.at_level(logging.ERROR):
            time.sleep(1.0)
            assert not link.state().claimed
            assert claims == [10]                        # no re-claim on a live connection
            assert sum('refused the claim' in m for m in caplog.messages) == 1
            r.kill()
            r.start(_cfg(medium, 0xA01))                 # the port is now configured
            assert wait_until(lambda: link.state().up, 10)
            assert claims == [10, 10]                    # exactly one attempt after the restart
            assert sum('refused the claim' in m for m in caplog.messages) == 1
    finally:
        link.close()
        r.stop()


def test_radio_down_reports_confirmed_fragments(tmp_path):
    medium = tmp_path / "air"
    medium.mkdir()
    cfg = _cfg(medium, 0xA01)
    cfg["driver"] = stub_driver("--medium", medium, "--node-addr", "0xa01", "--status-ms", "200",
                                "--delay-ms", "60")     # a slow air interface: ~60 ms per fragment
    r = Radiod(tmp_path, "r").start(cfg)
    la = RadioLink(r.socket, 10, logging.getLogger('a'))
    try:
        assert wait_until(lambda: la.state().up, 5)
        mtu = la.state().mtu
        frags = fr.fragment(bytes(3000), fr.PEER, 9, mtu, 16)
        assert la.submit(0xB02, frags, True)
        assert la.submit(0xB02, fr.fragment(bytes(100), fr.PEER, 10, mtu, 16), True)
        time.sleep(0.3)
        r.kill()
        got = []
        assert wait_until(lambda: got.extend(la.poll_results()) or len(got) == 2, 5)
        first, second = got
        assert first[:2] == (False, 'RADIO_DOWN') and 0 < first[2] < len(frags) == first[3]
        assert second[:3] == (False, 'RADIO_DOWN', 0)
    finally:
        la.close()
        r.stop()


def test_close_is_prompt_and_leaves_no_thread(pair):
    a, _ = pair
    la = RadioLink(a.socket, 10, logging.getLogger('a'))
    assert wait_until(lambda: la.state().up, 5)
    a.kill()
    assert wait_until(lambda: not la.state().up, 5)
    time.sleep(2.5)                  # let the client's reconnect backoff grow
    t0 = time.monotonic()
    la.close()
    assert time.monotonic() - t0 < 3.0
    assert not la._thread.is_alive()
    assert not [t for t in threading.enumerate() if t.name in ('lora-link', 'muudd-radio-client')]


def test_every_connection_is_logged(pair, caplog):
    """Each new connection to radiod logs one 'connected' line, as the C link
    does, including one the client re-established on its own."""
    a, _ = pair
    with caplog.at_level(logging.INFO, logger='lora-reconnect'):
        la = RadioLink(a.socket, 10, logging.getLogger('lora-reconnect'))
        try:
            assert wait_until(lambda: la.state().up, 5)
            a.kill()
            assert wait_until(lambda: not la.state().up, 5)
            a.start()
            assert wait_until(lambda: la.state().up, 10)
            msgs = caplog.messages
            assert sum('connected to radiod' in m for m in msgs) == 2
            assert sum('lost the connection' in m for m in msgs) == 1
        finally:
            la.close()
