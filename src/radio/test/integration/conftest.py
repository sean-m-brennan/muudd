# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Runs muudd-radiod and stub drivers for integration tests."""
import json
import os
import socket
import struct
import subprocess
import sys
import time
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
RADIO = HERE.parent.parent
BUILD = Path(os.environ.get("MR_BUILD_DIR", RADIO / "build"))
RADIOD = BUILD / "muudd-radiod"
STUB = BUILD / "stub-driver"
RADIOCTL = BUILD / "radioctl"

sys.path.insert(0, str(RADIO / "client" / "python"))
from muudd_radio import radio_pb2 as pb  # noqa: E402
from muudd_radio.client import BROADCAST, RadioClient  # noqa: E402,F401

MEDIUM_HDR = struct.Struct(">IIHI")


def free_udp_port():
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wait_until(pred, timeout=5.0, step=0.05):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if pred():
            return True
        time.sleep(step)
    return False


def stub_driver(*args):
    return {"exec": str(STUB), "args": [str(a) for a in args]}


def inject(medium_dir, dest_node, src, dest, port, payload, pid):
    """Hand one frame to the stub driver of node dest_node, as if heard on air."""
    s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    try:
        s.sendto(MEDIUM_HDR.pack(src, dest, port, pid) + payload, os.path.join(str(medium_dir), f"{dest_node:08x}.sock"))
    finally:
        s.close()


class Sniffer:
    """A listener on a shared medium that hears every transmission."""

    def __init__(self, medium_dir, node=0x5F5F5F5F):
        self.path = os.path.join(str(medium_dir), f"{node:08x}.sock")
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.sock.bind(self.path)
        self.sock.settimeout(0.05)

    def frames(self, seconds):
        out, end = [], time.monotonic() + seconds
        while time.monotonic() < end:
            try:
                data = self.sock.recv(65536)
            except socket.timeout:
                continue
            src, dest, port, pid = MEDIUM_HDR.unpack_from(data)
            out.append((src, dest, port, pid, data[MEDIUM_HDR.size :]))
        return out

    def close(self):
        self.sock.close()
        os.unlink(self.path)


class Radiod:
    def __init__(self, workdir, name):
        self.name = name
        self.dir = Path(workdir) / name
        self.dir.mkdir(parents=True, exist_ok=True)
        self.socket = str(self.dir / "r.sock")
        self.config_path = self.dir / "radiod.json"
        self.key_file = self.dir / "radiod.key"
        self.log = self.dir / "radiod.log"
        self.port = free_udp_port()
        self.proc = None
        self._logf = None

    @property
    def pubkey(self):
        out = subprocess.run([str(RADIOD), "--print-pubkey", "--key-file", str(self.key_file)],
                             check=True, capture_output=True, text=True)
        return out.stdout.strip()

    def write_config(self, cfg):
        cfg = json.loads(json.dumps(cfg))
        cfg["socket"] = self.socket
        if cfg.get("bridge") is not None:
            cfg["bridge"].setdefault("key_file", str(self.key_file))
            cfg["bridge"].setdefault("listen", f"127.0.0.1:{self.port}")
        self.config_path.write_text(json.dumps(cfg))

    def start(self, cfg=None):
        if cfg is not None:
            self.write_config(cfg)
        self._logf = open(self.log, "ab")
        self.proc = subprocess.Popen([str(RADIOD), "--config", str(self.config_path)],
                                     stdout=self._logf, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 5
        while not os.path.exists(self.socket):
            if self.proc.poll() is not None:
                raise RuntimeError(f"radiod {self.name} exited:\n{self.log.read_text()}")
            if time.monotonic() > deadline:
                raise TimeoutError(f"radiod {self.name} did not create its socket")
            time.sleep(0.02)
        return self

    def client(self, **kw):
        return RadioClient(self.socket, **kw)

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        if self._logf:
            self._logf.close()
            self._logf = None

    def kill(self):
        self.proc.kill()
        self.proc.wait()


@pytest.fixture
def radiods(tmp_path):
    made = []

    def make(name):
        r = Radiod(tmp_path, name)
        made.append(r)
        return r

    yield make
    for r in made:
        r.stop()


@pytest.fixture
def medium(tmp_path):
    d = tmp_path / "medium"
    d.mkdir()
    return d
