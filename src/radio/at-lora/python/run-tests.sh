#!/bin/bash
# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
cd "$(dirname "$0")"
python -m pytest -q tests/a_unit/ "$@"
