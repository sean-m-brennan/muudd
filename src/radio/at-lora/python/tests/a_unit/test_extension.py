# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import json
import logging
import queue
import types

import pytest

from autonomous_trust.lora._python import config, extension as ext


def test_the_extension_declares_its_wire_verb_and_hooks():
    e = ext.EXTENSION
    assert e.name == 'lora' and e.network_wire_verbs == ('lora_addr',)
    h = e.network
    assert h.on_start and h.periodic and h.also_unicast and h.fanout and h.on_exclusion


def test_check_config_refuses_a_bad_file_and_accepts_none(tmp_path):
    ext._check_config(str(tmp_path), logging.getLogger('t'))          # absent: fine
    (tmp_path / config.FILENAME).write_text('{"port": 1}')
    with pytest.raises(config.ConfigError):
        ext._check_config(str(tmp_path), logging.getLogger('t'))


def test_own_outbound_announcement_is_declined_so_it_is_sent():
    proc = types.SimpleNamespace(_lora=None, logger=logging.getLogger('t'))
    msg = types.SimpleNamespace(from_whom=None, obj='{}')
    assert ext._handle_lora_addr(proc, {}, msg) is False


def _me():
    return types.SimpleNamespace(uuid='00000000-0000-4000-8000-000000000000')


def test_own_signed_outbound_announcement_is_declined_so_it_is_sent():
    me = _me()
    proc = types.SimpleNamespace(_lora=None, myself=me, logger=logging.getLogger('t'))
    msg = types.SimpleNamespace(from_whom=me, obj='{}', verified=True)
    assert ext._handle_lora_addr(proc, {}, msg) is False


@pytest.mark.parametrize('verified', (True, False))
def test_a_peer_announcement_reaches_the_engine_with_its_verification(verified):
    seen = []
    eng = types.SimpleNamespace(
        on_announce=lambda u, o, v: seen.append((u, o, v)) or 'accepted')
    proc = types.SimpleNamespace(_lora=eng, myself=_me(), logger=logging.getLogger('t'))
    sender = types.SimpleNamespace(uuid='AAAAAAAA-0000-4000-8000-000000000001')
    msg = types.SimpleNamespace(from_whom=sender, verified=verified,
                                obj=json.dumps({'lora_addr': 7, 'issued_ms': 1}))
    assert ext._handle_lora_addr(proc, {}, msg) is True
    assert seen == [(sender.uuid, {'lora_addr': 7, 'issued_ms': 1}, verified)]


def test_an_announcement_without_a_verified_flag_counts_as_unverified():
    seen = []
    eng = types.SimpleNamespace(on_announce=lambda u, o, v: seen.append(v) or 'refused')
    proc = types.SimpleNamespace(_lora=eng, myself=_me(), logger=logging.getLogger('t'))
    sender = types.SimpleNamespace(uuid='AAAAAAAA-0000-4000-8000-000000000001')
    msg = types.SimpleNamespace(from_whom=sender, obj='{}')
    assert ext._handle_lora_addr(proc, {}, msg) is True
    assert seen == [False]


def test_stats_answer_locally_and_refuse_the_wire():
    class Q(list):
        def put(self, m, **kw):
            self.append(m)
    q = Q()
    eng = types.SimpleNamespace(stats=lambda: {'counters': {}})
    proc = types.SimpleNamespace(_lora=eng, logger=logging.getLogger('t'))
    assert ext._handle_lora_stats(proc, {'tool': q},
                                  types.SimpleNamespace(from_whom=None, return_to='tool'))
    assert q and q[0].function == ext.LORA_STATS_RESP
    assert ext._handle_lora_stats(proc, {'tool': q},
                                  types.SimpleNamespace(from_whom=object(), return_to='tool'))
    assert len(q) == 1


def test_hooks_do_nothing_when_inert():
    proc = types.SimpleNamespace(_lora=None)
    ext._also_unicast(proc, 'u', b'x', 'identity', 'peer_dm')
    ext._fanout(proc, 'group', b'x', 'identity', 'presence', None)
    ext._on_exclusion(proc, 'u', True)
    ext._periodic(proc, {})


class _Eng:
    def __init__(self, payload):
        self.payload, self.sent = payload, []

    def poll(self):
        return []

    def announce_due(self):
        return self.payload

    def announce_sent(self, p):
        self.sent.append(p)

    def summary_due(self):
        return None


def _periodic_proc(eng, group, myself=None):
    return types.SimpleNamespace(_lora=eng, group=group, name='network', myself=myself,
                                 logger=logging.getLogger('t'))


def test_periodic_puts_the_announcement_signed_then_reports_it_sent():
    from autonomous_trust.core._python.identity import Group, Identity
    payload = {'lora_addr': 7, 'issued_ms': 1}
    eng = _Eng(payload)
    grp = object.__new__(Group)
    me = Identity.initialize('me', 'me', '10.0.0.1')
    proc = _periodic_proc(eng, grp, me)
    q = queue.Queue()
    ext._periodic(proc, {'network': q})
    m = q.get_nowait()
    assert m.function == ext.LORA_ADDR and m.from_whom is me and m.to_whom is grp
    assert m.signature is not None
    assert json.loads(m.obj) == payload and eng.sent == [payload]


def test_a_full_network_queue_warns_once_a_minute(monkeypatch, caplog):
    from autonomous_trust.core._python.identity import Group
    clock = {'t': 1000.0}
    monkeypatch.setattr(ext.time, 'monotonic', lambda: clock['t'])
    eng = _Eng({'lora_addr': 7, 'issued_ms': 1})
    proc = _periodic_proc(eng, object.__new__(Group))
    q = queue.Queue(maxsize=1)
    q.put('busy')
    with caplog.at_level(logging.WARNING, logger='t'):
        ext._periodic(proc, {'network': q})
        clock['t'] += 30
        ext._periodic(proc, {'network': q})
        assert sum('queue full' in m for m in caplog.messages) == 1
        clock['t'] += 30
        ext._periodic(proc, {'network': q})
        assert sum('queue full' in m for m in caplog.messages) == 2


def test_periodic_without_a_group_sends_nothing_and_stays_due():
    eng = _Eng({'lora_addr': 7, 'issued_ms': 1})
    q = queue.Queue()
    ext._periodic(_periodic_proc(eng, None), {'network': q})
    assert q.empty() and eng.sent == []


def test_periodic_full_queue_stays_due():
    from autonomous_trust.core._python.identity import Group
    eng = _Eng({'lora_addr': 7, 'issued_ms': 1})
    q = queue.Queue(maxsize=1)
    q.put('busy')
    ext._periodic(_periodic_proc(eng, object.__new__(Group)), {'network': q})
    assert eng.sent == []


def test_periodic_swallows_a_raise():
    eng = _Eng(None)
    eng.poll = lambda: 1 / 0
    ext._periodic(_periodic_proc(eng, None), {})
