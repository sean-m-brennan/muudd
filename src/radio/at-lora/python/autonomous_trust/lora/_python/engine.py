# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The at-lora engine: decides what also goes by LoRa, frames and sends it,
reassembles what arrives, answers and sends NACKs, and keeps the address map.
It knows nothing of AT's processes: the glue hands it an environment
(roster, transport quiet time, group format and tag, clocks). Twin of C
lora_engine."""
import collections
import logging
import random

from . import frame as fr
from . import policy
from .addrmap import AddrMap
from .link import BROADCAST
from .reasm import Reassembler, SendHold

MEMBER_ANNOUNCE_S = 60.0
SUMMARY_S = 300.0
WARN_EVERY_S = 60.0
_CHANNEL = {fr.PEER: 'peer', fr.GROUP: 'group', fr.BCAST: 'broadcast'}
# Why an arriving frame was dropped: its counter (lora.rx.<key>) -> the log
# text and level. Duplicates are expected (NACK resends, multipath), so they
# log at DEBUG; the rest WARN.
_DROPS = {'bad_frame': ('a malformed frame', logging.WARNING),
          'unknown_src': ('a frame from an unmapped address', logging.WARNING),
          'conflict': ('a frame from a conflicted address', logging.WARNING),
          'foreign_tag': ("a frame with another group's tag", logging.WARNING),
          'duplicate': ('a duplicate frame', logging.DEBUG)}


class Engine:
    def __init__(self, cfg, link, env, addrmap_path, logger):
        self.cfg, self.link, self.env, self.path, self.logger = cfg, link, env, addrmap_path, logger
        self.counters = collections.Counter()
        self.addrmap, self._self_issued, err = AddrMap.load(addrmap_path)
        if err:
            logger.warning('lora: %s unreadable (%s); starting with an empty address map',
                           addrmap_path, err)
        elif self.addrmap.skipped:
            logger.warning('lora: %s: skipped %d bad entr%s (%s); kept the rest',
                           addrmap_path, len(self.addrmap.skipped),
                           'y' if len(self.addrmap.skipped) == 1 else 'ies',
                           ', '.join(str(u) for u in self.addrmap.skipped))
        self.reasm = Reassembler(cfg.nack_gap_s, cfg.reassembly_s)
        self.hold = SendHold(cfg.nack_hold_s)
        self._msg_id = random.randrange(0x10000)
        self._announced_addr = None
        self._last_announce = float('-inf')
        self._announced_to = set()
        self._was_down = False
        self._json_logged = False
        self._full_warned_at = float('-inf')
        self._drop_warned_at = dict.fromkeys(_DROPS, float('-inf'))
        self._last_summary = env.now()

    # ---- helpers ----

    def _next_id(self):
        mid = self._msg_id
        self._msg_id = (self._msg_id + 1) & 0xFFFF
        return mid

    def _save(self):
        try:
            self.addrmap.save(self.path, self._self_issued)
        except OSError as err:
            self.logger.error('lora: could not write %s: %s', self.path, err)

    def _proto(self):
        ok = self.env.group_proto()
        if not ok and not self._json_logged:
            self._json_logged = True
            self.logger.info('lora: this cohort uses the JSON wire format; LoRa carries only '
                             'proto-format cohorts, so nothing goes on the air')
        return ok

    def _members(self):
        me = self.env.my_uuid()
        return [u for u in self.env.members() if u != me]

    def _quiet_set(self):
        return [u for u in self._members()
                if self.addrmap.addr_of(u) is not None
                and self.env.quiet_for(u) >= self.cfg.ip_quiet_s]

    def _drop(self, why, src, now):
        """Count a frame from ``src`` dropped for ``why``, and log it unless
        that reason was logged in the last WARN_EVERY_S."""
        name = 'lora.rx.' + why
        self.counters[name] += 1
        if now - self._drop_warned_at[why] < WARN_EVERY_S:
            return
        self._drop_warned_at[why] = now
        what, level = _DROPS[why]
        self.logger.log(level, 'lora: dropped %s from 0x%x (counted in %s; logged at most once '
                        'a minute)', what, src, name)

    def _submit(self, dest, frags, want_ack):
        if not self.link.submit(dest, frags, want_ack):
            self.counters['lora.tx.abandoned'] += 1
            return False
        self.counters['lora.tx.msgs'] += 1
        self.counters['lora.tx.frames'] += len(frags)
        return True

    # ---- sends the core makes ----

    def on_unicast(self, peer_uuid, process, function, frame) -> bool:
        st = self.link.state()
        mtu = st.mtu if st.up else 0
        uuid = str(peer_uuid).lower()
        addr = self.addrmap.addr_of(uuid)
        if not policy.unicast(st.up, addr is not None, self.env.quiet_for(uuid),
                              self.cfg.ip_quiet_s, self.cfg.allows(process, function),
                              self._proto(), len(frame),
                              fr.max_message(fr.PEER, mtu, self.cfg.max_fragments)):
            return False
        msg_id = self._next_id()
        frags = fr.fragment(frame, fr.PEER, msg_id, mtu, self.cfg.max_fragments)
        if not self._submit(addr, frags, True):
            return False
        self.hold.put(addr, msg_id, frags, self.env.now())
        return True

    def on_fanout(self, channel, process, function, frame) -> bool:
        st = self.link.state()
        mtu = st.mtu if st.up else 0
        type_ = fr.GROUP if channel == 'group' else fr.BCAST
        tag = self.env.group_tag()
        if type_ == fr.GROUP and tag is None:
            return False
        proto = self._proto() if type_ == fr.GROUP else True
        if not policy.fanout(channel, st.up, len(self._quiet_set()),
                             self.cfg.allows(process, function), proto, len(frame),
                             fr.max_message(type_, mtu, self.cfg.max_fragments)):
            return False
        frags = fr.fragment(frame, type_, self._next_id(), mtu, self.cfg.max_fragments,
                            tag if type_ == fr.GROUP else None)
        return self._submit(BROADCAST, frags, False)

    # ---- the address map ----

    def on_announce(self, sender_uuid, obj, verified) -> str:
        """A member's announcement. ``verified``: its envelope's signature
        checked out against the key held for the sender; one that did not is
        refused, since anyone holding the group key could forge it."""
        uuid = str(sender_uuid).lower()
        if uuid == self.env.my_uuid():
            return 'self'
        if not verified:
            self.counters['lora.announce.unverified'] += 1
            return 'refused'
        members = set(self._members())
        if uuid not in members:
            return 'refused'
        if not isinstance(obj, dict) or type(obj.get('lora_addr')) is not int \
                or type(obj.get('issued_ms')) is not int:
            return 'bad'
        res = self.addrmap.announce(uuid, obj['lora_addr'], obj['issued_ms'],
                                    is_member=lambda u: u in members)
        if res == 'full':
            res = self._make_room(uuid, obj, members)
        if res == 'accepted':
            self.counters['lora.announce.accepted'] += 1
            self._save()
            if self.addrmap.addr_conflicted(obj['lora_addr']):
                others = [e['uuid'] for e in self.addrmap.snapshot()
                          if e['addr'] == obj['lora_addr'] and e['uuid'] != uuid]
                self.logger.warning('lora: %s and %s both claim LoRa address %#x; LoRa to and '
                                    'from either is off until one announces another',
                                    uuid, ', '.join(others), obj['lora_addr'])
        elif res == 'stale':
            self.counters['lora.announce.stale'] += 1
        return res

    def _make_room(self, uuid, obj, members) -> str:
        """The map is full: drop the entry off the roster with the oldest
        issued_ms (the lowest uuid on a tie) and try once more. With every
        entry a member, count it and WARN at most once a minute."""
        gone = [e for e in self.addrmap.snapshot() if e['uuid'] not in members]
        if gone:
            oldest = min(gone, key=lambda e: (e['issued_ms'], e['uuid']))
            self.addrmap.remove(oldest['uuid'])
            return self.addrmap.announce(uuid, obj['lora_addr'], obj['issued_ms'],
                                         is_member=lambda u: u in members)
        self.counters['lora.announce.full'] += 1
        now = self.env.now()
        if now - self._full_warned_at >= WARN_EVERY_S:
            self._full_warned_at = now
            self.logger.warning('lora: the address map is full (%d entries, all members); '
                                '%s is not mapped', AddrMap.MAX, uuid)
        return 'full'

    def on_exclusion(self, uuid, excluded) -> None:
        if excluded and self.addrmap.remove(uuid):
            self._save()

    def announce_due(self):
        """The announcement to send now, or None. Changes no announcement
        state: the caller reports a successful send with announce_sent."""
        st = self.link.state()
        if not st.up:
            self._was_down = True
            return None
        if not st.node_addr or self.env.group_tag() is None:
            return None
        now = self.env.now()
        members = set(self._members())
        due = (st.node_addr != self._announced_addr
               or self._was_down
               or now - self._last_announce >= self.cfg.announce_s
               or (members - self._announced_to and now - self._last_announce >= MEMBER_ANNOUNCE_S))
        if not due:
            return None
        return {'lora_addr': st.node_addr,
                'issued_ms': max(self.env.wall_ms(), self._self_issued + 1)}

    def announce_sent(self, payload) -> None:
        """The announcement from announce_due went out: remember it."""
        self._self_issued = payload['issued_ms']
        self._announced_addr = payload['lora_addr']
        self._last_announce = self.env.now()
        self._announced_to = set(self._members())
        self._was_down = False
        self._save()
        self.counters['lora.announce.sent'] += 1

    # ---- what arrives ----

    def poll(self) -> list:
        now = self.env.now()
        for ok, reason, _sent, _total in self.link.poll_results():
            if not ok:
                self.counters['lora.tx.refused.%s' % reason] += 1
        # results the link had no room to queue: refusals the counters above miss
        self.counters['lora.tx.result_dropped'] = self.link.results_dropped
        out = []
        for src, payload, via in self.link.poll_rx():
            self.counters['lora.rx.frames'] += 1
            if via != 0:
                self._drop('bad_frame', src, now)
                continue
            try:
                f = fr.decode(payload)
            except fr.FrameError:
                self._drop('bad_frame', src, now)
                continue
            uuid = self.addrmap.uuid_of(src)
            if uuid is None:
                self._drop('conflict' if self.addrmap.addr_conflicted(src) else 'unknown_src',
                           src, now)
                continue
            if f.type == fr.NACK:
                frames = self.hold.serve(src, f.msg_id, f.missing, now)
                if frames:
                    if self.link.submit(src, frames, True):
                        self.counters['lora.nack.served'] += 1
                        self.counters['lora.tx.frames'] += len(frames)
                    else:
                        self.counters['lora.tx.abandoned'] += 1
                continue
            if f.type == fr.GROUP and f.tag != self.env.group_tag():
                self._drop('foreign_tag', src, now)
                continue
            status, msg = self.reasm.add(src, f, now)
            if status == 'duplicate':
                self._drop('duplicate', src, now)
            elif status == 'bad':
                self._drop('bad_frame', src, now)
            elif status == 'complete':
                self.counters['lora.rx.delivered'] += 1
                out.append((_CHANNEL[f.type], uuid, msg))
        for src, msg_id, missing in self.reasm.tick(now):
            if self.link.submit(src, [fr.encode_nack(msg_id, missing)], False):
                self.counters['lora.nack.sent'] += 1
                self.counters['lora.tx.frames'] += 1
            else:
                self.counters['lora.tx.abandoned'] += 1
        self.counters['lora.rx.reasm_timeout'] = self.reasm.stats['timeouts']
        self.counters['lora.rx.evicted'] = self.reasm.stats['evicted']
        return out

    def stats(self) -> dict:
        st = self.link.state()
        return {'counters': dict(self.counters),
                'link': {'up': st.up, 'connected': st.connected, 'claimed': st.claimed,
                         'node_addr': st.node_addr, 'mtu': st.mtu},
                'addrmap': self.addrmap.snapshot()}

    def summary_due(self):
        now = self.env.now()
        if now - self._last_summary < SUMMARY_S:
            return None
        self._last_summary = now
        c = self.counters
        return ('lora: link %s; sent %d msgs (%d frames, %d refused, %d abandoned); '
                'delivered %d; nacks sent %d served %d; %d peers mapped'
                % ('up' if self.link.state().up else 'down', c['lora.tx.msgs'],
                   c['lora.tx.frames'],
                   sum(v for k, v in c.items() if k.startswith('lora.tx.refused.')),
                   c['lora.tx.abandoned'], c['lora.rx.delivered'], c['lora.nack.sent'],
                   c['lora.nack.served'], len(self.addrmap.snapshot())))
