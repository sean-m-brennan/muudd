#!/usr/bin/env bash
# Run the kith-covenant MVP test suite.
#
# Expects the kith-covenant conda env to be active. Create + activate first:
#
#   conda env create -f environment.yaml
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
        echo "         conda env create -f environment.yaml  # first time only" >&2
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
echo "=== all checks passed ==="
