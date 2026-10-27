# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import logging

import pytest

from autonomous_trust.lora._python import config, frame as fr
from autonomous_trust.lora._python.addrmap import AddrMap
from autonomous_trust.lora._python.engine import Engine
from autonomous_trust.lora._python.link import BROADCAST, LinkState

ME = '00000000-0000-4000-8000-000000000000'
A = 'aaaaaaaa-0000-4000-8000-000000000001'
B = 'bbbbbbbb-0000-4000-8000-000000000002'
TAG = bytes.fromhex('a1b2c3d4')


class FakeLink:
    def __init__(self):
        self.st = LinkState(True, True, True, True, 0xA01, 233)
        self.sent, self.rx, self.results, self.accept = [], [], [], True
        self.results_dropped = 0

    def state(self):
        return self.st

    def submit(self, dest, frags, want_ack):
        if not self.accept or not self.st.up:
            return False
        self.sent.append((dest, list(frags), want_ack))
        return True

    def poll_rx(self):
        out, self.rx = self.rx, []
        return out

    def poll_results(self):
        out, self.results = self.results, []
        return out


class FakeEnv:
    def __init__(self):
        self.t, self.wall, self.quiet = 1000.0, 1_700_000_000_000, {A: 500.0, B: 0.0}
        self.roster, self.proto, self.tag = [A, B], True, TAG

    def my_uuid(self): return ME
    def members(self): return list(self.roster)
    def quiet_for(self, u): return self.quiet.get(u, 0.0)
    def group_proto(self): return self.proto
    def group_tag(self): return self.tag
    def wall_ms(self): return self.wall
    def now(self): return self.t


@pytest.fixture
def eng(tmp_path):
    link, env = FakeLink(), FakeEnv()
    cfg = config.parse('{"port": 10, "allow": ["identity/group_key_update", "identity/presence"]}')
    e = Engine(cfg, link, env, str(tmp_path / 'lora_peers.json'), logging.getLogger('t'))
    assert e.on_announce(A, {'lora_addr': 0xB02, 'issued_ms': 5}, True) == 'accepted'
    assert e.on_announce(B, {'lora_addr': 0xC03, 'issued_ms': 5}, True) == 'accepted'
    return e, link, env


def test_unicast_to_a_quiet_peer_goes_by_lora_and_is_held(eng):
    e, link, env = eng
    assert e.on_unicast(A, 'identity', 'group_key_update', b'sealed' * 100)
    dest, frags, ack = link.sent[-1]
    assert dest == 0xB02 and ack and len(frags) == 3
    assert e.counters['lora.tx.msgs'] == 1 and e.counters['lora.tx.frames'] == 3
    msg_id = fr.decode(frags[0]).msg_id
    link.rx.append((0xB02, fr.encode_nack(msg_id, 0b010), 0))
    e.poll()
    assert link.sent[-1] == (0xB02, [frags[1]], True)
    assert e.counters['lora.nack.served'] == 1
    assert e.counters['lora.tx.frames'] == 4      # 3 sent + 1 resent


def test_unicast_is_skipped_when_any_condition_fails(eng):
    e, link, env = eng
    assert not e.on_unicast(B, 'identity', 'group_key_update', b'x')       # not quiet
    assert not e.on_unicast(A, 'identity', 'peer_dm', b'x')                # not allowed
    env.proto = False
    assert not e.on_unicast(A, 'identity', 'group_key_update', b'x')       # JSON group
    env.proto = True
    link.st = LinkState()
    assert not e.on_unicast(A, 'identity', 'group_key_update', b'x')       # link down
    assert link.sent == []


def test_job_queue_full_counts_abandoned(eng):
    e, link, env = eng
    link.accept = False
    assert not e.on_unicast(A, 'identity', 'group_key_update', b'x')
    assert e.counters['lora.tx.abandoned'] == 1


def test_fanout_needs_a_quiet_member(eng):
    e, link, env = eng
    assert e.on_fanout('group', 'identity', 'presence', b'g' * 10)
    dest, frags, ack = link.sent[-1]
    assert dest == BROADCAST and not ack and fr.decode(frags[0]).tag == TAG
    env.quiet[A] = 0.0
    assert not e.on_fanout('group', 'identity', 'presence', b'g')
    env.quiet[A] = 500.0
    env.proto = False
    assert not e.on_fanout('group', 'identity', 'presence', b'g')        # JSON group
    assert e.on_fanout('broadcast', 'identity', 'presence', b'p')        # broadcast ignores format
    assert not e.on_fanout('broadcast', 'identity', 'peer_dm', b'p')     # not allowed


def test_received_unicast_is_delivered_on_the_peer_channel(eng):
    e, link, env = eng
    frames = fr.fragment(b'sealed', fr.PEER, 5, 233, 8)
    link.rx.append((0xB02, frames[0], 0))
    assert e.poll() == [('peer', A, b'sealed')]


def test_received_group_frame_is_delivered_once(eng):
    e, link, env = eng
    frames = fr.fragment(b'hello', fr.GROUP, 77, 233, 8, TAG)
    link.rx += [(0xB02, frames[0], 0), (0xB02, frames[0], 0)]
    assert e.poll() == [('group', A, b'hello')]
    assert e.counters['lora.rx.delivered'] == 1 and e.counters['lora.rx.duplicate'] == 1


def test_receive_side_refusals_are_counted(eng):
    e, link, env = eng
    good = fr.fragment(b'x', fr.PEER, 1, 233, 8)[0]
    link.rx += [(0xD04, good, 0),                                   # unknown source
                (0xB02, b'\x99', 0),                                # bad frame
                (0xB02, good, 1),                                   # not via the radio
                (0xB02, fr.fragment(b'x', fr.GROUP, 2, 233, 8, b'zzzz')[0], 0)]   # foreign tag
    assert e.poll() == []
    c = e.counters
    assert (c['lora.rx.unknown_src'], c['lora.rx.bad_frame'], c['lora.rx.foreign_tag']) == (1, 2, 1)


def test_dropped_frames_warn_once_a_minute_per_reason(eng, caplog):
    """A dropped frame is logged as well as counted: one line per reason, at
    most once a minute; WARN, except a duplicate, which is expected and logs
    at DEBUG."""
    e, link, env = eng
    good = fr.fragment(b'x', fr.PEER, 1, 233, 8)[0]
    with caplog.at_level(logging.WARNING, logger='t'):
        link.rx += [(0xB02, b'\x99', 0), (0xB02, b'\x99', 0), (0xD04, good, 0)]
        assert e.poll() == []
        assert caplog.messages.count('lora: dropped a malformed frame from 0xb02 (counted in '
                                     'lora.rx.bad_frame; logged at most once a minute)') == 1
        assert sum('dropped a frame from an unmapped address from 0xd04' in m
                   for m in caplog.messages) == 1
        env.t += 30
        link.rx.append((0xB02, b'\x99', 0))
        e.poll()
        assert sum('malformed' in m for m in caplog.messages) == 1
        env.t += 31
        link.rx.append((0xB02, b'\x99', 0))
        e.poll()
        assert sum('malformed' in m for m in caplog.messages) == 2
        assert sum('unmapped' in m for m in caplog.messages) == 1
    assert e.counters['lora.rx.bad_frame'] == 4
    caplog.clear()
    with caplog.at_level(logging.DEBUG, logger='t'):
        link.rx += [(0xB02, good, 0)] * 3
        assert len(e.poll()) == 1
    assert e.counters['lora.rx.duplicate'] == 2
    dups = [r for r in caplog.records if 'duplicate' in r.getMessage()]
    assert [(r.levelno, r.getMessage()) for r in dups] == [
        (logging.DEBUG, 'lora: dropped a duplicate frame from 0xb02 (counted in '
                        'lora.rx.duplicate; logged at most once a minute)')]


def test_nack_is_sent_for_a_stalled_unicast(eng):
    e, link, env = eng
    frames = fr.fragment(bytes(500), fr.PEER, 9, 233, 8)
    link.rx.append((0xB02, frames[0], 0))
    e.poll()
    env.t += 20
    e.poll()
    dest, frags, ack = link.sent[-1]
    assert dest == 0xB02 and not ack and fr.decode(frags[0]).missing == 0b110


def _announce(e):
    p = e.announce_due()
    if p is not None:
        e.announce_sent(p)
    return p


def test_announce_due_rules(eng):
    e, link, env = eng
    first = e.announce_due()
    assert first == {'lora_addr': 0xA01, 'issued_ms': env.wall}
    assert e.announce_due() == first             # no side effects until sent
    e.announce_sent(first)
    assert e.announce_due() is None
    env.roster.append('cccccccc-0000-4000-8000-000000000003')
    env.t += 30
    assert e.announce_due() is None              # within 60 s of the last
    env.t += 31
    assert _announce(e) is not None              # a new member, 61 s later
    env.t += 600
    assert _announce(e) is not None              # announce_s
    link.st = LinkState(True, True, True, True, 0xA09, 233)
    assert _announce(e)['lora_addr'] == 0xA09    # address changed
    assert e.counters['lora.announce.sent'] == 4


def test_announce_not_due_before_the_group_is_known(eng):
    e, link, env = eng
    env.tag = None
    assert e.announce_due() is None
    env.tag = TAG
    assert e.announce_due() is not None


def test_announce_due_again_after_link_down_up_with_the_same_address(eng):
    e, link, env = eng
    assert _announce(e) is not None
    assert e.announce_due() is None
    up = link.st
    link.st = LinkState()
    assert e.announce_due() is None
    link.st = up
    assert e.announce_due() is not None


def test_unsent_announcement_stays_due_and_keeps_issued_ms(eng, tmp_path):
    e, link, env = eng
    p = e.announce_due()
    env.t += 1
    assert e.announce_due() == p
    assert e.counters['lora.announce.sent'] == 0
    back, issued, _ = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert issued == 0
    e.announce_sent(p)
    back, issued, _ = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert issued == p['issued_ms']


def test_issued_ms_survives_restart_with_clock_behind(tmp_path):
    path = str(tmp_path / 'lora_peers.json')
    cfg = config.parse('{"port": 10}')
    env = FakeEnv()
    e1 = Engine(cfg, FakeLink(), env, path, logging.getLogger('t'))
    issued = _announce(e1)['issued_ms']
    env.wall -= 3_600_000                        # rebooted, clock an hour behind
    e2 = Engine(cfg, FakeLink(), env, path, logging.getLogger('t'))
    assert e2.announce_due()['issued_ms'] == issued + 1


def test_announce_acceptance_rules(eng, tmp_path):
    e, link, env = eng
    assert e.on_announce(ME, {'lora_addr': 1, 'issued_ms': 9}, True) == 'self'
    assert e.on_announce('dddddddd-0000-4000-8000-000000000004', {'lora_addr': 7, 'issued_ms': 9}, True) == 'refused'
    assert e.on_announce(A, {'lora_addr': '7', 'issued_ms': 9}, True) == 'bad'
    assert e.on_announce(A, {'lora_addr': 0xB02, 'issued_ms': 5}, True) == 'stale'
    assert e.counters['lora.announce.stale'] == 1
    back, _, err = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert err is None and back.addr_of(A) == 0xB02


def test_exclusion_removes_and_persists(eng, tmp_path):
    e, link, env = eng
    e.on_exclusion(A, True)
    assert not e.on_unicast(A, 'identity', 'group_key_update', b'x')
    back, _, _ = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert back.addr_of(A) is None


def test_refusals_from_the_link_are_counted_by_reason(eng):
    e, link, env = eng
    link.results.append((False, 'OVER_BUDGET', 1, 3))
    e.poll()
    assert e.counters['lora.tx.refused.OVER_BUDGET'] == 1


def test_results_the_link_dropped_are_counted(eng):
    e, link, env = eng
    e.poll()
    assert e.counters['lora.tx.result_dropped'] == 0
    link.results_dropped = 3
    e.poll()
    assert e.counters['lora.tx.result_dropped'] == 3
    assert e.stats()['counters']['lora.tx.result_dropped'] == 3


def test_first_msg_id_is_random(tmp_path):
    ids = set()
    for i in range(8):
        link = FakeLink()
        e = Engine(config.parse('{"port": 10, "allow": ["identity/group_key_update"]}'),
                   link, FakeEnv(), str(tmp_path / ('m%d.json' % i)), logging.getLogger('t'))
        e.on_announce(A, {'lora_addr': 0xB02, 'issued_ms': 5}, True)
        e.on_unicast(A, 'identity', 'group_key_update', b'x')
        ids.add(fr.decode(link.sent[0][1][0]).msg_id)
    assert len(ids) > 1


def test_departed_holder_is_replaced_by_a_member(eng, tmp_path):
    e, link, env = eng
    C = 'cccccccc-0000-4000-8000-000000000003'
    env.roster.remove(A)
    env.roster.append(C)
    assert e.on_announce(C, {'lora_addr': 0xB02, 'issued_ms': 9}, True) == 'accepted'
    assert e.addrmap.addr_of(A) is None and e.addrmap.addr_of(C) == 0xB02
    back, _, _ = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert back.addr_of(A) is None and back.addr_of(C) == 0xB02


def test_conflict_warns_once_and_counts_received_frames(eng, caplog):
    e, link, env = eng
    with caplog.at_level(logging.WARNING, logger='t'):
        assert e.on_announce(A, {'lora_addr': 0xC03, 'issued_ms': 9}, True) == 'accepted'
    warns = [r.getMessage() for r in caplog.records if r.levelno == logging.WARNING]
    assert len(warns) == 1 and A in warns[0] and B in warns[0]
    link.rx.append((0xC03, fr.fragment(b'x', fr.PEER, 1, 233, 8)[0], 0))
    assert e.poll() == []
    assert e.counters['lora.rx.conflict'] == 1


def test_json_group_is_logged_once(eng, caplog):
    e, link, env = eng
    env.proto = False
    with caplog.at_level(logging.INFO, logger='t'):
        e.on_unicast(A, 'identity', 'group_key_update', b'x')
        e.on_unicast(A, 'identity', 'group_key_update', b'x')
        e.on_fanout('group', 'identity', 'presence', b'x')
    assert len([r for r in caplog.records if 'JSON' in r.getMessage()]) == 1


def test_summary_due_every_300_s(eng):
    e, link, env = eng
    assert e.summary_due() is None
    env.t += 299
    assert e.summary_due() is None
    env.t += 2
    line = e.summary_due()
    assert line.startswith('lora: link up') and '2 peers mapped' in line
    assert e.summary_due() is None


def test_stats_shape(eng):
    e, link, env = eng
    s = e.stats()
    assert set(s) == {'counters', 'link', 'addrmap'}
    assert s['link'] == {'up': True, 'connected': True, 'claimed': True,
                         'node_addr': 0xA01, 'mtu': 233}
    assert {x['uuid'] for x in s['addrmap']} == {A, B}


def test_nack_traffic_counts_frames_and_refusals(eng):
    e, link, env = eng
    frames = fr.fragment(bytes(500), fr.PEER, 9, 233, 8)
    link.rx.append((0xB02, frames[0], 0))
    e.poll()
    link.accept = False
    env.t += 20
    e.poll()
    assert e.counters['lora.tx.abandoned'] == 1 and e.counters['lora.nack.sent'] == 0


def test_sent_nack_counts_a_frame(eng):
    e, link, env = eng
    frames = fr.fragment(bytes(500), fr.PEER, 9, 233, 8)
    link.rx.append((0xB02, frames[0], 0))
    e.poll()
    env.t += 20
    e.poll()
    assert e.counters['lora.nack.sent'] == 1 and e.counters['lora.tx.frames'] == 1


def test_an_unverified_announcement_is_refused(eng):
    e, link, env = eng
    assert e.on_announce(A, {'lora_addr': 0xD04, 'issued_ms': 9}, False) == 'refused'
    assert e.counters['lora.announce.unverified'] == 1
    assert e.counters['lora.announce.accepted'] == 2
    assert e.addrmap.addr_of(A) == 0xB02


def _filler(i):
    return '%08x-0000-4000-8000-%012x' % (0x10000000 + i, i)


def test_a_full_map_makes_room_by_dropping_the_oldest_non_member(eng, tmp_path):
    e, link, env = eng                     # A and B mapped at issued_ms 5
    for i in range(AddrMap.MAX - 2):
        assert e.addrmap.announce(_filler(i), 0x1000 + i, 100 + i) == 'accepted'
    C = 'cccccccc-0000-4000-8000-000000000003'
    env.roster.append(C)
    assert e.on_announce(C, {'lora_addr': 0xE05, 'issued_ms': 9}, True) == 'accepted'
    assert e.addrmap.addr_of(_filler(0)) is None
    assert e.addrmap.addr_of(_filler(1)) == 0x1001
    assert e.addrmap.addr_of(A) == 0xB02 and e.addrmap.addr_of(C) == 0xE05
    assert e.counters['lora.announce.full'] == 0
    back, _, _ = AddrMap.load(str(tmp_path / 'lora_peers.json'))
    assert back.addr_of(C) == 0xE05 and back.addr_of(_filler(0)) is None


def test_a_map_full_of_members_counts_and_warns_once_a_minute(eng, caplog):
    e, link, env = eng
    for i in range(AddrMap.MAX - 2):
        env.roster.append(_filler(i))
        assert e.addrmap.announce(_filler(i), 0x1000 + i, 100 + i) == 'accepted'
    C = 'cccccccc-0000-4000-8000-000000000003'
    env.roster.append(C)
    with caplog.at_level(logging.WARNING, logger='t'):
        assert e.on_announce(C, {'lora_addr': 0xE05, 'issued_ms': 9}, True) == 'full'
        assert e.on_announce(C, {'lora_addr': 0xE05, 'issued_ms': 10}, True) == 'full'
        assert sum('address map is full' in m for m in caplog.messages) == 1
        env.t += 60
        assert e.on_announce(C, {'lora_addr': 0xE05, 'issued_ms': 11}, True) == 'full'
        assert sum('address map is full' in m for m in caplog.messages) == 2
    assert e.counters['lora.announce.full'] == 3
    assert len(e.addrmap.snapshot()) == AddrMap.MAX


def test_a_bad_entry_in_the_file_is_skipped_with_one_warning(tmp_path, caplog):
    path = tmp_path / 'lora_peers.json'
    path.write_text('{"version":1,"self_issued_ms":42,"peers":{'
                    '"%s":{"addr":2818,"issued_ms":5},"%s":{"addr":0,"issued_ms":5}}}' % (A, B))
    with caplog.at_level(logging.WARNING, logger='t'):
        e = Engine(config.parse('{"port": 10}'), FakeLink(), FakeEnv(), str(path),
                   logging.getLogger('t'))
    assert e.addrmap.addr_of(A) == 2818 and e.addrmap.addr_of(B) is None
    assert e._self_issued == 42
    warns = [m for m in caplog.messages if 'skipped' in m]
    assert len(warns) == 1 and B in warns[0]
