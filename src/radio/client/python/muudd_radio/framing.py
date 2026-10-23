# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Length-prefixed framing: a 2-byte big-endian length, then the bytes."""
import struct

MAX_FRAME = 8192


def encode(payload: bytes) -> bytes:
    if len(payload) > MAX_FRAME:
        raise ValueError(f"frame of {len(payload)} bytes exceeds {MAX_FRAME}")
    return struct.pack(">H", len(payload)) + payload


class FrameReader:
    def __init__(self):
        self._buf = bytearray()

    def feed(self, data: bytes) -> list:
        self._buf += data
        frames = []
        while len(self._buf) >= 2:
            (n,) = struct.unpack_from(">H", self._buf)
            if n > MAX_FRAME:
                raise ValueError(f"frame header declares {n} bytes, more than {MAX_FRAME}")
            if len(self._buf) < 2 + n:
                break
            frames.append(bytes(self._buf[2 : 2 + n]))
            del self._buf[: 2 + n]
        return frames
