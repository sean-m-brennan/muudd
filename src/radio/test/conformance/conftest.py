# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
import os
import shlex
import sys
from pathlib import Path

import pytest

RADIO = Path(__file__).resolve().parents[2]
BUILD = Path(os.environ.get("MR_BUILD_DIR", RADIO / "build"))
sys.path.insert(0, str(RADIO / "client" / "python"))
sys.path.insert(0, str(Path(__file__).resolve().parent))


def pytest_addoption(parser):
    parser.addoption(
        "--driver-cmd",
        default=f"{BUILD / 'stub-driver'} --medium {{medium}} --node-addr 0xA01",
        help="driver command line; {medium} becomes a shared-medium directory",
    )


@pytest.fixture
def driver(request, tmp_path):
    from driver_harness import DriverProc

    medium = tmp_path / "medium"
    medium.mkdir()
    argv = shlex.split(request.config.getoption("--driver-cmd").replace("{medium}", str(medium)))
    d = DriverProc(argv, medium)
    yield d
    d.kill()
