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

use super::*;
use kc_ceremony::inter::AntiSpiralFeature;
use kc_core::{canonical, keys, Alg, Cid, DidKey, PubKey, PrivKey, Signed};

fn fresh_id() -> (PrivKey, PubKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).expect("keygen");
    let did = DidKey::from_pubkey(&pk).expect("did");
    (sk, pk, did)
}

fn t(secs: i64) -> kc_core::Timestamp {
    jiff::Timestamp::from_second(secs).expect("timestamp")
}

fn sample_concern(subject: DidKey, reporter: DidKey, feature: AntiSpiralFeature) -> ConductRecord {
    ConductRecord {
        subject_community: subject,
        reporters: vec![reporter],
        reporter_perspective: Some("charter-originalist observer".into()),
        kind: ConductKind::Concern,
        bearing: ConductBearing::AntiSpiral(feature),
        severity: Severity::Minor,
        ladder_rung: None,
        decision_class: DecisionClass::FirstClass,
        references: vec![],
        commitments_cited: vec![],
        narrative: "decided a matter its own charter reserves to the substrate".into(),
        observed_at: t(1_700_000_000),
        recorded_at: t(1_700_000_100),
    }
}

#[test]
fn concern_is_adverse_and_carries_no_rung() {
    let (_, _, subject) = fresh_id();
    let (_, _, reporter) = fresh_id();
    let rec = sample_concern(subject, reporter, AntiSpiralFeature::NonUltimacyOfCompact);
    assert!(rec.kind.is_adverse());
    assert!(rec.ladder_rung.is_none(), "a Concern is pre-indictment: no ladder rung");
}

#[test]
fn resolution_kinds_are_not_adverse() {
    assert!(!ConductKind::Commendation.is_adverse());
    assert!(!ConductKind::DesignationLifted.is_adverse());
    assert!(!ConductKind::RestorationCompleted.is_adverse());
    assert!(ConductKind::Violation.is_adverse());
    assert!(ConductKind::DesignationAnathema.is_adverse());
}

#[test]
fn signed_conduct_roundtrips_through_cbor_and_verifies() {
    let (subj_sk, subj_pk, subject) = fresh_id();
    let (rep_sk, rep_pk, reporter) = fresh_id();
    let _ = (subj_sk, subj_pk);
    let rec = sample_concern(subject, reporter.clone(), AntiSpiralFeature::EnemyLoveOperational);
    let signed = Signed::build(rec, &[(reporter.as_str(), &rep_sk)]).expect("sign");

    let bytes = canonical::to_canonical_cbor(&signed).expect("cbor");
    let decoded: Signed<ConductRecord> =
        canonical::from_canonical_cbor(&bytes).expect("decode");

    let resolver = |did: &str| -> Option<PubKey> {
        if did == reporter.as_str() { Some(rep_pk.clone()) } else { None }
    };
    decoded.verify(resolver).expect("freshly built conduct record verifies");
}

#[test]
fn designation_can_be_multi_signed_quorum() {
    // Second-class decision: several communities co-sign. We only check the
    // envelope carries all signatures; quorum-count enforcement is deferred.
    let (_, _, subject) = fresh_id();
    let (sk1, pk1, r1) = fresh_id();
    let (sk2, pk2, r2) = fresh_id();
    let (sk3, pk3, r3) = fresh_id();
    let rec = ConductRecord {
        subject_community: subject,
        reporters: vec![r1.clone(), r2.clone(), r3.clone()],
        reporter_perspective: None,
        kind: ConductKind::DesignationAnathema,
        bearing: ConductBearing::CommonsObligation,
        severity: Severity::Grave,
        ladder_rung: Some(LadderRung::Expulsion),
        decision_class: DecisionClass::SecondClass,
        references: vec![],
        commitments_cited: vec![],
        narrative: "sustained refusal of rescue obligation".into(),
        observed_at: t(1_700_100_000),
        recorded_at: t(1_700_100_050),
    };
    let signed = Signed::build(
        rec,
        &[(r1.as_str(), &sk1), (r2.as_str(), &sk2), (r3.as_str(), &sk3)],
    )
    .expect("sign");
    assert_eq!(signed.signatures.len(), 3);
    let resolver = |did: &str| -> Option<PubKey> {
        if did == r1.as_str() { Some(pk1.clone()) }
        else if did == r2.as_str() { Some(pk2.clone()) }
        else if did == r3.as_str() { Some(pk3.clone()) }
        else { None }
    };
    signed.verify(resolver).expect("quorum-signed designation verifies");
}

#[test]
fn reference_chain_links_by_cid() {
    // A DesignationLifted references the prior DesignationAnathema by CID.
    let (_, _, subject) = fresh_id();
    let (_, _, reporter) = fresh_id();
    let designation = ConductRecord {
        subject_community: subject.clone(),
        reporters: vec![reporter.clone()],
        reporter_perspective: None,
        kind: ConductKind::DesignationAnathema,
        bearing: ConductBearing::AntiSpiral(AntiSpiralFeature::ScapegoatDisarming),
        severity: Severity::Grave,
        ladder_rung: Some(LadderRung::CollectiveSanctions),
        decision_class: DecisionClass::SecondClass,
        references: vec![],
        commitments_cited: vec![],
        narrative: "designation".into(),
        observed_at: t(1),
        recorded_at: t(2),
    };
    let designation_cid = Cid::of(&designation).expect("cid");
    let lifted = ConductRecord {
        kind: ConductKind::DesignationLifted,
        references: vec![designation_cid],
        observed_at: t(1_000),
        recorded_at: t(1_001),
        ..designation
    };
    assert_eq!(lifted.references, vec![designation_cid]);
    assert!(!lifted.kind.is_adverse());
}
