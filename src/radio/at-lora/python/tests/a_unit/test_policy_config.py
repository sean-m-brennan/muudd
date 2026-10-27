# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import pytest

from autonomous_trust.lora._python import config, policy

BASE = dict(link_up=True, have_addr=True, quiet_s=90, ip_quiet_s=90, allowed=True,
            proto=True, frame_len=100, max_len=1832)


def test_unicast_needs_every_condition():
    assert policy.unicast(**BASE)
    for k, v in (('link_up', False), ('have_addr', False), ('quiet_s', 89.9),
                 ('allowed', False), ('proto', False), ('frame_len', 1833), ('frame_len', 0)):
        assert not policy.unicast(**{**BASE, k: v}), k


def test_fanout_broadcast_ignores_format():
    kw = dict(link_up=True, n_quiet=1, allowed=True, proto=False, frame_len=100, max_len=1800)
    assert not policy.fanout('group', **kw)
    assert policy.fanout('broadcast', **kw)
    assert not policy.fanout('broadcast', **{**kw, 'n_quiet': 0})


def test_config_defaults_and_refusals(tmp_path):
    c = config.parse('{"port": 10}')
    assert c.socket == '/run/muudd/radio.sock' and c.ip_quiet_s == 65
    assert c.allow == config.DEFAULT_ALLOW and len(c.allow) == 4
    assert c.allows('identity', 'peer_dm') and not c.allows('identity', 'presence')
    assert config.parse('{"port": 10, "allow": ["x/y"]}').allow == ('x/y',)
    for text, field in (('{"port": true}', 'port'), ('{"port": 10, "nack_gap_s": 90}', 'nack_gap_s'),
                        ('{"port": 10, "allow": ["network/lora_addr"]}', 'allow'),
                        ('{"port": 10, "ip_quiet_s": 90.5}', 'ip_quiet_s'), ('[1]', '(file)')):
        with pytest.raises(config.ConfigError) as ei:
            config.parse(text)
        assert ei.value.field == field
    assert config.load(str(tmp_path / 'absent.json')) is None
    many = '{"port": 10, "allow": [%s]}' % ', '.join('"p/f%d"' % i for i in range(33))
    with pytest.raises(config.ConfigError):
        config.parse(many)


def test_allows_does_not_truncate():
    a, b = 'a' * 64, 'b' * 64
    c = config.parse('{"port": 10, "allow": ["%s/%s"]}' % (a, b))
    assert c.allows(a, b) and not c.allows(a, b + 'c')


def test_deeply_nested_document_is_unparseable():
    def doc(depth, port='10'):
        return '{"port": ' + '[' * depth + ']' * depth + ', "port": ' + port + '}'
    assert config.parse(doc(2047)).port == 10     # 2048 levels with the object
    for depth in (2048, 5000, 100000):
        with pytest.raises(config.ConfigError) as ei:
            config.parse(doc(depth))
        assert ei.value.field == '(file)'
    # Brackets inside a string, escaped quotes included, are not nesting.
    assert config.parse('{"port": ["\\"' + '[' * 3000 + '"], "port": 10}').port == 10


def test_config_load_edges(tmp_path):
    p = tmp_path / config.FILENAME
    p.write_bytes(b'{"port": 10}')
    assert config.load(str(p)).port == 10
    p.write_bytes(b'{"port": 10}\x00 ')
    with pytest.raises(config.ConfigError) as ei:
        config.load(str(p))
    assert ei.value.field == '(file)'
    p.write_bytes(b'{"port": 10, "socket": "\xff"}')
    with pytest.raises(config.ConfigError):
        config.load(str(p))
    p.write_bytes(b'{"port": 10}' + b' ' * 16384)
    with pytest.raises(config.ConfigError) as ei:
        config.load(str(p))
    assert ei.value.field == '(file)'
    with pytest.raises(config.ConfigError) as ei:
        config.load(str(tmp_path))
    assert ei.value.field == '(file)'
