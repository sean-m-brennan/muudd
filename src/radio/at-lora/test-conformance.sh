#!/bin/bash
# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
#
# AutonomousTrust's conformance run with the lora protocol plugged in: the C
# extension (c/lora, which registers the C adapter) and the Python plug-in
# (conformance/). Every argument is forwarded to AT's scripts/test-conformance.sh.
# AT is $AT_ROOT, else the muudd submodule ../../../lib/autonomous_trust.
set -e
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AT_ROOT="${AT_ROOT:-$here/../../../lib/autonomous_trust}"
if [ ! -x "$AT_ROOT/scripts/test-conformance.sh" ]; then
  echo "AutonomousTrust not found at $AT_ROOT (set AT_ROOT, or run:" \
       "git submodule update --init --recursive)" >&2
  exit 2
fi
export AT_EXTERNAL_EXTENSIONS="$here/c/lora${AT_EXTERNAL_EXTENSIONS:+:$AT_EXTERNAL_EXTENSIONS}"
export AT_CONFORMANCE_PLUGINS="$here/conformance${AT_CONFORMANCE_PLUGINS:+:$AT_CONFORMANCE_PLUGINS}"
export PYTHONPATH="$here/python:$here/../client/python${PYTHONPATH:+:$PYTHONPATH}"
cd "$AT_ROOT"
exec ./scripts/test-conformance.sh "$@"
