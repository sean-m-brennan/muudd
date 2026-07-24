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

# Run the ethne test suite: the Rust tier, then the Python surface over it.
#
# Expects the ethne conda env to be active. Create + activate first:
#
#   conda env create -f environment.yaml
#   conda activate ethne
#   ./run-tests.sh
#
# That env provides cargo/rustc (conda rust), maturin, and pytest. If any of
# those is missing the script errors early with an instruction.
#
# Pass extra cargo-test args on the command line, e.g.:
#   ./run-tests.sh --quiet
#   ./run-tests.sh -p en-charter
#   ./run-tests.sh -- --nocapture

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

require() {
    local name=$1
    if ! command -v "$name" >/dev/null 2>&1; then
        echo "error: '$name' not found on PATH." >&2
        echo "       Activate the ethne conda env first:" >&2
        echo "         conda env create -f environment.yaml  # first time only" >&2
        echo "         conda activate ethne" >&2
        exit 1
    fi
}

require cargo
require python
require maturin
require pytest

# en-pyo3 is excluded: it is a cdylib built against the Python extension ABI, so
# it has no test harness of its own — its surface is covered by pytest below.
echo "=== cargo test --workspace --exclude en-pyo3 ==="
cargo test --workspace --exclude en-pyo3 "$@"

echo
echo "=== ethne._core (PyO3 build via maturin) ==="
maturin develop --quiet

echo
echo "=== pytest src/python/tests/ ==="
pytest src/python/tests/

echo
echo "=== python src/python/examples/polity_demo.py ==="
python src/python/examples/polity_demo.py >/dev/null
echo "  ✓ exit 0"

echo
echo "=== all checks passed ==="
