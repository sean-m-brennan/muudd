#!/usr/bin/env bash
# ******************
#  Copyright 2025 Sean M. Brennan and contributors
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
# ******************
#
# Run the Braxton County terrain-aware mesh network simulation.
#
# Steps:
#   1. Run SPLAT! batch for all node pairs at specified frequencies
#   2. Generate SimConfig with terrain path-loss matrix
#   3. Run the simulator
#
# Prerequisites:
#   ./setup-dev.sh   (creates conda env, builds SPLAT!, downloads SRTM data)
#
# Usage:
#   ./run-simulation.sh                          # full pipeline
#   ./run-simulation.sh --splat-only             # SPLAT! batch only (generate CSV)
#   ./run-simulation.sh --skip-splat             # skip SPLAT!, use existing CSV
#   ./run-simulation.sh --hilltop-only           # 8 hilltop nodes only
#   ./run-simulation.sh --freq 5800              # single frequency (MHz)
#   ./run-simulation.sh --duration 120           # simulation duration (minutes)
#   ./run-simulation.sh -v                       # verbose output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AT_DIR="$SCRIPT_DIR/lib/autonomous_trust"
SIM_DIR="$AT_DIR/src/autonomous-trust-simulator"
SIM_CONFIG="$SIM_DIR/config"
DATA_DIR="$SIM_CONFIG/data"

ENV_NAME="muudd_simulation"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RESET='\033[0m'

info()  { echo -e "${GREEN}[sim]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[sim]${RESET} $*"; }
error() { echo -e "${RED}[sim]${RESET} $*" >&2; }

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
FREQUENCIES="900.0,2400.0,5800.0"
DURATION=60
HILLTOP_ONLY=""
SKIP_SPLAT=""
SPLAT_ONLY=""
VERBOSE=""
CSV_FILE="$DATA_DIR/braxton_county_path_loss.csv"

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --splat-only         Run SPLAT! batch only, generate path-loss CSV"
    echo "  --skip-splat         Skip SPLAT! batch, use existing CSV"
    echo "  --hilltop-only       Use only 8 hilltop relay nodes"
    echo "  --freq FREQ          Single frequency in MHz (default: 900,2400,5800)"
    echo "  --duration MIN       Simulation duration in minutes (default: 60)"
    echo "  --csv FILE           Path to path-loss CSV (default: $CSV_FILE)"
    echo "  -v, --verbose        Verbose output"
    echo "  -h, --help           Show this help"
    exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --splat-only)   SPLAT_ONLY=1; shift ;;
        --skip-splat)   SKIP_SPLAT=1; shift ;;
        --hilltop-only) HILLTOP_ONLY="--hilltop-only"; shift ;;
        --freq)         FREQUENCIES="$2"; shift 2 ;;
        --duration)     DURATION="$2"; shift 2 ;;
        --csv)          CSV_FILE="$2"; shift 2 ;;
        -v|--verbose)   VERBOSE=1; shift ;;
        -h|--help)      usage ;;
        *)              error "Unknown option: $1"; usage 1 ;;
    esac
done

# ---------------------------------------------------------------------------
# Check prerequisites
# ---------------------------------------------------------------------------
check_env() {
    if [[ -z "${CONDA_PREFIX:-}" ]] || [[ "$(basename "$CONDA_PREFIX")" != "$ENV_NAME" ]]; then
        error "Conda environment '$ENV_NAME' is not active."
        error "Run: conda activate $ENV_NAME"
        error "Or run ./setup-dev.sh first if the environment doesn't exist."
        exit 1
    fi
}

check_splat() {
    if ! command -v splat &>/dev/null; then
        error "SPLAT! not found. Run ./setup-dev.sh to build it."
        exit 1
    fi
}

check_sdf() {
    local sdf_dir="$DATA_DIR/sdf"
    if [ ! -d "$sdf_dir" ] || [ -z "$(ls "$sdf_dir"/*.sdf 2>/dev/null)" ]; then
        error "No SDF files found in $sdf_dir"
        error "Run ./setup-dev.sh to download SRTM data, or:"
        error "  bash $SIM_CONFIG/download_srtm.sh"
        exit 1
    fi
    info "SDF tiles: $(ls "$sdf_dir"/*.sdf | wc -l) files"
}

# ---------------------------------------------------------------------------
# SPLAT! batch run
# ---------------------------------------------------------------------------
run_splat_batch() {
    check_splat
    check_sdf

    local sdf_dir="$DATA_DIR/sdf"
    local work_dir="$DATA_DIR/splat_work"
    mkdir -p "$work_dir"

    info "Running SPLAT! batch for frequencies: $FREQUENCIES MHz"
    info "  SDF dir:  $sdf_dir"
    info "  Work dir: $work_dir"
    info "  Output:   $CSV_FILE"
    [ -n "$HILLTOP_ONLY" ] && info "  Mode: hilltop nodes only"

    python3 -c "
import sys, os
sys.path.insert(0, '$SIM_DIR')

from autonomous_trust.simulator.radio.terrain import TerrainPathLoss
from autonomous_trust.simulator.scenarios.appalachian import get_splat_sites, HILLTOP_NODES

sites = get_splat_sites()
hilltop_only = bool('$HILLTOP_ONLY')
if hilltop_only:
    hilltop_names = {name for name, _, _, _ in HILLTOP_NODES}
    sites = [s for s in sites if s.site_id in hilltop_names]

freq_list = [float(f) for f in '$FREQUENCIES'.split(',')]

print(f'Sites: {len(sites)}, Frequencies: {freq_list}')

terrain = TerrainPathLoss()
terrain.run_batch(sites, freq_list, '$sdf_dir', '$work_dir')

os.makedirs(os.path.dirname('$CSV_FILE'), exist_ok=True)
terrain.save_csv('$CSV_FILE')
print(f'Path-loss CSV written to: $CSV_FILE')
"
    info "SPLAT! batch complete"
}

# ---------------------------------------------------------------------------
# Validate SPLAT! results against network.md link budgets
# ---------------------------------------------------------------------------
validate_results() {
    info "Validating path-loss results ..."

    python3 -c "
import sys
sys.path.insert(0, '$SIM_DIR')

from autonomous_trust.simulator.radio.terrain import TerrainPathLoss

terrain = TerrainPathLoss()
terrain.load_csv('$CSV_FILE')

print('Frequencies: %s MHz' % terrain.frequencies)

# Sutton-Burnsville (22 km, hilltop-to-hilltop) at 5.8 GHz
# Expected: ~145 dB (free-space ~137 dB + terrain margin)
loss_5g = terrain.get_loss(5800.0, 'sutton_hilltop', 'burnsville_hilltop')
if loss_5g is not None:
    status = 'OK' if 135 <= loss_5g <= 155 else 'UNEXPECTED'
    print(f'Sutton-Burnsville @ 5800 MHz: {loss_5g:.1f} dB [{status}]')
    print(f'  (expected 135-155 dB per network.md link budget)')
else:
    print('Sutton-Burnsville @ 5800 MHz: no data')

# Print matrix summary
for freq in terrain.frequencies:
    matrix = terrain.get_matrix(freq)
    pairs = sum(len(dsts) for dsts in matrix.values())
    losses = [l for dsts in matrix.values() for l in dsts.values()]
    if losses:
        print(f'{freq} MHz: {pairs} pairs, range {min(losses):.1f}-{max(losses):.1f} dB')
"
}

# ---------------------------------------------------------------------------
# Run simulation
# ---------------------------------------------------------------------------
run_simulation() {
    if [ ! -f "$CSV_FILE" ]; then
        error "Path-loss CSV not found: $CSV_FILE"
        error "Run without --skip-splat first, or use --splat-only to generate it."
        exit 1
    fi

    info "Generating simulation config ..."
    info "  Duration: $DURATION minutes"
    info "  Path-loss CSV: $CSV_FILE"

    local config_file="$DATA_DIR/appalachian.cfg"
    local verbose_flag=""
    [ -n "$VERBOSE" ] && verbose_flag="--verbose"

    python3 -c "
import sys
sys.path.insert(0, '$SIM_DIR')

from autonomous_trust.simulator.scenarios.appalachian import (
    create_appalachian_config, load_terrain_matrix, HILLTOP_NODES, VALLEY_NODES
)
from datetime import timedelta

hilltop_only = bool('$HILLTOP_ONLY')

# Load terrain matrix at 5800 MHz (primary frequency for link budget)
matrix = load_terrain_matrix('$CSV_FILE', 5800.0)

config_path = create_appalachian_config(
    output_file='$config_file',
    duration=timedelta(minutes=$DURATION),
    path_loss_matrix=matrix if matrix else None,
    hilltop_only=hilltop_only,
)
node_count = len(HILLTOP_NODES) if hilltop_only else len(HILLTOP_NODES) + len(VALLEY_NODES)
print(f'Config written: {config_path} ({node_count} nodes)')
"

    info "Launching simulator ..."

    python3 -c "
import sys
sys.path.insert(0, '$SIM_DIR')

from autonomous_trust.simulator.simulator import Simulator

sim = Simulator.from_file('$config_file')
sim.run()
" && info "Simulation complete" || {
        warn "Simulator exited with errors (this module may not be fully implemented yet)"
    }
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    info "Braxton County mesh network simulation"
    echo

    check_env

    if [ -n "$SPLAT_ONLY" ]; then
        run_splat_batch
        validate_results
        return
    fi

    if [ -z "$SKIP_SPLAT" ]; then
        run_splat_batch
        validate_results
        echo
    fi

    run_simulation
}

main
