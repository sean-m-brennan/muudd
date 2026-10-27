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

# Build every muudd component: autonomous_trust, ethne, kith-covenant, Agora,
# Stele and the PDF edition. With --test, then run test.sh for every
# component's tests.
#
# Each component that has its own conda env (autonomous_trust, ethne,
# kith-covenant) is built inside it when that env exists; otherwise the step
# runs in the current environment, e.g. the composed one from setup.sh. Agora
# and Stele build with the system toolchain and strip conda themselves.
#
# Every step runs even if an earlier one fails, and a PASS/FAIL summary is
# printed at the end. Exits non-zero if any step failed.

set -uo pipefail

usage() {
    cat <<EOF
Usage: $0 [--test] [-h|--help]

Options:
  --test       Also run each component's tests (not run by default)
  -h, --help   Show this help
EOF
}

TEST=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --test)    TEST=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AT="$ROOT/lib/autonomous_trust"
ETHNE="$ROOT/lib/ethne"
KC="$ROOT/lib/kith-covenant"
AGORA="$ROOT/apps/agora"
STELE="$ROOT/apps/stele"

CONDA_BIN="${CONDA_BIN:-$(command -v mamba || command -v conda || command -v micromamba || true)}"
CONDA_RUN=()
if [[ -n "$CONDA_BIN" ]]; then
    CONDA_RUN=("$CONDA_BIN" run)
    # conda captures output until the command exits unless told not to.
    if "$CONDA_BIN" run --help 2>&1 | grep -q -- --no-capture-output; then
        CONDA_RUN+=(--no-capture-output)
    fi
fi

# Run a command in the named conda env if it exists, else in the current one.
in_env() {
    local name=$1 prefix=""
    shift
    if [[ -n "$CONDA_BIN" ]]; then
        prefix=$("$CONDA_BIN" env list 2>/dev/null \
            | awk -v n="$name" '$NF ~ ("/envs/" n "$") {print $NF; exit}')
    fi
    if [[ -n "$prefix" ]]; then
        "${CONDA_RUN[@]}" -p "$prefix" "$@"
    else
        "$@"
    fi
}

RESULTS=()
FAILED=0

# step LABEL DIR CMD...: run CMD in DIR and record whether it passed.
step() {
    local label=$1 dir=$2
    shift 2
    echo ""
    echo "=== muudd: $label ==="
    if (cd "$dir" && "$@"); then
        RESULTS+=("PASS  $label")
    else
        RESULTS+=("FAIL  $label")
        FAILED=1
    fi
}

# --- build -------------------------------------------------------------------
step "autonomous_trust build" "$AT"    in_env autonomous_trust ./at build
step "ethne build"            "$ETHNE" in_env ethne sh -c 'cargo build --workspace --exclude en-pyo3 && maturin build'
step "kith-covenant build"    "$KC"    in_env kith-covenant sh -c 'cargo build --workspace --exclude kc-pyo3 && maturin build'
step "agora build"            "$AGORA" ./build.sh
step "stele build"            "$STELE" ./build.sh
step "PDF edition"            "$ROOT"  tools/pdf/build.sh

# --- test --------------------------------------------------------------------
# test.sh holds the list of every component's tests.
if (( TEST )); then
    step "tests (test.sh)" "$ROOT" ./test.sh
fi

echo ""
echo "=== muudd summary ==="
printf '  %s\n' "${RESULTS[@]}"
exit "$FAILED"
