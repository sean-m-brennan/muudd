# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""A Meshtastic radio on a serial port, through the meshtastic library."""
from meshtastic.protobuf import channel_pb2, config_pb2, portnums_pb2
from pubsub import pub

_LORA = config_pb2.Config.LoRaConfig


def _portnum(value):
    if isinstance(value, int):
        return value
    if isinstance(value, str) and value in portnums_pb2.PortNum.keys():
        return portnums_pb2.PortNum.Value(value)
    return None


def normalize(packet):
    decoded = packet.get("decoded")
    if not decoded:
        return None
    # The library omits fields at their default of 0, so a missing hopLimit
    # next to a present hopStart means the packet used all its hops.
    hop_start = packet.get("hopStart")
    hop_limit = packet.get("hopLimit")
    if hop_start is not None and hop_limit is None:
        hop_limit = 0
    return {
        "from": packet.get("from", 0),
        "to": packet.get("to", 0),
        "id": packet.get("id", 0),
        "portnum": _portnum(decoded.get("portnum")),
        "payload": decoded.get("payload", b""),
        "rx_rssi": packet.get("rxRssi"),
        "rx_snr": packet.get("rxSnr"),
        "hop_start": hop_start,
        "hop_limit": hop_limit,
        "pki_encrypted": bool(packet.get("pkiEncrypted")),
        "rx_time": packet.get("rxTime"),
        "channel": packet.get("channel", 0),   # omitted by the library at its default of 0
    }


class SerialRadio:
    def __init__(self, dev, iface_factory=None):
        if iface_factory is None:
            from meshtastic.serial_interface import SerialInterface
            iface_factory = SerialInterface
        self.iface = iface_factory(devPath=dev)
        self._cb = None
        pub.subscribe(self._on_receive, "meshtastic.receive")

    def node_num(self):
        return self.iface.myInfo.my_node_num

    def firmware_version(self):
        return self.iface.metadata.firmware_version

    def lora_config(self):
        lora = self.iface.localNode.localConfig.lora
        return {
            "region": _LORA.RegionCode.Name(lora.region),
            "use_preset": lora.use_preset,
            "modem_preset": _LORA.ModemPreset.Name(lora.modem_preset),
            "bandwidth": lora.bandwidth,
            "spread_factor": lora.spread_factor,
            "coding_rate": lora.coding_rate,
        }

    def channel_index(self, name):
        for ch in self.iface.localNode.channels or []:
            if ch.role != channel_pb2.Channel.Role.DISABLED and ch.settings.name == name:
                return ch.index
        return None

    def add_channel(self, name, psk):
        node = self.iface.localNode
        ch = node.getDisabledChannel()
        if ch is None:
            raise RuntimeError("no free channel slot on the radio")
        ch.settings.name = name
        ch.settings.psk = psk
        ch.role = channel_pb2.Channel.Role.SECONDARY
        node.writeChannel(ch.index)
        return ch.index

    def set_preset(self, name):
        node = self.iface.localNode
        node.localConfig.lora.use_preset = True
        node.localConfig.lora.modem_preset = _LORA.ModemPreset.Value(name)
        node.writeConfig("lora")

    def send(self, payload, dest, portnum, want_ack, channel_index, on_ack):
        def on_response(packet):
            routing = packet.get("decoded", {}).get("routing", {})
            on_ack(routing.get("errorReason", "NONE") == "NONE")

        packet = self.iface.sendData(
            payload,
            destinationId=dest,
            portNum=portnum,
            wantAck=want_ack,
            channelIndex=channel_index,
            onResponse=on_response if on_ack else None,
            onResponseAckPermitted=bool(on_ack),
        )
        return packet.id

    def on_receive(self, cb):
        self._cb = cb

    def _on_receive(self, packet, interface):
        if interface is not self.iface or self._cb is None:
            return
        n = normalize(packet)
        if n is not None:
            self._cb(n)

    def close(self):
        try:
            pub.unsubscribe(self._on_receive, "meshtastic.receive")
        finally:
            self.iface.close()
