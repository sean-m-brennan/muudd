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

//! Constitutional-layer records (protocol component §9), v1 slice: the
//! substrate-deference test — the compact-incapable list and the structured-fast
//! cadence — as versioned, signed artifacts.
//!
//! The fifth anti-spiral feature holds that the compact is **not ultimate**: it
//! answers to something above itself (justus_hostis.md). Asserted, that claim is
//! unfalsifiable; the substrate-deference test **operationalizes** it
//! (capture_resistance.md §2) in two forms this crate carries:
//!
//! - *Categorical deference* — the **compact-incapable list**: a class of matters
//!   the compact will not decide even when it could and its members would consent
//!   (a member's internal theology and governance, its recognition-statement
//!   content, the truth-claims of its tradition). Founded at genesis and
//!   protected by genesis-immutability; amendable only by second-class procedure,
//!   with **expansion easier than contraction** (the asymmetric capture risk) and
//!   the genesis-original matters non-removable.
//! - *Temporal deference* — the **structured-fast cadence**: defined periods
//!   during which the compact's deliberative and adjudicative functions pause
//!   (life-critical commons continue). The historical models are Lent, *shmita*,
//!   and the Iroquois Condolence period. Adjusting the cadence is likewise
//!   asymmetric: lengthening a fast is easy, shortening it is hard.
//!
//! This crate carries only the *record* shapes. The genesis singleton, the
//! quorum on ratification, and the asymmetric-amendment / genesis-immutability
//! rules are enforced at the archive's authority layer (`kc_archive::authority`),
//! which ranges over the whole archive. Whether the fast is *honored* is
//! **diagnostic**, not hard-refused (capture_resistance.md: "observer corps
//! surfaces deviations"): the erosion report (`kc_reputation::substrate_deference`)
//! surfaces contractions of the list and deliberative acts initiated during a
//! fast, rather than the authority pass rejecting them.
//!
//! Recurring/calendar fast schedules are supported: a `FastPeriod` may carry a
//! `recurrence` cycle so a fast repeats on a cadence (an annual fast, the Iroquois
//! Condolence period) rather than occurring once. Deferred to later slices:
//! automatic escalation on detected erosion, and any hard-refusal of deliberative
//! acts during a fast.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
    FastChange, FastPeriod, IncapableMatter,
};
