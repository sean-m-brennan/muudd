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

"""Demonstrate §4 mediation over a signed audit commons.

Two communities become parties to a dispute; two *other* communities each
nominate mediators into the commons-wide pool. A three-person panel drawn
entirely from the non-party pool decides the dispute, and the decision passes
the §5 authority check. Then the same panel is rebuilt two ways that the
protocol must refuse — too few mediators, and a mediator from a party
community — and `check_authority` reports each violation with provenance.

Everything below is authored through the public Python API and crypto-verified.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    file_argument, decide_mediation, reputation_of, now_seconds,
    request_recusal, decide_recusal, designate_protecting_power,
    mediator_pool_concentration, attest_activity, silent_functions, erosion_alert,
)

# §4 caps a mediator's term: a seat with no end is a permanent mediator class.
MEDIATOR_TERM = 365 * 24 * 3600


def main() -> int:
    arch = Archive()
    # Parties to the dispute.
    alpha = new_identity(arch, "Alpha")     # complainant
    beta = new_identity(arch, "Beta")       # respondent
    # Non-party communities that staff the mediator pool.
    gamma = new_identity(arch, "Gamma")
    delta = new_identity(arch, "Delta")

    t0 = now_seconds()
    # The pool = the union of every community's mediator registry. Gamma
    # nominates two mediators, Delta one; none belongs to a party community.
    g1, g2, d1 = new_mediator(), new_mediator(), new_mediator()
    register_mediators(arch, gamma, [g1, g2], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    register_mediators(arch, delta, [d1], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    print("Mediator pool (union across communities):")
    print("  Gamma nominates 2, Delta nominates 1 — 3 non-party mediators\n")

    # Alpha files a dispute against Beta, reaching for mediated arbitration.
    dispute = file_dispute(
        arch, alpha, beta.did, "MediatedArbitration",
        summary="Beta decided a matter Alpha's charter reserves",
        filed_at=t0,
    )
    print("Alpha filed a dispute against Beta (rung=MediatedArbitration)")

    # Each party puts its case, and each panelist their own position. Both are §4's
    # "structured argument submission", and the decision must answer them: a panel that
    # ignores a filed case, or three signatures riding on one author's text, is refused.
    arguments = [
        file_argument(arch, party, dispute, "Party", side, submitted_at=t0 + 1)
        for party, side in ((alpha, "the matter is Alpha's to decide"),
                            (beta, "Beta acted within its own charter"))
    ]
    panel = [g1, g2, d1]
    arguments += [
        file_argument(arch, m, dispute, "Panelist", "as we read the charters",
                      submitted_at=t0 + 2 + i)
        for i, m in enumerate(panel)
    ]
    decide_mediation(
        arch, dispute, panel, "Upheld",
        "the three non-party mediators concur", arguments, decided_at=t0 + 10,
    )
    arch.verify_all()          # crypto integrity of the whole commons
    arch.check_authority()     # §4 panel rule + §5 authority: PASSES
    print("Panel of 3 non-party mediators decided: Upheld")
    print("  verify_all() + check_authority(): PASS\n")

    # The dispute and its outcome project onto both parties. The projection is
    # a point-in-time view, so ask as of a moment at or after the decision.
    va = reputation_of(arch, alpha.did, as_of=t0 + 100)
    vb = reputation_of(arch, beta.did, as_of=t0 + 100)
    print("Projection:")
    print(f"  Alpha.disputes_filed={len(va.disputes_filed)} "
          f"outcome={va.disputes_filed[0].decided_outcome}")
    print(f"  Beta.disputes_against={len(vb.disputes_against)} "
          f"outcome={vb.disputes_against[0].decided_outcome}\n")

    # --- Panels the protocol must refuse ------------------------------------
    # (a) Too few mediators for the default three-person rule.
    bad_small = Archive.load_json(arch.dump_json())
    d2 = file_dispute(bad_small, alpha, beta.did, "MediatedArbitration",
                      filed_at=t0)
    args2 = [
        file_argument(bad_small, m, d2, "Panelist", "as we read it",
                      submitted_at=t0 + 2 + i)
        for i, m in enumerate(panel[:2])
    ]
    decide_mediation(bad_small, d2, panel[:2], "Dismissed", "the two of us agree",
                     args2, decided_at=t0 + 10)
    bad_small.verify_all()  # crypto is fine
    violations = bad_small.authority_violations()
    print("Rejected panel — only 2 mediators (default panel_min=3):")
    print(f"  {_first_reason(violations)}")

    # (b) A mediator from a party community (non-overlap violated).
    bad_party = Archive.load_json(arch.dump_json())
    alpha_insider = new_mediator()
    register_mediators(bad_party, alpha, [alpha_insider], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    d3 = file_dispute(bad_party, alpha, beta.did, "MediatedArbitration",
                      filed_at=t0)
    seated = [g1, g2, alpha_insider]
    args3 = [
        file_argument(bad_party, m, d3, "Panelist", "as we read it",
                      submitted_at=t0 + 2 + i)
        for i, m in enumerate(seated)
    ]
    decide_mediation(bad_party, d3, seated, "Upheld", "we concur", args3,
                     decided_at=t0 + 10)
    bad_party.verify_all()  # crypto is fine: the insider's key resolves
    violations = bad_party.authority_violations()
    print("Rejected panel — a mediator from a party community (non-overlap):")
    print(f"  {_first_reason(violations)}")

    # A decision's `panel` field is now required to be exactly its co-signers. Until
    # that rule existed nothing read the field at all, so it could name the two *party*
    # communities while three legitimate non-party mediators signed, and the record was
    # authorized: the field an auditor reads was a claim nobody checked. **Not
    # demonstrable from here** — this binding derives the panel from the signing keys,
    # so the two cannot diverge through it. It was reachable from Rust and from any
    # record arriving over the wire, which is where the rule earns its place; the Rust
    # suite holds that case.

    # (d) A panel of three where only one of them argued: three signatures riding on
    # one author's text is a rubber stamp, not a deliberation.
    bad_silent = Archive.load_json(arch.dump_json())
    d5 = file_dispute(bad_silent, alpha, beta.did, "MediatedArbitration", filed_at=t0)
    only_one = [file_argument(bad_silent, panel[0], d5, "Panelist",
                              "as I read it", submitted_at=t0 + 2)]
    decide_mediation(bad_silent, d5, panel, "Upheld", "we concur", only_one,
                     decided_at=t0 + 10)
    bad_silent.verify_all()
    print("Rejected decision — two panelists co-signed without arguing:")
    print(f"  {_first_reason(bad_silent.authority_violations())}")

    # (e) A panel that decides while ignoring a case a party put to it.
    bad_deaf = Archive.load_json(arch.dump_json())
    d6 = file_dispute(bad_deaf, alpha, beta.did, "MediatedArbitration", filed_at=t0)
    file_argument(bad_deaf, beta, d6, "Party", "Beta's answer, filed and unanswered",
                  submitted_at=t0 + 1)
    args6 = [
        file_argument(bad_deaf, m, d6, "Panelist", "as we read it",
                      submitted_at=t0 + 2 + i)
        for i, m in enumerate(panel)
    ]
    decide_mediation(bad_deaf, d6, panel, "Upheld", "we concur", args6,
                     decided_at=t0 + 10)
    bad_deaf.verify_all()
    print("Rejected decision — a party's filed case went unanswered:")
    print(f"  {_first_reason(bad_deaf.authority_violations())}")

    # --- The pool's own caps -------------------------------------------------
    # The mediator pool is a roster, and it is held to the same four rules the §2
    # examiners and the §8 observers are, by the same code. A seat with no end is
    # a permanent mediator class, "the first step toward an inquisitorial caste".
    open_ended = Archive.load_json(arch.dump_json())
    register_mediators(open_ended, gamma, [new_mediator()], valid_from=t0 - 100)
    print("\nRejected seat — a mediator term with no end:")
    print(f"  {_first_reason(open_ended.authority_violations())}")

    # And no single community may hold a majority of the active pool, which is
    # what lets one community staff every panel in the compact.
    dominated = Archive.load_json(arch.dump_json())
    register_mediators(dominated, gamma, [new_mediator() for _ in range(8)],
                       valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    print("Rejected nomination — one community would hold a majority of the pool:")
    print(f"  {_first_reason(dominated.authority_violations())}")

    # --- Recusal, protecting powers, lineage --------------------------------
    # A party names one panelist and states grounds. The request is recorded and
    # answered; it does not block the mediation or unseat anyone by itself.
    live = Archive.load_json(arch.dump_json())
    d7 = file_dispute(live, alpha, beta.did, "MediatedArbitration", filed_at=t0)
    req = request_recusal(live, alpha, d7, g1.did, "prior dealings with Beta",
                          requested_at=t0 + 1)
    live.check_authority()
    print("\nAlpha asked that one panelist step down (grounds recorded, never weighed)")

    # Only the named mediator may grant it: nobody steps down for another.
    usurped = Archive.load_json(live.dump_json())
    decide_recusal(usurped, req, "Granted", [g2], decided_at=t0 + 2)
    print("Rejected answer — another mediator granted it for them:")
    print(f"  {_first_reason(usurped.authority_violations())}")

    decide_recusal(live, req, "Granted", [g1], decided_at=t0 + 2)
    live.check_authority()
    print("  granted by the mediator stepping down: recorded, and the mediation goes on")

    # A protecting power is the Geneva analogue: a neutral community standing for
    # a disputant, co-signed by both. It is a witness with standing, not an
    # arbitrator, so it gates nothing.
    neutral = new_identity(live, "Neutral")
    designate_protecting_power(live, beta, neutral, d7,
                              statement="Beta cannot send an envoy this cycle",
                              designated_at=t0 + 3)
    live.verify_all()
    live.check_authority()
    print("Neutral stands as protecting power for Beta (co-signed by both)")

    opposed = Archive.load_json(live.dump_json())
    designate_protecting_power(opposed, beta, alpha, d7, designated_at=t0 + 4)
    print("Rejected designation — the opposing party standing for its opponent:")
    print(f"  {_first_reason(opposed.authority_violations())}")

    # Training lineage is recorded and gates nothing; the pool board is its only
    # reader. A pool that all traces to one trainer is captured by the incumbent
    # generation, which is the apprenticeship model's own named weakness.
    lineage = Archive()
    epsilon, zeta = new_identity(lineage, "Epsilon"), new_identity(lineage, "Zeta")
    elder, other = new_identity(lineage, "Elder"), new_identity(lineage, "Other")
    for community in (epsilon, zeta):
        register_mediators(
            lineage, community, [new_mediator() for _ in range(3)],
            valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM,
            trained_by=[elder.did, elder.did, other.did],
        )
    lineage.check_authority()      # a legal pool: 3 and 3, every term bounded
    board = mediator_pool_concentration(lineage, as_of=t0)
    held = ("one community holds a majority" if board.majority_held
            else "no community holds a majority")
    print(f"\nPool board: {board.total_active} active, {held}, "
          f"one trainer behind {board.top_lineage} of "
          f"{board.with_lineage} recorded apprenticeships")
    print(f"  lineage_majority_held={board.lineage_majority_held} "
          f"(reported, never a gate: no rule reads trained_by)")

    # --- The pool that takes no disputes ------------------------------------
    # A pool with no disputes to take and a pool that stopped answering are the same
    # silence to every relative measure: the same shares, the same concentration,
    # the same zero decisions. The attested null is what separates them, and it is
    # the community holding the seats that owes the account.
    day = 24 * 3600
    accounted = Archive()
    steward = new_identity(accounted, "Steward")
    register_mediators(accounted, steward, [new_mediator()],
                       valid_from=t0, valid_until=t0 + 700 * day)
    silent = Archive.load_json(accounted.dump_json())

    attest_activity(accounted, steward, "Mediation", t0, t0 + 200 * day,
                    statement="no disputes were filed in this window",
                    attested_at=t0 + 200 * day)
    accounted.verify_all()
    accounted.check_authority()

    at = t0 + 240 * day
    print("\nTwo pools, neither with a dispute to its name, 240 days on:")
    print(f"  attested its null:   silent={len(silent_functions(accounted, as_of=at))}"
          f"  erosion={erosion_alert(accounted, as_of=at).severity}")
    print(f"  said nothing at all: silent={len(silent_functions(silent, as_of=at))}"
          f"  erosion={erosion_alert(silent, as_of=at).severity}")

    # The null is an account, not a formality: a community that seats no mediators
    # cannot give one, and an empty attestation must still say something.
    bad_null = Archive.load_json(silent.dump_json())
    outsider = new_identity(bad_null, "Outsider")
    attest_activity(bad_null, outsider, "Mediation", t0, t0 + 100 * day,
                    statement="nothing to report", attested_at=t0 + 100 * day)
    print("Rejected null — a community that seats no mediators:")
    print(f"  {_first_reason(bad_null.authority_violations())}")

    print("\n✓ mediation_demo (Python): SUCCESS")
    return 0


def _first_reason(violations) -> str:
    """The reason string of the first authority violation, or a sentinel."""
    return violations[0][1] if violations else "(no violation — unexpected!)"


if __name__ == "__main__":
    sys.exit(main())
