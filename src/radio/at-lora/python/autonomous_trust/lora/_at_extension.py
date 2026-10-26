# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""How the core finds the lora feature: named by the autonomous_trust.extensions
entry point when installed, and found by file name in a source tree."""
from autonomous_trust.lora._python.extension import EXTENSION  # noqa: F401
