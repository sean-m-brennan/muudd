#  Copyright 2026 Sean M. Brennan and contributors
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

"""The recited object: substitution measured against the compact's own denominator.

The substitution dimensions grade enforcement coverage against a fixed taxonomy of eight
grounds, which is what makes them absolute. This is the finer reading the substitution
work named as out of reach: the denominator is the object the compact's own members
recited, pinned to their first rite.

The measurement that produced it. A compact that halved what it recited across three
generational rites, with rising witness counts and nothing amended, scored **1.0** on
shared-taxonomy coverage at volume 32 and graded `None` on every one of the fourteen
erosion dimensions. Eight grounds are coarse enough that an undertaking can be abandoned
whole without moving any of them.

Exhaustive rule coverage is in the Rust suites (`kc-archive::tests::recited_object` for
the citation rule and `kc-reputation::tests::recited_object` for the readings); this is
the Python surface and the headline regression.
"""
import pytest

from kith_covenant import (
    Archive, new_identity, now_seconds, record_admission, record_conduct,
    run_intra_rite, compact_object, commitment_coverage, commitment_contraction,
    enforcement_convergence, erosion_alert,
)

DAY = 86_400
GROUNDS = (
    ("AntiSpiral", "EnemyLoveOperational"),
    ("AntiSpiral", "FallenActorAnthropology"),
    ("AntiSpiral", "GraceOverMerit"),
    ("AntiSpiral", "ScapegoatDisarming"),
    ("AntiSpiral", "NonUltimacyOfCompact"),
    ("Charity", None),
    ("Rescue", None),
    ("CommonsObligation", None),
)
RECITED = ("truthfulness", "restitution", "hospitality", "sabbath-rest", "rescue-readiness")


def _commitments(keys):
    return [(k, f"we hold {k}") for k in keys]


def _compact(recited=RECITED, later=None, n=5):
    """A sealed compact of `n` reciting `recited` at its founding rite, and `later` (if
    given) at the two rites after it, with rising witness counts. Every form kept."""
    arch = Archive()
    t0 = now_seconds()
    members = [new_identity(arch, chr(ord("A") + i), t0) for i in range(n)]
    record_admission(
        arch, members, [m.did for m in members],
        compact_label="compact", witnessed_at=t0,
    )
    for m in members:
        run_intra_rite(arch, m, commitments=_commitments(recited),
                       witness_count=4, sealed_at=t0 + 1)
        if later is not None:
            run_intra_rite(arch, m, commitments=_commitments(later),
                           witness_count=6, sealed_at=t0 + 2)
    return arch, t0, members


def _enforce(arch, members, cited, count, t0):
    """`count` enforcement acts citing only `cited`, spread over the whole shared
    taxonomy so the eight-ground reading has nothing to say."""
    for i in range(count):
        bearing, feature = GROUNDS[i % len(GROUNDS)]
        record_conduct(
            arch, members[i % 2], members[2 + i % (len(members) - 2)].did,
            "Violation", bearing, feature=feature,
            ladder_rung="MediatedArbitration",
            commitments_cited=[cited[i % len(cited)]],
            recorded_at=t0 + 100 * DAY + i * 60,
        )


def _alert(arch, at):
    return erosion_alert(
        arch, at,
        attestation_cadence_seconds=36500 * DAY,
        rite_cadence_seconds=36500 * DAY,
        grievance_cadence_seconds=36500 * DAY,
    )


# --- the headline regression --------------------------------------------------

def test_the_shared_taxonomy_cannot_see_a_narrowed_object():
    """Two compacts, the same volume of enforcement over the same eight grounds, and
    only what the enforcement is *about* differing. Before this reading existed they
    were the same archive on every instrument the audit had."""
    narrow_arch, nt0, nm = _compact(later=RECITED[:2])
    whole_arch, wt0, wm = _compact(later=RECITED)
    _enforce(narrow_arch, nm, RECITED[:2], 24, nt0)
    _enforce(whole_arch, wm, RECITED, 24, wt0)
    at = nt0 + 200 * DAY

    # The shared taxonomy reads both as perfectly covered.
    for arch, t in ((narrow_arch, nt0), (whole_arch, wt0)):
        conv = enforcement_convergence(arch, t + 200 * DAY)
        assert conv.coverage == 1.0 and conv.volume == 24

    # The recited object separates them.
    narrow = commitment_coverage(narrow_arch, at)
    whole = commitment_coverage(whole_arch, wt0 + 200 * DAY)
    assert narrow.object_size == whole.object_size == 5
    assert narrow.coverage == 0.4 and whole.coverage == 1.0
    assert set(narrow.unenforced) == set(RECITED[2:])
    assert whole.unenforced == ()

    an, aw = _alert(narrow_arch, at), _alert(whole_arch, wt0 + 200 * DAY)
    assert an.severity == "Alert" and aw.severity == "None"
    assert an.commitment_coverage is not None and aw.commitment_coverage is None
    assert an.commitment_contraction is not None and aw.commitment_contraction is None
    # And every other dimension is inert on the narrowing archive, which is the finding.
    assert an.enforcement_convergence is None and an.enforcement_direction is None
    assert an.reporter_targeting is None and an.unanswered_grievances == ()
    assert an.silent_functions == () and an.unstaffed_functions == ()
    assert not an.list_contraction and not an.minimum_contraction


# --- the denominator ----------------------------------------------------------

def test_the_object_is_pinned_to_the_first_rite():
    arch, t0, _m = _compact(later=RECITED[:1])
    obj = compact_object(arch, t0 + 100 * DAY)
    assert obj.size == 5 and set(obj.keys) == set(RECITED)
    assert len(obj.per_community) == 5


def test_below_the_scaled_floor_the_reading_is_untestable():
    arch, t0, m = _compact()
    _enforce(arch, m, RECITED[:2], 4, t0)
    c = commitment_coverage(arch, t0 + 200 * DAY)
    # Five commitments at two acts each: ten cited acts before the reading exists.
    assert c.cited_acts == 4
    assert _alert(arch, t0 + 200 * DAY).commitment_coverage is None


def test_a_compact_that_cites_nothing_is_untestable_rather_than_narrowing():
    """The correction the determined-adversary campaign forced. Silence about the
    object is not evidence about the object, and reading it as narrowing would measure
    adoption of a citation habit rather than what the compact enforces."""
    arch, t0, m = _compact()
    for i in range(24):
        bearing, feature = GROUNDS[i % len(GROUNDS)]
        record_conduct(
            arch, m[i % 2], m[2 + i % 3].did, "Violation", bearing, feature=feature,
            ladder_rung="MediatedArbitration", recorded_at=t0 + 100 * DAY + i * 60,
        )
    c = commitment_coverage(arch, t0 + 200 * DAY)
    assert (c.acts, c.cited_acts, c.uncited_share) == (24, 0, 1.0)
    assert c.coverage == 0.0
    assert _alert(arch, t0 + 200 * DAY).commitment_coverage is None


def test_a_commitment_taken_on_later_counts_off_object():
    arch, t0, m = _compact(later=RECITED + ("almsgiving",))
    _enforce(arch, m, ("truthfulness", "almsgiving"), 24, t0)
    c = commitment_coverage(arch, t0 + 200 * DAY)
    assert c.object_size == 5, "the pin does not grow with a later recitation"
    assert c.off_object.get("almsgiving") == 12
    assert "almsgiving" not in c.per_commitment


# --- the contraction ----------------------------------------------------------

def test_a_dropped_commitment_is_a_contraction_and_a_broadened_list_is_not():
    dropped, dt0, _ = _compact(later=RECITED[:2])
    c = commitment_contraction(dropped, dt0 + 100 * DAY)
    assert c.communities == 5 and c.total_dropped == 3

    broadened, bt0, _ = _compact(later=RECITED + ("almsgiving",))
    assert commitment_contraction(broadened, bt0 + 100 * DAY).communities == 0


# --- the rite refuses an empty denominator ------------------------------------

def test_a_rite_reciting_no_commitments_is_refused():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    with pytest.raises(ValueError, match="at least one commitment"):
        run_intra_rite(arch, a, commitments=[], sealed_at=t0 + 1)


@pytest.mark.parametrize("commitments, fragment", [
    ([("truthfulness", "we tell the truth"), ("truthfulness", "again")], "recited once"),
    ([("", "we tell the truth")], "key and a text"),
    ([("truthfulness", "  ")], "key and a text"),
])
def test_a_malformed_commitment_list_is_refused(commitments, fragment):
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    with pytest.raises(ValueError, match=fragment):
        run_intra_rite(arch, a, commitments=commitments, sealed_at=t0 + 1)


def test_a_cited_commitment_nobody_recited_is_refused():
    """The item-A shape one layer down: a citation that resolves to nothing would count
    as coverage of a commitment that does not exist."""
    arch, t0, m = _compact()
    cid = record_conduct(
        arch, m[0], m[2].did, "Violation", "Charity",
        ladder_rung="MediatedArbitration",
        commitments_cited=["obedience-to-the-council"],
        recorded_at=t0 + 100 * DAY,
    )
    assert "never recited" in dict(arch.authority_violations())[cid]
