# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import socket
import threading
import time

import pytest

from muudd_radio import radio_pb2 as pb
from muudd_radio.client import BROADCAST, RadioClient
from muudd_radio.framing import MAX_FRAME, FrameReader, encode


def test_framing_round_trip_and_split():
    data = encode(b"abc") + encode(b"") + encode(b"hello")
    reader = FrameReader()
    frames = reader.feed(data[:4]) + reader.feed(data[4:])
    assert frames == [b"abc", b"", b"hello"]


def test_framing_limits():
    with pytest.raises(ValueError):
        encode(b"x" * (MAX_FRAME + 1))
    with pytest.raises(ValueError):
        FrameReader().feed(b"\xff\xff")


class FakeRadiod:
    """Answers just enough of the consumer protocol to exercise the client."""

    def __init__(self, path):
        self.claims = []
        self.conns = []
        self.claim_status = pb.CLAIM_OK
        self.mute = False
        self.drop_next = 0
        self.srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.srv.bind(path)
        self.srv.listen(4)
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while True:
            try:
                conn, _ = self.srv.accept()
            except OSError:
                return
            if self.drop_next > 0:
                self.drop_next -= 1
                conn.close()
                continue
            self.conns.append(conn)
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    @staticmethod
    def _reply(conn, **kw):
        conn.sendall(encode(pb.RadiodToConsumer(**kw).SerializeToString()))

    def _serve(self, conn):
        try:
            self._reply(conn, radio_status=pb.RadioStatus(state=pb.STATE_UP))
        except OSError:
            return
        reader = FrameReader()
        while True:
            try:
                data = conn.recv(4096)
            except OSError:
                return
            if not data:
                return
            for frame in reader.feed(data):
                if self.mute:
                    continue
                m = pb.ConsumerToRadiod.FromString(frame)
                kind = m.WhichOneof("msg")
                if kind == "claim":
                    self.claims.append(m.claim.port)
                    self._reply(conn, claim_result=pb.ClaimResult(req_id=m.claim.req_id, status=self.claim_status, mtu=233))
                elif kind == "send":
                    s = m.send
                    self._reply(conn, send_result=pb.SendResult(req_id=s.req_id, status=pb.SENT, packet_id=5))
                    if s.want_ack:
                        self._reply(conn, send_result=pb.SendResult(req_id=s.req_id, status=pb.ACKED, packet_id=5))
                    self._reply(conn, received=pb.Received(src=0xB1, dest=s.dest, port=s.port, payload=s.payload))
                elif kind == "get_stats":
                    self._reply(conn, stats=pb.Stats(req_id=m.get_stats.req_id, airtime_us=42))

    def garbage(self):
        for c in self.conns:
            c.sendall(encode(b"\xff\xff\xff\xff"))

    def drop_all(self):
        for c in self.conns:
            try:
                c.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            c.close()
        self.conns.clear()

    def stop(self):
        self.srv.close()
        self.drop_all()


@pytest.fixture
def server(tmp_path):
    path = str(tmp_path / "r.sock")
    srv = FakeRadiod(path)
    yield path, srv
    srv.stop()


def test_status_claim_send_receive_stats(server):
    path, _ = server
    with RadioClient(path) as c:
        assert c.wait_status(pb.STATE_UP).state == pb.STATE_UP
        r = c.claim(16)
        assert r.status == pb.CLAIM_OK and r.mtu == 233
        req = c.send(BROADCAST, 16, b"x", want_ack=True)
        assert c.result(req).status == pb.SENT
        assert c.result(req).status == pb.ACKED
        rx = c.recv()
        assert rx.src == 0xB1 and rx.payload == b"x"
        assert c.stats().airtime_us == 42


def test_reconnect_reclaims(server):
    path, srv = server
    with RadioClient(path) as c:
        c.claim(16)
        srv.drop_all()
        deadline = time.monotonic() + 3
        while srv.claims.count(16) < 2 and time.monotonic() < deadline:
            time.sleep(0.05)
        assert srv.claims.count(16) == 2
        assert c.reconnects == 1
        req = c.send(BROADCAST, 16, b"y")
        assert c.result(req).status == pb.SENT


def wait_for(cond, timeout=5.0):
    deadline = time.monotonic() + timeout
    while not cond() and time.monotonic() < deadline:
        time.sleep(0.02)
    return cond()


def test_reconnect_survives_drop_during_reclaim(server):
    path, srv = server
    with RadioClient(path) as c:
        c.claim(16)
        srv.drop_next = 2
        srv.drop_all()
        assert wait_for(lambda: srv.claims.count(16) >= 2)
        assert c.reconnects >= 1
        assert c.result(c.send(BROADCAST, 16, b"z")).status == pb.SENT


def test_refused_reclaim_is_reported(server):
    path, srv = server
    with RadioClient(path) as c:
        c.claim(16)
        srv.claim_status = pb.PORT_TAKEN
        srv.drop_all()
        assert wait_for(lambda: 16 in c.lost_claims)
        assert 16 not in c._claims
        assert c.stats().airtime_us == 42


def test_garbage_frame_forces_reconnect(server):
    path, srv = server
    with RadioClient(path) as c:
        c.claim(16)
        srv.garbage()
        assert wait_for(lambda: srv.claims.count(16) >= 2)
        assert c.reconnects >= 1
        assert c.result(c.send(BROADCAST, 16, b"z")).status == pb.SENT


def test_waiters_fail_on_disconnect(server):
    path, srv = server
    with RadioClient(path) as c:
        c.claim(16)
        srv.mute = True
        req = c.send(BROADCAST, 16, b"q")
        errors = []

        def wait():
            try:
                c.result(req, timeout=10)
            except Exception as e:
                errors.append(e)

        t = threading.Thread(target=wait)
        t.start()
        time.sleep(0.2)
        srv.drop_all()
        t.join(3)
        assert not t.is_alive()
        assert len(errors) == 1 and isinstance(errors[0], ConnectionError)


def test_send_while_disconnected_raises(tmp_path):
    path = str(tmp_path / "r.sock")
    srv = FakeRadiod(path)
    c = RadioClient(path, reconnect=False)
    srv.stop()
    assert wait_for(lambda: c._sock is None)
    with pytest.raises(ConnectionError):
        c.send(BROADCAST, 16, b"x")
    c.close()


def test_received_overflow_drops_oldest(server):
    path, _ = server
    with RadioClient(path, max_received=3) as c:
        c.claim(16)
        for i in range(5):
            c.send(BROADCAST, 16, bytes([i]))
        c.stats()
        assert c.dropped_received == 2
        assert [c.recv().payload for _ in range(3)] == [b"\x02", b"\x03", b"\x04"]
        with pytest.raises(TimeoutError):
            c.recv(timeout=0.1)
