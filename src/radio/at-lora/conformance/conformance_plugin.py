# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""The lora protocol as an AutonomousTrust conformance plug-in.

AT's Python harness loads this file when its directory is named in
$AT_CONFORMANCE_PLUGINS: the corpus is scenarios/lora/, the adapter is
python/lora_adapter.py, and the lora distribution (../python) goes on sys.path
first. The C adapter is c/lora_adapter.c, registered by c/lora's
CMakeLists.txt. Run both through ../test-conformance.sh.
"""

#: Prepended to sys.path, relative to this directory, before adapters().
SYS_PATH = ['python', '../python']


def adapters():
    from lora_adapter import LoraAdapter
    return {'lora': LoraAdapter}
