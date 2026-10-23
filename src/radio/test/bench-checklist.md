# radiod hardware bench checklist

Equipment: two Linux machines (A and B) each with a Meshtastic board on USB,
and one phone running the Meshtastic app. All three radios on firmware
2.7.15 or later, region US, preset LONG_TURBO (500 kHz), and a channel named
`muudd` with the same key. Install the env from `src/radio/environment.yml`,
build, and `pip install -e client/python -e "drivers/meshtastic[serial]"`.

On each machine, `radiod.json`:

    {"node_addr": "auto",
     "driver": {"exec": "<venv>/bin/meshtastic-driver", "args": ["--port", "/dev/ttyACM0"],
                "settings": {"channel": "muudd"}},
     "ports": {"1": {"name": "text", "share_pct": 20, "policy": "radiod"},
               "16": {"name": "telemetry", "share_pct": 30, "policy": "radiod"}},
     "socket": "/tmp/radio.sock"}

Record pass or fail for each step.

1. Start radiod on A and B. `radioctl --socket /tmp/radio.sock status` on each
   shows `radio: up`, `driver: meshtastic 2.7.x`, and `bandwidth 500 kHz`.
2. Broadcast: on A, claim port 16 and send a broadcast with the Python client;
   on B, the consumer on port 16 receives it with `src` equal to A's node
   address and non-zero RSSI and SNR in `meta`.
3. Unicast with ack: from A to B's node address on port 16 with `want_ack`;
   A gets `SENT` then `ACKED`. Unplug B's radio and repeat; A gets `SENT` then
   `NO_ACK` within 60 s.
4. Handset text: send a text from the phone on the `muudd` channel; the
   consumer on port 1 at A receives it. Send text from A's port 1 to the
   phone's node address; the phone shows it.
5. Bandwidth guard: set B's radio to LONG_FAST (250 kHz) with the Meshtastic
   app, restart radiod on B; `radioctl status` shows `radio: down` with
   `250 kHz below minimum 500 kHz`, and it stays down.
6. Missing channel: remove the `muudd` channel on B's radio, restart radiod;
   it stays down with `configuration error`.
7. Bridging: add a `bridge` section making A a gateway (region `bench`) and a
   driverless radiod on a third machine C peering with A. On C, claim port 16
   and receive B's broadcasts; send a broadcast from C and see it arrive at B.
8. Hello with a large node database: on a radio whose node list holds many
   nodes (ideally 100 or more, as on a busy public mesh; otherwise the largest
   available), restart radiod and time from its `listening on` log line to
   `driver meshtastic ... up`. It must be under 10 s, or radiod kills the
   driver for a missing Hello (`no Hello within 10000 ms`). Repeat three times and record the longest.
