# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Whether a frame also goes by LoRa. Pure; twin of C lora_policy."""


def unicast(link_up, have_addr, quiet_s, ip_quiet_s, allowed, proto, frame_len, max_len) -> bool:
    return bool(link_up and have_addr and quiet_s >= ip_quiet_s and allowed and proto
                and 0 < frame_len <= max_len)


def fanout(channel, link_up, n_quiet, allowed, proto, frame_len, max_len) -> bool:
    return bool(link_up and n_quiet > 0 and allowed and (channel == 'broadcast' or proto)
                and 0 < frame_len <= max_len)
