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

"""§2 admissions integrity through the Python surface: admission authoring, the
sponsor rule, the examining pool, and the probation predicate.

The exhaustive rule-by-rule coverage lives in the Rust `kc-archive` authority tests
(`tests::admissions_integrity`); these confirm the bindings and facades wire the
whole chain — author -> verify_all -> check_authority -> reputation.
"""
import pytest

from kith_covenant import (
    Archive, new_identity, new_examiner, register_examiners, record_admission,
    probationary_at, vouch, reputation_of, now_seconds,
    ratify_constitution, amend_compact_incapable, deliberative_capacity,
)

YEAR = 365 * 24 * 3600


def _found(arch, t0, names):
    """Found a compact: each community publishes an identity, then all of them seal
    the formation record together. A formation has no sitting members to draw an
    admitting quorum from, so §2 exempts it — these are founders, not entrants."""
    members = [new_identity(arch, n, t0) for n in names]
    record_admission(
        arch, members, [m.did for m in members],
        compact_label="compact", witnessed_at=t0,
    )
    return members


def test_compact_formation_is_authorized():
    arch = Archive()
    t0 = now_seconds()
    _found(arch, t0, ("A", "B"))
    arch.verify_all()
    arch.check_authority()


def test_faction_of_one_cannot_seat_a_second_entrant():
    """Move 7, refused. The faction co-founds with its first entrant, then tries to
    admit a second: the only sitting community carrying weight is the faction itself,
    because its co-founder is inside the probation window the faction's own vouch
    opened."""
    arch = Archive()
    t0 = now_seconds()
    faction, first = _found(arch, t0, ("Faction", "Entrant-0"))
    vouch(arch, faction, first.did, probation_until=t0 + YEAR, sponsored_at=t0)

    second = new_identity(arch, "Entrant-1", t0)
    vouch(arch, faction, second.did, probation_until=t0 + YEAR, sponsored_at=t0)
    party = [faction, first, second]
    cid = record_admission(
        arch, party, [m.did for m in party],
        compact_label="compact", witnessed_at=t0 + 1,
    )
    arch.verify_all()  # crypto-intact either way; authority is the separate pass
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None, "a faction of one must not seat an entrant"
    assert "admitting quorum" in reason


def test_admission_needs_a_sponsor():
    """Sponsor stake with teeth: probation only opens when somebody stakes standing,
    so an admission that carries no vouch would leave nothing to de-weight."""
    arch = Archive()
    t0 = now_seconds()
    a, b = _found(arch, t0, ("A", "B"))
    entrant = new_identity(arch, "Entrant", t0)  # nobody vouches
    party = [a, b, entrant]
    cid = record_admission(
        arch, party, [m.did for m in party],
        compact_label="compact", witnessed_at=t0 + 1,
    )
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None and "sponsor" in reason


def test_probationary_community_may_not_sponsor():
    arch = Archive()
    t0 = now_seconds()
    a, b = _found(arch, t0, ("A", "B"))
    entrant = new_identity(arch, "Entrant", t0)
    vouch(arch, a, entrant.did, probation_until=t0 + YEAR, sponsored_at=t0)
    # The entrant, still in its window, immediately vouches for the next candidate.
    nxt = new_identity(arch, "Next", t0)
    cid = vouch(arch, entrant, nxt.did, probation_until=t0 + YEAR, sponsored_at=t0 + 1)
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None and "probation" in reason


def test_self_sponsorship_is_refused():
    """A vouch is another community's testimony; the stake and the conduct it rides
    on cannot belong to the same community."""
    arch = Archive()
    t0 = now_seconds()
    (a,) = _found(arch, t0, ("A",))
    cid = vouch(arch, a, a.did, probation_until=t0 + YEAR, sponsored_at=t0)
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None and "vouched for itself" in reason


def test_probationary_at_agrees_with_standing():
    """One predicate, two readers. `reputation_of(...).standing` is derived from the
    same `probationary_at` the §2 authority rules enforce, so a dashboard and a gate
    cannot disagree about who is in a window."""
    arch = Archive()
    t0 = now_seconds()
    a, entrant = _found(arch, t0, ("A", "Entrant"))
    vouch(arch, a, entrant.did, probation_until=t0 + 1000, sponsored_at=t0 + 100)

    for at in (t0, t0 + 99, t0 + 100, t0 + 500, t0 + 1000, t0 + 5000):
        standing = reputation_of(arch, entrant.did, as_of=at).standing
        assert (standing == "Probationary") == probationary_at(arch, entrant.did, at), at


# --- the examining pool -------------------------------------------------------

def test_open_ended_examining_term_is_refused():
    """§2 allows no open-ended examining standing: a permanent examiner is the
    standing admissions cadre the rotation rule exists to prevent. The authoring call
    accepts it; the authority pass is what refuses it."""
    arch = Archive()
    t0 = now_seconds()
    (a,) = _found(arch, t0, ("A",))
    ex = new_examiner()
    cid = register_examiners(arch, a, [ex], valid_from=t0)  # no valid_until
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None and "open-ended" in reason


def test_bounded_examining_term_is_authorized():
    arch = Archive()
    t0 = now_seconds()
    (a,) = _found(arch, t0, ("A",))
    ex = new_examiner()
    register_examiners(arch, a, [ex], valid_from=t0, valid_until=t0 + 90 * 24 * 3600)
    arch.verify_all()
    arch.check_authority()


def test_examining_term_beyond_the_cap_is_refused():
    arch = Archive()
    t0 = now_seconds()
    (a,) = _found(arch, t0, ("A",))
    ex = new_examiner()
    cid = register_examiners(arch, a, [ex], valid_from=t0, valid_until=t0 + 2 * YEAR)
    reason = dict(arch.authority_violations()).get(cid)
    assert reason is not None and "term" in reason


def test_register_examiners_requires_at_least_one():
    arch = Archive()
    t0 = now_seconds()
    (a,) = _found(arch, t0, ("A",))
    with pytest.raises(ValueError, match="at least one examiner"):
        register_examiners(arch, a, [], valid_from=t0)


# --- the reach of the withholding (the purposive reading) ---------------------

def test_a_probationary_quorum_cannot_amend_the_compact_incapable_list():
    """§7's withholding follows its reason, not its three-item list: probation
    carries no weight toward *any* second-class quorum. Measured green before the
    rule — two communities admitted this year, amending the compact's
    constitutional list between them."""
    arch = Archive()
    t0 = now_seconds()
    a, b = _found(arch, t0, ("A", "B"))
    ratify_constitution(arch, [a, b], [("genesis_m", "immutable")],
                        [("annual", t0 + 1000, t0 + 2000)], genesis_at=t0)
    e1 = new_identity(arch, "E1", t0)
    e2 = new_identity(arch, "E2", t0)
    for e in (e1, e2):
        vouch(arch, a, e.did, probation_until=t0 + YEAR, sponsored_at=t0 + 100)

    bad = amend_compact_incapable(arch, "Add", "m_x", [e1, e2], decided_at=t0 + 200)
    reason = dict(arch.authority_violations()).get(bad)
    assert reason is not None and "probationary" in reason and "§2" in reason
    # Control: the same act on standing weight is authorized.
    good = amend_compact_incapable(arch, "Add", "m_ok", [a, b], decided_at=t0 + 210)
    assert good not in dict(arch.authority_violations())


def test_deliberative_capacity_reports_what_the_compact_can_do():
    """The lens for the refusal above: a compact that admits a cohort all at once
    can lose the ability to take a second-class decision at all, and recovers by
    the passage of time alone."""
    arch = Archive()
    t0 = now_seconds()
    a, b, c = _found(arch, t0, ("A", "B", "C"))

    seasoned = deliberative_capacity(arch, "compact", quorum_threshold=2, as_of=t0 + 10)
    assert len(seasoned.members) == 3
    assert seasoned.standing == 3 and seasoned.standing_share == 1.0
    assert seasoned.can_act and seasoned.can_contract

    for m in (b, c):
        vouch(arch, a, m.did, probation_until=t0 + YEAR, sponsored_at=t0 + 100)
    thin = deliberative_capacity(arch, "compact", quorum_threshold=2, as_of=t0 + 200)
    assert len(thin.members) == 3, "probation does not remove anyone from the roll"
    assert thin.standing == 1 and len(thin.probationary) == 2
    assert not thin.can_act and not thin.can_contract

    later = deliberative_capacity(arch, "compact", quorum_threshold=2,
                                 as_of=t0 + YEAR + 1)
    assert later.standing == 3 and later.can_act and later.can_contract


def test_deliberative_capacity_of_an_unknown_compact_is_not_evaluated():
    arch = Archive()
    t0 = now_seconds()
    _found(arch, t0, ("A", "B"))
    c = deliberative_capacity(arch, "some-other-compact", as_of=t0 + 10)
    assert c.members == () and c.standing == 0
    assert c.standing_share is None, "an empty roll is not a lapsed one"
    assert not c.can_act
