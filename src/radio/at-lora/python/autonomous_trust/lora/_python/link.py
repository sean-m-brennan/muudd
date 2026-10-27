# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The radiod link: one thread owns a muudd_radio RadioClient; the network
process only touches locked queues. Twin of C lora_link.

Rules shared with the C link:
- No radiod client call (claim, send, result, recv) runs while the link's lock
  is held; log lines are emitted after it is released.
- A port MTU below the 64-byte floor is warned about once per connection and
  keeps the link down.
- A refused claim logs one error per connection and is retried only after the
  client reconnects.
- Every connection logs one 'connected' line and claims the port once, and
  every loss of one logs one 'lost' line and fails the queued jobs, even when
  the client has already dialled again by the time the link looks. All link
  state, the MTU included, starts over on each connection.
- When the connection is lost, the job being sent reports how many fragments
  radiod had already confirmed; the jobs queued behind it report 0.
- One write to radiod waits at most SEND_TIMEOUT_MS for room in the socket;
  past that the connection counts as lost, which also bounds how long close()
  waits for a link thread caught in a send.
- Results queue up to JOBS_MAX * 2 while nobody polls; a result finished while
  the queue is full is dropped (the newest goes) and counted in
  results_dropped.
"""
import collections
import threading
from dataclasses import dataclass, replace

from . import frame as fr

BROADCAST = 0xFFFFFFFF


@dataclass(frozen=True)
class LinkState:
    connected: bool = False
    claimed: bool = False
    radio_up: bool = False
    driver: bool = False
    node_addr: int = 0
    mtu: int = 0

    @property
    def up(self) -> bool:
        return (self.connected and self.claimed and self.radio_up and self.driver
                and self.node_addr != 0 and self.mtu >= fr.MIN_MTU)


class RadioLink:
    RX_MAX = 64
    JOBS_MAX = 16
    RESULT_WAIT = 2.0
    SEND_TIMEOUT_MS = 500

    def __init__(self, socket_path, port, logger):
        self.path, self.port, self.logger = socket_path, port, logger
        self._lock = threading.Lock()
        self._state = LinkState()
        self._jobs = collections.deque()       # (dest, frags, want_ack); head is being sent
        self._rx = collections.deque()
        self._results = []
        self.rx_dropped = 0
        self.results_dropped = 0
        # Test seam: while true the link thread sends nothing, so queued jobs stay queued.
        self._pause_sending = False
        self._stop = threading.Event()
        self._client = None
        self._thread = threading.Thread(target=self._run, name='lora-link', daemon=True)
        self._thread.start()

    # ---- the network process's side ----

    def state(self) -> LinkState:
        with self._lock:
            return self._state

    def submit(self, dest, frags, want_ack) -> bool:
        with self._lock:
            if not self._state.up or len(self._jobs) >= self.JOBS_MAX or not frags:
                return False
            self._jobs.append((dest, list(frags), want_ack))
            return True

    def poll_rx(self) -> list:
        with self._lock:
            out, self._rx = list(self._rx), collections.deque()
            return out

    def poll_results(self) -> list:
        with self._lock:
            out, self._results = self._results, []
            return out

    def close(self) -> None:
        self._stop.set()
        c = self._client
        if c is not None:
            c.close()          # wakes a send/claim the link thread is waiting on
        self._thread.join(timeout=5)

    # ---- the link thread ----

    def _set(self, **kw):
        with self._lock:
            self._state = replace(self._state, **kw)

    def _result(self, ok, reason, sent, total):
        """Queue one finished job's result (caller holds the lock)."""
        if len(self._results) < self.JOBS_MAX * 2:
            self._results.append((ok, reason, sent, total))
        else:
            self.results_dropped += 1       # nobody is polling; the newest result goes

    def _finish_head(self, ok, reason, sent):
        with self._lock:
            if self._jobs:
                _, frags, _ = self._jobs.popleft()
                self._result(ok, reason, sent, len(frags))

    def _fail_all(self, reason, head_sent=0):
        """Fail every queued job; the head reports head_sent confirmed fragments."""
        with self._lock:
            while self._jobs:
                _, frags, _ = self._jobs.popleft()
                self._result(False, reason, head_sent, len(frags))
                head_sent = 0

    def _check_mtu(self, mtu):
        """Warn once per connection about a port MTU below the floor."""
        if mtu and mtu < fr.MIN_MTU and not self._mtu_warned:
            self._mtu_warned = True
            self.logger.warning('lora: radiod port %d has MTU %d, below %d; LoRa stays off',
                                self.port, mtu, fr.MIN_MTU)

    def _claim(self, pb):
        res = self._client.claim(self.port, 'consumer', timeout=2.0)
        if res.status == pb.CLAIM_OK:
            self._held = True
            self._set(claimed=True, mtu=res.mtu)
            self._check_mtu(res.mtu)
        else:
            self._held = False
            self._set(claimed=False)
            if not self._refused_logged:
                self._refused_logged = True
                self.logger.error('lora: radiod refused the claim (%s) of port %d; give it a '
                                  'ports entry with policy consumer and no ip_fallback',
                                  pb.ClaimStatus.Name(res.status), self.port)

    def _try_claim(self, pb):
        try:
            self._claim(pb)
        except (ConnectionError, TimeoutError):
            pass

    def _run(self):
        from muudd_radio import radio_pb2 as pb
        self._refused_logged = self._mtu_warned = False
        self._held = False                      # radiod has granted us the port
        # the client's reconnects count for the connection in use; None while there is none
        self._conn_gen = None
        self._backoff = 0.1
        crash_logged = False
        while not self._stop.is_set():
            try:
                self._iterate(pb)
            except Exception:                   # the thread must not die with up=True frozen
                if not crash_logged:
                    crash_logged = True
                    self.logger.exception('lora: unexpected error in the radiod link; resetting')
                self._set(connected=False, claimed=False, radio_up=False, driver=False,
                          node_addr=0)
                self._fail_all('RADIO_DOWN')
                self._stop.wait(0.1)
        if self._client is not None:
            self._client.close()
        self._fail_all('RADIO_DOWN')

    def _iterate(self, pb):
        if self._client is None:
            from muudd_radio.client import RadioClient
            try:
                client = RadioClient(self.path, reconnect=True, connect_timeout=0.2)
            except OSError:                     # socket absent or refused
                self._stop.wait(self._backoff)
                self._backoff = min(self._backoff * 2, 2.0)
                return
            client.set_send_timeout(self.SEND_TIMEOUT_MS)
            self._client = client
            if self._stop.is_set():
                return
            self._backoff = 0.1
        c = self._client
        # connected before reconnects: the client counts a reconnect before it
        # shows itself connected again, so a live connection is never paired
        # with the previous connection's count
        alive = c.connected
        gen = c.reconnects
        if self._conn_gen is not None and (not alive or gen != self._conn_gen):
            # the connection is gone, though the client may already have another
            self._conn_gen = None
            self.logger.warning('lora: lost the connection to radiod at %s (port %d)',
                                self.path, self.port)
            self._fail_all('RADIO_DOWN')
            with self._lock:
                self._state = LinkState()
        if self._conn_gen is None:
            if not alive:
                self._stop.wait(0.05)
                return
            self._conn_gen = gen                # a new connection: warnings and claim start over
            self._refused_logged = self._mtu_warned = self._held = False
            with self._lock:
                self._state = LinkState(connected=True)
            self.logger.info('lora: connected to radiod at %s (port %d)', self.path, self.port)
            self._try_claim(pb)                 # once per connection: a refusal is retried
                                                # only after a reconnect
        status = c.status
        if status is not None:
            if self._held and self.port in c.lost_claims:
                self._held = False              # radiod refused the client's own re-claim
            caps = status.caps if status.HasField('caps') else None
            mtu = next((m.mtu for m in status.mtus if m.port == self.port), self.state().mtu)
            self._check_mtu(mtu)
            self._set(connected=True, radio_up=status.state == pb.STATE_UP,
                      driver=caps is not None and caps.node_addr != 0,
                      node_addr=caps.node_addr if caps is not None else 0, mtu=mtu,
                      claimed=self._held)
        # Drain received frames: the first wait is the loop's pacing, the rest never block,
        # and the batch is bounded so status, loss detection and sending are not starved.
        wait = 0.05
        for _ in range(self.RX_MAX):
            try:
                r = c.recv(timeout=wait)
            except TimeoutError:
                break
            wait = 0
            if r.port != self.port:
                continue
            with self._lock:
                if len(self._rx) >= self.RX_MAX:
                    self._rx.popleft()
                    self.rx_dropped += 1
                self._rx.append((r.src, bytes(r.payload), int(r.meta.via)))
        if self._pause_sending:
            return
        with self._lock:
            head = self._jobs[0] if self._jobs else None
            up = self._state.up
        if head is None:
            return
        if not up:
            self._fail_all('RADIO_DOWN')
            return
        self._send_job(c, pb, head)

    def _send_job(self, c, pb, head):
        dest, frags, want_ack = head
        for i, frag in enumerate(frags):
            if self._stop.is_set():
                self._finish_head(False, 'RADIO_DOWN', i)
                return
            try:
                req = c.send(dest, self.port, frag, want_ack=want_ack)
                res = c.result(req, timeout=self.RESULT_WAIT)
            except ConnectionError:
                self._finish_head(False, 'RADIO_DOWN', i)     # i fragments were confirmed
                return
            except TimeoutError:
                self._finish_head(False, 'TIMEOUT', i)
                return
            if res.status == pb.REFUSED:
                self._finish_head(False, pb.RefuseReason.Name(res.reason), i)
                return
        self._finish_head(True, '', len(frags))
