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

//! Observer / jester function records (protocol component §8), v1 slice: the
//! observer-corps registry, attributed public reports with a right of response,
//! and the whistleblower channel with a quorum-gated source reveal.
//!
//! The observer corps is the compact's cross-community journalism function
//! (jester_and_dissent.md "The traveling-observer model"): observers are
//! *nominated by member communities* (not self-appointed), carry a **declared
//! perspective** (openly partisan, per the adversarial-information model), and
//! report **publicly and attributably** on communities *other than their own*.
//! Publication is irrevocable; a subject community answers with an appended
//! [`ReportResponse`], never by suppression. This is the mechanism for the
//! "hard middle case" (Type-4 heterodoxy) the escalation ladder cannot reach.
//!
//! The whistleblower channel (jester_and_dissent.md "Whistleblower protection")
//! is the harder case: an insider reports a suspected virtue violation across the
//! AT/kith-covenant boundary. A mediator **reception panel** vouches that it
//! received a credible insider report, and the source is protected
//! cryptographically at both ends of its life:
//!
//! - an [`InsiderClaim`] ring-signed over the accused community's own resolvable
//!   keys proves an insider filed, without saying which one (`kc_core::ring`);
//! - the identity itself is held under k-of-n threshold custody keyed to the
//!   reception panel (`kc_core::custody`), so a minority cannot recover it early;
//! - a [`SourceReveal`] must open the filing's commitment with real shares, and a
//!   member wrongly named can answer with a [`SourceExculpation`] proving the
//!   filing's key image is not theirs.
//!
//! The v1 slice stored only a `source_commitment` and left both halves to a later
//! pass. That protected the source by omission, which turned out to prove nothing:
//! nothing in the archive read the commitment, so a reveal could name anyone, and
//! nothing bound a source to a filing at all, so a panel could manufacture insider
//! complaints against a rival. Both are closed here.
//!
//! This crate carries only the *record* shapes. The structural rules — an
//! observer is a registered, non-party member of the corps; a response comes from
//! the report's subject; a whistleblower reception (and any reveal) is a
//! non-party mediator panel; the ring is the accused community's key set; a reveal
//! recovers what the filing committed to — are enforced at the archive's authority
//! layer (`kc_archive::authority`), because they range over the whole archive (the
//! registries, the referenced report).
//!
//! Deferred: secure out-of-band whistleblower transport (how a source reaches a
//! panel unobserved is operational infrastructure, not a record shape), and a
//! separate encryption key per mediator so custody sealing stops reusing the
//! signing key.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    exculpation_context, InsiderClaim, ObserverEntry, ObserverRegistry, ObserverReport,
    ReportResponse, SourceExculpation, SourceReveal, WhistleblowerReport,
};
