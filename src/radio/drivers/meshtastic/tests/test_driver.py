# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
import base64
import time

from muudd_meshtastic.driver import EX_CONFIG, DriverCore
from muudd_meshtastic.fake_radio import FakeRadio
from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import FrameReader

BROADCAST = 0xFFFFFFFF


class Out:
    def __init__(self):
        self.reader = FrameReader()
        self.msgs = []

    def write(self, data):
        self.msgs += [pb.DriverToRadiod.FromString(f) for f in self.reader.feed(data)]

    def of(self, kind):
        return [getattr(m, kind) for m in self.msgs if m.WhichOneof("msg") == kind]

    def results(self, req):
        return [r for r in self.of("send_result") if r.req_id == req]


def core(**radio_kw):
    out = Out()
    radio = FakeRadio(**radio_kw)
    return DriverCore(radio, out.write), radio, out


def wait(pred, timeout=2.0):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if pred():
            return True
        time.sleep(0.01)
    return False


def test_hello_ok():
    c, _, out = core()
    caps = c.hello()
    assert caps.fatal == "" and caps.mtu == 233 and caps.radio_settings.bandwidth_khz == 500
    assert out.of("hello")[0].node_addr == 0xA0B0


def test_hello_old_firmware_and_unset_region():
    c, _, _ = core(firmware="2.7.14.abc")
    assert "older than 2.7.15" in c.hello().fatal
    c, _, _ = core(region="UNSET")
    assert "region" in c.hello().fatal


def test_configure_missing_channel_is_fatal():
    c, _, out = core(channels=())
    assert c.configure(pb.Configure(settings={"channel": "muudd"})) == EX_CONFIG
    st = out.of("status")[-1]
    assert st.state == pb.STATE_DOWN and "missing" in st.reason


def test_configure_apply_creates_channel_and_preset():
    c, radio, _ = core(channels=("primary",))
    psk = base64.b64encode(bytes(32)).decode()
    conf = pb.Configure(settings={"channel": "muudd", "channel_psk": psk, "preset": "SHORT_TURBO"},
                        apply_radio_config=True)
    assert c.configure(conf) is None
    assert ("channel", "muudd", bytes(32)) in radio.applied
    assert ("preset", "SHORT_TURBO") in radio.applied
    assert c.channel == 1


def test_send_maps_ports_and_channel():
    c, radio, out = core(channels=("primary", "muudd"))
    c.configure(pb.Configure(settings={"channel": "muudd"}))
    for req, port in ((1, 16), (2, 1), (3, 2)):
        c.send(pb.Send(req_id=req, dest=BROADCAST, port=port, payload=b"x"))
    assert [s["portnum"] for s in radio.sent] == [272, 1, 258]
    assert all(s["channel"] == 1 for s in radio.sent)
    assert [r.status for r in out.results(1)] == [pb.SENT]


def test_ack_order_even_when_ack_is_synchronous():
    class SyncRadio(FakeRadio):
        def send(self, payload, dest, portnum, want_ack, channel_index, on_ack):
            pid = super().send(payload, dest, portnum, False, channel_index, None)
            if on_ack:
                on_ack(True)
            return pid

    out = Out()
    c = DriverCore(SyncRadio(), out.write)
    c.send(pb.Send(req_id=5, dest=0xB2, port=16, payload=b"x", want_ack=True))
    assert wait(lambda: len(out.results(5)) == 2)
    assert [r.status for r in out.results(5)] == [pb.SENT, pb.ACKED]


def test_async_ack_and_broadcast_ack():
    c, _, out = core()
    c.send(pb.Send(req_id=6, dest=0xB2, port=16, payload=b"x", want_ack=True))
    assert wait(lambda: len(out.results(6)) == 2)
    assert [r.status for r in out.results(6)] == [pb.SENT, pb.ACKED]
    c.send(pb.Send(req_id=7, dest=BROADCAST, port=16, payload=b"x", want_ack=True))
    assert [r.status for r in out.results(7)] == [pb.SENT, pb.NO_ACK]


def test_ack_timeout():
    out = Out()
    c = DriverCore(FakeRadio(ack=None), out.write, ack_timeout=0.2)
    c.send(pb.Send(req_id=8, dest=0xB2, port=16, payload=b"x", want_ack=True))
    assert wait(lambda: len(out.results(8)) == 2, timeout=2)
    assert out.results(8)[-1].status == pb.NO_ACK


def test_refusals():
    c, radio, out = core()
    c.send(pb.Send(req_id=9, dest=BROADCAST, port=16, payload=b"x" * 234))
    c.send(pb.Send(req_id=10, dest=BROADCAST, port=0, payload=b"x"))
    assert out.results(9)[0].reason == pb.TOO_BIG
    assert out.results(10)[0].reason == pb.NO_ROUTE
    assert radio.sent == []


def test_received_mapping():
    c, radio, out = core()
    radio.deliver({"from": 0xB1, "to": BROADCAST, "id": 3, "portnum": 272, "payload": b"t", "rx_rssi": -80,
                   "rx_snr": 5.0, "hop_start": 3, "hop_limit": 1, "pki_encrypted": False, "rx_time": None})
    radio.deliver({"from": 0xB1, "to": 0xA0B0, "id": 4, "portnum": 1, "payload": b"hi", "pki_encrypted": True})
    radio.deliver({"from": 0xB1, "to": BROADCAST, "id": 5, "portnum": 67, "payload": b""})
    rx = out.of("received")
    assert [r.port for r in rx] == [16, 1]
    assert rx[0].meta.hops == 2 and rx[1].meta.protection == pb.PUBLIC_KEY_DM


def _conf(**settings):
    return pb.Configure(settings=settings, apply_radio_config=True)


def test_hello_unknown_preset_is_fatal_with_fallback_caps():
    c, _, out = core(preset="WARP_SPEED")
    caps = c.hello()
    assert "unknown modem preset WARP_SPEED" in caps.fatal
    assert caps.radio_settings.preset == "LONG_FAST" and caps.lora.bandwidth_hz == 250_000


def test_configure_rejects_bad_psk_and_preset_without_partial_changes():
    good = base64.b64encode(bytes(32)).decode()
    for settings in ({"channel": "muudd"},
                     {"channel": "muudd", "channel_psk": "abc"},
                     {"channel": "muudd", "channel_psk": ""},
                     {"channel": "muudd", "channel_psk": base64.b64encode(b"").decode() + "!!!!"},
                     {"channel": "muudd", "channel_psk": base64.b64encode(bytes(5)).decode()},
                     {"channel": "muudd", "channel_psk": good, "preset": "WARP_SPEED"}):
        c, radio, out = core(channels=("primary",))
        assert c.configure(_conf(**settings)) == EX_CONFIG, settings
        assert radio.applied == [], settings
        assert out.of("status")[-1].state == pb.STATE_DOWN
    c, radio, out = core(channels=("primary",))
    assert c.configure(_conf(channel="muudd", channel_psk="abc")) == EX_CONFIG
    assert "base64" in out.of("status")[-1].reason


def test_status_reports_preset_bandwidth():
    c, _, out = core(preset="SHORT_TURBO")
    c.status()
    st = out.of("status")[-1]
    assert st.state == pb.STATE_UP and st.radio_settings.preset == "SHORT_TURBO"
    assert st.radio_settings.bandwidth_khz == 500


def test_fake_radio_survives_malformed_datagrams(tmp_path):
    import socket
    from muudd_meshtastic.fake_radio import MEDIUM_HDR
    radio = FakeRadio(medium=tmp_path)
    got = []
    radio.on_receive(got.append)
    s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    path = str(tmp_path / f"{0xA0B0:08x}.sock")
    s.sendto(b"short", path)
    s.sendto(MEDIUM_HDR.pack(1, BROADCAST, 300, 1) + b"x", path)
    s.sendto(MEDIUM_HDR.pack(1, BROADCAST, 16, 2) + b"ok", path)
    assert wait(lambda: len(got) == 1)
    assert got[0]["payload"] == b"ok"
    radio.close()
