/*
  Copyright 2026 Sean M. Brennan and contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use en_archive::Archive;
use en_charter::{
    charter_state, check_authority, verify_all, BoundarySource, CharterEntry, Commitment,
    Substrate, SubstrateReading,
};
use en_core::{keys, Alg, DidKey, PrivKey, Signed, Timestamp};

use crate::{FoundingCandidate, ManualFounding, MemberCandidate, SubstrateProvider};

fn t(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("timestamp")
}

fn id() -> (PrivKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    (sk, DidKey::from_pubkey(&pk).unwrap())
}

fn substrate() -> Substrate {
    Substrate { name: "tradition".into(), statement: "answers to X above itself".into(), anchor: None }
}

/// Sign `candidate`'s reading of `polity` as a fresh reader — what a host does with
/// the key the provider does not hold (design **D22**).
fn read_by(candidate: &FoundingCandidate, polity: &DidKey) -> Signed<SubstrateReading> {
    let (sk, reader) = id();
    let reading = candidate.reading(polity.clone(), reader.clone(), t(40));
    Signed::build(reading, &[(reader.as_str(), &sk)]).unwrap()
}

/// The founding path end to end through the *manual* provider — with neither
/// `autonomous_trust` nor any kith-covenant governance crate in the dependency
/// graph (`en-substrate` depends only on `en-core` + `en-charter`; the archive is
/// a dev-dependency). This is the KC ⟂ AT decoupling, demonstrated: a polity is
/// chartered and its charter verifies and authorizes with no trust substrate and
/// no federation present.
#[test]
fn manual_founding_end_to_end_without_at() {
    let members: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
    let polity = id().1;
    let roll: Vec<DidKey> = members.iter().map(|(_, d)| d.clone()).collect();

    // 1. Emergence (here: a convener) proposes a candidate boundary.
    let provider = ManualFounding::new(roll.clone(), substrate());
    let candidate = provider.founding_candidate().unwrap();
    assert_eq!(candidate.members.len(), 3);
    assert!(candidate.substrate.is_some());

    // 2. The polity draws the proposal it will ratify.
    let proposal = candidate.propose(
        polity.clone(),
        substrate(),
        vec![Commitment { key: "mutual-aid".into(), description: "aid among members".into() }],
        0.0,
        vec![],
        t(50),
    );
    assert_eq!(proposal.proposed_members.len(), 3);

    // 3. A majority ratifies and co-signs.
    let charter = proposal.ratify(vec![roll[0].clone(), roll[1].clone()], t(100)).unwrap();
    let signed = Signed::build(
        charter,
        &[(roll[0].as_str(), &members[0].0), (roll[1].as_str(), &members[1].0)],
    )
    .unwrap();

    // 4. The charter lands in the archive, verifies, and authorizes.
    let mut archive = Archive::new();
    archive.append(CharterEntry::PolityChartered(signed)).unwrap();
    verify_all(&archive).expect("signatures valid");
    check_authority(&archive).expect("majority-ratified singleton is authorized");

    let st = charter_state(&archive, &polity, t(300));
    assert!(st.chartered);
    assert_eq!(st.members.len(), 3);
    let keys: Vec<&str> = st.commitments.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, vec!["mutual-aid"]);
}

#[test]
fn reputation_filter_trims_low_reputation_candidates_from_the_roll() {
    let keep_a = id().1;
    let drop = id().1;
    let keep_b = id().1;
    let candidate = FoundingCandidate {
        members: vec![
            MemberCandidate { did: keep_a.clone(), reputation: Some(0.9), derived: false, guardian: None, attested_at: None },
            MemberCandidate { did: drop.clone(), reputation: Some(0.4), derived: false, guardian: None, attested_at: None },
            MemberCandidate { did: keep_b.clone(), reputation: Some(0.8), derived: false, guardian: None, attested_at: None },
        ],
        substrate: Some(substrate()),
    };
    let provider = ManualFounding::from_candidate(candidate);

    let proposal = provider.founding_candidate().unwrap().propose(
        id().1,
        substrate(),
        vec![],
        0.5,
        vec![],
        t(50),
    );
    assert_eq!(proposal.proposed_members, vec![keep_a, keep_b]);
    assert!(provider.member_reputation(&drop) == Some(0.4));
}

#[test]
fn member_reputation_reads_and_misses() {
    let a = id().1;
    let stranger = id().1;
    let provider = ManualFounding::new(vec![a.clone()], substrate());
    assert_eq!(provider.member_reputation(&a), Some(1.0));
    assert_eq!(provider.member_reputation(&stranger), None);
}

// --- D18: the candidate's epistemic state -----------------------------------

/// A candidate member, spelled out: `(did, reputation, derived, guardian)`.
fn cand(
    did: &DidKey,
    reputation: Option<f64>,
    derived: bool,
    guardian: Option<&DidKey>,
) -> MemberCandidate {
    MemberCandidate {
        did: did.clone(),
        reputation,
        derived,
        guardian: guardian.cloned(),
        attested_at: None,
    }
}

#[test]
fn an_unguarded_machine_is_ratified_as_the_unguarded_derivation_it_is() {
    // D15's second honest limit, closed. `propose` used to infer derivation from
    // `guardian.is_some()`, so a machine the substrate reads as having nobody behind
    // it proposed silently as a human member and could only reach its true chartered
    // state through a deliberate post-founding designation. The charter always
    // understood `Derivation { guardian: None }`; the candidate could not propose it.
    let human = id().1;
    let guarded_machine = id().1;
    let unguarded_machine = id().1;
    let operator = id().1;

    let candidate = FoundingCandidate {
        members: vec![
            cand(&human, Some(0.9), false, None),
            cand(&guarded_machine, Some(0.9), true, Some(&operator)),
            cand(&unguarded_machine, Some(0.9), true, None),
        ],
        substrate: Some(substrate()),
    };
    let proposal = candidate.propose(id().1, substrate(), vec![], 0.0, vec![], t(50));

    assert_eq!(proposal.proposed_members.len(), 3, "all three are on the roll");
    // TWO derivations, and one of them is unguarded — the state that makes D15's
    // machine-majority office rule non-vacuous.
    assert_eq!(proposal.proposed_derivations.len(), 2);
    let unguarded: Vec<&DidKey> = proposal
        .proposed_derivations
        .iter()
        .filter(|d| d.guardian.is_none())
        .map(|d| &d.member)
        .collect();
    assert_eq!(unguarded, vec![&unguarded_machine]);
    // The human is not a derivation at all.
    assert!(proposal.proposed_derivations.iter().all(|d| d.member != human));
}

#[test]
fn a_guardian_on_a_non_derived_member_does_not_invent_a_derivation() {
    // The mirror: derivation now comes from `derived`, so a guardian reading on a
    // member the substrate does not read as a machine no longer conjures one.
    let member = id().1;
    let operator = id().1;
    let candidate = FoundingCandidate {
        members: vec![cand(&member, Some(0.9), false, Some(&operator))],
        substrate: Some(substrate()),
    };
    let proposal = candidate.propose(id().1, substrate(), vec![], 0.0, vec![], t(50));
    assert_eq!(proposal.proposed_members.len(), 1);
    assert!(proposal.proposed_derivations.is_empty());
}

#[test]
fn an_unrated_candidate_fails_a_demanded_floor_but_clears_no_floor_at_all() {
    // A positive floor is a positive assertion, and `None` cannot support it. A
    // floor of 0.0 demands nothing, so it admits everyone — which is what the
    // default expresses, and why this is behaviour-preserving for existing callers.
    let rated_ok = id().1;
    let unrated = id().1;
    let candidate = FoundingCandidate {
        members: vec![
            cand(&rated_ok, Some(0.9), false, None),
            cand(&unrated, None, false, None),
        ],
        substrate: Some(substrate()),
    };

    let demanded = candidate.propose(id().1, substrate(), vec![], 0.5, vec![], t(50));
    assert_eq!(demanded.proposed_members, vec![rated_ok.clone()], "unrated fails a floor");

    let no_floor = candidate.propose(id().1, substrate(), vec![], 0.0, vec![], t(50));
    assert_eq!(no_floor.proposed_members.len(), 2, "0.0 demands nothing");
}

#[test]
fn an_unrated_machine_trimmed_by_a_floor_takes_its_derivation_with_it() {
    // The two filters read the same retained set, so a candidate the floor drops
    // cannot leave a derivation behind for a member who is not on the roll.
    let unrated_machine = id().1;
    let candidate = FoundingCandidate {
        members: vec![cand(&unrated_machine, None, true, None)],
        substrate: Some(substrate()),
    };
    let proposal = candidate.propose(id().1, substrate(), vec![], 0.5, vec![], t(50));
    assert!(proposal.proposed_members.is_empty());
    assert!(proposal.proposed_derivations.is_empty());
}

#[test]
fn a_manual_convener_asserts_a_reading_rather_than_withholding_one() {
    // `ManualFounding::new` must report Some(1.0), not None: a convener vouching for
    // a roll IS the reading, where None would claim nothing is known about them —
    // and would put the whole roll into the unrated erosion component.
    let a = id().1;
    let provider = ManualFounding::new(vec![a.clone()], substrate());
    let c = provider.founding_candidate().unwrap();
    assert_eq!(c.members[0].reputation, Some(1.0));
    assert!(!c.members[0].derived);
}

// --- D21: the boundary a proposal was drawn from -----------------------------

fn rated(members: &[(DidKey, Option<f64>)]) -> FoundingCandidate {
    FoundingCandidate {
        members: members
            .iter()
            .map(|(did, rep)| MemberCandidate {
                did: did.clone(),
                reputation: *rep,
                derived: false,
                guardian: None,
                attested_at: None,
            })
            .collect(),
        substrate: Some(substrate()),
    }
}

#[test]
fn the_floor_records_whoever_it_trims_and_why() {
    // Before D21 the trimmed simply vanished, and since the genesis quorum counted
    // the proposal's own roll, a narrowed founding was indistinguishable from a
    // whole one. Now the proposal carries them, with the reason the floor gave —
    // and it distinguishes the two ways to fail a floor, which is D18's
    // unrated-vs-rated-low distinction said out loud where the excluded can read it.
    let keep = id().1;
    let low = id().1;
    let unrated = id().1;
    let candidate = rated(&[
        (keep.clone(), Some(0.9)),
        (low.clone(), Some(0.21)),
        (unrated.clone(), None),
    ]);

    let polity = id().1;
    let prop = candidate.propose(
        polity.clone(),
        substrate(),
        vec![],
        0.5,
        vec![read_by(&candidate, &polity)],
        t(50),
    );
    assert_eq!(prop.proposed_members, vec![keep]);
    assert!(prop.attestation.source.is_read());
    assert_eq!(prop.attestation.candidate_size, 3);
    assert!(prop.attestation.accounts_for(prop.proposed_members.len()));

    let by_member = |d: &DidKey| {
        prop.attestation.exclusions.iter().find(|e| &e.member == d).unwrap().reason.clone()
    };
    assert!(by_member(&low).contains("0.21") && by_member(&low).contains("below the floor"));
    assert!(by_member(&unrated).contains("unrated"), "{}", by_member(&unrated));
}

#[test]
fn a_floor_that_demands_nothing_excludes_nobody() {
    let a = id().1;
    let b = id().1;
    let candidate = rated(&[(a, Some(0.9)), (b, None)]);
    let prop = candidate.propose(id().1, substrate(), vec![], 0.0, vec![], t(50));
    assert_eq!(prop.proposed_members.len(), 2);
    assert!(prop.attestation.exclusions.is_empty());
    assert!(prop.attestation.accounts_for(2));
}

#[test]
fn deliberate_narrowing_is_a_first_class_act() {
    // A group of five inside a thirty-node cluster may have every right to charter
    // alone. If the only ergonomic path were the reputation floor, that founding
    // would be dressed up as a floor it is not — so `propose_selecting` names the
    // narrowing for what it is, and the accounting is correct by construction.
    let all: Vec<DidKey> = (0..8).map(|_| id().1).collect();
    let candidate = rated(&all.iter().map(|d| (d.clone(), Some(0.9))).collect::<Vec<_>>());
    let keep = &all[..3];

    let prop = candidate.propose_selecting(
        id().1,
        substrate(),
        vec![],
        keep,
        "founding on the older rite",
        vec![],
        t(50),
    );
    assert_eq!(prop.proposed_members.len(), 3);
    assert_eq!(prop.attestation.candidate_size, 8);
    assert_eq!(prop.attestation.exclusions.len(), 5);
    assert!(prop.attestation.accounts_for(3));
    assert!(prop
        .attestation
        .exclusions
        .iter()
        .all(|e| e.reason == "founding on the older rite"));
    // Nobody kept is also excluded.
    assert!(prop
        .attestation
        .exclusions
        .iter()
        .all(|e| !prop.proposed_members.contains(&e.member)));
}

#[test]
fn selecting_someone_the_boundary_never_showed_is_ignored() {
    // The attestation describes *this* boundary and cannot vouch for anyone outside
    // it, so a stray DID neither joins the roll nor corrupts the accounting.
    let seen: Vec<DidKey> = (0..3).map(|_| id().1).collect();
    let candidate = rated(&seen.iter().map(|d| (d.clone(), Some(0.9))).collect::<Vec<_>>());
    let mut keep = vec![seen[0].clone()];
    keep.push(id().1); // never in the candidate

    let prop = candidate.propose_selecting(id().1, substrate(), vec![], &keep, "chosen", vec![], t(50));
    assert_eq!(prop.proposed_members, vec![seen[0].clone()]);
    assert_eq!(prop.attestation.candidate_size, 3);
    assert_eq!(prop.attestation.exclusions.len(), 2);
    assert!(prop.attestation.accounts_for(1));
}

#[test]
fn a_convener_asserted_boundary_says_so_forever() {
    // The distinction the charter keeps for as long as it exists: an independent
    // reading, or the convener's word. Since D22 it is not a label anybody chooses
    // — a boundary is `Substrate`-sourced only if it carries a reading somebody
    // signed, and this provider holds no reader key, so it cannot claim one.
    let roll: Vec<DidKey> = (0..3).map(|_| id().1).collect();
    let provider = ManualFounding::new(roll, substrate());
    let prop = provider
        .founding_candidate()
        .unwrap()
        .propose(id().1, substrate(), vec![], 0.0, vec![], t(50));
    assert_eq!(prop.attestation.source, BoundarySource::ConvenerAsserted);
}

#[test]
fn a_proposal_drawn_from_a_substrate_reading_says_so() {
    let candidate = FoundingCandidate {
        members: (0..3)
            .map(|_| MemberCandidate {
                did: id().1,
                reputation: Some(0.9),
                derived: false,
                guardian: None,
                attested_at: None,
            })
            .collect(),
        substrate: Some(substrate()),
    };
    let polity = id().1;
    let reading = read_by(&candidate, &polity);
    let reader = reading.payload.reader.clone();
    let prop = candidate.propose(
        polity.clone(),
        substrate(),
        vec![],
        0.0,
        vec![reading],
        t(50),
    );
    assert!(prop.attestation.source.is_read());
    // Attributable, which is the whole of D22: the boundary names who read it.
    assert_eq!(prop.attestation.source.readings()[0].payload.reader, reader);
    assert_eq!(prop.attestation.source.readings()[0].payload.polity, polity);
    assert_eq!(prop.attestation.corroborating_readers(&prop.proposed_members), vec![reader]);
}
