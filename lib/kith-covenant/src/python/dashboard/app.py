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

"""Read-only JSON API + static host for the kith-covenant capture-signal
dashboard.

Thin wrapper over ``kith_covenant.reputation`` analytics. The archive is seeded
once at startup (see ``seed.py``) and never mutated; every endpoint is a pure
projection returned as JSON. Self-contained: no database, no auth, no writes.

Run (from ``src/python``)::

    uvicorn dashboard.app:app --reload

then open http://127.0.0.1:8000/ .
"""
from __future__ import annotations

import dataclasses
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from kith_covenant import (
    ceremony_cost_decay,
    composition_graph,
    feature_concern_profile,
    rejection_pattern,
    reputation_of,
    sponsor_stake,
)

from dashboard.seed import build_commons

_STATIC = Path(__file__).parent / "static"

app = FastAPI(title="kith-covenant capture-signal dashboard", version="0.1.0")

# One immutable, pre-verified commons for the life of the process.
_commons = build_commons()
_arch = _commons.archive


def _json(obj):
    """Frozen analytics dataclass -> plain JSON-able dict."""
    return dataclasses.asdict(obj)


@app.get("/api/meta")
def meta():
    """The seeded DIDs and their roles, so the UI can label and default panels."""
    return {
        "size": _commons.size,
        "actors": [
            {"role": "hub", "label": "Hub", "did": _commons.hub_did},
            {"role": "drifting", "label": "Drifting entrant", "did": _commons.drifting_did},
            {"role": "healthy", "label": "Healthy entrant", "did": _commons.healthy_did},
            {"role": "gatekeeper", "label": "Gatekeeper", "did": _commons.gatekeeper_did},
            {"role": "reaffirmer", "label": "Reaffirmer", "did": _commons.reaffirmer_did},
            {"role": "designated", "label": "Designated (anathema)", "did": _commons.designated_did},
        ],
    }


@app.get("/api/composition")
def composition():
    """Composition-drift lens: who is concentrating admissions."""
    return _json(composition_graph(_arch))


@app.get("/api/sponsor-stake")
def sponsor_stake_endpoint(did: str | None = None):
    """Sponsor-stake lens: does this sponsor's vouching precede misconduct."""
    return _json(sponsor_stake(_arch, did or _commons.hub_did))


@app.get("/api/feature-concern")
def feature_concern(did: str | None = None):
    """Feature-concern lens: behavioral shadow of a hollow substrate."""
    return _json(feature_concern_profile(_arch, did or _commons.drifting_did))


@app.get("/api/rejection-pattern")
def rejection():
    """Rejection-pattern lens: concentration of non-admissions."""
    return _json(rejection_pattern(_arch))


@app.get("/api/ceremony-cost-decay")
def ceremony_decay(did: str | None = None):
    """Ceremony-cost-decay lens: a community's rites hollowing out over time."""
    return _json(ceremony_cost_decay(_arch, did or _commons.reaffirmer_did))


@app.get("/api/reputation")
def reputation(did: str | None = None):
    """Full reputation view / recognition standing for one actor."""
    return _json(reputation_of(_arch, did or _commons.designated_did))


@app.get("/")
def index():
    return FileResponse(_STATIC / "index.html")


app.mount("/static", StaticFiles(directory=_STATIC), name="static")
