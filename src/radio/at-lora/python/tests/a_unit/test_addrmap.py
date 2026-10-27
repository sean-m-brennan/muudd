# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
from autonomous_trust.lora._python.addrmap import AddrMap

A = 'aaaaaaaa-0000-4000-8000-000000000001'
B = 'bbbbbbbb-0000-4000-8000-000000000002'


def test_accept_replay_and_lookup():
    m = AddrMap()
    assert m.announce(A, 0x1001, 100) == 'accepted'
    assert m.announce(A, 0x1002, 100) == 'stale'
    assert m.announce(A, 0x1002, 101) == 'accepted'
    assert m.addr_of(A) == 0x1002 and m.uuid_of(0x1001) is None and m.uuid_of(0x1002) == A
    assert m.announce(A.upper(), 0x1003, 102) == 'accepted' and m.addr_of(A) == 0x1003
    assert m.announce('nope', 1, 1) == 'bad'
    assert m.announce(B, 0, 1) == 'bad' and m.announce(B, 0xFFFFFFFF, 1) == 'bad'


def test_conflict_fails_closed_and_departed_holder_is_replaced():
    m = AddrMap()
    m.announce(A, 0x2000, 1)
    m.announce(B, 0x2000, 1)
    assert m.addr_of(A) is None and m.addr_conflicted(0x2000)
    m.announce(B, 0x2001, 2)
    assert m.addr_of(A) == 0x2000
    assert m.announce(B, 0x2000, 3, is_member=lambda u: u == B) == 'accepted'
    assert m.addr_of(A) is None and m.addr_of(B) == 0x2000


def test_full_and_remove():
    m = AddrMap()
    for i in range(AddrMap.MAX):
        assert m.announce('00000000-0000-4000-8000-%012x' % i, 0x100 + i, 1) == 'accepted'
    assert m.announce(A, 0x9999, 1) == 'full'
    first = '00000000-0000-4000-8000-%012x' % 0
    assert m.remove(first) and not m.remove(first)
    assert m.announce(A, 0x9999, 1) == 'accepted'


def test_persistence_round_trip_and_corrupt_file(tmp_path):
    path = str(tmp_path / 'lora_peers.json')
    empty, self_ms, err = AddrMap.load(path)
    assert (empty.snapshot(), self_ms, err) == ([], 0, None)
    m = AddrMap()
    m.announce(A, 0x3000, 5)
    m.save(path, 1234)
    back, self_ms, err = AddrMap.load(path)
    assert err is None and self_ms == 1234 and back.addr_of(A) == 0x3000
    assert back.announce(A, 0x3001, 5) == 'stale'
    (tmp_path / 'lora_peers.json').write_text('{not json')
    bad, self_ms, err = AddrMap.load(path)
    assert bad.snapshot() == [] and self_ms == 0 and err


def test_uuid_spellings_match_c():
    m = AddrMap()
    for bad in ('{' + A + '}', 'urn:uuid:' + A, A.replace('-', ''), A + '0', None, 5):
        assert m.announce(bad, 1, 1) == 'bad'
    assert m.announce(A.upper(), 1, 1) == 'accepted'


def test_types_and_ranges_are_strict():
    m = AddrMap()
    for addr, t in ((True, 1), (1.0, 1), (None, 1), ('1', 1), (1, None), (1, 1.5), (1, True),
                    (1, -1), (-5, 1), (0x100000000, 1)):
        assert m.announce(A, addr, t) == 'bad', (addr, t)
    assert m.snapshot() == []


def test_load_rejects_out_of_range_and_wrong_types(tmp_path):
    path = tmp_path / 'p.json'
    for body in ('{"version":1,"self_issued_ms":-1,"peers":{}}',
                 '{"version":true,"self_issued_ms":0,"peers":{}}',
                 '{"version":1.0,"self_issued_ms":0,"peers":{}}'):
        path.write_text(body)
        m, s, err = AddrMap.load(str(path))
        assert m.snapshot() == [] and s == 0 and err, body
    # A bad entry is skipped; the file is otherwise fine.
    for body in ('{"version":1,"self_issued_ms":3,"peers":{"%s":{"addr":1,"issued_ms":-1}}}' % A,
                 '{"version":1,"self_issued_ms":3,"peers":{"%s":{"addr":4294967295,"issued_ms":1}}}' % A,
                 '{"version":1,"self_issued_ms":3,"peers":{"%s":{"addr":0,"issued_ms":1}}}' % A,
                 '{"version":1,"self_issued_ms":3,"peers":{"%s":5}}' % A):
        path.write_text(body)
        m, s, err = AddrMap.load(str(path))
        assert m.snapshot() == [] and s == 3 and err is None and m.skipped == [A], body


def test_load_refuses_a_file_with_an_integer_or_real_it_cannot_hold(tmp_path):
    # C cannot parse these (jansson), so the whole file is refused there; so here.
    path = tmp_path / 'p.json'
    for bad in ('9223372036854775808', '-9223372036854775809', '1e400', 'NaN', 'Infinity'):
        path.write_text('{"version":1,"self_issued_ms":3,"peers":{"%s":{"addr":1,"issued_ms":%s}}}'
                        % (A, bad))
        m, s, err = AddrMap.load(str(path))
        assert m.snapshot() == [] and s == 0 and err, bad
        path.write_text('{"version":1,"self_issued_ms":%s,"peers":{}}' % bad)
        m, s, err = AddrMap.load(str(path))
        assert s == 0 and err, bad


def test_load_refuses_a_file_nested_past_2048_levels(tmp_path):
    # jansson stops at 2048 levels and refuses the whole file, even when the
    # depth sits in a key the map never reads; brackets inside a string do
    # not count.
    path = tmp_path / 'p.json'
    good = '"%s":{"addr":4097,"issued_ms":5}' % A

    def doc(extra):
        return '{"version":1,"self_issued_ms":5,"x":%s,"peers":{%s}}' % (extra, good)
    path.write_text(doc('[' * 2047 + ']' * 2047))       # 2048 with the root object
    m, s, err = AddrMap.load(str(path))
    assert err is None and s == 5 and m.addr_of(A) == 4097
    path.write_text(doc('[' * 2048 + ']' * 2048))
    m, s, err = AddrMap.load(str(path))
    assert err and s == 0 and m.snapshot() == []
    path.write_text(doc('"%s"' % ('[{' * 3000)))
    m, s, err = AddrMap.load(str(path))
    assert err is None and m.addr_of(A) == 4097


def test_conflicting_entries_survive_save_load(tmp_path):
    path = str(tmp_path / 'p.json')
    m = AddrMap()
    m.announce(A, 0x2000, 1)
    m.announce(B, 0x2000, 1)
    m.save(path)
    back, _, err = AddrMap.load(path)
    assert err is None and back.addr_conflicted(0x2000) and back.addr_of(A) is None
    assert len(back.snapshot()) == 2


def test_failed_save_leaves_no_tmp(tmp_path):
    import os
    import pytest
    path = str(tmp_path / 'missing' / 'p.json')
    with pytest.raises(OSError):
        AddrMap().save(path)
    d = tmp_path / 'dir'
    d.mkdir()
    with pytest.raises(OSError):
        AddrMap().save(str(d))   # rename onto a directory fails after the tmp write
    assert not os.path.exists(str(d) + '.tmp')


def test_issued_ms_is_a_signed_64_bit_value_as_in_c():
    m = AddrMap()
    assert m.announce(A, 0x1001, 2 ** 63) == 'bad'
    assert m.announce(A, 0x1001, 2 ** 63 - 1) == 'accepted'


def test_load_skips_a_bad_entry_and_keeps_the_rest(tmp_path):
    path = tmp_path / 'p.json'
    path.write_text('{"version":1,"self_issued_ms":77,"peers":{'
                    '"%s":{"addr":4660,"issued_ms":5},'
                    '"%s":{"addr":0,"issued_ms":5},'
                    '"not-a-uuid":{"addr":7,"issued_ms":5}}}' % (A, B))
    m, s, err = AddrMap.load(str(path))
    assert err is None and s == 77
    assert m.addr_of(A) == 4660 and m.addr_of(B) is None
    assert sorted(m.skipped) == sorted([B, 'not-a-uuid'])


def test_save_syncs_the_file_then_renames_then_syncs_the_directory(tmp_path, monkeypatch):
    import os
    import stat
    calls = []
    real_fsync, real_replace = os.fsync, os.replace

    def fsync(fd):
        st = os.fstat(fd)
        calls.append(('fsync dir' if stat.S_ISDIR(st.st_mode) else 'fsync file', st.st_ino))
        return real_fsync(fd)

    def replace(src, dst):
        calls.append(('replace', os.stat(src).st_ino))
        return real_replace(src, dst)
    monkeypatch.setattr(os, 'fsync', fsync)
    monkeypatch.setattr(os, 'replace', replace)
    path = str(tmp_path / 'p.json')
    m = AddrMap()
    m.announce(A, 0x3000, 5)
    m.save(path, 1)
    file_ino, dir_ino = os.stat(path).st_ino, os.stat(str(tmp_path)).st_ino
    assert calls == [('fsync file', file_ino), ('replace', file_ino), ('fsync dir', dir_ino)]
    assert AddrMap.load(path)[0].addr_of(A) == 0x3000
