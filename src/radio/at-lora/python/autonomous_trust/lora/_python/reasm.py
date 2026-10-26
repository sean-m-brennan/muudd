# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Reassembly with dedupe and NACK timers, and the sender's hold for
answering one NACK. Twin of C lora_reasm; both follow the same rules so the
conformance traces agree step for step."""
from . import frame as fr

REASM_MAX = 16
REASM_PER_SRC = 4
DEDUPE_MAX = 256
DEDUPE_S = 600.0
HOLD_MAX = 16


class _Entry:
    __slots__ = ('src', 'msg_id', 'type', 'count', 'tag', 'have', 'nacked',
                 'first', 'last', 'seq', 'frags')

    def __init__(self, src, f, now, seq):
        self.src, self.msg_id, self.type, self.count, self.tag = \
            src, f.msg_id, f.type, f.count, f.tag
        self.have, self.nacked, self.first, self.last, self.seq = 0, False, now, now, seq
        self.frags = {}


class Reassembler:
    def __init__(self, nack_gap_s: float, reassembly_s: float):
        self.nack_gap_s, self.reassembly_s = nack_gap_s, reassembly_s
        self._entries = {}       # (src, msg_id) -> _Entry
        self._seen = []          # [(src, msg_id, t)], oldest first
        self._seq = 0
        self.stats = {'timeouts': 0, 'evicted': 0, 'duplicate': 0, 'bad': 0}

    def _forget_old(self, now):
        self._seen = [s for s in self._seen if now - s[2] < DEDUPE_S]

    def _remember(self, key, now):
        if len(self._seen) == DEDUPE_MAX:
            self._seen.pop(0)
        self._seen.append((key[0], key[1], now))

    def _drop(self, key, now):
        del self._entries[key]
        self._remember(key, now)

    def _oldest(self, src=None):
        cands = [e for e in self._entries.values() if src is None or e.src == src]
        e = min(cands, key=lambda e: (e.first, e.seq))
        return (e.src, e.msg_id)

    def add(self, src: int, f: 'fr.Frame', now: float):
        self._forget_old(now)
        key = (src, f.msg_id)
        if any(s[0] == src and s[1] == f.msg_id for s in self._seen):
            self.stats['duplicate'] += 1
            return 'duplicate', None
        e = self._entries.get(key)
        if e is None:
            if sum(1 for x in self._entries.values() if x.src == src) >= REASM_PER_SRC:
                self._drop(self._oldest(src), now)
                self.stats['evicted'] += 1
            if len(self._entries) >= REASM_MAX:
                self._drop(self._oldest(), now)
                self.stats['evicted'] += 1
            e = self._entries[key] = _Entry(src, f, now, self._seq)
            self._seq += 1
        elif e.type != f.type or e.count != f.count or \
                (f.type == fr.GROUP and e.tag != f.tag):
            self._drop(key, now)
            self.stats['bad'] += 1
            return 'bad', None
        bit = 1 << f.idx
        if e.have & bit:
            return 'partial', None
        e.frags[f.idx] = f.payload
        e.have |= bit
        e.last = now
        if e.have != (1 << e.count) - 1:
            return 'partial', None
        msg = b''.join(e.frags[i] for i in range(e.count))
        self._drop(key, now)
        return 'complete', msg

    def tick(self, now: float) -> list:
        self._forget_old(now)
        nacks = []
        for key, e in sorted(self._entries.items(), key=lambda kv: kv[1].seq):
            if now - e.first >= self.reassembly_s:
                self._drop(key, now)
                self.stats['timeouts'] += 1
            elif e.type == fr.PEER and not e.nacked and now - e.last >= self.nack_gap_s:
                nacks.append((e.src, e.msg_id, ((1 << e.count) - 1) & ~e.have))
                e.nacked = True
        return nacks


class SendHold:
    def __init__(self, hold_s: float):
        self.hold_s = hold_s
        self._held = []          # [dict(dest, msg_id, put, served, frags)], oldest first
        self.served = 0

    def put(self, dest: int, msg_id: int, frags: list, now: float) -> None:
        if len(self._held) == HOLD_MAX:
            self._held.pop(0)
        self._held.append({'dest': dest, 'msg_id': msg_id, 'put': now,
                           'served': False, 'frags': list(frags)})

    def serve(self, dest: int, msg_id: int, missing: int, now: float) -> list:
        for h in reversed(self._held):    # the newest held message answers
            if h['dest'] != dest or h['msg_id'] != msg_id:
                continue
            if h['served'] or now - h['put'] >= self.hold_s:
                return []
            h['served'] = True
            self.served += 1
            return [f for i, f in enumerate(h['frags']) if missing & (1 << i)]
        return []
