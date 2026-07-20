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

//! Conduct records: the attributable, signed, append-only observations the
//! compact keeps about member communities' behavior over time.
//!
//! This crate seeds the archived data types for protocol components 5
//! (escalation and designation) and 6 (reputation ledger). It deliberately
//! carries only the *record* shapes, not the escalation state machine or the
//! quorum/decision-class enforcement policy, which live in those components.
//!
//! The load-bearing variant for capture resistance is [`ConductKind::Concern`]:
//! a signed observation that is explicitly *not* an adjudicated violation and
//! carries no escalation-ladder rung. It is the jester's pre-indictment
//! instrument (capture_resistance.md §§2,3,7; jester_and_dissent.md), legible
//! in aggregate before any single act is indictable. Reporters declare their
//! perspective (jester_and_dissent.md, adversarial-information model:
//! advocacy is openly partisan, adjudication is neutral), so downstream
//! analytics can discount declared
//! partiality rather than pretend to neutrality they do not have.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity,
};
