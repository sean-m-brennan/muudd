# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import collections
import logging
import socket
import threading
import time

import pytest
from radiod_harness import Radiod, stub_driver, wait_until

from autonomous_trust.lora._python import frame as fr
from autonomous_trust.lora._python.link import RadioLink
from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import FrameReader, encode

AT_PORT = {"10": {"name": "at", "share_pct": 40, "policy": "consumer"}}
# Long enough for several 200 ms statuses: a check that something happened
# only once waits this long after it first happens.
STATUSES = 0.6


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
            assert wait_until(lambda: any('port 10' in m for m in caplog.messages), 5)
            time.sleep(STATUSES)            # more statuses arrive; none logs again
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
            assert wait_until(lambda: any('below 64' in m for m in caplog.messages), 5)
            time.sleep(STATUSES)            # more statuses carry the MTU; none warns again
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
            assert wait_until(lambda: any('refused the claim' in m for m in caplog.messages), 5)
            time.sleep(STATUSES)            # the connection stays up; nothing claims again
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


def test_radio_down_reports_confirmed_fragments(tmp_path, monkeypatch):
    from muudd_radio.client import RadioClient
    confirmed = []
    real_result = RadioClient.result

    def spy(self, req_id, timeout=5.0):
        r = real_result(self, req_id, timeout)
        confirmed.append(req_id)
        return r

    monkeypatch.setattr(RadioClient, 'result', spy)
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
        assert wait_until(lambda: len(confirmed) >= 2, 5)
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


# ---- a fake client: the link against exactly the connection events a test sets up ----

def _status(mtu=200, port=10):
    return pb.RadioStatus(state=pb.STATE_UP, caps=pb.Caps(node_addr=0xA01),
                          mtus=[pb.PortMtu(port=port, mtu=mtu)] if mtu else [])


class FakeClient:
    """Stands in for muudd_radio's RadioClient. The test moves `connected`,
    `reconnects` and `status` the way the real client does on a loss and a
    reconnect, and chooses what claims, sends and receives return."""

    def __init__(self, path, reconnect=True, connect_timeout=5.0):
        self.connected, self.reconnects, self.status = True, 0, None
        self.lost_claims = set()
        self.claims, self.claim_mtu, self.claim_error = 0, 200, None
        self.result_status, self.result_reason = pb.SENT, pb.REASON_NONE
        self.rx, self.endless_rx, self.sends = collections.deque(), False, 0
        self.send_timeout_ms = None
        FakeClient.made.append(self)

    def set_send_timeout(self, ms):
        self.send_timeout_ms = ms

    def claim(self, port, policy='radiod', timeout=5.0):
        self.claims += 1
        if self.claim_error is not None:
            raise self.claim_error
        return pb.ClaimResult(status=pb.CLAIM_OK, mtu=self.claim_mtu)

    def recv(self, timeout=5.0):
        if self.endless_rx:
            return pb.Received(src=0xB02, port=10, payload=b'x')
        if self.rx:
            return self.rx.popleft()
        time.sleep(min(timeout, 0.01))
        raise TimeoutError

    def send(self, dest, port, payload, want_ack=False):
        self.sends += 1
        return self.sends

    def result(self, req_id, timeout=5.0):
        return pb.SendResult(req_id=req_id, status=self.result_status, reason=self.result_reason)

    def close(self):
        self.connected = False


@pytest.fixture
def fake(monkeypatch):
    import muudd_radio.client
    FakeClient.made = []
    monkeypatch.setattr(muudd_radio.client, 'RadioClient', FakeClient)
    link = RadioLink('/nonexistent', 10, logging.getLogger('lora-fake'))
    try:
        assert wait_until(lambda: FakeClient.made, 5)
        yield link, FakeClient.made[0]
    finally:
        link.close()


def _lines(caplog, text):
    return sum(text in m for m in caplog.messages)


def test_link_sets_its_send_timeout(fake):
    _, c = fake
    assert wait_until(lambda: c.send_timeout_ms == RadioLink.SEND_TIMEOUT_MS == 500, 5)


def test_loss_before_the_first_status_is_reported(fake, caplog):
    link, c = fake
    with caplog.at_level(logging.INFO, logger='lora-fake'):
        assert wait_until(lambda: c.claims == 1 and link.state().connected, 5)
        c.connected = False                     # dropped before radiod sent any status
        assert wait_until(lambda: not link.state().connected, 5)
        assert wait_until(lambda: _lines(caplog, 'lost the connection') == 1, 5)
        c.reconnects, c.connected = 1, True
        c.status = _status()
        assert wait_until(lambda: link.state().up, 5)
        assert c.claims == 2
        assert _lines(caplog, 'connected to radiod') == 1     # the first came before capture
        assert _lines(caplog, 'lost the connection') == 1


def test_drop_and_fast_reconnect_is_a_loss(fake, caplog):
    link, c = fake
    c.status = _status()
    assert wait_until(lambda: link.state().up, 5)
    link._pause_sending = True                 # test seam: hold the queue, send nothing
    assert link.submit(0xB02, [b'a', b'b'], False) and link.submit(0xB02, [b'c'], False)
    with caplog.at_level(logging.INFO, logger='lora-fake'):
        # the client lost the connection and dialled again between two looks
        c.status, c.reconnects = None, 1
        got = []
        assert wait_until(lambda: got.extend(link.poll_results()) or len(got) == 2, 5)
        assert got == [(False, 'RADIO_DOWN', 0, 2), (False, 'RADIO_DOWN', 0, 1)]
        assert _lines(caplog, 'lost the connection') == 1
        assert wait_until(lambda: c.claims == 2, 5)
        assert _lines(caplog, 'connected to radiod') == 1
        assert not link.state().up
        c.status = _status()
        assert wait_until(lambda: link.state().up, 5)


def test_mtu_starts_over_on_each_connection(fake):
    link, c = fake
    c.status = _status(mtu=0)                  # the status carries no MTU for the port
    assert wait_until(lambda: link.state().up and link.state().mtu == 200, 5)   # the claim's
    c.claim_error = TimeoutError()             # the next connection's claim gets no answer
    c.status, c.reconnects = None, 1
    assert wait_until(lambda: c.claims == 2, 5)
    c.status = _status(mtu=0)
    assert wait_until(lambda: link.state().radio_up, 5)
    st = link.state()
    assert st.connected and st.mtu == 0 and not st.up
    time.sleep(0.1)                            # and it stays that way
    st = link.state()
    assert st.connected and st.radio_up and st.mtu == 0 and not st.up


def test_results_nobody_polls_are_dropped_and_counted(fake):
    link, c = fake
    c.result_status, c.result_reason = pb.REFUSED, pb.OVER_BUDGET
    c.status = _status()
    assert wait_until(lambda: link.state().up, 5)
    accepted, deadline = 0, time.monotonic() + 10
    while accepted < 40 and time.monotonic() < deadline:
        if link.submit(0xB02, [b'x'], False):
            accepted += 1
        else:
            time.sleep(0.005)
    assert accepted == 40
    assert wait_until(lambda: link.results_dropped == 8, 5)
    got = link.poll_results()
    assert len(got) == RadioLink.JOBS_MAX * 2 == 32
    assert all(r == (False, 'OVER_BUDGET', 0, 1) for r in got)


def test_receiving_is_bounded_so_sending_goes_on(fake):
    link, c = fake
    c.status = _status()
    assert wait_until(lambda: link.state().up, 5)
    c.endless_rx = True                        # a frame is always waiting
    assert link.submit(0xB02, [b'a', b'b', b'c'], False)
    got = []
    assert wait_until(lambda: got.extend(link.poll_results()) or got, 5)
    assert got == [(True, '', 3, 3)]
    assert link.state().up                     # status is still being read
    c.endless_rx = False
    assert len(link.poll_rx()) <= RadioLink.RX_MAX and link.rx_dropped > 0


def test_unexpected_error_resets_the_link_and_fails_jobs(fake, caplog):
    link, c = fake
    c.status = _status()
    assert wait_until(lambda: link.state().up, 5)
    real_send_job, raised = link._send_job, []

    def broken(*a):
        if not raised:
            raised.append(1)
            raise RuntimeError('boom')
        return real_send_job(*a)

    link._send_job = broken
    with caplog.at_level(logging.ERROR, logger='lora-fake'):
        assert link.submit(0xB02, [b'a', b'b'], False)
        got = []
        assert wait_until(lambda: got.extend(link.poll_results()) or got, 5)
        assert got == [(False, 'RADIO_DOWN', 0, 2)]
        assert _lines(caplog, 'unexpected error in the radiod link') == 1
    assert wait_until(lambda: link.state().up, 5)    # the next status brings it back
    assert link._thread.is_alive()
    assert link.submit(0xB02, [b'c'], False)
    got = []
    assert wait_until(lambda: got.extend(link.poll_results()) or got, 5)
    assert got == [(True, '', 1, 1)]


# ---- a radiod that stops reading ----

class StallingRadiod:
    """Grants claims with a large MTU and reports the radio up. stall() stops
    reading the connections open at the time (in small reads, so it takes
    almost nothing of the frame that follows), and the link's next big write
    finds no room in the socket. Later connections are served as normal."""

    def __init__(self, path):
        self.srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.srv.bind(path)
        self.srv.listen(4)
        self.conns = []
        self.stalled = set()
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while True:
            try:
                conn, _ = self.srv.accept()
            except OSError:
                return
            self.conns.append(conn)
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    def _serve(self, conn):
        reader = FrameReader()
        try:
            while True:
                data = conn.recv(256)
                if not data or conn in self.stalled:
                    return              # the socket stays open, unread
                for frame in reader.feed(data):
                    m = pb.ConsumerToRadiod.FromString(frame)
                    if m.WhichOneof('msg') == 'claim':
                        for r in (pb.RadiodToConsumer(claim_result=pb.ClaimResult(
                                      req_id=m.claim.req_id, status=pb.CLAIM_OK, mtu=16000)),
                                  pb.RadiodToConsumer(radio_status=_status(mtu=16000))):
                            conn.sendall(encode(r.SerializeToString()))
        except OSError:
            return

    def stall(self):
        self.stalled.update(self.conns)

    def stop(self):
        self.srv.close()
        for c in self.conns:
            c.close()


def test_stalled_radiod_fails_the_send_and_close_is_prompt(tmp_path, caplog):
    srv = StallingRadiod(str(tmp_path / 'stall.sock'))
    link = RadioLink(str(tmp_path / 'stall.sock'), 10, logging.getLogger('lora-stall'))
    try:
        assert wait_until(lambda: link.state().up, 5)
        link._client._sock.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)
        srv.stall()
        with caplog.at_level(logging.WARNING, logger='lora-stall'):
            t0 = time.monotonic()
            assert link.submit(0xB02, [bytes(15000)], False)   # more than the socket holds
            got = []
            assert wait_until(lambda: got.extend(link.poll_results()) or got, 5)
            took = time.monotonic() - t0
            assert got == [(False, 'RADIO_DOWN', 0, 1)]
            assert took < 1.5                   # the send gave up, not the 2 s result wait
            assert _lines(caplog, 'lost the connection') == 1
        # stalled again on the next connection: close must not wait on it
        assert wait_until(lambda: link.state().up, 5)
        client = link._client
        client._sock.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)
        # no send timeout on this connection: only close can end the send
        client.set_send_timeout(-1)
        writes = {'in': 0, 'out': 0}
        write = client._write

        def counted(msg):
            writes['in'] += 1
            try:
                return write(msg)
            finally:
                writes['out'] += 1

        client._write = counted
        srv.stall()
        assert link.submit(0xB02, [bytes(15000)], False)
        assert wait_until(lambda: writes['in'] > writes['out'], 5)
        time.sleep(0.2)
        assert writes['in'] > writes['out']     # the link thread is blocked in the send
        t0 = time.monotonic()
        link.close()
        assert time.monotonic() - t0 < 1.0
        assert not link._thread.is_alive()
        assert writes['in'] == writes['out']
    finally:
        link.close()
        srv.stop()
