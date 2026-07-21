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

use kc_conduct::{DecisionClass, LadderRung};
use kc_core::{keys, Alg, Cid, DidKey, Timestamp};

use crate::record::*;

fn t(s: i64) -> Timestamp {
    jiff::Timestamp::from_second(s).unwrap()
}

fn did() -> DidKey {
    let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
    DidKey::from_pubkey(&pk).unwrap()
}

#[test]
fn dispute_ladder_rank_orders_the_seven_dispute_rungs() {
    use LadderRung::*;
    assert_eq!(dispute_ladder_rank(DisputeRegistered), Some(0));
    assert_eq!(dispute_ladder_rank(RegisteredProtest), Some(1));
    assert_eq!(dispute_ladder_rank(ReciprocalSuspension), Some(2));
    assert_eq!(dispute_ladder_rank(MediatedArbitration), Some(3));
    assert_eq!(dispute_ladder_rank(LimitedSanctions), Some(4));
    assert_eq!(dispute_ladder_rank(CollectiveSanctions), Some(5));
    assert_eq!(dispute_ladder_rank(Expulsion), Some(6));
    // The ceremony-cost track is not a dispute rung.
    assert_eq!(dispute_ladder_rank(SubstantiveReEngagement), None);
}

#[test]
fn second_class_rungs_are_collective_sanctions_and_expulsion() {
    use LadderRung::*;
    assert!(is_second_class_rung(CollectiveSanctions));
    assert!(is_second_class_rung(Expulsion));
    assert!(!is_second_class_rung(MediatedArbitration));
    assert!(!is_second_class_rung(LimitedSanctions));
}

#[test]
fn tier_and_urgency_names_are_stable() {
    assert_eq!(DesignationTier::Anathema.as_str(), "Anathema");
    assert_eq!(DesignationTier::HostisHumaniGeneris.as_str(), "HostisHumaniGeneris");
    assert_eq!(DesignationUrgency::Normal.as_str(), "Normal");
    assert_eq!(DesignationUrgency::Expedited.as_str(), "Expedited");
}

#[test]
fn ladder_transition_roundtrips_json() {
    let x = LadderTransition {
        dispute_cid: Cid::of_bytes(b"dispute"),
        from_rung: LadderRung::RegisteredProtest,
        to_rung: LadderRung::ReciprocalSuspension,
        initiators: vec![did()],
        decision_class: DecisionClass::FirstClass,
        reasoning: "named courtesies suspended".into(),
        transitioned_at: t(500),
    };
    let json = serde_json::to_string(&x).unwrap();
    let back: LadderTransition = serde_json::from_str(&json).unwrap();
    assert_eq!(x, back);
}

#[test]
fn designation_roundtrips_json() {
    let x = Designation {
        subject_community: did(),
        tier: DesignationTier::Anathema,
        urgency: DesignationUrgency::Normal,
        deliberators: vec![did(), did()],
        criteria: "three failed restorations, each graver".into(),
        cited_acts: vec![Cid::of_bytes(b"act1")],
        failed_restoration_cycles: vec![
            Cid::of_bytes(b"c1"),
            Cid::of_bytes(b"c2"),
            Cid::of_bytes(b"c3"),
        ],
        deliberation_cycles: vec![DeliberationCycle {
            cycle_index: 0,
            representatives: vec![did()],
            opened_at: t(100),
            concluded_at: t(200),
            reasoning: "first cycle".into(),
        }],
        reversal_conditions: Some("sustained changed conduct over probation".into()),
        reexamine_after: Some(t(10_000)),
        decision_class: DecisionClass::SecondClass,
        designated_at: t(300),
    };
    let json = serde_json::to_string(&x).unwrap();
    let back: Designation = serde_json::from_str(&json).unwrap();
    assert_eq!(x, back);
}

#[test]
fn restitution_records_roundtrip_json() {
    let ob = RestitutionObligation {
        subject_community: did(),
        beneficiary: did(),
        dispute_cid: Some(Cid::of_bytes(b"d")),
        terms: "return the disputed relay time".into(),
        imposed_by: vec![did()],
        due_by: Some(t(900)),
        decision_class: DecisionClass::FirstClass,
        imposed_at: t(800),
    };
    let back: RestitutionObligation = serde_json::from_str(&serde_json::to_string(&ob).unwrap()).unwrap();
    assert_eq!(ob, back);

    let done = RestitutionCompletion {
        obligation_cid: Cid::of_bytes(b"ob"),
        verifiers: vec![did(), did(), did()],
        satisfied: true,
        narrative: "verified complete".into(),
        verified_at: t(950),
    };
    let back: RestitutionCompletion = serde_json::from_str(&serde_json::to_string(&done).unwrap()).unwrap();
    assert_eq!(done, back);
}

#[test]
fn capability_names_are_stable_and_all_has_five() {
    assert_eq!(Capability::RescueCoordination.as_str(), "RescueCoordination");
    assert_eq!(Capability::CommonsWithdrawal.as_str(), "CommonsWithdrawal");
    assert_eq!(Capability::all().len(), 5);
}

#[test]
fn injunction_records_roundtrip_json() {
    let inj = InterimInjunction {
        subject_community: did(),
        dispute_cid: Some(Cid::of_bytes(b"d")),
        harm: "ongoing interference with a relay".into(),
        protective_measures: "suspend trade and navigation access".into(),
        revoked_capabilities: vec![Capability::Trade, Capability::Navigation],
        panel: vec![did(), did(), did()],
        issued_at: t(1000),
        expires_at: t(1000 + 90 * 24 * 3600),
    };
    let back: InterimInjunction = serde_json::from_str(&serde_json::to_string(&inj).unwrap()).unwrap();
    assert_eq!(inj, back);

    let conf = InjunctionConfirmation {
        injunction_cid: Cid::of_bytes(b"inj"),
        confirmers: vec![did(), did()],
        confirmed_at: t(1100),
    };
    let back: InjunctionConfirmation =
        serde_json::from_str(&serde_json::to_string(&conf).unwrap()).unwrap();
    assert_eq!(conf, back);

    let chal = InjunctionChallenge {
        injunction_cid: Cid::of_bytes(b"inj"),
        panel: vec![did(), did(), did()],
        overturned: true,
        reasoning: "the harm was not ongoing".into(),
        decided_at: t(1200),
    };
    let back: InjunctionChallenge =
        serde_json::from_str(&serde_json::to_string(&chal).unwrap()).unwrap();
    assert_eq!(chal, back);
}

#[test]
fn substantive_reengagement_roundtrips_json() {
    let x = SubstantiveReEngagement {
        subject_community: did(),
        decay_evidence: vec![Cid::of_bytes(b"rite1"), Cid::of_bytes(b"rite2")],
        invoked_by: vec![did()],
        required_reaffirmation_by: t(20_000),
        decision_class: DecisionClass::FirstClass,
        invoked_at: t(1000),
    };
    let back: SubstantiveReEngagement =
        serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
    assert_eq!(x, back);
}
