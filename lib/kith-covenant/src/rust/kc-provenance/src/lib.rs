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
//! This crate carries the two records that protocol produces:
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
