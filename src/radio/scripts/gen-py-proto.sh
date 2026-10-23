#!/usr/bin/env bash
# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
#
# Generates the Python protobuf module for the consumer client, which the
# Meshtastic driver and the tests import too.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="$here/client/python/muudd_radio"
mkdir -p "$out"
protoc -I"$here/proto" --python_out="$out" "$here/proto/radio.proto"
