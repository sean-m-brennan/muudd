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
            self.conns.append(conn)
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    @staticmethod
    def _reply(conn, **kw):
        conn.sendall(encode(pb.RadiodToConsumer(**kw).SerializeToString()))

    def _serve(self, conn):
        self._reply(conn, radio_status=pb.RadioStatus(state=pb.STATE_UP))
        reader = FrameReader()
        while True:
            try:
                data = conn.recv(4096)
            except OSError:
                return
            if not data:
                return
            for frame in reader.feed(data):
                m = pb.ConsumerToRadiod.FromString(frame)
                kind = m.WhichOneof("msg")
                if kind == "claim":
                    self.claims.append(m.claim.port)
                    self._reply(conn, claim_result=pb.ClaimResult(req_id=m.claim.req_id, status=pb.CLAIM_OK, mtu=233))
                elif kind == "send":
                    s = m.send
                    self._reply(conn, send_result=pb.SendResult(req_id=s.req_id, status=pb.SENT, packet_id=5))
                    if s.want_ack:
                        self._reply(conn, send_result=pb.SendResult(req_id=s.req_id, status=pb.ACKED, packet_id=5))
                    self._reply(conn, received=pb.Received(src=0xB1, dest=s.dest, port=s.port, payload=s.payload))
                elif kind == "get_stats":
                    self._reply(conn, stats=pb.Stats(req_id=m.get_stats.req_id, airtime_us=42))

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
