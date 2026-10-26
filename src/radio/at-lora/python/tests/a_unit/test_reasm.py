# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
from autonomous_trust.lora._python import frame as fr
from autonomous_trust.lora._python.reasm import Reassembler, SendHold


def _add(r, src, hexs, now):
    return r.add(src, fr.decode(bytes.fromhex(hexs)), now)


def test_out_of_order_completes_then_duplicates():
    r = Reassembler(20, 90)
    assert _add(r, 1, '11010222cc', 0) == ('partial', None)
    assert _add(r, 1, '11010202aa', 1) == ('partial', None)
    assert _add(r, 1, '11010212bb', 2) == ('complete', bytes.fromhex('aabbcc'))
    assert _add(r, 1, '11010202aa', 3) == ('duplicate', None)
    assert r.stats['duplicate'] == 1


def test_nack_once_after_the_gap_then_timeout():
    r = Reassembler(20, 90)
    _add(r, 1, '11010202aa', 0)
    assert r.tick(19) == []
    assert r.tick(20) == [(1, 258, 6)]
    assert r.tick(50) == []
    assert r.tick(90) == []
    assert r.stats['timeouts'] == 1
    assert _add(r, 1, '11010212bb', 91)[0] == 'duplicate'


def test_group_is_never_nacked():
    r = Reassembler(20, 90)
    _add(r, 1, '12000901a1b2c3d4aa', 0)
    assert r.tick(30) == []


def test_caps_evict_oldest():
    r = Reassembler(20, 90)
    for i, h in enumerate(['1100010101', '1100020102', '1100030103', '1100040104', '1100050105']):
        assert _add(r, 1, h, i)[0] == 'partial'
    assert r.stats['evicted'] == 1
    assert _add(r, 1, '1100011101', 6)[0] == 'duplicate'
    g = Reassembler(20, 90)
    for src in range(1, 5):
        for m in range(1, 5):
            assert _add(g, src, '11%04x01%02x' % (m, m), src * 10 + m)[0] == 'partial'
    assert _add(g, 5, '1100090109', 100)[0] == 'partial'
    assert g.stats['evicted'] == 1


def test_inconsistent_count_is_bad():
    r = Reassembler(20, 90)
    assert _add(r, 1, '11000302aa', 0)[0] == 'partial'
    assert _add(r, 1, '11000311bb', 1)[0] == 'bad'
    assert _add(r, 1, '11000322cc', 2)[0] == 'duplicate'


def test_hold_serves_once_and_expires():
    h = SendHold(120)
    frames = fr.fragment(b'\xaa\xbb\xcc', fr.PEER, 258, 64, 16)
    h.put(9, 258, frames, 0)
    assert h.serve(9, 258, 1, 10) == frames[:1]
    assert h.serve(9, 258, 1, 11) == []
    assert h.served == 1
    h.put(9, 259, frames, 0)
    assert h.serve(9, 259, 1, 120) == []


def test_hold_serves_the_newest_for_a_repeated_key():
    h = SendHold(120)
    h.put(9, 1, [b'\x11\x00\x01\x00\xaa'], 0)
    h.put(9, 1, [b'\x11\x00\x01\x00\xbb'], 1)
    assert h.serve(9, 1, 1, 2) == [b'\x11\x00\x01\x00\xbb']
