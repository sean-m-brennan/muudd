# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""SerialRadio against a fake interface object, so no radio is needed."""
import types

import pytest

pytest.importorskip("meshtastic")
from meshtastic.protobuf import channel_pb2, config_pb2, localonly_pb2  # noqa: E402

from muudd_meshtastic.serial_radio import SerialRadio, normalize  # noqa: E402


class FakeNode:
    def __init__(self):
        self.localConfig = localonly_pb2.LocalConfig()
        self.localConfig.lora.region = config_pb2.Config.LoRaConfig.RegionCode.Value("US")
        self.localConfig.lora.use_preset = True
        self.localConfig.lora.modem_preset = config_pb2.Config.LoRaConfig.ModemPreset.Value("LONG_TURBO")
        self.channels = []
        for i in range(8):
            ch = channel_pb2.Channel(index=i)
            ch.role = channel_pb2.Channel.Role.DISABLED
            self.channels.append(ch)
        self.channels[0].role = channel_pb2.Channel.Role.PRIMARY
        self.channels[0].settings.name = "primary"
        self.written = []

    def getDisabledChannel(self):
        return next((c for c in self.channels if c.role == channel_pb2.Channel.Role.DISABLED), None)

    def writeChannel(self, index):
        self.written.append(("channel", index))

    def writeConfig(self, name):
        self.written.append(("config", name))


class FakeIface:
    def __init__(self, devPath=None):
        self.devPath = devPath
        self.myInfo = types.SimpleNamespace(my_node_num=0xA0B0)
        self.metadata = types.SimpleNamespace(firmware_version="2.7.15.abc")
        self.localNode = FakeNode()
        self.sent = []
        self.closed = False

    def sendData(self, data, destinationId=None, portNum=None, wantAck=False, channelIndex=0, onResponse=None,
                 onResponseAckPermitted=False):
        self.sent.append(dict(data=data, dest=destinationId, portnum=portNum, want_ack=wantAck,
                              channel=channelIndex, on_response=onResponse, ack_permitted=onResponseAckPermitted))
        return types.SimpleNamespace(id=4242)

    def close(self):
        self.closed = True


def make():
    return SerialRadio("/dev/fake", iface_factory=FakeIface)


def test_identity_and_config():
    r = make()
    assert r.node_num() == 0xA0B0
    assert r.firmware_version() == "2.7.15.abc"
    lora = r.lora_config()
    assert lora["region"] == "US" and lora["use_preset"] and lora["modem_preset"] == "LONG_TURBO"


def test_channels():
    r = make()
    assert r.channel_index("primary") == 0
    assert r.channel_index("muudd") is None
    idx = r.add_channel("muudd", bytes(32))
    assert idx == 1 and r.channel_index("muudd") == 1
    assert ("channel", 1) in r.iface.localNode.written


def test_set_preset():
    r = make()
    r.set_preset("SHORT_TURBO")
    assert r.lora_config()["modem_preset"] == "SHORT_TURBO"
    assert ("config", "lora") in r.iface.localNode.written


def test_send_and_ack():
    r = make()
    results = []
    pid = r.send(b"x", 0xB2, 272, True, 1, results.append)
    assert pid == 4242
    call = r.iface.sent[0]
    assert call["dest"] == 0xB2 and call["portnum"] == 272 and call["want_ack"] and call["channel"] == 1
    assert call["ack_permitted"]
    call["on_response"]({"decoded": {"routing": {"errorReason": "NONE"}}})
    call["on_response"]({"decoded": {"routing": {"errorReason": "MAX_RETRANSMIT"}}})
    assert results == [True, False]


def test_receive_normalised():
    r = make()
    got = []
    r.on_receive(got.append)
    r._on_receive({"from": 0xB1, "to": 0xFFFFFFFF, "id": 7, "rxRssi": -90, "rxSnr": 3.5, "hopStart": 3,
                   "hopLimit": 2, "rxTime": 1700000000, "decoded": {"portnum": "PRIVATE_APP", "payload": b"p"}},
                  r.iface)
    r._on_receive({"from": 1, "to": 2, "decoded": {"portnum": "TEXT_MESSAGE_APP", "payload": b"hi"},
                   "pkiEncrypted": True}, r.iface)
    r._on_receive({"from": 1, "to": 2, "decoded": {"portnum": 272, "payload": b"n"}}, r.iface)
    r._on_receive({"from": 1, "to": 2, "decoded": {"portnum": "PRIVATE_APP", "payload": b"x"}}, object())
    assert [g["portnum"] for g in got] == [256, 1, 272]
    assert got[0]["rx_rssi"] == -90 and got[0]["hop_start"] == 3 and got[0]["rx_time"] == 1700000000
    assert got[1]["pki_encrypted"] is True


def test_normalize_skips_undecoded():
    assert normalize({"from": 1, "to": 2, "encrypted": b"..."}) is None


def test_normalize_hops_default_zero_limit():
    from muudd_meshtastic import mapping
    n = normalize({"from": 1, "to": 2, "decoded": {"portnum": 272, "payload": b"x"}, "hopStart": 3})
    assert n["hop_start"] == 3 and n["hop_limit"] == 0
    meta = mapping.received_from_packet(n).meta
    assert meta.has_hops and meta.hops == 3
    n2 = normalize({"from": 1, "to": 2, "decoded": {"portnum": "PRIVATE_APP", "payload": b"x"}})
    assert n2["hop_start"] is None and n2["hop_limit"] is None


def test_normalize_carries_channel():
    n = normalize({"from": 1, "to": 2, "channel": 3, "decoded": {"portnum": 272, "payload": b"x"}})
    assert n["channel"] == 3
    # the library leaves the field out at its default of 0
    assert normalize({"from": 1, "to": 2, "decoded": {"portnum": 272, "payload": b"x"}})["channel"] == 0
