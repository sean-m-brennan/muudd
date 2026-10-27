# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The lora protocol adapter (kind: scenario). Twin of c/lora_adapter.c.

Reads fixtures.lora.op, runs the extension's pure units, and asserts
expected_state.host. Every op is deterministic; both adapters must assert the
same values.
"""
import json
import os
import tempfile

from autonomous_trust.lora._python import config, policy
from autonomous_trust.lora._python import frame as fr
from autonomous_trust.lora._python.addrmap import AddrMap
from autonomous_trust.lora._python.reasm import Reassembler, SendHold

_TYPES = {'peer': fr.PEER, 'group': fr.GROUP, 'bcast': fr.BCAST, 'nack': fr.NACK}
_NAMES = {v: k for k, v in _TYPES.items()}


def _pattern(n):
    return bytes(i % 256 for i in range(n))


class LoraAdapter:
    def __init__(self, corpus_root=None) -> None:
        self.corpus_root = corpus_root

    def run_wire_vector(self, case):  # noqa: ARG002
        raise NotImplementedError('lora adapter handles kind:scenario only')

    run_crypto_vector = run_agreement_vector = run_negative = run_wire_vector

    def run_scenario(self, case) -> None:
        spec = case.data
        fx = (spec.get('fixtures') or {}).get('lora')
        assert isinstance(fx, dict), 'missing fixtures.lora'
        exp = (spec.get('expected_state') or {}).get('host') or {}
        handler = getattr(self, '_op_' + str(fx.get('op')), None)
        assert handler is not None, 'unknown lora op %r' % fx.get('op')
        handler(fx, exp)

    @staticmethod
    def _op_fragment(fx, exp):
        msg = _pattern(fx['msg_len'])
        tag = bytes.fromhex(fx['tag_hex']) if fx.get('tag_hex') else None
        try:
            frames = fr.fragment(msg, _TYPES[fx['type']], fx['msg_id'], fx['mtu'],
                                 fx['max_fragments'], tag)
        except fr.FrameError:
            assert exp.get('error') is True, 'refused, but the scenario expects frames'
            return
        assert exp.get('error') is not True, 'expected a refusal'
        assert [len(f) for f in frames] == exp['lengths'], [len(f) for f in frames]
        hdr = fr.HEADER[_TYPES[fx['type']]]
        assert [f[:hdr].hex() for f in frames] == exp['headers_hex']
        assert b''.join(f[hdr:] for f in frames) == msg

    @staticmethod
    def _op_decode(fx, exp):
        f = fr.decode(bytes.fromhex(fx['hex']))
        assert _NAMES[f.type] == exp['type']
        assert f.msg_id == exp['msg_id']
        if f.type == fr.NACK:
            assert f.missing == exp['missing']
            return
        assert (f.idx, f.count) == (exp['idx'], exp['count'])
        assert f.payload.hex() == exp['payload_hex']
        if 'tag_hex' in exp:
            assert f.tag.hex() == exp['tag_hex']

    @staticmethod
    def _op_decode_all(fx, exp):
        refused = 0
        for h in fx['hexes']:
            try:
                fr.decode(bytes.fromhex(h))
            except fr.FrameError:
                refused += 1
        assert (refused == len(fx['hexes'])) == exp['all_refused']

    @staticmethod
    def _op_nack(fx, exp):
        try:
            got = fr.encode_nack(fx['msg_id'], fx['missing'])
        except fr.FrameError:
            assert exp.get('error') is True
            return
        assert got.hex() == exp['hex']

    @staticmethod
    def _op_reasm_trace(fx, exp):
        r = Reassembler(fx['nack_gap_s'], fx['reassembly_s'])
        got = []
        for ev in fx['events']:
            if ev.get('tick'):
                got.append('tick' + ''.join(' nack:%d:%d:%d' % n for n in r.tick(ev['at'])))
                continue
            status, msg = r.add(ev['src'], fr.decode(bytes.fromhex(ev['hex'])), ev['at'])
            got.append(status if msg is None else '%s %s' % (status, msg.hex()))
        assert got == exp['outputs'], got
        for k, v in (exp.get('stats') or {}).items():
            assert r.stats[k] == v, (k, r.stats[k])

    @staticmethod
    def _op_hold_trace(fx, exp):
        h = SendHold(fx['hold_s'])
        got = []
        for ev in fx['events']:
            if 'put' in ev:
                p = ev['put']
                h.put(p['dest'], p['msg_id'], [bytes.fromhex(x) for x in p['frames_hex']], ev['at'])
                got.append('held')
            else:
                s = ev['serve']
                frames = h.serve(s['dest'], s['msg_id'], s['missing'], ev['at'])
                got.append('served ' + (','.join(f.hex() for f in frames) or 'none'))
        assert got == exp['outputs'], got

    @staticmethod
    def _op_addrmap_trace(fx, exp):
        m, members, got = AddrMap(), None, []
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, 'addrmap.json')
            for ev in fx['events']:
                if 'members' in ev:
                    members = set(u.lower() for u in ev['members'])
                    got.append('ok')
                elif 'announce' in ev:
                    u, a, t = ev['announce']
                    got.append(m.announce(u, a, t, None if members is None else (lambda x: x in members)))
                elif 'addr_of' in ev:
                    a = m.addr_of(ev['addr_of'])
                    got.append('none' if a is None else 'addr %d' % a)
                elif 'uuid_of' in ev:
                    u = m.uuid_of(ev['uuid_of'])
                    got.append('none' if u is None else 'uuid ' + u)
                elif 'save' in ev:
                    try:
                        m.save(path, ev['save'])
                        got.append('saved')
                    except OSError:
                        got.append('save failed')
                elif 'saved_file' in ev:
                    # The file's JSON, keys sorted and no spaces, as the C adapter prints it.
                    try:
                        with open(path) as f:
                            got.append('file ' + json.dumps(json.load(f), sort_keys=True,
                                                            separators=(',', ':')))
                    except (OSError, ValueError):
                        got.append('no file')
                elif 'load' in ev:
                    if os.path.exists(path):
                        os.remove(path)
                    if 'text' in ev['load']:
                        with open(path, 'w') as f:
                            f.write(ev['load']['text'])
                    m, self_ms, err = AddrMap.load(path)
                    got.append('load failed' if err else
                               'loaded self=%d skipped=%d' % (self_ms, len(m.skipped)))
                else:
                    got.append('removed' if m.remove(ev['remove']) else 'absent')
        assert got == exp['outputs'], got

    @staticmethod
    def _op_policy(fx, exp):
        bad = []
        for row in fx['rows']:
            if row['kind'] == 'unicast':
                got = policy.unicast(row['link_up'], row['have_addr'], row['quiet_s'],
                                     row['ip_quiet_s'], row['allowed'], row['proto'],
                                     row['frame_len'], row['max_len'])
            else:
                got = policy.fanout(row['channel'], row['link_up'], row['n_quiet'],
                                    row['allowed'], row['proto'], row['frame_len'], row['max_len'])
            if got != row['send']:
                bad.append(row)
        assert (not bad) == exp['all_rows_match'], bad

    @staticmethod
    def _op_config(fx, exp):
        bad = []
        for c in fx['cases']:
            try:
                cfg = config.parse(c['json'])
            except config.ConfigError as err:
                if c.get('field') != err.field:
                    bad.append((c, err.field))
                continue
            if not c.get('ok'):
                bad.append((c, 'accepted'))
                continue
            for k, v in (c.get('values') or {}).items():
                got = list(cfg.allow) if k == 'allow' else getattr(cfg, k)
                if got != v:
                    bad.append((c, k, got))
        assert (not bad) == exp['all_cases_match'], bad
