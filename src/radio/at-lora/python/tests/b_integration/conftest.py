# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Integration tests run real muudd-radiod processes with stub drivers, using
radiod's own test harness (src/radio/test/integration)."""
import os
import sys

_radio = os.path.abspath(os.path.join(os.path.dirname(__file__), *[os.pardir] * 4))
sys.path.insert(0, os.path.join(_radio, 'test', 'integration'))
