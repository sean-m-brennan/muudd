# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Fixtures for the radiod integration tests."""
import pytest

from radiod_harness import Radiod


@pytest.fixture
def radiods(tmp_path):
    made = []

    def make(name):
        r = Radiod(tmp_path, name)
        made.append(r)
        return r

    yield make
    for r in made:
        r.stop()


@pytest.fixture
def medium(tmp_path):
    d = tmp_path / "medium"
    d.mkdir()
    return d
