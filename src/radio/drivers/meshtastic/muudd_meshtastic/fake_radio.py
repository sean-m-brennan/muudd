# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""A Meshtastic radio stand-in for tests.

With a medium directory it speaks the stub driver's shared-medium format
(src, dest, port, packet id, payload over Unix datagram sockets named by node
address), so the driver conformance suite can inject frames. Without one it
only records what was sent."""
import os
import random
import socket
import struct
import threading
import time

from .mapping import BROADCAST, from_portnum, to_portnum

MEDIUM_HDR = struct.Struct(">IIHI")


class FakeRadio:
    def __init__(self, node=0xA0B0, firmware="2.7.15", preset="LONG_TURBO", region="US", channels=("muudd",),
                 medium=None, ack=True):
        self.node = node
        self.firmware = firmware
        self.preset = preset
        self.region = region
        self.channels = list(channels)
        self.medium = str(medium) if medium else None
        self.ack = ack
        self.sent = []
        self.applied = []
        self._cb = None
        self._next_id = random.randint(1, 2 ** 31)
        self._closed = False
        self._sock = None
        if self.medium:
            self._path = os.path.join(self.medium, f"{node:08x}.sock")
            if os.path.exists(self._path):
                os.unlink(self._path)
            self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
            self._sock.bind(self._path)
            self._sock.settimeout(0.1)
            threading.Thread(target=self._listen, daemon=True).start()

    def node_num(self):
        return self.node

    def firmware_version(self):
        return self.firmware

    def lora_config(self):
        return {"region": self.region, "use_preset": True, "modem_preset": self.preset}

    def channel_index(self, name):
        return self.channels.index(name) if name in self.channels else None

    def add_channel(self, name, psk):
        self.channels.append(name)
        self.applied.append(("channel", name, psk))
        return len(self.channels) - 1

    def set_preset(self, name):
        self.preset = name
        self.applied.append(("preset", name))

    def send(self, payload, dest, portnum, want_ack, channel_index, on_ack):
        self._next_id = (self._next_id + 1) & 0xFFFFFFFF
        pid = self._next_id
        self.sent.append({"payload": payload, "dest": dest, "portnum": portnum, "want_ack": want_ack,
                          "channel": channel_index, "id": pid})
        reachable = dest != BROADCAST
        if self.medium:
            data = MEDIUM_HDR.pack(self.node, dest, from_portnum(portnum) or 0, pid) + payload
            for name in os.listdir(self.medium):
                path = os.path.join(self.medium, name)
                if name.endswith(".sock") and path != self._path:
                    try:
                        self._sock.sendto(data, path)
                    except OSError:
                        pass
            reachable = reachable and os.path.exists(os.path.join(self.medium, f"{dest:08x}.sock"))
        if on_ack is not None and self.ack is not None:
            threading.Timer(0.01, on_ack, args=(bool(self.ack and reachable),)).start()
        return pid

    def on_receive(self, cb):
        self._cb = cb

    def deliver(self, pkt):
        if self._cb:
            self._cb(pkt)

    def _listen(self):
        while not self._closed:
            try:
                data = self._sock.recv(65536)
            except socket.timeout:
                continue
            except OSError:
                return
            if len(data) < MEDIUM_HDR.size:
                continue
            src, dest, port, pid = MEDIUM_HDR.unpack_from(data)
            if dest not in (BROADCAST, self.node) or not 1 <= port <= 255:
                continue
            self.deliver({"from": src, "to": dest, "id": pid, "portnum": to_portnum(port),
                          "payload": data[MEDIUM_HDR.size:], "rx_rssi": -80, "rx_snr": 5.0, "hop_start": 3,
                          "hop_limit": 3, "pki_encrypted": dest != BROADCAST, "rx_time": int(time.time())})

    def close(self):
        self._closed = True
        if self._sock:
            self._sock.close()
            try:
                os.unlink(self._path)
            except OSError:
                pass
