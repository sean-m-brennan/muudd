# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: GPL-3.0-only
"""meshtastic-driver: a radiod driver for a radio running Meshtastic firmware.

radiod starts it with stdin and stdout as the driver protocol pipes."""
import argparse
import os
import sys
import threading
import time

from muudd_radio import radio_pb2 as pb
from muudd_radio.framing import FrameReader

from .driver import DriverCore


def _int(s):
    return int(s, 0)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="meshtastic-driver")
    ap.add_argument("--port", help="serial device of the Meshtastic radio")
    ap.add_argument("--status-s", type=float, default=5.0)
    ap.add_argument("--fake-medium", help="use a fake radio on this shared-medium directory (tests)")
    ap.add_argument("--fake-node", type=_int, default=0xA0B0)
    ap.add_argument("--fake-firmware", default="2.7.15")
    ap.add_argument("--fake-preset", default="LONG_TURBO")
    ap.add_argument("--fake-region", default="US")
    ap.add_argument("--fake-no-channel", action="store_true")
    args = ap.parse_args(argv)

    if args.fake_medium:
        from .fake_radio import FakeRadio
        radio = FakeRadio(node=args.fake_node, firmware=args.fake_firmware, preset=args.fake_preset,
                          region=args.fake_region, channels=() if args.fake_no_channel else ("muudd",),
                          medium=args.fake_medium)
    elif args.port:
        from .serial_radio import SerialRadio
        radio = SerialRadio(args.port)
    else:
        ap.error("--port or --fake-medium is required")

    out = sys.stdout.buffer

    def write(data):
        out.write(data)
        out.flush()

    core = DriverCore(radio, write)
    caps = core.hello()

    if not caps.fatal:
        def heartbeat():
            while True:
                time.sleep(args.status_s)
                core.status()
        threading.Thread(target=heartbeat, daemon=True).start()

    reader = FrameReader()
    fd = sys.stdin.buffer.fileno()
    while True:
        data = os.read(fd, 4096)
        if not data:
            break
        for frame in reader.feed(data):
            m = pb.RadiodToDriver.FromString(frame)
            kind = m.WhichOneof("msg")
            if kind == "configure":
                code = core.configure(m.configure)
                if code is not None:
                    out.flush()
                    radio.close()
                    os._exit(code)
            elif kind == "send" and not caps.fatal:
                core.send(m.send)
    radio.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
