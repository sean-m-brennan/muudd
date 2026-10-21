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

//! Manufactured-event provenance and the structured-skepticism protocol
//! (capture-resistance move 4).
//!
//! Bad actors can deliberately generate **ambiguous-transcendent events** to
//! direct interpretation, relying on being the *loudest interpreter*: the
//! manufacturer has prepared framings the ambient community has not
//! (capture_resistance.md move 4; liminal_institutions.md
//! §"Manufactured-event resistance"). The strengthening move is a
//! **structured-skepticism protocol** for events meeting specific criteria —
//! high interpretive charge, convenient alignment with one faction's interest,
//! sudden onset, and originating witnesses who hold stake. The protocol does not
//! prejudge; it imposes provenance tagging, stake disclosure, and
//! multi-community corroboration that buy the ambient community time to develop
//! its own interpretation, depriving the manufacturer of the loudest-interpreter
//! advantage.
//!
//! The **invocation is the mediator pool's**, not the originating community's. That is
//! the doctrine's assignment ("The mediator pool is empowered to invoke the
//! structured-skepticism protocol on its own initiative"), and until
//! [`SkepticismInvocation`] existed the protocol's teeth rested entirely on
//! [`EventReport`]'s self-declared triggers — so a manufacturer that declared its own
//! event mundane was never held, and nothing in the compact could hold it.
//! [`SkepticismReview`] is the other half the same passage requires: pool review against
//! a hold used as a tactic to suppress a legitimate urgent matter.
//!
//! This crate carries the records that protocol produces:
//! - [`EventReport`] — a claim injected into the compact, **provenance-tagged**
//!   at intake with its originator, declared incentives, and information
//!   history, plus the structured-skepticism qualifying triggers and the
//!   originator's stake disclosure.
//! - [`EventCorroboration`] — an independent corroboration of a report's
//!   observable facts by another community, optionally supplying a
//!   cross-substrate translation.
//!
//! Both are *data only*. Whether a qualifying event has been sufficiently
//! corroborated is a **detector** signal (kc-reputation `structured_skepticism`)
//! — flags with provenance, no verdict; adjudication is the mediator pool's job.

use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// A claim injected into the compact, provenance-tagged at intake. Wrapped in
/// `Signed<EventReport>` by the originating community (root or a delegate key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReport {
    /// Who injected the claim (provenance).
    pub originator: DidKey,
    pub summary: String,
    /// Structured-skepticism qualifying trigger: the event carries high
    /// interpretive charge.
    pub high_interpretive_charge: bool,
    /// Qualifying trigger: the event appeared suddenly.
    pub sudden_onset: bool,
    /// Qualifying trigger: the originating witness holds stake in the
    /// interpretation.
    pub originator_holds_stake: bool,
    /// Qualifying trigger: the event is conveniently aligned with one faction's
    /// interest (that faction, if any).
    pub aligned_faction: Option<DidKey>,
    /// Provenance tag: the incentives the originator declares.
    pub declared_incentives: String,
    /// Provenance tag: the chain the information travelled before intake.
    pub information_history: String,
    /// Stake disclosure: what parties disclose they gain from a framing. An
    /// undisclosed interest later discovered is bad faith (left to the conduct
    /// machinery), so a stake-holding originator that discloses nothing is a
    /// provenance gap the detector surfaces.
    pub disclosed_interests: Vec<String>,
    pub onset_at: Timestamp,
    pub reported_at: Timestamp,
}

impl EventReport {
    /// The qualifying triggers that fired, for the reader. Any non-empty result
    /// means the structured-skepticism protocol applies to this event.
    pub fn qualifying_triggers(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.high_interpretive_charge {
            v.push("high_interpretive_charge");
        }
        if self.sudden_onset {
            v.push("sudden_onset");
        }
        if self.originator_holds_stake {
            v.push("originator_holds_stake");
        }
        if self.aligned_faction.is_some() {
            v.push("aligned_faction");
        }
        v
    }

    /// Does the event meet the structured-skepticism criteria at all?
    pub fn qualifies(&self) -> bool {
        !self.qualifying_triggers().is_empty()
    }

    /// Are the provenance tags present — declared incentives and information
    /// history non-empty, and a stake-holding originator having disclosed at
    /// least one interest? Incomplete provenance on a qualifying event is itself
    /// a signal.
    pub fn provenance_complete(&self) -> bool {
        !self.declared_incentives.trim().is_empty()
            && !self.information_history.trim().is_empty()
            && (!self.originator_holds_stake || !self.disclosed_interests.is_empty())
    }
}

/// An independent corroboration of an event report's observable facts (separable
/// from any interpretation of them) by another community. Wrapped in
/// `Signed<EventCorroboration>` by the corroborating community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCorroboration {
    /// The `EventReport` entry this corroborates.
    pub event_cid: Cid,
    pub corroborating_community: DidKey,
    /// A cross-substrate translation of the interpretation, if supplied: an
    /// interpretation surviving only in the originating framing is structurally
    /// suspect.
    pub translation: Option<String>,
    pub corroborated_at: Timestamp,
}

/// A formal action premised on an event report — an escalation, a designation, a
/// sanction: any consequence a community records as following *from* the event
/// (capture move 4). This is where the structured-skepticism protocol acquires
/// teeth: a *qualifying* manufactured event may not drive a recorded action until
/// its structured hold elapses or independent corroboration releases it, depriving
/// the manufacturer of the loudest-interpreter advantage. Wrapped in
/// `Signed<EventActionInvoked>` by the acting community; the hold itself is
/// enforced in the archive's authority pass (kc-archive), not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventActionInvoked {
    /// The `EventReported` entry this action is premised on.
    pub event_cid: Cid,
    /// The community recording the action on the event's basis.
    pub acting_community: DidKey,
    /// What consequence is being invoked (free text; the compact's own vocabulary).
    pub action: String,
    pub invoked_at: Timestamp,
}

/// One qualifying criterion, as the **mediator pool** finds it rather than as the
/// originator declared it. The four are the doctrine's:
/// "high interpretive charge, conveniently-aligned with one faction's interests,
/// sudden onset, originating from witnesses with stake in the interpretation"
/// (`liminal_institutions.md` "Manufactured-event resistance").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SkepticismCriterion {
    HighInterpretiveCharge,
    SuddenOnset,
    OriginatorHoldsStake,
    AlignedFaction,
}

impl SkepticismCriterion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HighInterpretiveCharge => "high_interpretive_charge",
            Self::SuddenOnset => "sudden_onset",
            Self::OriginatorHoldsStake => "originator_holds_stake",
            Self::AlignedFaction => "aligned_faction",
        }
    }
}

/// The mediator pool placing an event under the structured-skepticism protocol on its
/// own initiative. Co-signed by the invoking panel; wrapped in
/// `Signed<SkepticismInvocation>`.
///
/// **Why the pool and not the originator.** [`EventReport`]'s four triggers are what
/// the originating community *declares* about its own claim, and a manufactured event's
/// manufacturer declares whatever suits it. The protocol's teeth — a formal action on a
/// qualifying, under-corroborated event is refused — therefore rested on the word of the
/// party the protocol exists to constrain: an originator that declared all four false
/// was never held, and nothing in the compact could hold it. §4 assigns the invocation
/// to the mediator pool, "the compact's standing evaluation competency"
/// (`liminal_institutions.md`), and this is that record.
///
/// The self-declared triggers are unchanged and still bite. They are a voluntary
/// submission to the protocol, which is no capture risk; what they could not do is
/// reach an event whose originator declined to declare it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkepticismInvocation {
    /// The `EventReported` entry placed under the protocol.
    pub event_cid: Cid,
    /// The invoking mediators. **Derived, not chosen**: the authority pass requires
    /// this to be exactly the envelope's co-signers.
    pub panel: Vec<DidKey>,
    /// Which criteria the pool finds, in its own judgment. Required non-empty: an
    /// invocation on no stated criterion is the pretextual use the review guards
    /// against, and the doctrine asks that "the criteria must be documented
    /// specifically".
    pub criteria: Vec<SkepticismCriterion>,
    /// The pool's account of why those criteria are met here. Required non-empty and
    /// never read for content, as every stated ground in this tier is.
    pub grounds: String,
    /// How long the hold runs from the event's intake. The doctrine's first element:
    /// "the interval scales with interpretive charge... For most events this is hours
    /// to days; for high-charge events of major interpretive consequence it may extend
    /// to weeks or months." Bounded at both ends by the authority pass — an unbounded
    /// hold is suppression, and a nominal one is no protocol at all.
    pub hold_seconds: i64,
    pub invoked_at: Timestamp,
}

/// How a [`SkepticismInvocation`] was reviewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewAnswer {
    /// The hold stands.
    Upheld,
    /// The hold was a tactic, and the event is released from it.
    Pretextual,
}

impl ReviewAnswer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upheld => "Upheld",
            Self::Pretextual => "Pretextual",
        }
    }
}

/// The mediator pool's internal review of a hold, against pretextual use. Co-signed by
/// reviewing mediators; wrapped in `Signed<SkepticismReview>`.
///
/// §4 does not stop at giving the pool the invocation. It requires the pool to check
/// "that the protocol's invocation itself is not used as a tactic: a faction attempting
/// to suppress legitimate urgent matters by invoking structured skepticism against
/// them" (`liminal_institutions.md`). Without this record a hold answered to nobody —
/// including the self-declared hold, which a community could place on its own event with
/// no reviewer on the record anywhere.
///
/// So the review is against the **hold**, not against the invocation record, and reaches
/// a self-declared hold exactly as it reaches an invoked one. A `Pretextual` finding
/// releases the event; an `Upheld` finding changes nothing and exists because
/// "invocations are auditable" requires the audit to be legible whichever way it came
/// out.
///
/// Who may sign is the rule with the teeth: not the originator, which benefits from
/// release, and not the invoking panel, since a panel that reviews its own invocation is
/// the conflict the review exists to raise — the same asymmetry [`crate`]'s sibling
/// tier applies to a declined recusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkepticismReview {
    /// The `EventReported` entry whose hold is under review.
    pub event_cid: Cid,
    /// The reviewing mediators. **Derived, not chosen**, as the invoking panel is.
    pub reviewers: Vec<DidKey>,
    pub answer: ReviewAnswer,
    /// Required non-empty on both answers: a hold released and a hold sustained both owe
    /// an account, and the audit the doctrine asks for is worthless on one side only.
    pub reasoning: String,
    pub reviewed_at: Timestamp,
}

/// A retroactive finding that an event's originator held, at intake, an interest
/// it did not disclose — the bad-faith signal move 4 leaves to be surfaced after
/// the fact (an event may already have driven action before the omission came to
/// light). Wrapped in `Signed<RetroactiveInterestFinding>` by the `finder`. Data
/// only: the record surfaces that an undisclosed interest was found and by whom;
/// the *consequence* of bad faith is the conduct machinery's, not this crate's.
/// The `structured_skepticism` lens counts a finding only when the finder is not
/// the event's own originator (a self-finding is honest late disclosure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroactiveInterestFinding {
    /// The `EventReported` entry whose originator is impugned.
    pub event_cid: Cid,
    /// The party that discovered the omission.
    pub finder: DidKey,
    /// The interest the originator held at intake but did not disclose.
    pub undisclosed_interest: String,
    pub found_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{canonical, keys, time, Alg};

    fn did() -> DidKey {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        DidKey::from_pubkey(&pk).unwrap()
    }

    fn manufactured() -> EventReport {
        EventReport {
            originator: did(),
            summary: "a sudden, convenient omen".into(),
            high_interpretive_charge: true,
            sudden_onset: true,
            originator_holds_stake: true,
            aligned_faction: Some(did()),
            declared_incentives: String::new(),
            information_history: String::new(),
            disclosed_interests: vec![],
            onset_at: time::now(),
            reported_at: time::now(),
        }
    }

    #[test]
    fn manufactured_event_qualifies_with_all_triggers() {
        let e = manufactured();
        assert!(e.qualifies());
        assert_eq!(e.qualifying_triggers().len(), 4);
        // Bare-provenance manufactured event: incomplete.
        assert!(!e.provenance_complete());
    }

    #[test]
    fn mundane_event_does_not_qualify() {
        let mut e = manufactured();
        e.high_interpretive_charge = false;
        e.sudden_onset = false;
        e.originator_holds_stake = false;
        e.aligned_faction = None;
        assert!(!e.qualifies());
        assert!(e.qualifying_triggers().is_empty());
    }

    #[test]
    fn provenance_complete_when_tags_and_disclosure_present() {
        let mut e = manufactured();
        e.declared_incentives = "commons allocation".into();
        e.information_history = "relayed via envoy X".into();
        e.disclosed_interests = vec!["gains relay priority".into()];
        assert!(e.provenance_complete());
    }

    #[test]
    fn report_roundtrips_through_cbor() {
        let e = manufactured();
        let bytes = canonical::to_canonical_cbor(&e).unwrap();
        let decoded: EventReport = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(e, decoded);
    }
}
