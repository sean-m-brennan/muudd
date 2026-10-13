#!/usr/bin/env bash
#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

# Run the kith-covenant MVP test suite.
#
# Expects the kith-covenant conda env to be active. Create + activate first:
#
#   conda env create -f environment.yml
#   conda activate kith-covenant
#   ./run-tests.sh
#
# That env provides cargo/rustc (conda rust), maturin, and pytest. If any of
# those is missing the script errors early with an instruction.
#
# Pass extra cargo-test args on the command line, e.g.:
#   ./run-tests.sh --quiet
#   ./run-tests.sh -p kc-core
#   ./run-tests.sh -- --nocapture

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

require() {
    local name=$1
    if ! command -v "$name" >/dev/null 2>&1; then
        echo "error: '$name' not found on PATH." >&2
        echo "       Activate the kith-covenant conda env first:" >&2
        echo "         conda env create -f environment.yml  # first time only" >&2
        echo "         conda activate kith-covenant" >&2
        exit 1
    fi
}

require cargo
require python
require maturin
require pytest

echo "=== cargo test --workspace --exclude kc-pyo3 ==="
cargo test --workspace --exclude kc-pyo3 "$@"

echo
echo "=== two_communities_demo end-to-end smoke test ==="
cargo run --quiet -p two_communities_demo >/dev/null
echo "  ✓ exit 0"

echo
echo "=== kith_covenant._core (PyO3 build via maturin) ==="
maturin develop --quiet

echo
echo "=== pytest src/python/tests/ ==="
pytest src/python/tests/

echo
echo "=== python src/python/examples/two_communities_demo.py ==="
python src/python/examples/two_communities_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/capture_signals_demo.py ==="
python src/python/examples/capture_signals_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/authoring_demo.py ==="
python src/python/examples/authoring_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/mediation_demo.py ==="
python src/python/examples/mediation_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/escalation_demo.py ==="
python src/python/examples/escalation_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/injunction_demo.py ==="
python src/python/examples/injunction_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/commons_demo.py ==="
python src/python/examples/commons_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/observer_demo.py ==="
python src/python/examples/observer_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/constitution_demo.py ==="
python src/python/examples/constitution_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/capture_harness_demo.py ==="
python src/python/examples/capture_harness_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/adversary_campaign_demo.py ==="
python src/python/examples/adversary_campaign_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== python src/python/examples/non_pursuit_demo.py ==="
python src/python/examples/non_pursuit_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== all checks passed ==="
