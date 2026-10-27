# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Two AutonomousTrust nodes on loopback, each with its own radiod on a shared
stub medium. With IP up nothing goes on the air; with IP cut, the presence
heartbeat crosses over LoRa both ways and each keeps the other present; with
IP healed the air goes quiet again; and killing a radiod does not disturb IP.

UDP broadcast does not work on loopback, so the nodes' UDP transport is
patched (as AT's own two-node test does) to unicast to the other node, and
the patch drops every send while the cut file exists.

Node A forms a proto-format group (AT_NET_WIRE_MODE=proto) and node B joins it
over the live admission handshake, adopting the group's format, before any
LoRa check starts: LoRa carries only proto cohorts."""
import json
import logging
import multiprocessing
import os
import queue
import re
import shutil
import socket
import tempfile
import threading
import time
from pathlib import Path
from unittest.mock import patch

import pytest
from radiod_harness import Radiod, Sniffer, stub_driver, wait_until

pytestmark = pytest.mark.slow

NODES = {'a': ('127.0.0.1', 0xA01), 'b': ('127.0.0.2', 0xB02)}
AT_PORT = 10
CAST_OFFSET = 5
RUNTIME = 300
STAGGER = 15
DOWN_S = 40           # radiod A down for longer than AT_PRESENCE_ABSENT_SEC (30)
MP = multiprocessing.get_context('forkserver')
LORA = Path(__file__).resolve().parents[3]
RADIO = LORA.parent


def _addresses(ip):
    return {'ip4': ip, 'ip6': None, 'mac': '00:11:22:33:44:%02x' % int(ip.split('.')[-1]),
            'ip4_subnet': '255.0.0.0', 'ip6_subnet': None, 'mac_bcast': 'ff:ff:ff:ff:ff:ff'}


def _patch_udp(my_ip, peers, port, cut_file):
    from autonomous_trust.core.network.udp import TransmissionError, UDPNetworkProcess
    cast_port = port + CAST_OFFSET

    def init_mcast(self, use_mcast=False):
        self.manycast_packet_size = 65507
        self.sock_options = (socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        self.manycast_addr = my_ip
        self.recv_cast_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        self.recv_cast_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.recv_cast_sock.bind((my_ip, cast_port))

    def send_any(self, msg):
        if os.path.exists(cut_file):
            return
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP) as s:
            s.bind((my_ip, 0))
            data = msg if isinstance(msg, bytes) else msg.encode(self.enc)
            for addr in peers:
                if addr != my_ip:
                    s.sendto(data, (addr, cast_port))

    def send_udp(self, msg, host, port_):
        if os.path.exists(cut_file):
            return
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP) as s:
            s.bind((my_ip, 0))
            data = msg if isinstance(msg, bytes) else msg.encode(self.enc)
            if s.sendto(data, (host, port_)) == 0:
                raise TransmissionError('no bytes sent')

    UDPNetworkProcess._init_mcast = init_mcast
    UDPNetworkProcess.send_any = send_any
    UDPNetworkProcess._send_udp = send_udp


def _run_node(root, ip, peers, cut_file, runtime, log_file, env):
    os.environ.update(env)
    from autonomous_trust.core import AutonomousTrust, Process
    from autonomous_trust.core.config import Configuration
    from autonomous_trust.core.system import comm_port
    os.environ[Configuration.ROOT_VARIABLE_NAME] = root
    _patch_udp(ip, peers, comm_port, cut_file)
    with patch('autonomous_trust.core.network.network.Network.get_addresses',
               return_value=_addresses(ip)):
        q = queue.Queue()
        timer = threading.Timer(runtime, lambda: q.put(Process.sig_quit))
        timer.daemon = True
        timer.start()
        # Not testing=True: its synthetic tasks keep each node talking to the
        # other, and a node that keeps talking never sends a presence heartbeat.
        AutonomousTrust(multiproc=False, log_level=logging.DEBUG, logfile=log_file,
                        testing=False, silent=True).run_forever(q_in=q)


def _read(path):
    try:
        return Path(path).read_text(errors='replace')
    except OSError:
        return ''


def _group(node):
    """The group a node has saved, as its raw JSON fields, or {}."""
    try:
        with open(os.path.join(node['root'], 'etc', 'at', 'group.cfg.json')) as f:
            saved = json.load(f)
    except (OSError, ValueError):
        return {}
    return saved[0] if isinstance(saved, list) and saved else {}


def _one_cohort(nodes):
    """Every node holds the same proto group, and its group names every other
    node. Not itself: a joiner's group comes from the admitter's history, which
    is taken before the admitter adds the joiner."""
    groups = {name: _group(n) for name, n in nodes.items()}
    if len({g.get('_uuid') for g in groups.values()}) != 1:
        return False
    for name, g in groups.items():
        others = {n['ip'] for other, n in nodes.items() if other != name}
        if g.get('_wire_format') != 'proto' or \
                not others <= set((g.get('_address_map') or {}).values()):
            return False
    return True


def _where(nodes, radios):
    """The logs to read when an assertion fails."""
    return ' logs: ' + ', '.join([n['log'] for n in nodes.values()] +
                                 [str(r.kept_log) for r in radios.values()])


@pytest.fixture
def cohort(tmp_path):
    from autonomous_trust.core.config import Configuration
    from autonomous_trust.core.config.generate import generate_identity
    # Unix socket paths are limited to 107 characters, so radiod and the
    # medium live in a short directory of their own.
    short = Path(tempfile.mkdtemp(prefix='atl-'))
    medium = short / 'air'
    medium.mkdir()
    cut = str(tmp_path / 'ip_cut')
    radios, nodes, procs = {}, {}, {}
    import autonomous_trust.core as _core
    core_src = Path(_core.__file__).resolve().parents[2]      # src/autonomous-trust
    pythonpath = os.pathsep.join([str(core_src), str(LORA / 'python'),
                                  str(RADIO / 'client' / 'python'), os.environ.get('PYTHONPATH', '')])
    # After the cut a peer goes quiet only ip_quiet_s (10) after its last IP
    # frame, and the next heartbeat can be up to two intervals (5 + 5, when an
    # announcement to the group resets its clock) later: about 21 s before
    # LoRa carries anything. Absence at 30 s leaves room for that and is still
    # well inside the 50 s the test keeps IP cut.
    env = {'PYTHONPATH': pythonpath, 'AT_NET_WIRE_MODE': 'proto',
           'AT_PRESENCE_HEARTBEAT_SEC': '5', 'AT_PRESENCE_ABSENT_SEC': '30'}
    saved_root = os.environ.get(Configuration.ROOT_VARIABLE_NAME)
    try:
        for name, (ip, node) in NODES.items():
            r = Radiod(short, 'radio_' + name)
            radios[name] = r
            r.kept_log = tmp_path / 'radio' / r.name / 'radiod.log'     # copied there at teardown
            r.start({
                'node_addr': 'auto', 'duty_cycle_pct': 100,
                'driver': stub_driver('--medium', medium, '--node-addr', hex(node), '--status-ms', '200'),
                'ports': {str(AT_PORT): {'name': 'at', 'share_pct': 40, 'policy': 'consumer'}}})
            root = tmp_path / ('node_' + name)
            os.environ[Configuration.ROOT_VARIABLE_NAME] = str(root)
            cfg_dir = Configuration.get_cfg_dir()
            os.makedirs(cfg_dir, exist_ok=True)
            with patch('autonomous_trust.core.network.network.Network.get_addresses',
                       return_value=_addresses(ip)):
                generate_identity(cfg_dir, randomize=True, seed=int(ip.split('.')[-1]))
            Path(cfg_dir, 'lora.cfg.json').write_text(json.dumps({
                'socket': r.socket, 'port': AT_PORT, 'ip_quiet_s': 10,
                'allow': ['identity/presence'], 'announce_s': 30}))
            log_dir = root / 'var' / 'at'
            log_dir.mkdir(parents=True, exist_ok=True)
            nodes[name] = {'root': str(root), 'ip': ip, 'log': str(log_dir / 'at.log'),
                           'data': Configuration.get_data_dir()}
        peers = [ip for ip, _ in NODES.values()]
        for i, (name, n) in enumerate(nodes.items()):
            if i:
                time.sleep(STAGGER)          # as AT's two-node test staggers them
            procs[name] = MP.Process(target=_run_node, daemon=True,
                                     args=(n['root'], n['ip'], peers, cut, RUNTIME, n['log'], env))
            procs[name].start()
        # The extension loaded and its engine reached radiod on both nodes.
        assert wait_until(lambda: all('lora: connected to radiod' in _read(n['log'])
                                      for n in nodes.values()), 30, step=0.5), \
            'the lora extension did not start on both nodes;' + _where(nodes, radios)
        # A admitted B over the live handshake, into A's proto group.
        assert wait_until(lambda: _one_cohort(nodes), 90, step=1), \
            'B was not admitted into A\'s proto cohort;' + _where(nodes, radios)
        yield nodes, radios, medium, cut
    finally:
        for p in procs.values():
            p.terminate()
        for p in procs.values():
            p.join(10)
            if p.is_alive():
                p.kill()
                p.join(5)
        for r in radios.values():
            r.stop()
        try:
            shutil.copytree(short, tmp_path / 'radio', dirs_exist_ok=True,
                            ignore=shutil.ignore_patterns('*.sock'))
        finally:
            shutil.rmtree(short, ignore_errors=True)
        if saved_root is None:
            os.environ.pop(Configuration.ROOT_VARIABLE_NAME, None)
        else:
            os.environ[Configuration.ROOT_VARIABLE_NAME] = saved_root


def _peers_file(node):
    try:
        with open(os.path.join(node['data'], 'lora_peers.json')) as f:
            return json.load(f)['peers']
    except (OSError, ValueError, KeyError):
        return {}


def _air(medium, seconds):
    s = Sniffer(medium)
    try:
        return [f for f in s.frames(seconds) if f[2] == AT_PORT]
    finally:
        s.close()


def _absent(log):
    return re.findall(r'^.*peer \S+ is absent.*$', log, re.MULTILINE)


def _lora_errors(nodes):
    """ERROR lines mentioning lora, and Tracebacks, in every node's log."""
    out = []
    for n in nodes.values():
        log = _read(n['log'])
        out += [e for e in re.findall(r'^.*- ERROR .*$', log, re.MULTILINE) if 'lora' in e]
        out += ['%s: Traceback' % n['log']] * log.count('Traceback')
    return out


def test_lora_carries_presence_only_while_ip_is_cut(cohort):
    nodes, radios, medium, cut = cohort
    a, b = nodes['a'], nodes['b']
    where = _where(nodes, radios)
    # Both learned the other's radio address over IP.
    assert wait_until(lambda: len(_peers_file(a)) == 1 and len(_peers_file(b)) == 1, 90, step=1), \
        'no address exchange;' + where

    # IP up: nothing on the air.
    assert _air(medium, 15) == [], 'LoRa used while IP works;' + where

    # IP cut: presence crosses over LoRa both ways, and nobody goes absent.
    Path(cut).touch()
    frames = _air(medium, 30)
    srcs = {f[0] for f in frames}
    assert {0xA01, 0xB02} <= srcs, 'sources on air %s;%s' % (sorted(map(hex, srcs)), where)
    assert all(f[4][0] == 0x12 for f in frames), \
        'non-group frames %s;%s' % (sorted({f[4][0] for f in frames}), where)
    time.sleep(20)
    for n in (a, b):
        absent = _absent(_read(n['log']))
        assert not absent, 'a peer went absent although LoRa carried presence: %s;%s' % (absent, where)

    # IP healed: the air goes quiet again.
    os.remove(cut)
    time.sleep(10)
    assert _air(medium, 15) == [], 'LoRa still used after IP healed;' + where
    errors = _lora_errors(nodes)
    assert not errors, 'lora errors: %s;%s' % (errors, where)


def test_killing_a_radiod_leaves_ip_alone(cohort):
    nodes, radios, medium, cut = cohort
    a, b = nodes['a'], nodes['b']
    where = _where(nodes, radios)
    assert wait_until(lambda: len(_peers_file(a)) == 1 and len(_peers_file(b)) == 1, 90, step=1), \
        'no address exchange;' + where
    (a_uuid, a_entry), = _peers_file(b).items()
    connected = _read(a['log']).count('lora: connected to radiod')
    radios['a'].kill()
    assert wait_until(lambda: 'lora: lost the connection' in _read(a['log']).split(
        'lora: connected to radiod')[-1], 10, step=0.5), 'the link did not notice the kill;' + where
    # Radio A stays down longer than the absence threshold. B's radio is up
    # and LoRa may carry its presence heartbeat, so a single frame from B
    # would mean A had gone quiet over IP; none means B kept hearing A.
    down = _air(medium, DOWN_S)
    assert not [f for f in down if f[0] == 0xB02], \
        'B used LoRa while only A\'s radiod was down (A quiet over IP);' + where
    for n in (a, b):
        assert not _absent(_read(n['log'])), 'a peer went absent while a radiod was down;' + where
    radios['a'].start()
    assert wait_until(lambda: _read(a['log']).count('lora: connected to radiod') > connected,
                      30, step=1), 'the link did not reconnect;' + where
    # Back up, A announces again, and B hears it over IP.
    assert wait_until(lambda: _peers_file(b).get(a_uuid, {}).get('issued_ms', 0) > a_entry['issued_ms'],
                      30, step=1), 'no announcement over IP after the reconnect;' + where
    # IP carried on throughout, so presence never flipped on either node.
    for n in (a, b):
        assert not _absent(_read(n['log'])), 'a peer went absent while IP was up;' + where
    errors = _lora_errors(nodes)
    assert not errors, 'lora errors: %s;%s' % (errors, where)
