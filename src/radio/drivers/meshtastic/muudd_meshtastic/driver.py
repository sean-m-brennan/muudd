# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""The driver's protocol logic, independent of how the radio is reached."""
import base64
import threading

from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import encode

from .mapping import BROADCAST, MTU, PRESETS, build_caps, firmware_ok, lora_params, received_from_packet, to_portnum

EX_CONFIG = 78


class DriverCore:
    def __init__(self, radio, write, ack_timeout=60.0):
        self.radio = radio
        self.channel = 0
        self.ack_timeout = ack_timeout
        self._write = write
        self._wlock = threading.Lock()
        self._alock = threading.Lock()
        self._acks = {}
        radio.on_receive(self._on_packet)

    def emit(self, **kw):
        data = encode(pb.DriverToRadiod(**kw).SerializeToString())
        with self._wlock:
            self._write(data)

    def hello(self):
        fw = self.radio.firmware_version()
        lora = self.radio.lora_config()
        fatal = ""
        if not firmware_ok(fw):
            fatal = f"firmware {fw} older than 2.7.15"
        elif lora.get("region") in (None, "", "UNSET"):
            fatal = "radio region is not set"
        try:
            caps = build_caps(self.radio.node_num(), fw, lora, fatal)
        except KeyError:
            fallback = {"region": lora.get("region", ""), "use_preset": True, "modem_preset": "LONG_FAST"}
            caps = build_caps(self.radio.node_num(), fw, fallback,
                              fatal or f"unknown modem preset {lora.get('modem_preset')}")
        self.emit(hello=caps)
        return caps

    def configure(self, conf):
        settings = dict(conf.settings)
        name = settings.get("channel", "muudd")
        preset = settings.get("preset")
        apply_preset = conf.apply_radio_config and preset
        if apply_preset and preset not in PRESETS:
            self._status_down(f"unknown modem preset {preset}")
            return EX_CONFIG
        idx = self.radio.channel_index(name)
        key = None
        if idx is None:
            if not conf.apply_radio_config:
                self._status_down(f"channel {name} is missing on the radio")
                return EX_CONFIG
            psk = settings.get("channel_psk")
            if not psk:
                self._status_down("settings.channel_psk is required to create the channel")
                return EX_CONFIG
            try:
                key = base64.b64decode(psk, validate=True)
            except ValueError:
                key = None
            if key is None or len(key) not in (1, 16, 32):
                self._status_down("settings.channel_psk must be base64 of a 1, 16 or 32-byte key")
                return EX_CONFIG
        if key is not None:
            idx = self.radio.add_channel(name, key)
        if apply_preset:
            self.radio.set_preset(preset)
        self.channel = idx
        return None

    def send(self, s):
        try:
            portnum = to_portnum(s.port)
        except ValueError:
            self._result(s.req_id, pb.REFUSED, pb.NO_ROUTE)
            return
        if len(s.payload) > MTU:
            self._result(s.req_id, pb.REFUSED, pb.TOO_BIG)
            return
        ackable = s.want_ack and s.dest != BROADCAST
        state = None
        if ackable:
            state = {"pid": 0, "sent": False, "pending": None, "timer": None}
            with self._alock:
                self._acks[s.req_id] = state
        on_ack = (lambda ok, req=s.req_id: self._ack(req, ok)) if ackable else None
        try:
            pid = self.radio.send(bytes(s.payload), s.dest, portnum, ackable, self.channel, on_ack)
        except Exception:
            with self._alock:
                self._acks.pop(s.req_id, None)
            self._result(s.req_id, pb.REFUSED, pb.RADIO_DOWN)
            return
        self._result(s.req_id, pb.SENT, packet_id=pid)
        if s.want_ack and not ackable:
            self._result(s.req_id, pb.NO_ACK, packet_id=pid)
            return
        if not ackable:
            return
        with self._alock:
            state["pid"] = pid
            state["sent"] = True
            pending = state["pending"]
            if pending is None:
                timer = threading.Timer(self.ack_timeout, self._ack, args=(s.req_id, False))
                timer.daemon = True
                state["timer"] = timer
        if pending is not None:
            self._finish(s.req_id, pending)
        else:
            timer.start()

    def _ack(self, req, ok):
        status = pb.ACKED if ok else pb.NO_ACK
        with self._alock:
            state = self._acks.get(req)
            if state is None:
                return
            if not state["sent"]:
                state["pending"] = status
                return
        self._finish(req, status)

    def _finish(self, req, status):
        with self._alock:
            state = self._acks.pop(req, None)
        if state is None:
            return
        if state["timer"]:
            state["timer"].cancel()
        self._result(req, status, packet_id=state["pid"])

    def status(self):
        lora = self.radio.lora_config()
        try:
            _, bw, _, preset = lora_params(lora)
        except KeyError:
            bw, preset = 0, str(lora.get("modem_preset"))
        settings = pb.RadioSettings(region=lora.get("region", ""), preset=preset, bandwidth_khz=bw // 1000)
        self.emit(status=pb.DriverStatus(state=pb.STATE_UP, radio_settings=settings))

    def _status_down(self, reason):
        self.emit(status=pb.DriverStatus(state=pb.STATE_DOWN, reason=reason))

    def _result(self, req, status, reason=pb.REASON_NONE, packet_id=0):
        self.emit(send_result=pb.SendResult(req_id=req, status=status, reason=reason, packet_id=packet_id))

    def _on_packet(self, pkt):
        r = received_from_packet(pkt)
        if r is not None:
            self.emit(received=r)
