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

//! The polity at kith-covenant's border as a member of a compact (design **D40**).
//!
//! The roll decides each compact act in the charter log ([`CompactAct`]), and the
//! records below are what its envoy and root then sign at KC's border. Each builder
//! returns the **payload** unsigned, because in a deployment the key that signs it is
//! on another device: the caller hands the canonical bytes to whoever holds it and
//! assembles the signatures it gets back. Each also refuses to draft what no act in
//! force covers. The mandate is checked where the record is made, so an envoy cannot
//! be handed bytes the roll never decided.

use en_charter::{CompactActKind, CompactState};
use en_core::{DidKey, KcError, KcResult, PubKey, Timestamp};
use kc_ceremony::inter::{
    AntiSpiralFeatureClaim, EvidenceKind, EvidenceRef, ExaminerEntry, ExaminerRegistry,
    FeatureAcceptance, SponsorshipRecord, REQUIRED_FEATURES,
};
use kc_conduct::{ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity};
use kc_core::Cid;

/// The longest examining term KC's default caps allow, in seconds (182 days).
pub const EXAMINER_TERM_MAX_SECONDS: i64 = 182 * 24 * 3600;

/// Refuse unless an act in force lets the polity do `kind` in `compact` towards
/// `counterparty`.
pub fn mandate(
    state: &CompactState,
    kind: CompactActKind,
    compact: &str,
    counterparty: &DidKey,
) -> KcResult<()> {
    if state.covers(kind, compact, counterparty) {
        Ok(())
    } else {
        Err(KcError::Authority(format!(
            "no {kind:?} act in force covers {counterparty} in '{compact}' (D40)"
        )))
    }
}

/// The five anti-spiral claims the polity presents, one per required feature, each
/// citing the polity's own sealed rite as its evidence. Presenting needs a Join act
/// towards one of `counterparties`, or an Admit act when the polity is already
/// sitting: either way the roll decided to meet them.
pub fn anti_spiral_claims(
    state: &CompactState,
    compact: &str,
    polity: &DidKey,
    counterparties: &[DidKey],
    rite_cid: Cid,
    at: Timestamp,
) -> KcResult<Vec<AntiSpiralFeatureClaim>> {
    let decided = counterparties.iter().any(|c| {
        state.covers(CompactActKind::Join, compact, c) || state.covers(CompactActKind::Admit, compact, c)
    });
    if !decided {
        return Err(KcError::Authority(format!(
            "no Join or Admit act in force towards these counterparties in '{compact}' (D40)"
        )));
    }
    Ok(REQUIRED_FEATURES
        .iter()
        .copied()
        .map(|feature| AntiSpiralFeatureClaim {
            community_did: polity.clone(),
            feature,
            claim_text: format!("{feature:?}, as the substrate our rite recites supplies it"),
            evidence_refs: vec![EvidenceRef {
                kind: EvidenceKind::OwnRite,
                identifier: rite_cid.to_string(),
                description: Some("the polity's own sealed rite".into()),
            }],
            claimed_at: at,
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
/// The polity's verdict on another community's claim. Accepting needs a Join or
/// Admit act towards the presenter; refusing needs a Refuse act, which takes more
/// than a majority.
pub fn feature_acceptance(
    state: &CompactState,
    compact: &str,
    polity: &DidKey,
    presenter: &DidKey,
    claim_cid: Cid,
    accepted: bool,
    rationale: Option<String>,
    at: Timestamp,
) -> KcResult<FeatureAcceptance> {
    if accepted {
        if !(state.covers(CompactActKind::Join, compact, presenter)
            || state.covers(CompactActKind::Admit, compact, presenter))
        {
            return Err(KcError::Authority(format!(
                "no Join or Admit act in force towards {presenter} in '{compact}' (D40)"
            )));
        }
    } else {
        mandate(state, CompactActKind::Refuse, compact, presenter)?;
    }
    Ok(FeatureAcceptance {
        accepting_community_did: polity.clone(),
        claim_cid,
        accepted,
        rationale,
        decided_at: at,
    })
}

/// The polity's vouch for an entrant, staking its standing for the probation the
/// Sponsor act names.
pub fn sponsorship(
    state: &CompactState,
    compact: &str,
    polity: &DidKey,
    entrant: &DidKey,
    at: Timestamp,
) -> KcResult<SponsorshipRecord> {
    mandate(state, CompactActKind::Sponsor, compact, entrant)?;
    let days = state
        .acts
        .iter()
        .filter(|(_, a)| {
            a.kind == CompactActKind::Sponsor && a.compact == compact && a.counterparties.contains(entrant)
        })
        .filter_map(|(_, a)| a.probation_days)
        .max()
        .unwrap_or(0);
    let until = Timestamp::from_second(at.as_second() + i64::from(days) * 86_400)
        .map_err(|e| KcError::Archive(e.to_string()))?;
    Ok(SponsorshipRecord {
        sponsor_community: polity.clone(),
        entrant_community: entrant.clone(),
        admission_cid: None,
        probation_until: until,
        statement: "we answer for them while they find their footing".into(),
        sponsored_at: at,
    })
}

/// The polity's examining seats, from `from` for at most KC's 182-day term. A roster
/// is the root's to sign. Seating needs no act of its own: a polity that sits in a
/// compact examines its entrants, and who examines is the root's choice of members.
pub fn examiner_registry(
    polity: &DidKey,
    seats: &[(DidKey, PubKey)],
    from: Timestamp,
    until: Timestamp,
) -> KcResult<ExaminerRegistry> {
    if seats.is_empty() {
        return Err(KcError::Authority("an examining roster seats someone".into()));
    }
    let term = until.as_second() - from.as_second();
    if term <= 0 || term > EXAMINER_TERM_MAX_SECONDS {
        return Err(KcError::Authority(format!(
            "an examining term runs 1..={EXAMINER_TERM_MAX_SECONDS} seconds, got {term}"
        )));
    }
    Ok(ExaminerRegistry {
        nominating_community: polity.clone(),
        entries: seats
            .iter()
            .map(|(did, pk)| ExaminerEntry {
                examiner_did: did.clone(),
                examiner_pubkey: pk.clone(),
                valid_from: from,
                valid_until: Some(until),
            })
            .collect(),
        issued_at: from,
    })
}

/// A report for designation anathema, filed by `reporters` together. Always
/// `SecondClass`, so KC counts the reporters as a quorum (two or more), and the
/// drafting polity's own Anathematize act must cover the subject. Every reporter's
/// node checks its own mandate before it signs.
pub fn anathema(
    state: &CompactState,
    compact: &str,
    reporters: &[DidKey],
    subject: &DidKey,
    narrative: String,
    at: Timestamp,
) -> KcResult<ConductRecord> {
    mandate(state, CompactActKind::Anathematize, compact, subject)?;
    if reporters.len() < 2 {
        return Err(KcError::Authority(
            "an anathema is a second-class act and needs two or more reporting communities".into(),
        ));
    }
    Ok(ConductRecord {
        subject_community: subject.clone(),
        reporters: reporters.to_vec(),
        reporter_perspective: None,
        kind: ConductKind::DesignationAnathema,
        bearing: ConductBearing::CommonsObligation,
        severity: Severity::Grave,
        ladder_rung: Some(LadderRung::Expulsion),
        decision_class: DecisionClass::SecondClass,
        references: vec![],
        commitments_cited: vec![],
        narrative,
        observed_at: at,
        recorded_at: at,
    })
}

/// The lifting of a designation the polity reported, referencing it by CID.
pub fn lift(
    state: &CompactState,
    compact: &str,
    reporters: &[DidKey],
    subject: &DidKey,
    designation: Cid,
    at: Timestamp,
) -> KcResult<ConductRecord> {
    mandate(state, CompactActKind::Lift, compact, subject)?;
    if reporters.len() < 2 {
        return Err(KcError::Authority(
            "lifting a second-class designation needs two or more reporting communities".into(),
        ));
    }
    Ok(ConductRecord {
        subject_community: subject.clone(),
        reporters: reporters.to_vec(),
        reporter_perspective: None,
        kind: ConductKind::DesignationLifted,
        bearing: ConductBearing::CommonsObligation,
        severity: Severity::Grave,
        ladder_rung: None,
        decision_class: DecisionClass::SecondClass,
        references: vec![designation],
        commitments_cited: vec![],
        narrative: "the compact stands with them again".into(),
        observed_at: at,
        recorded_at: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::{CompactAct, CompactState};
    use en_core::{keys, Alg};

    fn did() -> (DidKey, PubKey) {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        (DidKey::from_pubkey(&pk).unwrap(), pk)
    }
    fn t(n: i64) -> Timestamp {
        Timestamp::from_second(n).unwrap()
    }
    fn state(polity: &DidKey, kind: CompactActKind, other: &DidKey) -> CompactState {
        CompactState {
            acts: vec![(
                Cid::of_bytes(b"act"),
                CompactAct {
                    polity: polity.clone(),
                    kind,
                    compact: "river".into(),
                    counterparties: vec![other.clone()],
                    probation_days: (kind == CompactActKind::Sponsor).then_some(30),
                    rationale: "r".into(),
                    deciders: vec![],
                    decided_at: t(100),
                    expires_at: None,
                },
            )],
        }
    }

    #[test]
    fn claims_need_a_decision_to_meet_the_counterparty() {
        let (a, _) = did();
        let (b, _) = did();
        let (c, _) = did();
        let st = state(&a, CompactActKind::Join, &b);
        let claims = anti_spiral_claims(&st, "river", &a, &[b.clone()], Cid::of_bytes(b"rite"), t(200)).unwrap();
        assert_eq!(claims.len(), 5);
        assert!(claims.iter().all(|c| c.evidence_refs[0].kind == EvidenceKind::OwnRite));
        assert!(anti_spiral_claims(&st, "river", &a, &[c], Cid::of_bytes(b"rite"), t(200)).is_err());
        assert!(anti_spiral_claims(&st, "lake", &a, &[b], Cid::of_bytes(b"rite"), t(200)).is_err());
    }

    #[test]
    fn refusing_needs_a_refuse_act_and_accepting_a_join() {
        let (a, _) = did();
        let (b, _) = did();
        let join = state(&a, CompactActKind::Join, &b);
        let claim = Cid::of_bytes(b"claim");
        assert!(feature_acceptance(&join, "river", &a, &b, claim, true, None, t(200)).is_ok());
        assert!(feature_acceptance(&join, "river", &a, &b, claim, false, None, t(200)).is_err());
        let refuse = state(&a, CompactActKind::Refuse, &b);
        assert!(feature_acceptance(&refuse, "river", &a, &b, claim, false, None, t(200)).is_ok());
    }

    #[test]
    fn a_sponsorship_rides_the_probation_the_act_names() {
        let (a, _) = did();
        let (c, _) = did();
        let st = state(&a, CompactActKind::Sponsor, &c);
        let rec = sponsorship(&st, "river", &a, &c, t(1000)).unwrap();
        assert_eq!(rec.probation_until.as_second(), 1000 + 30 * 86_400);
        assert!(sponsorship(&state(&a, CompactActKind::Join, &c), "river", &a, &c, t(1000)).is_err());
    }

    #[test]
    fn an_examining_term_is_bounded() {
        let (a, _) = did();
        let seat = did();
        assert!(examiner_registry(&a, &[seat.clone()], t(0), t(EXAMINER_TERM_MAX_SECONDS)).is_ok());
        assert!(examiner_registry(&a, &[seat.clone()], t(0), t(EXAMINER_TERM_MAX_SECONDS + 1)).is_err());
        assert!(examiner_registry(&a, &[], t(0), t(10)).is_err());
    }

    #[test]
    fn an_anathema_is_second_class_and_needs_two_reporters() {
        let (a, _) = did();
        let (b, _) = did();
        let (c, _) = did();
        let st = state(&a, CompactActKind::Anathematize, &c);
        let rec = anathema(&st, "river", &[a.clone(), b.clone()], &c, "n".into(), t(5)).unwrap();
        assert!(matches!(rec.decision_class, DecisionClass::SecondClass));
        assert!(anathema(&st, "river", &[a.clone()], &c, "n".into(), t(5)).is_err());
        assert!(anathema(&state(&a, CompactActKind::Join, &c), "river", &[a, b], &c, "n".into(), t(5)).is_err());
    }
}
