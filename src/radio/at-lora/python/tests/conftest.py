# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""Run from a checkout: this distribution, the AutonomousTrust core and the
muudd_radio client on sys.path. The core is $AT_PY_SRC if set, else the muudd
submodule lib/autonomous_trust found by walking up; the client is
src/radio/client/python beside this extension."""
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
_dist = os.path.dirname(_here)
_radio = os.path.dirname(os.path.dirname(_dist))


def _find_core():
    env = os.environ.get('AT_PY_SRC')
    if env:
        return env
    d = _dist
    while True:
        cand = os.path.join(d, 'lib', 'autonomous_trust', 'src', 'autonomous-trust')
        if os.path.isdir(os.path.join(cand, 'autonomous_trust', 'core')):
            return cand
        parent = os.path.dirname(d)
        if parent == d:
            return None
        d = parent


for _pkg in (_find_core(), os.path.join(_radio, 'client', 'python'), _dist):
    if _pkg and _pkg not in sys.path and os.path.isdir(_pkg):
        sys.path.insert(0, _pkg)


def pytest_configure(config):
    config.addinivalue_line('markers', 'slow: runs real AutonomousTrust nodes for minutes; '
                                       "select with -m slow, skip with -m 'not slow'")
