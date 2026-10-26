# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import pytest

from autonomous_trust.lora._python import frame as fr


def _pattern(n):
    return bytes(i % 256 for i in range(n))


def test_fragment_peer_three_pieces():
    msg = _pattern(500)
    frames = fr.fragment(msg, fr.PEER, 0x0102, 233, 8)
    assert [len(f) for f in frames] == [233, 233, 46]
    assert frames[0][:4] == bytes.fromhex('11010202')
    assert frames[2][:4] == bytes.fromhex('11010222')
    assert b''.join(f[4:] for f in frames) == msg


def test_fragment_limits():
    assert len(fr.fragment(_pattern(1832), fr.PEER, 1, 233, 8)) == 8
    for args in ((_pattern(1833), fr.PEER, 1, 233, 8),
                 (_pattern(10), fr.PEER, 1, 63, 8),
                 (b'', fr.PEER, 1, 233, 8),
                 (_pattern(10), fr.GROUP, 1, 233, 8),
                 (_pattern(10), fr.NACK, 1, 233, 8)):
        with pytest.raises(fr.FrameError):
            fr.fragment(*args)
    assert fr.max_message(fr.GROUP, 233, 8) == 1800
    assert fr.payload_per_frag(fr.PEER, 63) == 0


def test_decode_group_and_nack():
    f = fr.decode(bytes.fromhex('12000711a1b2c3d4ff'))
    assert (f.type, f.msg_id, f.idx, f.count, f.tag, f.payload) == \
        (fr.GROUP, 7, 1, 2, bytes.fromhex('a1b2c3d4'), b'\xff')
    n = fr.encode_nack(5, 3)
    assert n == bytes.fromhex('1400050003')
    assert fr.decode(n).missing == 3
    with pytest.raises(fr.FrameError):
        fr.encode_nack(5, 0)


@pytest.mark.parametrize('hexs', ['', '21010200ff', '15000100ff', '11010220ff',
                                  '11010200', '14000500', '1400050000',
                                  '140005000300', '12000700a1b2c3'])
def test_decode_refusals(hexs):
    with pytest.raises(fr.FrameError):
        fr.decode(bytes.fromhex(hexs))
