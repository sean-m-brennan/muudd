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


def _watched(polity, count, first_at, step, monitors=3):
    """Rotated over three watchers since **D28**: the retention floor counts distinct
    observers, so a lone monitor filing repeatedly pins the horizon and these tests
    would have nothing to prune."""
    keys = [Signer.generate() for _ in range(monitors)]
    watched = polity.founders[2].did
    for i in range(count):
        record_observation(polity.archive, polity.did, keys[i % monitors],
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


# --- D30: two writers, on a real polity ---------------------------------------
#
# `en-store`'s Rust tests cover the mechanics over a synthetic entry type. These are
# the same two failures against signed charter entries, which is where they would
# actually have cost a polity its record.


def test_a_second_host_cannot_fork_a_real_charter_log(polity, store, tmp_path):
    """Measured before D30: both saves returned successfully and the log became
    **unloadable** — and `recover()` reported nothing to do, because the file was not
    torn but forked. A polity's archive was left unreadable with no way back."""
    store.save(polity.archive)
    path = store.path

    first, second = e.FileStore(path), e.FileStore(path)
    a, b = first.load(), second.load()
    assert first.cursor() == second.cursor()

    # Each host records its own monitor's report, as two live hosts do.
    record_observation(a, polity.did, Signer.generate(),
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW)
    record_observation(b, polity.did, Signer.generate(),
                       [(polity.founders[1].did, 0.8, None, None)], observed_at=NOW)

    first.save(a)
    with pytest.raises(RuntimeError, match="another writer has been here"):
        second.save(b)

    # The refusal is the point: the log still loads and still verifies.
    back = e.FileStore(path).load()
    assert back.cids() == a.cids()
    back.verify_all()
    back.check_authority()


def test_a_stale_hosts_pruning_cannot_discard_persisted_entries(polity, store):
    """The silent one, and the reason D30 overturned D20's assignment.

    Measured: a second host that had not seen the first host's entries pruned, which
    forces the full-rewrite branch, and the first host's already-persisted signed
    observations were **gone** — with every call returning successfully, the reloaded
    log well-formed, and both verification passes green. The first host's cursor still
    read *current*, so it never re-wrote them either.
    """
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    _watched(polity, 10, first_at=NOW - 300 * DAY, step=10 * DAY)
    store.save(polity.archive)
    path = store.path

    # A second host loads, and knows nothing of what comes next.
    stale = e.FileStore(path)
    theirs = stale.load()

    # The first host records something and makes it durable.
    record_observation(polity.archive, polity.did, Signer.generate(),
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW)
    store.save(polity.archive)
    persisted = polity.archive.cids()

    # The stale host prunes, which forces a rewrite of the whole log.
    rec = theirs.prune_recommendation(polity.did, min_dropped=1, as_of=NOW)
    assert rec is not None and rec.dropped > 0
    prune_observations(theirs, polity.did, horizon=rec.horizon,
                       deciders=[keeper], pruned_at=NOW)
    compacted = theirs.apply_prunings(polity.did, as_of=NOW)
    assert compacted.generation == 1, "the rewrite branch, which used to take the log"

    with pytest.raises(RuntimeError, match="already durable"):
        stale.save(compacted)

    back = e.FileStore(path).load()
    assert back.cids() == persisted, "nothing durable went missing"
    back.verify_all()
    back.check_authority()


def test_a_claim_refuses_a_second_host_at_startup(polity, store):
    store.claim()
    assert store.is_claimed
    store.save(polity.archive)

    second = e.FileStore(store.path)
    with pytest.raises(RuntimeError, match="another writer holds"):
        second.claim()

    # A reader is never blocked by a claim — the log is evidence, and reading it must
    # not depend on who is writing.
    assert e.FileStore(store.path).load().cids() == polity.archive.cids()

    store.release()
    assert not store.is_claimed
    second.claim()


def test_stored_end_reads_the_tail_not_the_log(polity, store):
    assert store.stored_end() is None, "an absent store ends nowhere"
    store.save(polity.archive)
    assert store.stored_end() == e.Cursor(0, 1) == store.cursor()

    record_observation(polity.archive, polity.did, Signer.generate(),
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW)
    store.save(polity.archive)
    assert store.stored_end() == e.Cursor(0, 2) == store.cursor()


def test_a_forked_charter_log_reconciles_without_losing_a_signed_entry(polity, tmp_path):
    """The repair, on real signed entries — the case that used to have no way back.

    Build the fork by hand, since the store now refuses to create one.
    """
    path = str(tmp_path / "forked.log")
    seed = e.FileStore(path)
    seed.save(polity.archive)
    base_cursor = polity.archive.cursor()

    mine = e.FileStore(path).load()
    theirs = e.FileStore(path).load()
    record_observation(mine, polity.did, Signer.generate(),
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW)
    record_observation(theirs, polity.did, Signer.generate(),
                       [(polity.founders[1].did, 0.8, None, None)], observed_at=NOW + 1)
    orphaned_cids = [c for c in theirs.cids() if c not in polity.archive.cids()]

    with open(path, "a") as f:
        f.write(mine.dump_json_since(base_cursor) + "\n")
        f.write(theirs.dump_json_since(base_cursor) + "\n")

    # Confirm the fork is genuinely unloadable, so the repair is not being shown off
    # against a healthy log.
    broken = e.FileStore(path)
    with pytest.raises(ValueError, match="already applied"):
        broken.load()
    assert broken.recover() == 0, "recover knows torn tails, not forked ones"

    store = e.FileStore(path)
    report = store.reconcile()
    assert report.was_forked
    assert report.segments_orphaned == 1
    assert report.entries_moved == 1
    assert report.entries_already_present == 0
    assert report.cids_moved == orphaned_cids, "and it says which entries moved"
    assert "Reconciled" in repr(report)

    # NOTHING WAS FORGOTTEN, and every CID is unchanged — so a receipt naming any of
    # these entries still resolves.
    back = e.FileStore(path).load()
    for cid in set(mine.cids()) | set(theirs.cids()):
        assert cid in back.cids(), f"a signed entry went missing: {cid}"
    back.verify_all()
    back.check_authority()

    # And the log is writable again from here.
    record_observation(back, polity.did, Signer.generate(),
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=NOW + 2)
    assert store.save(back).wrote_anything


def test_reconciling_a_healthy_charter_log_moves_nothing(polity, store):
    store.save(polity.archive)
    report = store.reconcile()
    assert not report.was_forked
    assert report.entries_moved == 0 and report.cids_moved == []
    assert e.FileStore(store.path).load().cids() == polity.archive.cids()
