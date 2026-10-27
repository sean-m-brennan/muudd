# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""A lora_addr announcement crosses runtimes: one signed by a Python node
verifies in AT's C core, and one signed by a C node verifies in Python and
reaches the Python engine, which takes it.

Each side builds the envelope as its own extension does: Python puts
json.dumps of the body (with spaces), C net_msg_pack_json's compact JSON. The
envelope is the proto form LoRa cohorts use. The C half runs through AT's
native library (cffi), so the module is skipped where that is not built."""
import json
import logging
import types
import uuid

import pytest

try:
    from autonomous_trust.core._native._ffi import ffi, lib
    from autonomous_trust.core._native.identity.identity import NativeIdentity
except Exception as err:        # noqa: BLE001 - any failure to load means no native library
    pytest.skip('AT native library not available: %s' % err, allow_module_level=True)

from autonomous_trust.core._python.identity import Encryptor, Identity, Signature
from autonomous_trust.core._python.network.message import Message
from autonomous_trust.core.config import NetWireFormat

from autonomous_trust.lora._python import config, extension as ext
from autonomous_trust.lora._python.engine import Engine
from autonomous_trust.lora._python.link import LinkState

PROTO = 1                       # NET_WIRE_PROTO in network/net_wire_format.h
BODY = {'lora_addr': 0xA01, 'issued_ms': 1234}


def _c_peer(ident):
    """The C public identity a C node holds for Python node `ident`: its uuid
    and signing key, as admission leaves them."""
    p = ffi.new('public_identity_t *')
    ffi.memmove(p.uuid, uuid.UUID(str(ident.uuid)).bytes, 16)
    hexkey = ident.signature.publish()
    ffi.memmove(p.signature.public_key, bytes.fromhex(hexkey.decode('ascii')), 32)
    ffi.memmove(p.signature.public_hex, hexkey + b'\0', 65)
    return p


def _c_parse(wire, peer):
    """(rc, process, function, data, verified) from the C core's parse."""
    msg = ffi.new('net_wire_msg_t *')
    rc = lib.net_message_from_wire_fmt(wire, len(wire), peer, PROTO, msg)
    if rc != 0:
        return rc, None, None, None, False
    try:
        return (0, ffi.string(msg.process).decode(), ffi.string(msg.function).decode(),
                bytes(ffi.buffer(msg.data, msg.data_len)), bool(msg.verified))
    finally:
        lib.net_wire_msg_free(msg)


def _c_announcement(cid, body):
    """The wire bytes a C node's network process sends for its announcement."""
    pub = cid.publish()
    w = ffi.new('net_wire_msg_t *')
    w.process = b'network'
    function = ffi.new('char[]', ext.LORA_ADDR.encode())
    data = json.dumps(body, separators=(',', ':')).encode()    # net_msg_pack_json: JSON_COMPACT
    buf = ffi.new('uint8_t[]', data)
    w.function, w.data, w.data_len, w.encrypt = function, buf, len(data), True
    w.from_whom = pub._ptr[0]
    out, out_len = ffi.new('uint8_t **'), ffi.new('size_t *')
    assert lib.net_message_to_wire_fmt(w, cid._ptr, PROTO, out, out_len) == 0
    try:
        return bytes(ffi.buffer(out[0], out_len[0])), pub
    finally:
        lib.at_free(out[0])


def _py_identity(pub):
    """The Python public identity a Python node holds for C node `pub`."""
    key = lambda k: bytes(ffi.buffer(k.public_key, 32)).hex().encode('ascii')   # noqa: E731
    return Identity(str(uuid.UUID(bytes=bytes(ffi.buffer(pub._ptr.uuid, 16)))),
                    ffi.string(pub._ptr.address).decode(), ffi.string(pub._ptr.nickname).decode(),
                    Signature(key(pub._ptr.signature), public_only=True),
                    Encryptor(key(pub._ptr.encryptor), public_only=True))


class _Link:
    def state(self):
        return LinkState(True, True, True, True, 0xB02, 233)


class _Env:
    def __init__(self, me, roster):
        self.me, self.roster = me, roster

    def my_uuid(self): return self.me
    def members(self): return list(self.roster)
    def quiet_for(self, u): return 0.0
    def group_proto(self): return True
    def group_tag(self): return bytes.fromhex('a1b2c3d4')
    def wall_ms(self): return 1_700_000_000_000
    def now(self): return 1000.0


def test_a_python_signed_announcement_verifies_in_c():
    py = Identity.initialize('py', 'py', '127.0.0.1')
    msg = Message('network', ext.LORA_ADDR, json.dumps(BODY), from_whom=py, encrypt=True)
    wire = msg.to_wire(NetWireFormat.proto)
    rc, process, function, data, verified = _c_parse(wire, _c_peer(py))
    assert (rc, process, function, verified) == (0, 'network', 'lora_addr', True)
    # What the C glue hands the engine: the body, decoded (json_loadb).
    assert json.loads(data) == BODY
    # Against another node's key it does not verify, and the engine would
    # refuse it as unverified.
    other = Identity.initialize('other', 'other', '127.0.0.9')
    assert _c_parse(wire, _c_peer(other))[4] is False


def test_a_c_signed_announcement_verifies_in_python_and_the_engine_takes_it(tmp_path):
    cid = NativeIdentity('c', '127.0.0.3')
    wire, pub = _c_announcement(cid, BODY)
    sender = _py_identity(pub)
    got = Message.parse(wire, sender, wire_format=NetWireFormat.proto)
    assert (got.process, got.function, got.verified) == ('network', 'lora_addr', True)
    me = '00000000-0000-4000-8000-000000000000'
    cfg = config.parse('{"port": 10}')
    eng = Engine(cfg, _Link(), _Env(me, [str(sender.uuid)]), str(tmp_path / 'lora_peers.json'),
                 logging.getLogger('t'))
    proc = types.SimpleNamespace(_lora=eng, myself=types.SimpleNamespace(uuid=me),
                                 logger=logging.getLogger('t'))
    assert ext._handle_lora_addr(proc, {}, got) is True
    assert eng.counters['lora.announce.accepted'] == 1
    assert eng.addrmap.addr_of(str(sender.uuid)) == 0xA01
    # Checked against a key that is not the signer's, it does not verify and
    # the engine refuses it.
    impostor = Identity.initialize('other', 'other', '127.0.0.9').publish()
    forged = Message.parse(wire, impostor, wire_format=NetWireFormat.proto)
    assert forged.verified is False
    assert ext._handle_lora_addr(proc, {}, forged) is True
    assert eng.counters['lora.announce.unverified'] == 1
