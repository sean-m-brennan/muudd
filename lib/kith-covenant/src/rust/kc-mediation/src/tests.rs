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

use kc_conduct::LadderRung;
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
fn mediator_entry_validity_window() {
    let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
    let (_, enc) = keys::generate(Alg::Ed25519).unwrap();
    let e = MediatorEntry {
        mediator_did: DidKey::from_pubkey(&pk).unwrap(),
        mediator_pubkey: pk,
        encryption_pubkey: enc,
        valid_from: t(100),
        valid_until: Some(t(200)),
    };
    assert!(!e.active_at(t(99)));
    assert!(e.active_at(t(100)));
    assert!(e.active_at(t(199)));
    assert!(!e.active_at(t(200)), "half-open window");
}

#[test]
fn mediator_entry_open_ended() {
    let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
    let (_, enc) = keys::generate(Alg::Ed25519).unwrap();
    let e = MediatorEntry {
        mediator_did: DidKey::from_pubkey(&pk).unwrap(),
        mediator_pubkey: pk,
        encryption_pubkey: enc,
        valid_from: t(100),
        valid_until: None,
    };
    assert!(e.active_at(t(10_000)));
}

#[test]
fn decision_outcome_names_are_stable() {
    assert_eq!(DecisionOutcome::Upheld.as_str(), "Upheld");
    assert_eq!(DecisionOutcome::Dismissed.as_str(), "Dismissed");
    assert_eq!(DecisionOutcome::Settled.as_str(), "Settled");
    assert_eq!(DecisionOutcome::Referred.as_str(), "Referred");
}

#[test]
fn dispute_and_decision_roundtrip_json() {
    let dispute = DisputeRecord {
        complainant: did(),
        respondent: did(),
        rung: LadderRung::MediatedArbitration,
        summary: "a boundary dispute".into(),
        references: vec![],
        filed_at: t(500),
    };
    let json = serde_json::to_string(&dispute).unwrap();
    let back: DisputeRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(dispute, back);

    let decision = MediationDecision {
        dispute_cid: Cid::of_bytes(b"dispute"),
        panel: vec![did(), did(), did()],
        outcome: DecisionOutcome::Settled,
        reasoning: "parties agreed".into(),
        decided_at: t(600),
    };
    let json = serde_json::to_string(&decision).unwrap();
    let back: MediationDecision = serde_json::from_str(&json).unwrap();
    assert_eq!(decision, back);
}
