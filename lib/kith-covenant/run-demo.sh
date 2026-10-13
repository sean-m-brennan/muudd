#!/usr/bin/env bash
#
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
#
# Launch the kith-covenant capture-signal dashboard prototype in the project's
# existing conda env (default: 'kith-covenant', per environment.yml).
#
# Self-contained: read-only, demo-seeded data, no writes.
#
# Once the server is up, a browser is opened to the served page.
#
# Usage:
#   ./run-demo.sh                          # serve on 127.0.0.1:8000, open browser
#   CONDA_ENV=kith-covenant HOST=0.0.0.0 PORT=9000 ./run-demo.sh
#   OPEN_BROWSER=0 ./run-demo.sh           # serve only, do not open a browser
#   ./run-demo.sh --reload                 # extra args pass through to uvicorn

set -euo pipefail

# Resolve locations relative to this script, not the caller's cwd.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # kith-covenant package root
PY_ROOT="$ROOT/src/python"                              # kith_covenant importable here
DASHBOARD_DIR="$PY_ROOT/dashboard"

CONDA_ENV="${CONDA_ENV:-kith-covenant}"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-8000}"

# Locate conda (PATH, $CONDA_EXE, or a common install prefix).
CONDA="$(command -v conda || true)"
if [[ -z "$CONDA" && -n "${CONDA_EXE:-}" ]]; then CONDA="$CONDA_EXE"; fi
if [[ -z "$CONDA" ]]; then
  for d in "$HOME/miniconda3" "$HOME/anaconda3" "$HOME/miniforge3" \
           "$HOME/mambaforge" /opt/conda /opt/miniconda3; do
    [[ -x "$d/bin/conda" ]] && { CONDA="$d/bin/conda"; break; }
  done
fi
if [[ -z "$CONDA" ]]; then
  echo "ERROR: conda not found. Activate your conda install or set CONDA_EXE." >&2
  exit 1
fi

# conda run -n ENV -- <cmd> works without a login shell / 'conda activate'.
run() { "$CONDA" run -n "$CONDA_ENV" --no-capture-output "$@"; }

if ! "$CONDA" env list | awk '{print $1}' | grep -qx "$CONDA_ENV"; then
  echo "ERROR: conda env '$CONDA_ENV' not found. Create it from environment.yml," >&2
  echo "       or pass CONDA_ENV=<name>." >&2
  exit 1
fi

# 1. Ensure the dashboard deps are present in the env (only touches the network
#    on first run; installs into the conda env, not a separate venv).
if ! run python -c "import fastapi, uvicorn" 2>/dev/null; then
  echo "==> installing dashboard deps into conda env '$CONDA_ENV'"
  run pip install -r "$DASHBOARD_DIR/requirements.txt"
fi

# 2. The compiled kith_covenant extension must already be built into the env.
if ! ( cd "$PY_ROOT" && run python -c "import kith_covenant" ) 2>/dev/null; then
  echo "ERROR: kith_covenant is not importable in conda env '$CONDA_ENV'." >&2
  echo "       Build it first from this directory:  maturin develop" >&2
  exit 1
fi

# 3. Serve. Run from PY_ROOT so 'dashboard.app' and 'kith_covenant' both resolve.
# Browsers can't reach 0.0.0.0; point them at loopback instead.
BROWSE_HOST="$HOST"; [[ "$BROWSE_HOST" == "0.0.0.0" ]] && BROWSE_HOST="127.0.0.1"
URL="http://$BROWSE_HOST:$PORT/"

# Open the page once the server accepts connections. 'exec' below replaces this
# shell with uvicorn, so the opener must be backgrounded here, first. It polls
# the port (best-effort, ~10s max) so the browser opens only after the server
# is listening. Disable with OPEN_BROWSER=0.
if [[ "${OPEN_BROWSER:-1}" != "0" ]]; then
  (
    set +e  # best-effort; never let the opener disturb the server
    for _ in $(seq 1 50); do
      (exec 3<>"/dev/tcp/$BROWSE_HOST/$PORT") 2>/dev/null && { exec 3>&- 3<&-; break; }
      sleep 0.2
    done
    if command -v xdg-open >/dev/null 2>&1; then xdg-open "$URL"
    elif command -v open >/dev/null 2>&1; then open "$URL"
    elif command -v powershell.exe >/dev/null 2>&1; then
      powershell.exe -NoProfile -Command "Start-Process '$URL'"
    else echo "==> open $URL in your browser"; fi
  ) >/dev/null 2>&1 &
fi

echo "==> serving on $URL  (conda env: $CONDA_ENV, Ctrl-C to stop)"
cd "$PY_ROOT"
exec "$CONDA" run -n "$CONDA_ENV" --no-capture-output \
     uvicorn dashboard.app:app --host "$HOST" --port "$PORT" "$@"
