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

"""A reference host for a polity, in Python: the loop, and the one clock.

The mirror of `src/rust/examples/polity_host`, and it exists for the same reason: a
host is the one thing in this tier nobody can write for you, so there should be
something short enough to read in one sitting and copy. Before **D31** there was no
Python version to copy, because `en-host` did not cross the binding at all; a Python
host reimplemented the cadence bookkeeping and the read-model assembly, and got the
second one wrong in the direction of health.

Every cadence in ethne is a query-time parameter, which is what makes the tier's
judgements reproducible: no library that *decides* anything reads a clock. **This file
is where that stops.** The single `now_seconds()` in the loop below is the only clock
read on the Python side outside a test, save for the binding's `*_at=None ⇒ now`
convenience, which is an affordance for callers rather than a judgement made from the
time.

    python polity_host.py                    # a simulated year, exits 0
    python polity_host.py --ticks 30 --step 3600
    python polity_host.py --follow           # the real loop: wall clock, sleeps
    python polity_host.py --second-writer    # D30: what a second host meets
"""
from __future__ import annotations

import argparse
import os
import tempfile
import time
from dataclasses import dataclass
from typing import List

import ethne as e
from ethne.authoring import (
    Signer,
    prune_observations,
    ratify_charter,
    reaffirm_member,
    record_observation,
)
from ethne.host import HostConfig, HostCycle
from ethne.monitor import founding_candidate, manual_founding
from ethne.uplift import coherence_concern

DAY = 86_400

SUBSTRATE = ("the long watch", "answers to what it did not author")


@dataclass
class Keys:
    """The keys a host holds. In a real deployment these come from custody the host
    arranges; that they are generated here is the one thing this example fakes."""

    polity: str
    members: List[Signer]
    #: **Three of them, rotated by `observe`.** D19's retention floor counts distinct
    #: *watchers* per member since D28, so a lone monitor filing every day pins the
    #: horizon and no `Prune` ever comes due. Measured on one log: one monitor gives
    #: `prune_recommendation` `None`, three gives `dropped=370`. The Rust reference host
    #: held one until D31 and its compaction path was dead for the same reason.
    monitors: List[Signer]
    observer: Signer

    @classmethod
    def generate(cls, n: int) -> "Keys":
        _, _, polity_did = e.generate_keypair()
        return cls(
            polity=polity_did,
            members=[Signer.generate() for _ in range(n)],
            monitors=[Signer.generate() for _ in range(3)],
            observer=Signer.generate(),
        )

    def candidate(self):
        """What the substrate provider currently reads: everyone present, well
        regarded, and nobody derived."""
        return founding_candidate(
            [(m.did, 0.9, None, None, False) for m in self.members]
        )

    def found(self, archive, at: int) -> None:
        candidate = manual_founding([m.did for m in self.members], *SUBSTRATE)
        proposal = e.propose_from_candidate(
            candidate,
            self.polity,
            commitments=[("mutual-aid", "we answer for one another")],
            min_reputation=0.5,
            proposed_at=at,
        )
        # The WHOLE roll ratifies, which since D29 is what inducts them: a founder who
        # never signed holds no substrate, so a host that ratified by bare majority
        # would start its polity with members who carry no weight and cannot renew.
        ratify_charter(archive, proposal, self.members, founded_at=at)

    def observe(self, archive, at: int) -> None:
        """One monitor's filing, rotating over the three by the day being observed: a
        real cohort of monitors files on its own schedule, and the retention floor needs
        distinct watchers per member rather than one party's word repeated (D28)."""
        monitor = self.monitors[(at // DAY) % len(self.monitors)]
        record_observation(
            archive,
            self.polity,
            monitor,
            [(m.did, 0.9, None, at) for m in self.members],
            observed_at=at,
        )

    def reaffirm(self, archive, at: int) -> None:
        for m in self.members:
            reaffirm_member(archive, self.polity, m, affirmed_at=at)

    def prune(self, archive, horizon: int, at: int):
        """Author a pruning at `horizon` on the roll-majority route, then return the
        archive with the observations actually dropped."""
        prune_observations(
            archive,
            self.polity,
            horizon,
            self.members,
            rationale="retention policy",
            pruned_at=at,
        )
        return archive.apply_prunings(self.polity, as_of=at)


def perform(action, keys: Keys, archive, store, as_of: int):
    """Carry out one action. **This is the half `tick` deliberately does not do**:
    every arm here either touches storage or uses a signing key.

    Returns `(line, archive)`, because pruning replaces the archive rather than
    mutating it.
    """
    if action.kind == "Save":
        out = store.save(archive)
        if not out.wrote_anything:
            return "save: nothing new", archive
        if out.rewrote:
            return (
                f"save: REWROTE {out.appended} entries ({out.bytes} bytes), "
                "the log was compacted"
            ), archive
        return f"save: appended {out.appended} ({out.bytes} bytes)", archive

    if action.kind == "Cohesion":
        r = action.report
        return (
            f"cohesion: {r.erosion}, {r.observed} of {r.chartered} observed, "
            f"{r.stale_affirmations} lapsed on affirming, "
            f"{r.watchers[0] if r.watchers else 0} watcher(s)"
        ), archive

    if action.kind == "RaiseConcern":
        # The host signs it; `tick` only said it was warranted.
        record = coherence_concern(
            action.report, keys.polity, keys.observer, at=as_of
        )
        size = len(record) if record else 0
        return (
            f"CONCERN raised to kith-covenant ({size} bytes of signed record)"
        ), archive

    if action.kind == "Prune":
        rec = action.recommendation
        # The recommendation names a horizon; the polity ratifies it.
        archive = keys.prune(archive, rec.horizon, as_of)
        # The store must be told. Its own generation check turns this into a full
        # rewrite rather than an append, which is D20's whole point.
        out = store.save(archive)
        return (
            f"pruned {rec.dropped} observations (keeping "
            f"{rec.seconds_retained // DAY}d); store "
            f"{'REWROTE' if out.rewrote else 'appended'} {out.bytes} bytes"
        ), archive

    raise AssertionError(f"unknown action kind {action.kind!r}")


def second_writer(path: str, first, keys: Keys, archive, as_of: int) -> None:
    """What a second host actually meets, the **D30** half of this example.

    Two writers on one log is not hypothetical: a supervisor that restarts a host while
    the old one is still alive, or a one-shot invocation run alongside a following loop,
    is the ordinary way it happens. Before D30 the second writer's compaction silently
    discarded the first's already-fsynced entries, with every call returning normally
    and the reloaded log verifying clean.
    """
    print("\n--- a second writer arrives ---------------------------------------")

    # 1. The claim: the refusal a supervisor can act on.
    try:
        e.FileStore(path).claim()
        print("  claim: TAKEN, which would be the bug")
    except Exception as exc:  # noqa: BLE001 - the refusal is the point
        print(f"  claim refused, at boot: {exc}")

    # 2. And with no claim held, the compare-and-swap catches it anyway. That is the
    #    case a lock cannot reach: a second host running *after* this one exited has
    #    nothing to contend for, so there was never anything for a lock to do.
    first.release()
    print("  (first host released its claim)\n")

    second = e.FileStore(path)
    borrowed = second.load()
    print(f"  second host loaded {len(borrowed)} entries at {second.cursor()}")

    # The first host moves on and makes it durable.
    keys.observe(archive, as_of)
    out = first.save(archive)
    print(f"  first host appended {out.appended} entry, fsynced ({out.bytes} bytes)")

    # And the second host records its own work, not knowing any of that happened.
    keys.observe(borrowed, as_of)
    try:
        out = second.save(borrowed)
        print(f"  second host's save: {out}, which would be the bug")
    except Exception as exc:  # noqa: BLE001 - the refusal is the point
        print(f"  second host's save refused:\n    {exc}")

    # And the log is intact, which is the whole point of refusing.
    back = e.FileStore(path).load()
    back.verify_all()
    back.check_authority()
    print(
        f"\n  the log still loads, verifies, and holds all {len(back)} entries, "
        "including the one the second host would have discarded"
    )
    assert len(back) == len(archive), "nothing durable may go missing"


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--ticks", type=int, default=40)
    p.add_argument("--step", type=int, default=10 * DAY, metavar="SECONDS")
    p.add_argument("--start", type=int, default=200 * DAY, metavar="SECONDS")
    p.add_argument("--follow", action="store_true", help="the real loop: wall clock")
    p.add_argument("--second-writer", action="store_true")
    p.add_argument(
        "--path",
        default=os.path.join(tempfile.gettempdir(), "polity_host_py_demo.log"),
    )
    return p.parse_args()


def main() -> None:
    args = parse_args()
    keys = Keys.generate(3)
    if os.path.exists(args.path):
        os.remove(args.path)  # a fresh demo each run
    store = e.FileStore(args.path)

    # D30: claim the log before doing anything with it. A host that does this finds out
    # at boot that another host is already writing, which is a refusal a supervisor can
    # act on; without it the discovery comes at the first save. Every write is guarded
    # either way, so this only moves the discovery earlier.
    store.claim()

    print(f"polity host, storage at {args.path}")
    print(f"write lock claimed ({args.path}.lock)")
    if args.follow:
        print("following the wall clock (Ctrl-C to stop)")
    else:
        print(
            f"simulating {args.ticks} ticks of {args.step // DAY}d "
            f"from t={args.start // DAY}d"
        )

    # --- found, and persist the founding before anything else ----------------
    archive = store.load()
    if len(archive) == 0:
        keys.found(archive, 100)
        archive.check_authority()
        out = store.save(archive)
        print(f"founded and persisted ({out.bytes} bytes)\n")

    cycle = HostCycle(HostConfig(keys.polity))
    compactions = 0
    as_of = args.start

    # --- the loop ------------------------------------------------------------
    for n in range(args.ticks):
        # THE ONE CLOCK. Everything below this line is given a time rather than
        # reading one, which is why `en-host` is testable at any moment and this file
        # is not.
        as_of = e.now_seconds() if args.follow else args.start + n * args.step

        # What the substrate reported since the last tick, recorded as evidence. A real
        # monitor reports far faster than the host wakes, so a tick folds in the
        # interval's worth, which is also what makes the log grow enough for retention
        # to have anything to say.
        per_tick = min(max(args.step // DAY, 1), 24)
        for k in range(per_tick):
            keys.observe(archive, as_of - (per_tick - 1 - k) * DAY)

        # And the members renew their hold on the substrate well inside the affirmation
        # cadence, so this is a healthy polity rather than one that has quietly stopped.
        # (Drop this and D13's transmission decay reaches Concern on its own within a
        # year, which it should.)
        if n % 10 == 0:
            keys.reaffirm(archive, as_of)

        for action in cycle.tick(archive, keys.candidate(), as_of=as_of):
            was_prune = action.kind == "Prune"
            line, archive = perform(action, keys, archive, store, as_of)
            # Steady state is the boring majority: print it periodically, and let the
            # events that actually change something always show.
            notable = was_prune or "CONCERN" in line or "REWROTE" in line
            if notable or n % 10 == 0:
                print(f"  t={as_of // DAY:>5}d  {line}")
            if was_prune:
                compactions += 1

        if args.follow:
            time.sleep(max(args.step, 1))

    if args.second_writer:
        last = args.start + args.ticks * args.step
        second_writer(args.path, store, keys, archive, last)

    # --- what the cycle guarantees that a hand-rolled grade does not ---------
    #
    # D31. The report says which of the seven readings it was given, so this is one
    # assertion rather than a survey of the measures, and it stays true as read-models
    # are added.
    # Graded at the last tick, where the polity is genuinely healthy: grading well after
    # the monitors stopped filing would report a sustained absence that is an artifact of
    # the demo ending rather than anything about the polity.
    final = as_of
    graded = cycle.grade(archive, keys.candidate(), as_of=final)
    assert not graded.omitted_readings, f"grade forgot {graded.omitted_readings}"
    assert not graded.blind_to_concern
    by_hand = e.cohesion_report(
        archive.charter_state(keys.polity, as_of=final).members,
        keys.candidate(),
        as_of=final,
    )
    print("\n--- the assembly the cycle does for you --------------------------")
    print(f"through the cycle: erosion={graded.erosion}, omitted=[]")
    print(
        f"by hand         : erosion={by_hand.erosion}, "
        f"omitted={len(by_hand.omitted_readings)} of 7, "
        f"blind_to_concern={by_hand.blind_to_concern}"
    )
    print(f"  silent: {', '.join(by_hand.silent_measures)}")

    # --- restart: what a host actually depends on ----------------------------
    #
    # The check D20 exists for. Reload from storage alone and demand it match the live
    # log exactly. Before that slice the two silently diverged the moment a pruning
    # happened, and every verification pass still said the reloaded log was fine.
    print("\n--- restart ------------------------------------------------------")
    reloaded = e.FileStore(args.path).load()
    print(f"live    : {len(archive)} entries, generation {archive.generation}")
    print(f"reloaded: {len(reloaded)} entries, generation {reloaded.generation}")
    print(f"{compactions} compaction(s) survived the round trip")

    reloaded.verify_all()
    reloaded.check_authority()
    assert archive.cids() == reloaded.cids(), (
        "storage and the live log must agree, CID for CID"
    )
    print("signatures verify, authority holds, and the two agree CID for CID")

    os.remove(args.path)


if __name__ == "__main__":
    main()
