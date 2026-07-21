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
//! received a credible insider report; the [`WhistleblowerReport`] stores only a
//! `source_commitment` (a hash the panel holds the preimage of out-of-band), so
//! the archive never carries the source identity in the clear. A
//! [`SourceReveal`] discloses it only on a mediator-panel quorum citing named
//! cause.
//!
//! This crate carries only the *record* shapes. The structural rules — an
//! observer is a registered, non-party member of the corps; a response comes from
//! the report's subject; a whistleblower reception (and any reveal) is a
//! non-party mediator panel — are enforced at the archive's authority layer
//! (`kc_archive::authority`), because they range over the whole archive (the
//! registries, the referenced report).
//!
//! Deferred to later slices (MVP crypto is Ed25519-only): ring-signature source
//! *anonymity* (prove insider membership without revealing which member),
//! threshold-*encrypted* source records, the substrate-deference dashboard (which
//! needs §9's compact-incapable list and structured-fast state), observer
//! pool-share and rotation caps, and secure out-of-band whistleblower transport.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    ObserverEntry, ObserverRegistry, ObserverReport, ReportResponse, SourceReveal,
    WhistleblowerReport,
};
