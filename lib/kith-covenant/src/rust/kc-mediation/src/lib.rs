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
//! Deferred to later slices: argument/deliberation records, interim injunctions
//! (§9 enforcement linkage), the structured-skepticism protocol,
//! protecting-powers, recusal workflow, and mediator training lineage.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    DecisionOutcome, DisputeRecord, MediationDecision, MediatorEntry, MediatorRegistry,
};
