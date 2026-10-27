# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The frames AutonomousTrust traffic rides on its radiod port. Twin of C
lora_frame.

Byte 0 is the version (high nibble, 1) and type (low nibble). Then, big-
endian: DATA_PEER and DATA_BCAST carry msg_id(2) frag(1) payload; DATA_GROUP
carries msg_id(2) frag(1) group_tag(4) payload; NACK carries msg_id(2)
missing(2) and nothing else. frag is the fragment index (high nibble) and the
fragment count less one (low nibble), so a message has at most 16 fragments.
missing is a bitmap of fragment indexes a receiver lacks.
"""
from dataclasses import dataclass

VERSION = 1
PEER, GROUP, BCAST, NACK = 1, 2, 3, 4
MAX_FRAGS = 16
MIN_MTU = 64
TAG_LEN = 4
HEADER = {PEER: 4, GROUP: 8, BCAST: 4, NACK: 5}


class FrameError(ValueError):
    """A malformed frame, or a message that cannot be framed."""


@dataclass(frozen=True)
class Frame:
    type: int
    msg_id: int
    idx: int = 0
    count: int = 0
    tag: bytes = b''
    missing: int = 0
    payload: bytes = b''


def decode(buf: bytes) -> Frame:
    if not buf or buf[0] >> 4 != VERSION:
        raise FrameError('unknown version')
    type_ = buf[0] & 0x0F
    hdr = HEADER.get(type_)
    if hdr is None or len(buf) < hdr:
        raise FrameError('unknown type or short frame')
    msg_id = int.from_bytes(buf[1:3], 'big')
    if type_ == NACK:
        if len(buf) != hdr:
            raise FrameError('NACK length')
        missing = int.from_bytes(buf[3:5], 'big')
        if missing == 0:
            raise FrameError('NACK names no fragment')
        return Frame(NACK, msg_id, missing=missing)
    if len(buf) == hdr:
        raise FrameError('empty payload')
    idx, count = buf[3] >> 4, (buf[3] & 0x0F) + 1
    if idx >= count:
        raise FrameError('index not below count')
    tag = bytes(buf[4:8]) if type_ == GROUP else b''
    return Frame(type_, msg_id, idx, count, tag, 0, bytes(buf[hdr:]))


def encode_nack(msg_id: int, missing: int) -> bytes:
    if not 0 < missing <= 0xFFFF or not 0 <= msg_id <= 0xFFFF:
        raise FrameError('NACK fields')
    return bytes([(VERSION << 4) | NACK]) + msg_id.to_bytes(2, 'big') + missing.to_bytes(2, 'big')


def payload_per_frag(type_: int, mtu: int) -> int:
    if type_ == NACK or type_ not in HEADER or mtu < MIN_MTU:
        return 0
    return mtu - HEADER[type_]


def max_message(type_: int, mtu: int, max_frags: int) -> int:
    return payload_per_frag(type_, mtu) * max(1, min(MAX_FRAGS, max_frags))


def fragment(msg: bytes, type_: int, msg_id: int, mtu: int, max_frags: int,
             tag: bytes = None) -> list:
    per = payload_per_frag(type_, mtu)
    if not 0 <= msg_id <= 0xFFFF:
        raise FrameError('msg_id out of range')
    if not msg or per == 0 or (type_ == GROUP and (tag is None or len(tag) != TAG_LEN)) \
            or len(msg) > max_message(type_, mtu, max_frags):
        raise FrameError('cannot frame this message')
    count = (len(msg) + per - 1) // per
    head = bytes([(VERSION << 4) | type_]) + msg_id.to_bytes(2, 'big')
    out = []
    for i in range(count):
        frag = bytes([(i << 4) | (count - 1)])
        out.append(head + frag + (tag if type_ == GROUP else b'') + msg[i * per:(i + 1) * per])
    return out
