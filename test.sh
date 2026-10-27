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

# Run the tests of every muudd component, or of the ones named on the command
# line. Components: at, ethne, kc, agora, stele, radio, at-lora, newsbrief.
#
# Each component runs in its own conda env when that env exists (autonomous_trust,
# ethne, kith-covenant, muudd_radio), else in the current environment. The radio
# and at-lora C builds are brought up to date first, since their tests run the
# binaries those builds make; every other component's tests are run as the
# component itself runs them.
#
# Every step runs even if an earlier one fails, and a PASS/FAIL/SKIP summary is
# printed at the end. Exits non-zero if any step failed.

set -uo pipefail

ALL=(at ethne kc agora stele radio at-lora newsbrief)

usage() {
    cat <<EOF
Usage: $0 [--slow] [-j N] [-h|--help] [COMPONENT...]

Components (default: all): ${ALL[*]}

Options:
  --slow       Also run the slow at-lora test (two real AT nodes, ~4 min)
  -j N         Parallel jobs for the radio and at-lora C builds (default 4)
  -h, --help   Show this help
EOF
}

SLOW=0
JOBS=4
WANT=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --slow)    SLOW=1; shift ;;
        -j)        JOBS=${2:?-j needs a number}; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        -*) echo "Unknown option: $1" >&2; usage; exit 1 ;;
        *)
            if [[ " ${ALL[*]} " != *" $1 "* ]]; then
                echo "Unknown component: $1" >&2; usage; exit 1
            fi
            WANT+=("$1"); shift ;;
    esac
done
(( ${#WANT[@]} )) || WANT=("${ALL[@]}")

wanted() { [[ " ${WANT[*]} " == *" $1 "* ]]; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AT="$ROOT/lib/autonomous_trust"
ETHNE="$ROOT/lib/ethne"
KC="$ROOT/lib/kith-covenant"
AGORA="$ROOT/apps/agora"
STELE="$ROOT/apps/stele"
NEWSBRIEF="$ROOT/apps/newsbrief"
RADIO="$ROOT/src/radio"
LORA="$RADIO/at-lora"

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
    echo "=== muudd test: $label ==="
    if (cd "$dir" && "$@"); then
        RESULTS+=("PASS  $label")
    else
        RESULTS+=("FAIL  $label")
        FAILED=1
    fi
}

skip() {
    echo ""
    echo "=== muudd test: $1 === skipped: $2"
    RESULTS+=("SKIP  $1 ($2)")
}

# --- lib ---------------------------------------------------------------------
wanted at    && step "autonomous_trust" "$AT"    in_env autonomous_trust ./at test
wanted ethne && step "ethne"            "$ETHNE" in_env ethne ./run-tests.sh
wanted kc    && step "kith-covenant"    "$KC"    in_env kith-covenant ./run-tests.sh

# --- apps --------------------------------------------------------------------
wanted agora && step "agora" "$AGORA" ./test.sh
if wanted stele; then
    step "stele conformance" "$STELE" at-stele/test-conformance.sh
    # Each pytest directory holds its own `tests` package, so run them one at a time.
    for d in at-stele/python/tests tools/tests verifier/tests viewer/tests; do
        step "stele pytest $d" "$STELE" in_env autonomous_trust python -m pytest "$d"
    done
    step "stele extension" "$STELE" node --test extension/tests/panel.test.mjs extension/tests/background.test.mjs
fi
if wanted newsbrief; then
    # newsbrief keeps its own uv venv (see its README), not a conda env.
    # `uv run` brings that venv up to date with pyproject.toml first, so a
    # dependency added since it was made is installed rather than missing.
    if command -v uv >/dev/null; then
        step "newsbrief" "$NEWSBRIEF" uv run --extra dev python -m pytest -q tests
    else
        skip "newsbrief" "uv not found; see apps/newsbrief/README.md"
    fi
fi

# --- radio -------------------------------------------------------------------
# at-lora's integration tests run the radiod binaries, so its tests need this
# build as well.
if wanted radio || wanted at-lora; then
    step "radio build" "$RADIO" in_env muudd_radio sh -c \
        "cmake -S . -B build >/dev/null && cmake --build build -j $JOBS"
fi
if wanted radio; then
    step "radio C" "$RADIO" in_env muudd_radio ctest --test-dir build --output-on-failure
    # Separate runs: each directory brings its own conftest and harness.
    for d in test/integration test/conformance client/python/tests drivers/meshtastic/tests; do
        step "radio pytest $d" "$RADIO" in_env muudd_radio python -m pytest -q "$d"
    done
fi

# The extension builds inside AT's CMake project, so its build dir holds AT's
# whole C suite as well; only the lora tests are run here (AT's own are under
# the autonomous_trust step).
if wanted at-lora; then
    step "at-lora build" "$LORA" in_env autonomous_trust sh -c \
        "cmake -S '$AT/src/c' -B build -DAT_EXTERNAL_EXTENSIONS='$LORA/c/lora' \
               -DMUUDD_RADIO_DIR='$RADIO' >/dev/null && cmake --build build -j $JOBS"
    step "at-lora C" "$LORA" in_env autonomous_trust ctest --test-dir build -R '^lora_' --output-on-failure
    step "at-lora python" "$LORA/python" in_env autonomous_trust python -m pytest -q tests -m 'not slow'
    if (( SLOW )); then
        step "at-lora python (slow)" "$LORA/python" in_env autonomous_trust python -m pytest -q tests -m slow
    fi
    step "at-lora conformance" "$LORA" in_env autonomous_trust ./test-conformance.sh
fi

echo ""
echo "=== muudd test summary ==="
printf '  %s\n' "${RESULTS[@]}"
exit "$FAILED"
