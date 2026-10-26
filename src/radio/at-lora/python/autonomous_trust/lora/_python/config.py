# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""<cfg_dir>/lora.cfg.json. Absent: the extension is loaded and inert.
Anything malformed refuses the network start, naming the field. Twin of C
lora_config; the two agree on every edge:

- integers only (booleans and floats are refused);
- an integer outside int64, or a non-finite number, makes the file
  unparseable, as does a string holding U+0000 or a lone surrogate;
- a duplicate key keeps its last value;
- errors are reported in this order: unknown key (document order), port, the
  numeric fields, the nack_gap_s/reassembly_s pair, socket, allow;
- field "(file)" means the text is not a JSON object, or the file is
  unreadable or over 16 KiB.
"""
import json
import re
from dataclasses import dataclass

FILENAME = 'lora.cfg.json'
MAX_BYTES = 16384
DEFAULT_ALLOW = ('identity/group_key_update', 'identity/group_partition_probe',
                 'identity/group_partition_response', 'identity/peer_dm')
_RANGES = {'ip_quiet_s': (10, 86400, 90), 'max_fragments': (1, 16, 8),
           'nack_gap_s': (1, 600, 20), 'nack_hold_s': (1, 3600, 120),
           'reassembly_s': (2, 3600, 90), 'announce_s': (30, 86400, 600)}
_VERB = re.compile(r'[a-z0-9_]{1,64}/[a-z0-9_]{1,64}')
_KNOWN = {'socket', 'port', 'allow', *_RANGES}
_INT64 = (-2 ** 63, 2 ** 63 - 1)


class ConfigError(ValueError):
    def __init__(self, field, why):
        super().__init__('%s: %s' % (field, why))
        self.field, self.why = field, why


@dataclass(frozen=True)
class LoraConfig:
    socket: str
    port: int
    ip_quiet_s: int
    max_fragments: int
    nack_gap_s: int
    nack_hold_s: int
    reassembly_s: int
    announce_s: int
    allow: tuple

    def allows(self, process, function) -> bool:
        return '%s/%s' % (process, function) in self.allow


def _int(doc, key, lo, hi, default):
    v = doc.get(key, default)
    if type(v) is not int or not lo <= v <= hi:
        raise ConfigError(key, 'must be an integer from %d to %d' % (lo, hi))
    return v


def _parse_int(text):
    v = int(text)
    if not _INT64[0] <= v <= _INT64[1]:
        raise ValueError('integer out of range')
    return v


def _parse_float(text):
    v = float(text)
    if v in (float('inf'), float('-inf')):
        raise ValueError('real number out of range')
    return v


def _refuse_constant(name):
    raise ValueError(name)


def _strings_ok(doc):
    """No string or key holds U+0000 or a surrogate (iterative: no recursion limit)."""
    todo = [doc]
    while todo:
        v = todo.pop()
        if isinstance(v, dict):
            todo.extend(v.keys())
            todo.extend(v.values())
        elif isinstance(v, list):
            todo.extend(v)
        elif isinstance(v, str):
            if any(c == '\x00' or '\ud800' <= c <= '\udfff' for c in v):
                return False
    return True


def parse(text) -> LoraConfig:
    try:
        doc = json.loads(text, parse_int=_parse_int, parse_float=_parse_float,
                         parse_constant=_refuse_constant)
    except (ValueError, RecursionError):
        raise ConfigError('(file)', 'not a JSON object') from None
    if not isinstance(doc, dict) or not _strings_ok(doc):
        raise ConfigError('(file)', 'not a JSON object')
    for key in doc:
        if key not in _KNOWN:
            raise ConfigError(key, 'unknown field')
    if 'port' not in doc:
        raise ConfigError('port', 'required')
    port = _int(doc, 'port', 3, 255, None)
    vals = {k: _int(doc, k, lo, hi, d) for k, (lo, hi, d) in _RANGES.items()}
    if vals['nack_gap_s'] >= vals['reassembly_s']:
        raise ConfigError('nack_gap_s', 'must be less than reassembly_s')
    sock = doc.get('socket', '/run/muudd/radio.sock')
    if not isinstance(sock, str) or not sock or len(sock.encode()) > 107:
        raise ConfigError('socket', 'must be a non-empty path of at most 107 bytes')
    allow = doc.get('allow', list(DEFAULT_ALLOW))
    if not isinstance(allow, list) or len(allow) > 32:
        raise ConfigError('allow', 'must be a list of at most 32 process/function names')
    for v in allow:
        if not isinstance(v, str) or not _VERB.fullmatch(v) or v == 'network/lora_addr':
            raise ConfigError('allow', 'entries are process/function of [a-z0-9_], '
                                       'never network/lora_addr')
    if len(set(allow)) != len(allow):
        raise ConfigError('allow', 'entries must be distinct')
    return LoraConfig(socket=sock, port=port, allow=tuple(allow), **vals)


def load(path):
    """The configuration, or None when the file is absent (inert)."""
    try:
        with open(path, 'rb') as f:
            raw = f.read(MAX_BYTES + 1)
    except FileNotFoundError:
        return None
    except OSError:
        raise ConfigError('(file)', 'unreadable') from None
    if len(raw) > MAX_BYTES:
        raise ConfigError('(file)', 'larger than 16 KiB')
    try:
        text = raw.decode('utf-8')
    except UnicodeDecodeError:
        raise ConfigError('(file)', 'not a JSON object') from None
    return parse(text)
