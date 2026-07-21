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

use kc_conduct::DecisionClass;
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
fn commons_kind_names_are_stable_and_all_has_five() {
    assert_eq!(CommonsKind::LifeSupport.as_str(), "LifeSupport");
    assert_eq!(CommonsKind::KnowledgeArchive.as_str(), "KnowledgeArchive");
    assert_eq!(CommonsKind::Rescue.as_str(), "Rescue");
    assert_eq!(CommonsKind::all().len(), 5);
}

#[test]
fn sanction_rank_orders_the_ladder() {
    use SanctionLevel::*;
    assert_eq!(sanction_rank(None), 0);
    assert_eq!(sanction_rank(Monitoring), 1);
    assert_eq!(sanction_rank(Warning), 2);
    assert_eq!(sanction_rank(Restriction), 3);
    assert_eq!(sanction_rank(Exclusion), 4);
}

#[test]
fn second_class_levels_are_restriction_and_exclusion() {
    use SanctionLevel::*;
    assert!(is_second_class_level(Restriction));
    assert!(is_second_class_level(Exclusion));
    assert!(!is_second_class_level(Monitoring));
    assert!(!is_second_class_level(Warning));
    assert!(!is_second_class_level(None));
}

#[test]
fn sanction_level_names_are_stable() {
    assert_eq!(SanctionLevel::None.as_str(), "None");
    assert_eq!(SanctionLevel::Monitoring.as_str(), "Monitoring");
    assert_eq!(SanctionLevel::Warning.as_str(), "Warning");
    assert_eq!(SanctionLevel::Restriction.as_str(), "Restriction");
    assert_eq!(SanctionLevel::Exclusion.as_str(), "Exclusion");
}

#[test]
fn commons_registration_roundtrips_json() {
    let a = did();
    let b = did();
    let x = CommonsRegistration {
        kind: CommonsKind::LifeSupport,
        name: "shared atmosphere loop".into(),
        description: "the sealed-habitat atmosphere and water recycling commons".into(),
        appropriators: vec![a.clone(), b.clone()],
        provision_rule: "provision scales with draw; overdraw is the gravest violation".into(),
        registered_by: vec![a, b],
        registered_at: t(100),
    };
    let json = serde_json::to_string(&x).unwrap();
    let back: CommonsRegistration = serde_json::from_str(&json).unwrap();
    assert_eq!(x, back);
}

#[test]
fn usage_record_roundtrips_json() {
    let x = UsageRecord {
        commons_cid: Cid::of_bytes(b"commons"),
        community: did(),
        consumed: 1200,
        contributed: 300,
        period_start: t(1000),
        period_end: t(1000 + 30 * 24 * 3600),
        recorded_at: t(1000 + 30 * 24 * 3600),
    };
    let back: UsageRecord = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
    assert_eq!(x, back);
}

#[test]
fn commons_sanction_roundtrips_json() {
    let x = CommonsSanction {
        commons_cid: Cid::of_bytes(b"commons"),
        subject_community: did(),
        from_level: SanctionLevel::Warning,
        to_level: SanctionLevel::Restriction,
        basis: "repeated bandwidth overdraw beyond the provision rule".into(),
        references: vec![Cid::of_bytes(b"usage1"), Cid::of_bytes(b"usage2")],
        imposers: vec![did(), did()],
        decision_class: DecisionClass::SecondClass,
        decided_at: t(5000),
    };
    let back: CommonsSanction = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
    assert_eq!(x, back);
}
