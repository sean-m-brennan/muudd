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

//! Escalation and designation records (protocol component §5): the dispute
//! ladder, the anathema designation with its four-condition test, restitution
//! obligations and their verification, re-entry, and the
//! substantive-re-engagement rung.
//!
//! This crate carries only the *record* shapes plus the pure dispute-ladder
//! ordering (`dispute_ladder_rank`, `is_second_class_rung`). The structural
//! rules the research requires — a legal ladder transition follows the current
//! rung and does not skip; a designation satisfies all four conditions; a
//! restitution completion is verified by a non-party mediator panel; a re-entry
//! carries a full recitation and a sincerity-affirmed witness quorum — are
//! enforced at the archive's authority layer (`kc_archive::authority`), because
//! they range over the whole archive (the referenced dispute, the restoration
//! cycles, the designation being answered).
//!
//! Note on `SubstantiveReEngagement`: the *rung* value lives on
//! `kc_conduct::LadderRung` (so the audit commons holds one conduct timeline),
//! while a deliberate *invocation* of it is the structured
//! [`SubstantiveReEngagement`] record here. The two are the same rung viewed as
//! a label vs. as an attributable act.
//!
//! Deferred to later slices: the structured-skepticism protocol and
//! protecting-powers (§4/§8), automatic decay-triggered re-engagement (kept a
//! deliberate act on purpose), and the periodic-re-examination scheduler that
//! would act on `Designation::reexamine_after`.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    dispute_ladder_rank, is_second_class_rung, Capability, DeliberationCycle, Designation,
    DesignationTier, DesignationUrgency, InjunctionChallenge, InjunctionConfirmation,
    InterimInjunction, LadderTransition, ReEntry, ReEntryWitnessAttestation, RestitutionCompletion,
    RestitutionObligation, SubstantiveReEngagement,
};
