# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""What every radiod driver must do: Hello first, one terminal result per send, heartbeat, packet ids, clean exit on EOF."""
import time

from muudd_radio import radio_pb2 as pb

BROADCAST = 0xFFFFFFFF
TERMINAL = (pb.ACKED, pb.NO_ACK, pb.REFUSED)


def hello(driver):
    m = driver.next(10)
    assert m is not None, "no frame within 10 s"
    assert m.WhichOneof("msg") == "hello", "the first frame must be Hello"
    return m.hello


def configure(driver):
    driver.send(configure=pb.Configure(settings={"channel": "muudd"}))


def test_hello_first_with_all_fields(driver):
    h = hello(driver)
    assert h.name and h.version
    assert h.reach in (pb.MESH, pb.LINK)
    assert h.mtu > 0
    assert h.bitrate_bps > 0
    assert h.supports_ack is True
    assert h.node_addr not in (0, BROADCAST)
    assert h.HasField("radio_settings")
    assert h.radio_settings.region and h.radio_settings.bandwidth_khz > 0
    assert h.fatal == ""


def test_send_without_ack_gets_one_result(driver):
    hello(driver)
    configure(driver)
    driver.send(send=pb.Send(req_id=1, dest=BROADCAST, port=16, payload=b"x"))
    mine = [m.send_result for m in driver.collect("send_result", 3) if m.send_result.req_id == 1]
    assert len(mine) == 1
    assert mine[0].status in (pb.SENT, pb.REFUSED)


def test_send_with_ack_gets_one_terminal(driver):
    hello(driver)
    configure(driver)
    driver.send(send=pb.Send(req_id=2, dest=0x00ABCDEF, port=16, payload=b"x", want_ack=True))
    mine = [m.send_result for m in driver.collect("send_result", 5) if m.send_result.req_id == 2]
    terminal = [r for r in mine if r.status in TERMINAL]
    sent = [r for r in mine if r.status == pb.SENT]
    assert len(terminal) == 1
    assert len(sent) <= 1
    assert mine[-1].status in TERMINAL


def test_status_heartbeat(driver):
    hello(driver)
    configure(driver)
    times = []
    for _ in range(3):
        assert driver.next_of("status", 7) is not None, "missing Status heartbeat"
        times.append(driver.last_time)
    gap = times[2] - times[1]
    assert 3.0 <= gap <= 6.5, f"Status interval {gap:.2f} s, expected about 5 s"


def test_received_carries_packet_id(driver):
    h = hello(driver)
    configure(driver)
    time.sleep(0.2)
    for pid in (0x1234, 0x1235):
        driver.inject(h.node_addr, 0xB001, BROADCAST, 16, b"hello", pid)
    got = [m.received for m in driver.collect("received", 3)]
    assert [r.meta.packet_id for r in got] == [0x1234, 0x1235]
    assert all(r.src == 0xB001 and r.port == 16 and r.payload == b"hello" for r in got)


def test_eof_exits_cleanly(driver):
    hello(driver)
    configure(driver)
    driver.p.stdin.close()
    assert driver.p.wait(timeout=2) == 0
