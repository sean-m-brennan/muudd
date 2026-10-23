# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""Translation between radiod's driver protocol and Meshtastic packets."""
import time

from muudd_radio import radio_pb2 as pb

BROADCAST = 0xFFFFFFFF
TEXT_PORTNUM = 1          # Meshtastic TEXT_MESSAGE_APP
PRIVATE_BASE = 256        # Meshtastic's private port numbers run 256..511
MTU = 233                 # largest Meshtastic data payload
AIR_OVERHEAD = 16         # Meshtastic's on-air packet header
PREAMBLE = 16
MIN_FIRMWARE = (2, 7, 15)

# Modem presets: (spreading factor, bandwidth in Hz, coding rate as 1..4 for 4/5..4/8).
PRESETS = {
    "SHORT_TURBO": (7, 500_000, 1),
    "SHORT_FAST": (7, 250_000, 1),
    "SHORT_SLOW": (8, 250_000, 1),
    "MEDIUM_FAST": (9, 250_000, 1),
    "MEDIUM_SLOW": (10, 250_000, 1),
    "LONG_TURBO": (11, 500_000, 4),
    "LONG_FAST": (11, 250_000, 1),
    "LONG_MODERATE": (11, 125_000, 4),
    "LONG_SLOW": (12, 125_000, 4),
    "VERY_LONG_SLOW": (12, 62_500, 4),
}

# Meshtastic stores 31.25 and 62.5 kHz as 31 and 62.
_ODD_BANDWIDTHS = {31: 31_250, 62: 62_500}


def to_portnum(port):
    if port == 1:
        return TEXT_PORTNUM
    if 2 <= port <= 255:
        return PRIVATE_BASE + port
    raise ValueError(f"MUUDD port {port} has no Meshtastic port number")


def from_portnum(portnum):
    if portnum == TEXT_PORTNUM:
        return 1
    if portnum is not None and PRIVATE_BASE + 2 <= portnum <= PRIVATE_BASE + 255:
        return portnum - PRIVATE_BASE
    return None


def parse_firmware(version):
    parts = []
    for piece in version.split(".")[:3]:
        digits = ""
        for ch in piece:
            if not ch.isdigit():
                break
            digits += ch
        if not digits:
            raise ValueError(f"cannot read firmware version {version!r}")
        parts.append(int(digits))
    while len(parts) < 3:
        parts.append(0)
    return tuple(parts)


def firmware_ok(version):
    try:
        return parse_firmware(version) >= MIN_FIRMWARE
    except ValueError:
        return False


def lora_params(lora):
    if lora.get("use_preset", True):
        name = lora.get("modem_preset", "LONG_FAST")
        sf, bw, cr = PRESETS[name]
        return sf, bw, cr, name
    bw_khz = lora.get("bandwidth") or 250
    bw = _ODD_BANDWIDTHS.get(bw_khz, bw_khz * 1000)
    sf = lora.get("spread_factor") or 11
    cr = (lora.get("coding_rate") or 5) - 4
    return sf, bw, cr, "CUSTOM"


def bitrate(sf, bw, cr):
    return int(sf * bw / (2 ** sf) * 4 / (4 + cr))


def build_caps(node, firmware, lora, fatal=""):
    sf, bw, cr, preset = lora_params(lora)
    return pb.Caps(
        name="meshtastic",
        version=firmware,
        reach=pb.MESH,
        mtu=MTU,
        bitrate_bps=bitrate(sf, bw, cr),
        lora=pb.LoraParams(spreading_factor=sf, bandwidth_hz=bw, coding_rate=cr, preamble_symbols=PREAMBLE),
        reports_link_quality=True,
        supports_ack=True,
        node_addr=node & 0xFFFFFFFF,
        radio_settings=pb.RadioSettings(region=lora.get("region", ""), preset=preset, bandwidth_khz=bw // 1000),
        overhead_bytes=AIR_OVERHEAD,
        fatal=fatal,
    )


def received_from_packet(pkt):
    port = from_portnum(pkt.get("portnum"))
    if port is None:
        return None
    meta = pb.Meta(
        protection=pb.PUBLIC_KEY_DM if pkt.get("pki_encrypted") else pb.CHANNEL_KEY,
        via=pb.VIA_RADIO,
        packet_id=(pkt.get("id") or 0) & 0xFFFFFFFF,
        rx_time_ms=int((pkt.get("rx_time") or time.time()) * 1000),
    )
    if pkt.get("rx_rssi") is not None and pkt.get("rx_snr") is not None:
        meta.rssi_dbm = int(pkt["rx_rssi"])
        meta.snr_db = float(pkt["rx_snr"])
        meta.has_link_quality = True
    hop_start, hop_limit = pkt.get("hop_start"), pkt.get("hop_limit")
    if hop_start is not None and hop_limit is not None and hop_start >= hop_limit:
        meta.hops = hop_start - hop_limit
        meta.has_hops = True
    return pb.Received(src=pkt["from"] & 0xFFFFFFFF, dest=pkt["to"] & 0xFFFFFFFF, port=port,
                       payload=bytes(pkt.get("payload") or b""), meta=meta)
