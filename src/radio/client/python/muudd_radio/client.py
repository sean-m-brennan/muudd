# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""A consumer of muudd-radiod.

RadioClient connects to radiod's Unix socket, claims ports, sends frames and
receives them. A background thread reads the socket and matches replies to
requests by req_id. If radiod restarts, the client reconnects and claims its
ports again; a port the daemon refuses to give back is reported in
`lost_claims`.
"""
import collections
import itertools
import queue
import socket
import threading
import time

from google.protobuf.message import DecodeError

from . import radio_pb2 as pb
from .framing import FrameReader, encode

BROADCAST = 0xFFFFFFFF
POLICIES = {"radiod": pb.RADIOD_BRIDGED, "consumer": pb.CONSUMER_BRIDGED}

MAX_OUTSTANDING = 4096
MAX_RECEIVED = 4096
RECLAIM_TIMEOUT = 2.0

# Put on a request's queue when the connection drops, to wake its waiter.
_DISCONNECTED = object()


class RadioClient:
    def __init__(self, path, reconnect=True, connect_timeout=5.0, max_received=MAX_RECEIVED):
        self.path = path
        self.reconnect = reconnect
        self.reconnects = 0
        self.lost_claims = set()
        self.dropped_received = 0
        self._max_received = max_received
        self._received = collections.deque()
        self._received_cv = threading.Condition()
        self._wlock = threading.Lock()
        self._rlock = threading.Lock()
        self._ids = itertools.count(1)
        self._claims = {}
        self._reclaims = {}
        self._replies = {}
        self._sends = collections.OrderedDict()
        self._status = None
        self._status_cv = threading.Condition()
        self._closed = False
        self._sock = self._dial(connect_timeout)
        self._reader = threading.Thread(target=self._run, name="muudd-radio-client", daemon=True)
        self._reader.start()

    # ---- public API ----

    @property
    def status(self):
        """The last RadioStatus, or None while disconnected; after a reconnect
        it stays None until radiod sends a fresh one."""
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
            with self._rlock:
                self._claims[port] = policy
                self.lost_claims.discard(port)
        return result

    def release(self, port, timeout=5.0):
        with self._rlock:
            self._claims.pop(port, None)
            self.lost_claims.discard(port)
        req = next(self._ids)
        self._request(pb.ConsumerToRadiod(release=pb.Release(req_id=req, port=port)), req, timeout)

    def send(self, dest, port, payload, want_ack=False):
        req = next(self._ids)
        with self._rlock:
            self._replies[req] = queue.Queue()
            self._sends[req] = want_ack
            while len(self._sends) > MAX_OUTSTANDING:
                old, _ = self._sends.popitem(last=False)
                self._replies.pop(old, None)
        send = pb.Send(req_id=req, dest=dest, port=port, payload=payload, want_ack=want_ack)
        try:
            self._write(pb.ConsumerToRadiod(send=send))
        except ConnectionError:
            self._forget(req)
            raise
        return req

    def result(self, req_id, timeout=5.0):
        with self._rlock:
            q = self._replies.get(req_id)
        if q is None:
            raise KeyError(f"no outstanding request {req_id}")
        try:
            item = q.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError(f"no result for request {req_id}") from None
        if item is _DISCONNECTED:
            self._forget(req_id)
            raise ConnectionError("connection to radiod lost")
        r = item.send_result
        with self._rlock:
            if (r.status != pb.SENT or not self._sends.get(req_id)) and q.empty():
                self._replies.pop(req_id, None)
                self._sends.pop(req_id, None)
        return r

    def recv(self, timeout=5.0):
        deadline = time.monotonic() + timeout
        with self._received_cv:
            while not self._received:
                left = deadline - time.monotonic()
                if left <= 0:
                    raise TimeoutError("nothing received")
                self._received_cv.wait(left)
            return self._received.popleft()

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
        self._fail_waiters()
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
            try:
                self._sock.sendall(data)
            except OSError as e:
                raise ConnectionError(f"write to radiod failed: {e}") from e

    def _forget(self, req):
        with self._rlock:
            self._replies.pop(req, None)
            self._sends.pop(req, None)

    def _fail_waiters(self):
        with self._rlock:
            for q in self._replies.values():
                q.put(_DISCONNECTED)

    def _request(self, msg, req, timeout):
        q = queue.Queue()
        with self._rlock:
            self._replies[req] = q
        try:
            self._write(msg)
            item = q.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError(f"no reply to request {req}") from None
        finally:
            self._forget(req)
        if item is _DISCONNECTED:
            raise ConnectionError("connection to radiod lost")
        return item

    def _drop_socket(self):
        with self._wlock:
            if self._sock is not None:
                try:
                    self._sock.close()
                except OSError:
                    pass
            self._sock = None

    def _run(self):
        reader = FrameReader()
        while not self._closed:
            sock = self._sock
            try:
                data = sock.recv(65536) if sock is not None else b""
            except OSError:
                data = b""
            if data:
                try:
                    for frame in reader.feed(data):
                        self._dispatch(pb.RadiodToConsumer.FromString(frame))
                except (ValueError, DecodeError):
                    # The stream is corrupt: resynchronising is impossible.
                    data = b""
            if not data:
                self._drop_socket()
                with self._status_cv:
                    self._status = None
                self._fail_waiters()
                if self._closed or not self.reconnect:
                    return
                reader = self._reconnect()
                if reader is None:
                    return

    def _dispatch(self, m):
        kind = m.WhichOneof("msg")
        if kind == "received":
            with self._received_cv:
                self._received.append(m.received)
                while len(self._received) > self._max_received:
                    self._received.popleft()
                    self.dropped_received += 1
                self._received_cv.notify()
        elif kind == "radio_status":
            with self._status_cv:
                self._status = m.radio_status
                self._status_cv.notify_all()
        elif kind is not None:
            req = getattr(getattr(m, kind), "req_id", None)
            if req is None:
                return
            if kind == "claim_result" and req in self._reclaims:
                self._reclaimed(req, m.claim_result)
                return
            with self._rlock:
                q = self._replies.get(req)
            if q is not None:
                q.put(m)

    def _reclaimed(self, req, result):
        port = self._reclaims.pop(req)
        if result.status != pb.CLAIM_OK:
            with self._rlock:
                self._claims.pop(port, None)
                self.lost_claims.add(port)

    def _reconnect(self):
        """Dial again with backoff and re-claim held ports.

        Returns the FrameReader for the new connection, or None if the client
        was closed meanwhile.
        """
        delay = 0.1
        while not self._closed:
            try:
                s = self._dial(0)
            except OSError:
                time.sleep(delay)
                delay = min(delay * 2, 2.0)
                continue
            with self._wlock:
                if self._closed:
                    s.close()
                    return None
                self._sock = s
            self.reconnects += 1
            try:
                return self._reclaim(s)
            except (OSError, ValueError, DecodeError):
                self._drop_socket()
                with self._status_cv:
                    self._status = None
                self._fail_waiters()
                time.sleep(delay)
                delay = min(delay * 2, 2.0)
        return None

    def _reclaim(self, sock):
        """Claim every held port again and await the answers.

        Other messages that arrive meanwhile are dispatched normally. Raises on
        a broken connection.
        """
        with self._rlock:
            held = list(self._claims.items())
        self._reclaims = {}
        for port, policy in held:
            req = next(self._ids)
            self._reclaims[req] = port
            self._write(pb.ConsumerToRadiod(claim=pb.Claim(req_id=req, port=port, policy=POLICIES[policy])))
        reader = FrameReader()
        deadline = time.monotonic() + RECLAIM_TIMEOUT
        try:
            while self._reclaims and not self._closed:
                left = deadline - time.monotonic()
                if left <= 0:
                    break
                sock.settimeout(left)
                try:
                    data = sock.recv(65536)
                except TimeoutError:
                    break
                if not data:
                    raise ConnectionError("radiod closed the connection")
                for frame in reader.feed(data):
                    self._dispatch(pb.RadiodToConsumer.FromString(frame))
        finally:
            self._reclaims = {}
            try:
                sock.settimeout(None)
            except OSError:
                pass
        return reader
