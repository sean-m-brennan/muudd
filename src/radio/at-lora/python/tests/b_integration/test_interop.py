# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The C and Python engines exchange a three-fragment unicast over two real
radiods sharing a stub medium. The first transmission drops one fragment;
the receiver's NACK and the sender's hold recover it."""
import logging
import os
import subprocess
import threading
import time
from pathlib import Path

import pytest
from radiod_harness import Radiod, stub_driver, wait_until

from autonomous_trust.lora._python import config
from autonomous_trust.lora._python.engine import Engine
from autonomous_trust.lora._python.link import RadioLink

LORA = Path(__file__).resolve().parents[3]
PROBE = Path(os.environ.get('AT_LORA_PROBE', LORA / 'build' / 'extensions' / 'lora' / 'at_lora_probe'))
AT_PORT = {"10": {"name": "at", "share_pct": 40, "policy": "consumer"}}
C_UUID = 'cccccccc-0000-4000-8000-00000000000c'
P_UUID = 'dddddddd-0000-4000-8000-00000000000d'
MSG = bytes(i % 256 for i in range(600))      # three DATA_PEER fragments at MTU 233


class _Env:
    def __init__(self, me, peer):
        self.me, self.peer = me, peer

    def my_uuid(self): return self.me
    def members(self): return [self.peer]
    def quiet_for(self, u): return 1e9
    def group_proto(self): return True
    def group_tag(self): return bytes.fromhex('a1b2c3d4')
    def wall_ms(self): return int(time.time() * 1000)
    def now(self): return time.monotonic()


class _DropFirst:
    """Wraps a RadioLink and drops fragment `idx` of the first message."""

    def __init__(self, link, idx):
        self.link, self.idx, self.done = link, idx, False

    def __getattr__(self, name):
        return getattr(self.link, name)

    def submit(self, dest, frags, want_ack):
        if not self.done and len(frags) > self.idx + 1:
            self.done = True
            frags = frags[:self.idx] + frags[self.idx + 1:]
        return self.link.submit(dest, frags, want_ack)


def _cfg(medium, node):
    return {"node_addr": "auto", "duty_cycle_pct": 100,
            "driver": stub_driver("--medium", medium, "--node-addr", hex(node), "--status-ms", "200"),
            "ports": AT_PORT}


@pytest.fixture
def radios(tmp_path):
    assert PROBE.exists(), 'build at_lora_probe first (Task 13 Step 1): %s' % PROBE
    medium = tmp_path / 'air'
    medium.mkdir()
    c = Radiod(tmp_path, 'c').start(_cfg(medium, 0xC0C))
    p = Radiod(tmp_path, 'p').start(_cfg(medium, 0xD0D))
    yield c, p
    c.stop()
    p.stop()


def _python_engine(tmp_path, socket, peer_addr, drop=None):
    cfg = config.parse('{"port": 10, "nack_gap_s": 2, "reassembly_s": 10, "allow": ["test/probe"]}')
    link = RadioLink(socket, 10, logging.getLogger('py'))
    eng = Engine(cfg, _DropFirst(link, drop) if drop is not None else link,
                 _Env(P_UUID, C_UUID), str(tmp_path / 'py_peers.json'), logging.getLogger('py'))
    assert eng.on_announce(C_UUID, {'lora_addr': peer_addr, 'issued_ms': 1}, True) == 'accepted'
    assert wait_until(lambda: link.state().up, 5)
    return eng, link


def test_c_sends_python_recovers_by_nack(radios, tmp_path):
    c, p = radios
    eng, link = _python_engine(tmp_path, p.socket, 0xC0C)
    probe = subprocess.Popen([str(PROBE), '--socket', c.socket, '--port', '10', '--me', C_UUID,
                              '--peer', P_UUID, '--peer-addr', 'd0d', '--send-hex', MSG.hex(),
                              '--drop-first-frag', '1', '--seconds', '15'],
                             stdout=subprocess.PIPE, text=True)
    try:
        got = []
        assert wait_until(lambda: got.extend(eng.poll()) or got, 15, step=0.1)
        assert got == [('peer', C_UUID, MSG)]
        assert eng.counters['lora.nack.sent'] == 1
        out = probe.communicate(timeout=30)[0]
        assert 'counter lora.nack.served 1' in out
    finally:
        probe.kill()
        link.close()


def test_python_sends_c_recovers_by_nack(radios, tmp_path):
    c, p = radios
    probe = subprocess.Popen([str(PROBE), '--socket', c.socket, '--port', '10', '--me', C_UUID,
                              '--peer', P_UUID, '--peer-addr', 'd0d', '--seconds', '15'],
                             stdout=subprocess.PIPE, text=True)
    eng, link = _python_engine(tmp_path, p.socket, 0xC0C, drop=1)
    try:
        time.sleep(1.0)                        # the probe's link comes up
        assert eng.on_unicast(C_UUID, 'test', 'probe', MSG)
        stop = threading.Event()

        def pump():
            while not stop.is_set():
                eng.poll()
                time.sleep(0.05)
        t = threading.Thread(target=pump, daemon=True)
        t.start()
        out = probe.communicate(timeout=30)[0]
        stop.set()
        t.join()
        assert 'deliver peer %s %s' % (P_UUID, MSG.hex()) in out
        assert 'counter lora.nack.sent 1' in out
        assert eng.counters['lora.nack.served'] == 1
    finally:
        probe.kill()
        link.close()
