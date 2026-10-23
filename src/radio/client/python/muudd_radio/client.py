# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""A consumer of muudd-radiod.

RadioClient connects to radiod's Unix socket, claims ports, sends frames and
receives them. A background thread reads the socket and matches replies to
requests by req_id. If radiod restarts, the client reconnects and claims its
ports again.
"""
import itertools
import queue
import socket
import threading
import time

from . import radio_pb2 as pb
from .framing import FrameReader, encode

BROADCAST = 0xFFFFFFFF
POLICIES = {"radiod": pb.RADIOD_BRIDGED, "consumer": pb.CONSUMER_BRIDGED}


class RadioClient:
    def __init__(self, path, reconnect=True, connect_timeout=5.0):
        self.path = path
        self.reconnect = reconnect
        self.reconnects = 0
        self.received = queue.Queue()
        self._wlock = threading.Lock()
        self._rlock = threading.Lock()
        self._ids = itertools.count(1)
        self._claims = {}
        self._replies = {}
        self._sends = {}
        self._status = None
        self._status_cv = threading.Condition()
        self._closed = False
        self._sock = self._dial(connect_timeout)
        self._reader = threading.Thread(target=self._run, name="muudd-radio-client", daemon=True)
        self._reader.start()

    # ---- public API ----

    @property
    def status(self):
        with self._status_cv:
            return self._status

    def wait_status(self, state, timeout=5.0):
        deadline = time.monotonic() + timeout
        with self._status_cv:
            while self._status is None or self._status.state != state:
                left = deadline - time.monotonic()
                if left <= 0:
                    raise TimeoutError(f"radio state did not become {state}")
                self._status_cv.wait(left)
            return self._status

    def claim(self, port, policy="radiod", timeout=5.0):
        req = next(self._ids)
        msg = pb.ConsumerToRadiod(claim=pb.Claim(req_id=req, port=port, policy=POLICIES[policy]))
        result = self._request(msg, req, timeout).claim_result
        if result.status == pb.CLAIM_OK:
            self._claims[port] = policy
        return result

    def release(self, port, timeout=5.0):
        self._claims.pop(port, None)
        req = next(self._ids)
        self._request(pb.ConsumerToRadiod(release=pb.Release(req_id=req, port=port)), req, timeout)

    def send(self, dest, port, payload, want_ack=False):
        req = next(self._ids)
        with self._rlock:
            self._replies[req] = queue.Queue()
            self._sends[req] = want_ack
        send = pb.Send(req_id=req, dest=dest, port=port, payload=payload, want_ack=want_ack)
        self._write(pb.ConsumerToRadiod(send=send))
        return req

    def result(self, req_id, timeout=5.0):
        with self._rlock:
            q = self._replies.get(req_id)
        if q is None:
            raise KeyError(f"no outstanding request {req_id}")
        try:
            r = q.get(timeout=timeout).send_result
        except queue.Empty:
            raise TimeoutError(f"no result for request {req_id}") from None
        if r.status != pb.SENT or not self._sends.get(req_id):
            with self._rlock:
                if q.empty():
                    self._replies.pop(req_id, None)
                    self._sends.pop(req_id, None)
        return r

    def recv(self, timeout=5.0):
        try:
            return self.received.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError("nothing received") from None

    def stats(self, timeout=5.0):
        req = next(self._ids)
        return self._request(pb.ConsumerToRadiod(get_stats=pb.GetStats(req_id=req)), req, timeout).stats

    def close(self):
        self._closed = True
        with self._wlock:
            if self._sock is not None:
                try:
                    self._sock.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
                self._sock.close()
                self._sock = None
        self._reader.join(timeout=2)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    # ---- internals ----

    def _dial(self, timeout):
        deadline = time.monotonic() + timeout
        delay = 0.05
        while True:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                s.connect(self.path)
                return s
            except OSError:
                s.close()
                if time.monotonic() >= deadline:
                    raise
                time.sleep(delay)
                delay = min(delay * 2, 1.0)

    def _write(self, msg):
        data = encode(msg.SerializeToString())
        with self._wlock:
            if self._sock is None:
                raise ConnectionError("not connected to radiod")
            self._sock.sendall(data)

    def _request(self, msg, req, timeout):
        q = queue.Queue()
        with self._rlock:
            self._replies[req] = q
        try:
            self._write(msg)
            return q.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError(f"no reply to request {req}") from None
        finally:
            with self._rlock:
                self._replies.pop(req, None)

    def _run(self):
        reader = FrameReader()
        while not self._closed:
            sock = self._sock
            try:
                data = sock.recv(65536) if sock is not None else b""
            except OSError:
                data = b""
            if not data:
                if self._closed or not self.reconnect:
                    return
                self._reconnect()
                reader = FrameReader()
                continue
            try:
                frames = reader.feed(data)
            except ValueError:
                frames, reader = [], FrameReader()
            for frame in frames:
                self._dispatch(pb.RadiodToConsumer.FromString(frame))

    def _dispatch(self, m):
        kind = m.WhichOneof("msg")
        if kind == "received":
            self.received.put(m.received)
        elif kind == "radio_status":
            with self._status_cv:
                self._status = m.radio_status
                self._status_cv.notify_all()
        elif kind is not None:
            req = getattr(m, kind).req_id
            with self._rlock:
                q = self._replies.get(req)
            if q is not None:
                q.put(m)

    def _reconnect(self):
        with self._wlock:
            if self._sock is not None:
                self._sock.close()
            self._sock = None
        delay = 0.1
        while not self._closed:
            try:
                s = self._dial(0)
                break
            except OSError:
                time.sleep(delay)
                delay = min(delay * 2, 2.0)
        else:
            return
        with self._wlock:
            self._sock = s
        self.reconnects += 1
        for port, policy in list(self._claims.items()):
            req = next(self._ids)
            self._write(pb.ConsumerToRadiod(claim=pb.Claim(req_id=req, port=port, policy=POLICIES[policy])))
