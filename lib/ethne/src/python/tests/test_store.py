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

"""Durable storage (D20), on a real polity with real signed prunings.

`en-store`'s own tests cover the generation/cursor mechanics over a synthetic entry
type. These cover the thing that motivated the slice: a host that persists
incrementally *and* prunes must reload what it actually has, and neither
`verify_all` nor `check_authority` will catch it if it does not.
"""
from __future__ import annotations

import pytest

import ethne as e
from ethne.authoring import (
    Signer,
    confer_office,
    prune_observations,
    record_observation,
)

DAY = 86_400
NOW = 400 * DAY


@pytest.fixture
def store(tmp_path):
    return e.FileStore(str(tmp_path / "charter.log"))


def _watched(polity, count, first_at, step):
    monitor = Signer.generate()
    watched = polity.founders[2].did
    for i in range(count):
        record_observation(polity.archive, polity.did, monitor,
                           [(watched, 0.9, None, None)], observed_at=first_at + i * step)
    return watched


def test_an_absent_file_loads_as_an_empty_log(store):
    assert len(store.load()) == 0, "a first run needs no special case"


def test_a_growing_log_saves_incrementally_and_reloads(polity, store):
    store.save(polity.archive)
    monitor = Signer.generate()
    appended = 0
    for i in range(10):
        record_observation(polity.archive, polity.did, monitor,
                           [(polity.founders[2].did, 0.9, None, None)], observed_at=100 + i)
        out = store.save(polity.archive)
        assert not out.rewrote, "no compaction, so nothing should be rewritten"
        appended += out.appended
    assert appended == 10

    back = e.FileStore(store.path).load()
    assert back.cids() == polity.archive.cids()
    assert back.generation == polity.archive.generation


def test_saving_with_nothing_new_writes_nothing(polity, store):
    store.save(polity.archive)
    out = store.save(polity.archive)
    assert not out.wrote_anything and out.appended == 0


# --- the regression this whole slice exists for ------------------------------


def test_a_pruned_log_does_not_resurrect_what_it_dropped(polity, store):
    """**The D20 regression.**

    Measured before the fix, with a hand-kept cursor: 12 entries in memory became 12
    on reload, 3 observations became 10 — seven resurrected — and the reloaded log
    passed both verification passes, because every CID the pruning receipt named was
    back. Worse, the pruning record itself never reached storage, so `pruned_before`
    was gone and the measured absence *lengthened*.
    """
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    watched = _watched(polity, 10, first_at=NOW - 300 * DAY, step=10 * DAY)

    # Persist as it grows — the natural host loop.
    store.save(polity.archive)

    rec = polity.archive.prune_recommendation(polity.did, min_dropped=1, as_of=NOW)
    assert rec is not None and rec.dropped > 0
    prune_observations(polity.archive, polity.did, horizon=rec.horizon,
                       deciders=[keeper], pruned_at=NOW)
    live = polity.archive.apply_prunings(polity.did, as_of=NOW)
    assert live.generation == 1

    out = store.save(live)
    assert out.rewrote, "a changed generation must force a full rewrite"

    back = e.FileStore(store.path).load()
    kinds = [k for _c, k in back.entries()]
    assert kinds.count("Observed") == [k for _c, k in live.entries()].count("Observed")
    assert back.cids() == live.cids(), "nothing resurrected, nothing lost"
    assert "ObservationsPruned" in kinds, "the pruning record reached storage"
    assert back.generation == 1

    # And the clamp survived with it, so the absence cannot read longer than it is.
    w_live = live.observation_window(polity.did, since=0, as_of=NOW)
    w_back = back.observation_window(polity.did, since=0, as_of=NOW)
    assert w_back.pruned_before == w_live.pruned_before
    assert w_back.unattended_span(watched, as_of=NOW) == \
        w_live.unattended_span(watched, as_of=NOW)


def test_a_cursor_from_before_a_pruning_is_refused(polity):
    """The same defect at the API, for a host that keeps its own cursor anyway."""
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    _watched(polity, 10, first_at=NOW - 300 * DAY, step=10 * DAY)
    stale = polity.archive.cursor()

    rec = polity.archive.prune_recommendation(polity.did, min_dropped=1, as_of=NOW)
    prune_observations(polity.archive, polity.did, horizon=rec.horizon,
                       deciders=[keeper], pruned_at=NOW)
    live = polity.archive.apply_prunings(polity.did, as_of=NOW)

    with pytest.raises(RuntimeError, match="compacted"):
        live.dump_json_since(stale)
    # Past the end, which version 1 read as "already current" and answered with an
    # empty segment — the silent half of the bug.
    with pytest.raises(RuntimeError, match="past the end"):
        live.dump_json_since(e.Cursor(1, 999))


def test_appending_after_a_pruning_keeps_working(polity, store):
    """What a running host actually does: prune, then carry on."""
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    _watched(polity, 10, first_at=NOW - 300 * DAY, step=10 * DAY)
    store.save(polity.archive)

    rec = polity.archive.prune_recommendation(polity.did, min_dropped=1, as_of=NOW)
    prune_observations(polity.archive, polity.did, horizon=rec.horizon,
                       deciders=[keeper], pruned_at=NOW)
    live = polity.archive.apply_prunings(polity.did, as_of=NOW)
    store.save(live)

    monitor = Signer.generate()
    for i in range(3):
        record_observation(live, polity.did, monitor,
                           [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW + i)
        out = store.save(live)
        assert not out.rewrote, "within a generation, appends stay appends"

    back = e.FileStore(store.path).load()
    assert back.cids() == live.cids()
    back.verify_all()
    back.check_authority()


# --- durability ---------------------------------------------------------------


def test_a_torn_tail_is_reported_rather_than_silently_dropped(polity, store, tmp_path):
    store.save(polity.archive)
    monitor = Signer.generate()
    record_observation(polity.archive, polity.did, monitor,
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=300)
    store.save(polity.archive)

    path = tmp_path / "charter.log"
    raw = path.read_text()
    path.write_text(raw[: len(raw) - 20])

    reader = e.FileStore(str(path))
    with pytest.raises(ValueError, match="torn tail"):
        reader.load()

    dropped = reader.recover()
    assert dropped > 0
    back = reader.load()
    assert len(back) == 1, "back to the last complete segment"
    assert reader.recover() == 0, "a complete file is left alone"


def test_the_store_reports_what_it_did(polity, store):
    out = store.save(polity.archive)
    assert out.rewrote and out.appended == 1 and out.bytes > 0
    assert "SaveOutcome" in repr(out)
    assert store.cursor() == e.Cursor(0, 1)
