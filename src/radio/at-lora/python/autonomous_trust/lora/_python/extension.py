# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The lora extension as AT's network process sees it: a config gate, the
announce and stats verbs, and the network hooks that feed the engine.
Present means loaded; without <cfg_dir>/lora.cfg.json it stays inert."""
import functools
import json
import os
import time
import uuid as _uuid
from queue import Full

from autonomous_trust.core._python.config import NetWireFormat
from autonomous_trust.core._python.config.configuration import Configuration
from autonomous_trust.core._python.extensions import Extension, NetworkHooks
from autonomous_trust.core._python.network.message import Message
from autonomous_trust.core._python.system import CfgIds

from . import config
from .engine import Engine

LORA_ADDR = 'lora_addr'
LORA_STATS = 'lora_stats'
LORA_STATS_RESP = 'lora_stats_resp'
ADDRMAP_FILE = 'lora_peers.json'
ERROR_LOG_S = 60.0
WARN_EVERY_S = 60.0


class _Env:
    """The engine's view of the network process."""

    def __init__(self, proc):
        self.proc = proc

    def my_uuid(self):
        return str(self.proc.myself.uuid).lower()

    def members(self):
        return [str(p.uuid).lower() for p in self.proc.peers.all]

    def quiet_for(self, uuid):
        return self.proc.transport_quiet_for(uuid)

    def group_proto(self):
        g = self.proc.group
        return g is not None and g.wire_format == NetWireFormat.proto

    def group_tag(self):
        g = self.proc.group
        return None if g is None else _uuid.UUID(str(g.uuid)).bytes[:4]

    def wall_ms(self):
        return int(time.time() * 1000)

    def now(self):
        return time.monotonic()


def _enabled():
    return True


def _check_config(cfg_dir, logger):
    path = os.path.join(cfg_dir or Configuration.get_cfg_dir(), config.FILENAME)
    try:
        config.load(path)
    except config.ConfigError as err:
        if logger is not None:
            logger.error('lora: %s refused: %s', path, err)
        raise


def _on_start(proc, queues):
    proc._lora = None
    cfg = config.load(os.path.join(Configuration.get_cfg_dir(), config.FILENAME))
    if cfg is None:
        proc.logger.info('lora: no %s; the extension is inert', config.FILENAME)
        return
    from .link import RadioLink
    link = RadioLink(cfg.socket, cfg.port, proc.logger)
    path = os.path.join(Configuration.get_data_dir(), ADDRMAP_FILE)
    proc._lora = Engine(cfg, link, _Env(proc), path, proc.logger)


def _periodic(proc, queues):
    eng = getattr(proc, '_lora', None)
    if eng is None:
        return
    try:
        _periodic_body(proc, queues, eng)
    except Exception as err:  # noqa: BLE001 -- never disturb the network loop
        now = time.monotonic()
        if now - getattr(proc, '_lora_err_at', float('-inf')) >= ERROR_LOG_S:
            proc._lora_err_at = now
            proc.logger.error('lora: periodic work failed: %s', err)


def _periodic_body(proc, queues, eng):
    for channel, uuid, msg in eng.poll():
        proc.inject_inbound(queues, channel, msg, uuid)
    payload = eng.announce_due()
    if payload is not None and proc.group is not None:
        # Signed by this node, as every group message is: a receiver takes an
        # announcement only from an envelope whose signature checks out.
        try:
            queues[proc.name].put(Message(CfgIds.network, LORA_ADDR, json.dumps(payload),
                                          to_whom=proc.group, from_whom=proc.myself,
                                          encrypt=True), block=False)
        except Full:
            now = time.monotonic()
            if now - getattr(proc, '_lora_full_warned_at', float('-inf')) >= WARN_EVERY_S:
                proc._lora_full_warned_at = now
                proc.logger.warning('lora: network queue full; announcement not queued')
        else:
            eng.announce_sent(payload)
    line = eng.summary_due()
    if line:
        proc.logger.info(line)


def _also_unicast(proc, peer_uuid, frame, process, function):
    eng = getattr(proc, '_lora', None)
    if eng is not None:
        eng.on_unicast(peer_uuid, process, function, frame)


def _fanout(proc, channel, frame, process, function, group):
    eng = getattr(proc, '_lora', None)
    if eng is not None:
        eng.on_fanout(channel, process, function, frame)


def _on_exclusion(proc, uuid, excluded):
    eng = getattr(proc, '_lora', None)
    if eng is not None:
        eng.on_exclusion(uuid, excluded)


def _handle_lora_addr(proc, queues, message):
    """A peer's announcement (from the wire, so it carries a sender), or our
    own outbound one (we are its sender), which is declined so the core sends
    it. A peer's counts only if its envelope verified."""
    sender = getattr(message, 'from_whom', None)
    if sender is None or str(sender.uuid).lower() == \
            str(getattr(getattr(proc, 'myself', None), 'uuid', '')).lower():
        return False
    eng = getattr(proc, '_lora', None)
    if eng is not None:
        obj = message.obj
        try:
            obj = json.loads(obj) if isinstance(obj, (str, bytes)) else obj
        except ValueError:
            obj = None
        eng.on_announce(sender.uuid, obj, getattr(message, 'verified', False) is True)
    return True


def _handle_lora_stats(proc, queues, message):
    """Local only: the counters, the link and the address map, to return_to."""
    if getattr(message, 'from_whom', None) is not None:
        proc.logger.warning('lora: refusing lora_stats from the wire')
        return True
    eng = getattr(proc, '_lora', None)
    body = eng.stats() if eng is not None else {'inert': True}
    rt = getattr(message, 'return_to', None)
    if rt in queues:
        queues[rt].put(Message(CfgIds.network, LORA_STATS_RESP, json.dumps(body)))
    return True


def _register(proc, proc_name):
    if proc_name != CfgIds.network:
        return
    proc.protocol.register_handler(LORA_ADDR, functools.partial(_handle_lora_addr, proc))
    proc.protocol.register_handler(LORA_STATS, functools.partial(_handle_lora_stats, proc))


EXTENSION = Extension(
    name='lora', enabled=_enabled, register_handlers=_register,
    check_config=_check_config, network_wire_verbs=(LORA_ADDR,),
    network=NetworkHooks(on_start=_on_start, periodic=_periodic,
                         also_unicast=_also_unicast, fanout=_fanout,
                         on_exclusion=_on_exclusion))
