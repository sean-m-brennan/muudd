#!/usr/bin/env bash
# Run the kith-covenant MVP test suite.
#
# Runs every crate in the workspace except kc-pyo3 (Python-binding cdylib;
# builds via maturin in Phase 7, has no Rust tests of its own yet), then
# executes the two_communities_demo binary as an end-to-end smoke test.
#
# Pass extra cargo args on the command line, e.g.:
#   ./run-tests.sh --quiet
#   ./run-tests.sh -p kc-core
#   ./run-tests.sh -- --nocapture

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Load cargo/rustc from ~/.cargo/env if rustup installed them outside PATH.
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found. Install Rust via https://rustup.rs/" >&2
    exit 1
fi

echo "=== cargo test --workspace --exclude kc-pyo3 ==="
cargo test --workspace --exclude kc-pyo3 "$@"

echo
echo "=== two_communities_demo end-to-end smoke test ==="
cargo run --quiet -p two_communities_demo

echo
echo "=== all checks passed ==="
