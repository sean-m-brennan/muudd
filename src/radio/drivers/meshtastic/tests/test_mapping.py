# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
import pytest

from muudd_meshtastic import mapping as mp
from muudd_radio import radio_pb2 as pb


def test_port_mapping_both_ways():
    assert mp.to_portnum(1) == 1
    assert mp.to_portnum(2) == 258
    assert mp.to_portnum(16) == 272
    assert mp.to_portnum(255) == 511
    for bad in (0, 256, -1):
        with pytest.raises(ValueError):
            mp.to_portnum(bad)
    assert mp.from_portnum(1) == 1
    assert mp.from_portnum(272) == 16
    assert mp.from_portnum(258) == 2
    assert mp.from_portnum(257) is None      # ATAK forwarder, not ours
    assert mp.from_portnum(67) is None
    assert mp.from_portnum(None) is None


def test_firmware():
    assert mp.parse_firmware("2.7.15.567b8ea") == (2, 7, 15)
    assert mp.parse_firmware("2.8") == (2, 8, 0)
    assert mp.firmware_ok("2.7.15")
    assert mp.firmware_ok("3.0.0")
    assert not mp.firmware_ok("2.7.14.abcdef")
    assert not mp.firmware_ok("garbage")


def test_lora_params():
    assert mp.lora_params({"use_preset": True, "modem_preset": "LONG_TURBO"}) == (11, 500_000, 4, "LONG_TURBO")
    assert mp.lora_params({"use_preset": False, "bandwidth": 62, "spread_factor": 12, "coding_rate": 8}) == (
        12, 62_500, 4, "CUSTOM")
    assert mp.lora_params({"use_preset": False, "bandwidth": 500, "spread_factor": 7, "coding_rate": 5})[:3] == (
        7, 500_000, 1)
    with pytest.raises(KeyError):
        mp.lora_params({"use_preset": True, "modem_preset": "WARP_SPEED"})


def test_build_caps():
    caps = mp.build_caps(0xA0B0, "2.7.15", {"region": "US", "use_preset": True, "modem_preset": "SHORT_TURBO"})
    assert caps.name == "meshtastic" and caps.reach == pb.MESH and caps.mtu == 233
    assert caps.lora.spreading_factor == 7 and caps.lora.bandwidth_hz == 500_000
    assert caps.radio_settings.bandwidth_khz == 500 and caps.radio_settings.preset == "SHORT_TURBO"
    assert caps.overhead_bytes == 16 and caps.supports_ack and caps.reports_link_quality
    assert caps.bitrate_bps == mp.bitrate(7, 500_000, 1) > 0


def test_received_from_packet():
    r = mp.received_from_packet({
        "from": 0xB1, "to": 0xFFFFFFFF, "id": 99, "portnum": 272, "payload": b"t",
        "rx_rssi": -91, "rx_snr": 4.5, "hop_start": 3, "hop_limit": 1, "pki_encrypted": False, "rx_time": 1700000000,
    })
    assert r.src == 0xB1 and r.dest == 0xFFFFFFFF and r.port == 16 and r.payload == b"t"
    assert r.meta.packet_id == 99 and r.meta.rssi_dbm == -91 and r.meta.has_link_quality
    assert r.meta.hops == 2 and r.meta.has_hops
    assert r.meta.protection == pb.CHANNEL_KEY and r.meta.rx_time_ms == 1700000000000
    dm = mp.received_from_packet({"from": 1, "to": 2, "id": 1, "portnum": 1, "payload": b"hi", "pki_encrypted": True})
    assert dm.port == 1 and dm.meta.protection == pb.PUBLIC_KEY_DM and not dm.meta.has_link_quality
    assert mp.received_from_packet({"from": 1, "to": 2, "id": 1, "portnum": 67, "payload": b""}) is None
