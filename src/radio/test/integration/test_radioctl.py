# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess

from radiod_harness import RADIOCTL, pb, stub_driver

PORTS = {"16": {"name": "telemetry", "share_pct": 50, "policy": "radiod"}}


def run(*args):
    return subprocess.run([str(RADIOCTL), *map(str, args)], capture_output=True, text=True, timeout=10)


def started(radiods):
    r = radiods("a").start({"driver": stub_driver("--status-ms", "200"), "ports": PORTS})
    with r.client() as c:
        c.wait_status(pb.STATE_UP)
    return r


def test_status_human(radiods):
    r = started(radiods)
    out = run("--socket", r.socket, "status")
    assert out.returncode == 0, out.stderr
    lines = out.stdout.splitlines()
    assert "radio: up" in lines
    assert any(l.startswith("driver: stub 1.0, node 0x00000a01, mtu 233") for l in lines)
    assert any(l.startswith("settings: region US, preset STUB, bandwidth 500 kHz") for l in lines)
    assert any(l.startswith("port 16 telemetry: ") for l in lines)
    assert any(l.startswith("drops: unclaimed 0") and "bridge_replay 0" in l for l in lines)
    assert "relay: inactive" in lines


def test_stats_json(radiods):
    r = started(radiods)
    out = run("--socket", r.socket, "stats", "--json")
    assert out.returncode == 0, out.stderr
    data = json.loads(out.stdout)
    assert data["state"] == "up"
    assert data["driver"]["name"] == "stub"
    assert data["ports"][0]["port"] == 16 and data["ports"][0]["name"] == "telemetry"
    assert data["drops"]["queue_overflow"] == 0
    assert data["relay_active"] is False


def test_status_json(radiods):
    r = started(radiods)
    out = run("--socket", r.socket, "status", "--json")
    assert out.returncode == 0, out.stderr
    data = json.loads(out.stdout)
    assert data["state"] == "up" and data["driver"]["mtu"] == 233
    assert data["drops"]["bridge_replay"] == 0


def test_missing_socket(tmp_path):
    out = run("--socket", tmp_path / "nope.sock", "status")
    assert out.returncode == 1
    assert "cannot connect" in out.stderr


def test_bad_usage():
    assert run("frobnicate").returncode == 2
