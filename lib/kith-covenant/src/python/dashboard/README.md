<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->
# kith-covenant capture-signal dashboard (prototype)

A self-contained, read-only dashboard over the kith-covenant capture-signal
analytics (`kith_covenant.reputation`). It renders the jester's lenses — the
same descriptive detectors the corpus specifies — over a signed, crypto-verified
audit commons.

**Descriptive only.** The lenses surface flags with provenance and render no
verdict; adjudication is the mediator pool's job. Nothing here writes to a
commons.

This prototype lives entirely inside the kith-covenant package (which is part of
the `muudd` repo) and has **no dependency on the wider sustainable-space app**.

## Layout

```
run-demo.sh              launcher (at the package root)
src/python/dashboard/
├── app.py               FastAPI: read-only JSON endpoints + static host
├── seed.py              builds one verified commons exercising every lens
├── static/index.html    offline single-page UI (inline SVG charts, no CDN)
└── requirements.txt     extra deps (fastapi, uvicorn) for the conda env
```

## Run

From the package root, using the project's conda env (`kith-covenant`):

```bash
./run-demo.sh                 # serve on http://127.0.0.1:8000/
# CONDA_ENV=kith-covenant HOST=0.0.0.0 PORT=9000 ./run-demo.sh
# ./run-demo.sh --reload      # extra args pass through to uvicorn
```

The launcher installs `fastapi`/`uvicorn` into the conda env on first run
(only then does it touch the network), verifies the compiled `kith_covenant`
extension is importable, and serves. The extension must already be built
(`maturin develop` from the package root).

## Endpoints

| Route | Lens |
|---|---|
| `GET /api/meta` | seeded actors + verified entry count |
| `GET /api/composition` | composition-drift (admissions concentration) |
| `GET /api/sponsor-stake?did=` | sponsor stake / taint rate |
| `GET /api/feature-concern?did=` | anti-spiral feature concerns |
| `GET /api/rejection-pattern` | non-admission concentration |
| `GET /api/ceremony-cost-decay?did=` | witness-count decay across rites |
| `GET /api/reputation?did=` | recognition standing + record counts |

The archive is seeded once at startup and never mutated; every endpoint is a
pure projection serialized from the analytics dataclasses.
