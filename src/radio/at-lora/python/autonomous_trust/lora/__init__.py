# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""AutonomousTrust traffic over muudd-radiod when the IP path to a peer is
down, as a distribution separate from the core.

Laid out as the core is, ``_python`` beside an empty ``_native`` (the C twin
is its own library, libat_lora). Importing this package registers nothing:
the core finds the feature through ``_at_extension.py`` in a source tree or
the ``autonomous_trust.extensions`` entry point when installed.
"""
from autonomous_trust.core import register_backend_prefix

register_backend_prefix('autonomous_trust.lora.')
