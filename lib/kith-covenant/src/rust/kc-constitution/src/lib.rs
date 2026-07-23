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
//! Condolence period) rather than occurring once.
//!
//! **Erosion-detection alerting** (capture_resistance.md §2, protocol-components
//! §9) is realized as `kc_reputation::erosion_alert`: a graded, derived reading of
//! the erosion report that **escalates** (`None`→`Notice`→`Alert`) as the erosion
//! signals compound, so the observer corps sees a crossed threshold rather than raw
//! counts. It stays diagnostic — no record is emitted and nothing is signed.
//!
//! **Non-goal — hard-refusal of deliberative acts during a fast.** The fast is
//! honored *by practice* and made *legible* here, never enforced by the authority
//! pass. The source is explicit that non-ultimacy is "a **disposition** rather than
//! a **compliance** ... [that] cannot be fully encoded in rules"
//! (capture_resistance.md §2): converting the fast into an authority-layer refusal
//! would make it the compliance rule the doc says it must not become, and would
//! also cut across the life-critical-commons continuity the fast explicitly
//! preserves. Deviations are therefore surfaced (`substrate_deference`,
//! `erosion_alert`), not rejected.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
    FastChange, FastPeriod, IncapableMatter,
};
