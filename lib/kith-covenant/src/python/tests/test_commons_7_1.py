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

"""§7.1 — the parts §7 specifies and the tier did not carry.

Each began as a measurement against the code as it stood:

- A commons could state a provision rule and no rule read it, so a record drawing ten
  times the written cap was authorized exactly like one that honoured it. And every
  figure was public, so a community accounting honestly disclosed its capacity, its
  consumption pattern and its reserves to every other member.
- A community could be excluded from a commons, restored, and excluded again across
  years, and reach §5's designation track by no commons-specific route at all.
- Nothing distinguished a compact-level commons from an inter-compact one, because a
  compact had no identity, so a body governing a commons shared *between* compacts
  reached past a compact and sanctioned one of its member communities.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, new_identity, register_commons, record_usage, apply_sanction,
    open_usage_share, open_usage, charter_compact, escalate_commons, record_admission,
    free_rider_signals, now_seconds,
)

YEAR = 365 * 24 * 3600
ALLOWANCE = 1000


def _commons(allowance=None, level="Community", kind="Communication"):
    """Four appropriators A/B/C/D of one commons. B is the reporter and subject."""
    arch = Archive()
    parties = [new_identity(arch, n) for n in ("A", "B", "C", "D")]
    t0 = now_seconds()
    commons = register_commons(
        arch, kind, "the relay", [p.did for p in parties], parties,
        description="shared bandwidth",
        provision_rule="no member may draw more than 1000 units per period",
        registered_at=t0, draw_allowance=allowance, level=level,
    )
    return arch, parties, commons, t0


# --- Sealed usage and the allowance ------------------------------------------


def test_a_community_can_account_without_publishing_its_draw():
    """THE MEASUREMENT, now the rule."""
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, d = parties
    usage = record_usage(
        arch, commons, b, 900, 100, recorded_at=t0 + 10,
        seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
    )
    arch.verify_all()
    arch.check_authority()

    # A custodian quorum recovers the figures for an audit.
    shares = [open_usage_share(arch, usage, h) for h in (a, c)]
    open_usage(arch, usage, [a, c], 900, 100, shares, "the annual audit",
               opened_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()


def test_a_draw_over_the_allowance_cannot_be_sealed():
    """The property the whole construction exists for: the proof cannot be made."""
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, d = parties
    with pytest.raises(RuntimeError):
        record_usage(
            arch, commons, b, 1500, 0, recorded_at=t0 + 10,
            seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
        )
    # NEGATIVE CONTROL: exactly at the allowance is within it.
    record_usage(
        arch, commons, b, ALLOWANCE, 0, recorded_at=t0 + 20,
        seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
    )
    arch.check_authority()


def test_a_commons_without_an_allowance_cannot_host_sealed_usage():
    """Sealing against nothing would leave the compact holding a number it can neither
    see nor bound, which is worse than publishing."""
    arch, parties, commons, t0 = _commons(allowance=None)
    a, b, c, d = parties
    with pytest.raises(ValueError):
        record_usage(
            arch, commons, b, 100, 0, recorded_at=t0 + 10,
            seal_custodian_dids=[a.did, c.did, d.did],
        )


def test_the_reporter_may_not_hold_custody_of_its_own_figures():
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, _d = parties
    record_usage(
        arch, commons, b, 900, 0, recorded_at=t0 + 10,
        seal_custodian_dids=[b.did, a.did, c.did], seal_threshold=2,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("over its own usage figures" in reason
               for _cid, reason in arch.authority_violations())


def test_an_opening_cannot_publish_figures_the_custody_does_not_recover():
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, d = parties
    usage = record_usage(
        arch, commons, b, 900, 100, recorded_at=t0 + 10,
        seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
    )
    shares = [open_usage_share(arch, usage, h) for h in (a, c)]
    open_usage(arch, usage, [a, c], 10, 900, shares, "audit", opened_at=t0 + 100)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("do not recover" in reason
               for _cid, reason in arch.authority_violations())


def test_opening_must_cite_a_cause():
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, d = parties
    usage = record_usage(
        arch, commons, b, 900, 100, recorded_at=t0 + 10,
        seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
    )
    shares = [open_usage_share(arch, usage, h) for h in (a, c)]
    with pytest.raises(ValueError):
        open_usage(arch, usage, [a, c], 900, 100, shares, "   ")


def test_sealing_does_not_hide_a_drawer_from_the_free_rider_lens():
    """The measurement that shaped the design: sealing must not be a way to disappear
    from the detector. The lens reports what it cannot see, and turns the proofs into a
    bound on the hidden draw."""
    arch, parties, commons, t0 = _commons(allowance=ALLOWANCE)
    a, b, c, d = parties
    record_usage(arch, commons, c, 100, 500, recorded_at=t0 + 10)
    record_usage(
        arch, commons, b, 900, 0, recorded_at=t0 + 20,
        seal_custodian_dids=[a.did, c.did, d.did], seal_threshold=2,
    )
    report = free_rider_signals(arch, commons, as_of=t0 + 1000)
    assert report.total_consumed == 100
    assert not report.is_complete
    assert report.partially_sealed == ((b.did, 1),)
    assert report.max_hidden_draw == ALLOWANCE


# --- The commons escalation trigger ------------------------------------------


def _climb(arch, commons, subject, quorum, one, start, restore):
    """Climb None -> Exclusion, optionally restoring afterwards. Returns the CIDs."""
    steps = [
        ("None", "Monitoring", one, "FirstClass"),
        ("Monitoring", "Warning", one, "FirstClass"),
        ("Warning", "Restriction", quorum, "SecondClass"),
        ("Restriction", "Exclusion", quorum, "SecondClass"),
    ]
    if restore:
        steps.append(("Exclusion", "None", one, "FirstClass"))
    cids = []
    for i, (frm, to, movers, dc) in enumerate(steps):
        cids.append(apply_sanction(
            arch, commons, subject.did, frm, to, movers,
            basis="sustained overdraw", decision_class=dc, decided_at=start + i * 10,
        ))
    return cids


def test_two_cycles_and_a_year_excluded_reach_the_designation_track():
    """THE MEASUREMENT, now the rule."""
    arch, parties, commons, t0 = _commons()
    a, b, c, _d = parties
    first = _climb(arch, commons, b, [a, c], [a], t0 + 100, True)
    second = _climb(arch, commons, b, [a, c], [a], t0 + 1000, False)
    escalate_commons(
        arch, commons, b.did, "twice excluded and drawing down again",
        first + second, [a, c], escalated_at=t0 + 1030 + YEAR,
    )
    arch.verify_all()
    arch.check_authority()


def test_one_exclusion_is_not_defiance():
    arch, parties, commons, t0 = _commons()
    a, b, c, _d = parties
    only = _climb(arch, commons, b, [a, c], [a], t0 + 100, False)
    escalate_commons(
        arch, commons, b.did, "once excluded", only, [a, c],
        escalated_at=t0 + 130 + YEAR,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("one offence is what the ladder is for" in reason
               for _cid, reason in arch.authority_violations())


def test_a_life_critical_commons_needs_only_one_cycle():
    """Life support and rescue are what the compact keeps running when it pauses
    everything else."""
    for kind, expect_ok in [("LifeSupport", True), ("Rescue", True),
                            ("Communication", False)]:
        arch, parties, commons, t0 = _commons(kind=kind)
        a, b, c, _d = parties
        only = _climb(arch, commons, b, [a, c], [a], t0 + 100, False)
        escalate_commons(
            arch, commons, b.did, "once excluded", only, [a, c],
            escalated_at=t0 + 130 + YEAR,
        )
        arch.verify_all()
        if expect_ok:
            arch.check_authority()
        else:
            with pytest.raises(RuntimeError):
                arch.check_authority()


def test_a_subject_mid_restoration_is_not_escalated():
    """Restoration is always available from exclusion, and the finding is that it was
    available and not taken."""
    arch, parties, commons, t0 = _commons(kind="LifeSupport")
    a, b, c, _d = parties
    only = _climb(arch, commons, b, [a, c], [a], t0 + 100, False)
    escalate_commons(
        arch, commons, b.did, "a week in", only, [a, c],
        escalated_at=t0 + 130 + 7 * 24 * 3600,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("has not refused it" in reason
               for _cid, reason in arch.authority_violations())


def test_the_finding_must_cite_sanctions():
    arch, parties, commons, t0 = _commons(kind="Rescue")
    a, b, c, _d = parties
    _climb(arch, commons, b, [a, c], [a], t0 + 100, False)
    with pytest.raises(ValueError):
        escalate_commons(arch, commons, b.did, "no citation", [], [a, c])


# --- Nested inter-compact enterprises ----------------------------------------


def _federation():
    """Two compacts of two communities each, sharing an inter-compact commons."""
    arch = Archive()
    members = [new_identity(arch, n) for n in ("M1", "M2", "M3", "M4")]
    t0 = now_seconds()
    one_key = new_identity(arch, "unused-keypair-for-compact-one")
    two_key = new_identity(arch, "unused-keypair-for-compact-two")
    # A charter may not name a community the admission history does not seat, so each
    # compact's members are admitted under its label before it is chartered.
    record_admission(arch, members[:2], [m.did for m in members[:2]],
                     compact_label="CompactOne", witnessed_at=t0 - 10)
    record_admission(arch, members[2:], [m.did for m in members[2:]],
                     compact_label="CompactTwo", witnessed_at=t0 - 10)
    charter_compact(arch, "CompactOne", one_key, [m.did for m in members[:2]],
                    members[:2], chartered_at=t0)
    charter_compact(arch, "CompactTwo", two_key, [m.did for m in members[2:]],
                    members[2:], chartered_at=t0)
    commons = register_commons(
        arch, "Communication", "the interstellar relay",
        [one_key.did, two_key.did], [one_key, two_key],
        description="bandwidth shared between compacts",
        registered_at=t0 + 10, level="Compact",
    )
    return arch, members, one_key, two_key, commons, t0


def test_the_higher_level_may_not_reach_past_a_compact_to_its_members():
    """THE MEASUREMENT, now the rule. Ostrom's principle 8 is not "there is a higher
    level" but that each level governs at its own scale."""
    arch, members, one, two, commons, t0 = _federation()
    apply_sanction(
        arch, commons, members[0].did, "None", "Monitoring", [two],
        basis="reaching past the compact", decision_class="FirstClass",
        decided_at=t0 + 100,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    violations = arch.authority_violations()
    assert any("reaches" in reason for _cid, reason in violations)
    # The refusal names the compact the act should have gone through.
    assert any(one.did in reason for _cid, reason in violations)


def test_the_higher_level_may_sanction_a_compact():
    """NEGATIVE CONTROL for the above: the same move against the compact itself."""
    arch, _members, one, two, commons, t0 = _federation()
    apply_sanction(
        arch, commons, one.did, "None", "Monitoring", [two],
        basis="sustained overdraw on the shared relay", decision_class="FirstClass",
        decided_at=t0 + 100,
    )
    arch.verify_all()
    arch.check_authority()


def test_a_compact_charter_needs_its_members():
    arch = Archive()
    key = new_identity(arch, "compact-key")
    m1 = new_identity(arch, "M1")
    with pytest.raises(ValueError):
        charter_compact(arch, "Empty", key, [], [m1])


def test_an_unknown_appropriator_level_is_refused_in_python():
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    with pytest.raises(ValueError):
        register_commons(arch, "Navigation", "n", [a.did, b.did], [a, b],
                         level="Galactic")


def test_a_charter_may_not_name_a_community_no_admission_seats():
    """One definition of membership. Two records said who belongs to a compact and
    nothing joined them, so a charter could name a community the admission history had
    never seated. The admission history is the authoritative one."""
    arch = Archive()
    members = [new_identity(arch, n) for n in ("M1", "M2", "M3")]
    t0 = now_seconds()
    key = new_identity(arch, "unused-keypair-for-the-compact")
    record_admission(arch, members[:2], [m.did for m in members[:2]],
                     compact_label="Alpha", witnessed_at=t0 - 10)
    charter_compact(arch, "Alpha", key, [m.did for m in members],
                    members, chartered_at=t0)
    arch.verify_all()
    with pytest.raises(RuntimeError, match="no admission record seats it"):
        arch.check_authority()
