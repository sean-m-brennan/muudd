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

//! Mediation records (protocol component §4), v1 slice: the mediator pool,
//! dispute intake, and paneled decisions.
//!
//! This crate carries only the *record* shapes. The structural rules the
//! research requires — panels of at least three registered mediators, drawn
//! from communities that are **not** party to the dispute (non-overlap;
//! liminal_institutions.md "The mediator pool") — are enforced at the archive's
//! authority layer (`kc_archive::authority`), alongside §5 authority
//! enforcement, because they need to range over the whole archive (the mediator
//! registries and the referenced dispute).
//!
//! The pool is modeled per community: each community nominates and signs its own
//! [`MediatorRegistry`]; the compact-wide pool is the union across the archive,
//! and each mediator's nominating community is therefore known (which the
//! non-overlap rule needs). Registries supersede by latest `issued_at` per
//! nominating community, mirroring the witness registry.
//!
//! Argument and deliberation records arrived later, with [`ArgumentRecord`]: §4's
//! "structured argument submission" and "evidence packaging", append-only and
//! self-signed, cited by the decision they are answered in. That slice also made §4's
//! *mandatory* reasoning trail actually mandatory and bound the decision's `panel`
//! field to its co-signers, both of which the record had carried as decoration.
//!
//! [`PanelConsent`] completed §4's "recusal and consent tracking" from the other side.
//! Objection was a record and structural consent was a record; a party's consent to the
//! panel *as constituted* was inferred from its having filed an argument. It gates one
//! thing and deliberately no more: a `Settled` outcome claims the parties settled, so
//! both must have consented to the panel that recorded it.
//!
//! The recusal workflow ([`RecusalRequest`] / [`RecusalDecision`]), protecting powers
//! ([`ProtectingPower`]) and the mediator's training lineage
//! ([`MediatorEntry::trained_by`]) arrived together with the §4 roster caps, which put
//! the mediator pool through the same bounded-term, rotation and no-majority-share
//! rules §8's observers and §2's examiners already carried — it had been the third
//! roster and the one wired to nothing.
//!
//! [`SealedReasoning`] and [`DecisionUnsealed`] carry §4's "sealed decisions with
//! threshold-reveal", the sealed-mediation-proceedings exception to the public-archive
//! principle. What is sealed is the reasoning; the dispute, panel and outcome stay
//! public, so every rule already built on a decision keeps running over it. Custody is
//! the §8 primitive, put to the second use it was always shaped for — and the authority
//! pass bars the deciding panel from holding a quorum of the shares, since a panel that
//! can unseal itself has deferred publication rather than sealed anything.
//!
//! The mediator pool's authority over invoking the structured-skepticism protocol lives
//! one crate over, in `kc_provenance`, with the event records it acts on.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    ArgumentRecord, ArgumentRole, ConsentAnswer, DecisionOutcome, DecisionUnsealed,
    DisputeRecord, MediationDecision, MediatorEntry, MediatorRegistry, PanelConsent,
    ProtectingPower, RecusalAnswer, RecusalDecision, RecusalRequest, SealedReasoning,
};
