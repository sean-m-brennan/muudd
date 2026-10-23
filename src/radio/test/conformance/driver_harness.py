# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Runs a driver on pipes and speaks the driver protocol to it."""
import os
import queue
import socket
import struct
import subprocess
import threading
import time

from google.protobuf.message import DecodeError
from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import FrameReader, encode

MEDIUM_HDR = struct.Struct(">IIHI")
MALFORMED = object()


class DriverProc:
    def __init__(self, argv, medium):
        self.medium = medium
        self.stderr_path = os.path.join(str(medium), os.pardir, "driver.stderr")
        self.stderr_file = open(self.stderr_path, "wb")
        self.p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr_file)
        self.q = queue.Queue()
        self.last_time = None
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        reader = FrameReader()
        while True:
            data = self.p.stdout.read1(65536)
            if not data:
                self.q.put((time.monotonic(), None))
                return
            try:
                for frame in reader.feed(data):
                    self.q.put((time.monotonic(), pb.DriverToRadiod.FromString(frame)))
            except (ValueError, DecodeError):
                self.q.put((time.monotonic(), MALFORMED))
                return

    def send(self, **kw):
        self.p.stdin.write(encode(pb.RadiodToDriver(**kw).SerializeToString()))
        self.p.stdin.flush()

    def next(self, timeout):
        """Next decoded frame, or None on timeout or EOF; records its arrival time."""
        try:
            t, m = self.q.get(timeout=timeout)
        except queue.Empty:
            return None
        if m is MALFORMED:
            raise AssertionError("malformed frame from driver")
        self.last_time = t
        return m

    def next_of(self, kind, timeout):
        deadline = time.monotonic() + timeout
        while (left := deadline - time.monotonic()) > 0:
            m = self.next(left)
            if m is None:
                return None
            if m.WhichOneof("msg") == kind:
                return m
        return None

    def collect(self, kind, seconds):
        out, deadline = [], time.monotonic() + seconds
        while (left := deadline - time.monotonic()) > 0:
            m = self.next(left)
            if m is None:
                break
            if m.WhichOneof("msg") == kind:
                out.append(m)
        return out

    def inject(self, node, src, dest, port, payload, pid):
        s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        try:
            s.sendto(MEDIUM_HDR.pack(src, dest, port, pid) + payload, os.path.join(str(self.medium), f"{node:08x}.sock"))
        finally:
            s.close()

    def kill(self):
        if self.p.poll() is None:
            self.p.kill()
            self.p.wait()
        for f in (self.p.stdin, self.p.stdout, self.stderr_file):
            try:
                f.close()
            except OSError:
                pass
