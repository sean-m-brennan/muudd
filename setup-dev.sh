#!/usr/bin/env bash
# ******************
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
# ******************
#
# Development environment setup for muudd simulation.
# Creates the muudd_simulation conda environment (separate from autonomous_trust),
# builds SPLAT! RF propagation tool, and downloads SRTM elevation data.
#
# Usage:
#   ./setup-dev.sh              # full setup (env + SPLAT! + SRTM)
#   ./setup-dev.sh --update     # update existing environment
#   ./setup-dev.sh --env-only   # conda env only (skip SPLAT! build and SRTM download)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AT_DIR="$SCRIPT_DIR/lib/autonomous_trust"
SIM_DIR="$AT_DIR/src/autonomous-trust-simulator"
SIM_CONFIG="$SIM_DIR/config"

ENV_NAME="muudd_simulation"
ENV_YAML="$SIM_CONFIG/simulation_environ.yml"
MINIFORGE_HOME="$HOME/.miniforge3"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RESET='\033[0m'

info()  { echo -e "${GREEN}[setup]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[setup]${RESET} $*"; }
error() { echo -e "${RED}[setup]${RESET} $*" >&2; }

# ---------------------------------------------------------------------------
# Detect platform
# ---------------------------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  PLATFORM="Linux" ;;
    Darwin) PLATFORM="MacOSX" ;;
    *)      error "Unsupported OS: $OS"; exit 1 ;;
esac

# ---------------------------------------------------------------------------
# 1. Miniforge (conda-forge only)
# ---------------------------------------------------------------------------
install_miniforge() {
    if command -v conda &>/dev/null; then
        info "conda already available: $(conda --version)"
        return 0
    fi

    if [ -x "$MINIFORGE_HOME/bin/conda" ]; then
        info "Miniforge found at $MINIFORGE_HOME but not on PATH"
        eval "$("$MINIFORGE_HOME/bin/conda" shell.bash hook)"
        return 0
    fi

    info "Installing Miniforge to $MINIFORGE_HOME ..."
    local installer="/tmp/miniforge-installer.sh"
    local url="https://github.com/conda-forge/miniforge/releases/latest/download/Miniforge3-${PLATFORM}-${ARCH}.sh"
    curl -fsSL "$url" -o "$installer"
    bash "$installer" -b -p "$MINIFORGE_HOME"
    rm -f "$installer"

    eval "$("$MINIFORGE_HOME/bin/conda" shell.bash hook)"
    conda config --system --set auto_update_conda false
    conda config --system --add channels conda-forge
    conda config --system --set channel_priority strict
    info "Miniforge installed"
}

activate_conda() {
    if ! command -v conda &>/dev/null; then
        if [ -x "$MINIFORGE_HOME/bin/conda" ]; then
            eval "$("$MINIFORGE_HOME/bin/conda" shell.bash hook)"
        else
            error "conda not found. Run without --update first."
            exit 1
        fi
    fi
}

# ---------------------------------------------------------------------------
# 2. Conda environment
# ---------------------------------------------------------------------------
create_conda_env() {
    activate_conda

    if conda env list | grep -q "/${ENV_NAME}\$"; then
        info "Conda environment '$ENV_NAME' already exists"
        return 0
    fi

    if [ ! -f "$ENV_YAML" ]; then
        error "Environment file not found: $ENV_YAML"
        exit 1
    fi

    info "Creating conda environment '$ENV_NAME' ..."
    # pip -e paths in the yaml are relative to the yaml file's directory
    (cd "$SIM_CONFIG" && conda env create -f "$ENV_YAML")
    info "Conda environment '$ENV_NAME' created"
}

update_conda_env() {
    activate_conda
    info "Updating conda environment '$ENV_NAME' ..."
    (cd "$SIM_CONFIG" && conda env update -n "$ENV_NAME" --file "$ENV_YAML" --prune)
    info "Environment updated"
}

# ---------------------------------------------------------------------------
# 3. Build SPLAT!
# ---------------------------------------------------------------------------
build_splat() {
    activate_conda
    info "Building SPLAT! RF propagation tool ..."

    if [ ! -f "$SIM_CONFIG/build_splat.sh" ]; then
        error "build_splat.sh not found in $SIM_CONFIG"
        exit 1
    fi

    # build_splat.sh requires the conda env to be active
    (
        eval "$(conda shell.bash hook)"
        conda activate "$ENV_NAME"
        bash "$SIM_CONFIG/build_splat.sh"
    )

    info "SPLAT! build complete"
}

# ---------------------------------------------------------------------------
# 4. Download SRTM elevation data
# ---------------------------------------------------------------------------
download_srtm() {
    activate_conda
    info "Downloading SRTM elevation tiles for Braxton County ..."

    if [ ! -f "$SIM_CONFIG/download_srtm.sh" ]; then
        error "download_srtm.sh not found in $SIM_CONFIG"
        exit 1
    fi

    (
        eval "$(conda shell.bash hook)"
        conda activate "$ENV_NAME"
        bash "$SIM_CONFIG/download_srtm.sh"
    )
}

# ---------------------------------------------------------------------------
# 5. Verify setup
# ---------------------------------------------------------------------------
verify() {
    activate_conda
    info "Verifying environment ..."

    (
        eval "$(conda shell.bash hook)"
        conda activate "$ENV_NAME"

        python -c "from autonomous_trust.simulator.radio.terrain import TerrainPathLoss; print('  terrain.py: OK')" || {
            error "Failed to import TerrainPathLoss"
            return 1
        }

        python -c "from autonomous_trust.simulator.scenarios.appalachian import get_splat_sites; print('  appalachian.py: OK')" || {
            error "Failed to import appalachian scenario"
            return 1
        }

        if command -v splat &>/dev/null; then
            echo -n "  SPLAT!: "
            splat -v 2>&1 | head -1 || true
        else
            warn "  SPLAT! not found in PATH (run without --env-only to build)"
        fi
    )

    info "Verification complete"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    local mode="full"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --update)    mode="update"; shift ;;
            --env-only)  mode="env-only"; shift ;;
            -h|--help)
                echo "Usage: $0 [--update|--env-only]"
                echo ""
                echo "  (no args)    Full setup: conda env + SPLAT! build + SRTM download"
                echo "  --update     Update existing conda environment"
                echo "  --env-only   Create conda environment only (skip SPLAT! and SRTM)"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    info "Setting up muudd simulation environment"
    echo

    case "$mode" in
        update)
            update_conda_env
            ;;
        env-only)
            install_miniforge
            create_conda_env
            verify
            ;;
        full)
            install_miniforge
            create_conda_env
            build_splat
            download_srtm
            verify
            ;;
    esac

    echo
    info "Setup complete. Activate with:"
    info "  conda activate $ENV_NAME"
}

main "$@"
