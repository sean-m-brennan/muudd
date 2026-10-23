# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import socket
import struct
import time

from radiod_harness import BROADCAST, inject, pb, stub_driver, wait_until

PORTS = {
    "16": {"name": "telemetry", "share_pct": 50, "policy": "radiod"},
    "32": {"name": "at", "share_pct": 40, "policy": "consumer"},
}


def cfg(*driver_args):
    return {"node_addr": "auto", "driver": stub_driver("--status-ms", "200", *driver_args), "ports": PORTS}


def test_status_claim_send_ack(radiods):
    r = radiods("a").start(cfg())
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        claim = c.claim(16)
        assert claim.status == pb.CLAIM_OK and claim.mtu == 233
        req = c.send(0xB2, 16, b"hi", want_ack=True)
        assert c.result(req).status == pb.SENT
        assert c.result(req).status == pb.ACKED


def test_refusals(radiods):
    r = radiods("a").start(cfg())
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        c.claim(16)
        assert c.result(c.send(BROADCAST, 32, b"x")).reason == pb.NOT_CLAIMED
        assert c.result(c.send(BROADCAST, 16, b"x" * 234)).reason == pb.TOO_BIG


def test_receive_from_medium(radiods, medium):
    r = radiods("a").start(cfg("--medium", medium, "--node-addr", "0xA1"))
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        c.claim(16)
        inject(medium, 0xA1, 0xB1, BROADCAST, 16, b"temp=21", 7)
        rx = c.recv()
        assert rx.src == 0xB1 and rx.payload == b"temp=21" and rx.meta.packet_id == 7


def _frame(msg):
    data = msg.SerializeToString()
    return struct.pack(">H", len(data)) + data


def test_reclaim_after_half_frame(radiods):
    r = radiods("a").start(cfg())
    raw = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    raw.connect(r.socket)
    raw.sendall(_frame(pb.ConsumerToRadiod(claim=pb.Claim(req_id=1, port=16, policy=pb.RADIOD_BRIDGED))))
    half = _frame(pb.ConsumerToRadiod(get_stats=pb.GetStats(req_id=2)))
    raw.sendall(half[: len(half) // 2])
    time.sleep(0.2)
    raw.close()
    with r.client() as c:
        assert c.claim(16).status == pb.CLAIM_OK


def test_malformed_frame_disconnects(radiods):
    r = radiods("a").start(cfg())
    raw = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    raw.connect(r.socket)
    raw.sendall(b"\xff\xff\x00\x00")
    raw.settimeout(2)
    got_eof = False
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if raw.recv(65536) == b"":
            got_eof = True
            break
    assert got_eof


def test_slow_consumer_overflow(radiods, medium):
    r = radiods("a").start(cfg("--medium", medium, "--node-addr", "0xA1"))
    raw = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    raw.connect(r.socket)
    raw.sendall(_frame(pb.ConsumerToRadiod(claim=pb.Claim(req_id=1, port=16, policy=pb.RADIOD_BRIDGED))))
    time.sleep(0.2)
    for i in range(1500):
        inject(medium, 0xA1, 0xB1, BROADCAST, 16, bytes(200), 1000 + i)
    with r.client() as c:
        assert wait_until(lambda: c.stats().drops.queue_overflow > 0, timeout=5)
    raw.close()


def test_radiod_restart_reclaims(radiods):
    r = radiods("a").start(cfg())
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        c.claim(16)
        r.stop()
        r.start()
        assert wait_until(lambda: c.reconnects >= 1, timeout=5)
        c.wait_status(pb.STATE_UP)
        assert wait_until(lambda: c.result(c.send(BROADCAST, 16, b"x")).status == pb.SENT, timeout=3)


def test_driver_crash_restarts(radiods):
    r = radiods("a").start(cfg("--exit-after-ms", "1500"))
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        st = c.wait_status(pb.STATE_DOWN, timeout=4)
        assert "exited with status 3" in st.reason
        c.wait_status(pb.STATE_UP, timeout=4)


def test_fatal_hello_stays_down(radiods):
    r = radiods("a").start(cfg("--fatal", "bad radio"))
    with r.client() as c:
        assert wait_until(lambda: c.status is not None and "bad radio" in c.status.reason, timeout=5)
        time.sleep(2.5)
        assert c.status.state == pb.STATE_DOWN
        assert "bad radio" in c.status.reason


def test_bandwidth_below_minimum_stays_down(radiods):
    r = radiods("a").start(cfg("--bandwidth-khz", "250"))
    with r.client() as c:
        assert wait_until(lambda: c.status is not None and "250 kHz below minimum 500 kHz" in c.status.reason,
                          timeout=5)
        time.sleep(2.5)
        assert c.status.state == pb.STATE_DOWN


def test_bandwidth_drop_in_status_stays_down(radiods):
    r = radiods("a").start(cfg("--bandwidth-after-ms", "600:250"))
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        st = c.wait_status(pb.STATE_DOWN, timeout=4)
        assert st.reason == "radio bandwidth 250 kHz below minimum 500 kHz"
        time.sleep(2.5)    # a restarted driver would say Hello at 500 kHz and come back up
        assert c.status.state == pb.STATE_DOWN
        assert c.status.reason == "radio bandwidth 250 kHz below minimum 500 kHz"
    assert "driver refused: radio bandwidth 250 kHz" in r.log.read_text()


def test_exit_78_stays_down(radiods):
    r = radiods("a").start(cfg("--exit-after-ms", "300", "--exit-code", "78"))
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
        st = c.wait_status(pb.STATE_DOWN, timeout=3)
        assert "configuration error" in st.reason
        time.sleep(2.5)
        assert c.status.state == pb.STATE_DOWN


def test_bad_config_exits_2(tmp_path):
    import subprocess
    from radiod_harness import RADIOD
    p = tmp_path / "bad.json"
    p.write_text('{"ports": {"2": {"name": "x", "policy": "radiod"}}}')
    out = subprocess.run([str(RADIOD), "--config", str(p)], capture_output=True, text=True)
    assert out.returncode == 2
    assert "ports.2" in out.stderr


def test_boot_counter_rises_across_starts(radiods):
    r = radiods("a")
    config = cfg()
    config["bridge"] = {"peers": []}
    r.start(config)
    boot = r.key_file.with_name(r.key_file.name + ".boot")
    assert boot.read_bytes() == (1).to_bytes(8, "big")
    r.stop()
    r.start()
    assert boot.read_bytes() == (2).to_bytes(8, "big")
    assert oct(boot.stat().st_mode & 0o777) == "0o600"


def test_unusable_boot_counter_falls_back(radiods):
    r = radiods("a")
    config = cfg()
    config["bridge"] = {"peers": []}
    r.write_config(config)
    boot = r.key_file.with_name(r.key_file.name + ".boot")
    boot.write_bytes(b"abc")
    r.start()
    assert wait_until(lambda: "cannot use boot counter" in r.log.read_text(), timeout=2)
    assert "using wall-clock start time" in r.log.read_text()
    assert boot.read_bytes() == b"abc"


def test_unknown_sender_logged_once(radiods):
    r = radiods("a")
    config = cfg()
    config["bridge"] = {"peers": []}
    r.start(config)
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    for i in range(5):
        s.sendto(bytes([i + 1]) * 100, ("127.0.0.1", r.port))
    s.close()
    with r.client() as c:
        assert wait_until(lambda: c.stats().drops.bridge_auth >= 5, timeout=3)
    lines = [ln for ln in r.log.read_text().splitlines() if "unknown sender key" in ln]
    assert len(lines) == 1
    assert "dropped datagram from 127.0.0.1:" in lines[0]


def test_malformed_listen_exits_2(tmp_path):
    import json
    import subprocess
    from radiod_harness import RADIOD
    p = tmp_path / "bad.json"
    p.write_text(json.dumps({"node_addr": "auto", "driver": None, "bridge": {"listen": "4403"}}))
    out = subprocess.run([str(RADIOD), "--config", str(p)], capture_output=True, text=True)
    assert out.returncode == 2
    assert "bridge.listen" in out.stderr
