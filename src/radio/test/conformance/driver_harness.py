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

from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import FrameReader, encode

MEDIUM_HDR = struct.Struct(">IIHI")


class DriverProc:
    def __init__(self, argv, medium):
        self.medium = medium
        self.p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.q = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        reader = FrameReader()
        while True:
            data = self.p.stdout.read1(65536)
            if not data:
                self.q.put(None)
                return
            for frame in reader.feed(data):
                self.q.put(pb.DriverToRadiod.FromString(frame))

    def send(self, **kw):
        self.p.stdin.write(encode(pb.RadiodToDriver(**kw).SerializeToString()))
        self.p.stdin.flush()

    def next(self, timeout):
        try:
            return self.q.get(timeout=timeout)
        except queue.Empty:
            return None

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
        for f in (self.p.stdin, self.p.stdout, self.p.stderr):
            try:
                f.close()
            except OSError:
                pass
