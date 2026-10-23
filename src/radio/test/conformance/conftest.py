# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import importlib.util
import os
import shlex
import sys
from pathlib import Path

import pytest

RADIO = Path(__file__).resolve().parents[2]
BUILD = Path(os.environ.get("MR_BUILD_DIR", RADIO / "build"))
sys.path.insert(0, str(RADIO / "client" / "python"))
sys.path.insert(0, str(Path(__file__).resolve().parent))


def default_drivers():
    """Every driver the suite checks by default: the stub, and the Meshtastic
    driver against its fake radio when that package is importable."""
    drivers = {"stub": f"{BUILD / 'stub-driver'} --medium {{medium}} --node-addr 0xA01"}
    if importlib.util.find_spec("muudd_meshtastic") is not None:
        drivers["meshtastic"] = (f"{shlex.quote(sys.executable)} -m muudd_meshtastic"
                                 " --fake-medium {medium} --fake-node 0xA0B0")
    return drivers


def pytest_addoption(parser):
    parser.addoption(
        "--driver-cmd",
        default=None,
        help="check only this driver command line; {medium} becomes a shared-medium directory",
    )


def pytest_generate_tests(metafunc):
    if "driver_cmd" not in metafunc.fixturenames:
        return
    cmd = metafunc.config.getoption("--driver-cmd")
    drivers = {"custom": cmd} if cmd else default_drivers()
    metafunc.parametrize("driver_cmd", list(drivers.values()), ids=list(drivers.keys()))


@pytest.fixture
def driver(driver_cmd, tmp_path):
    from driver_harness import DriverProc

    medium = tmp_path / "medium"
    medium.mkdir()
    argv = shlex.split(driver_cmd.replace("{medium}", str(medium)))
    d = DriverProc(argv, medium)
    yield d
    d.kill()
