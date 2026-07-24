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

//! `en-charter` — the polity **founding** primitive (the keystone of the ethne
//! tier).
//!
//! A charter turns an emergent set of cooperators into a **polity**: a bounded,
//! substrate-anchored community that persists across its members. Its shape is
//! borrowed from kith-covenant's compact constitution but scoped to one
//! community, and — per design decision D1 — this crate depends only on
//! `kc-core` / `en-core` and the neutral `en-archive`, never on KC's governance
//! crates.
//!
//! # Ratify, do not replace
//!
//! The distinctive discipline of the tier is that structure is **ratified**, not
//! imposed. A [`FoundingProposal`] carries the candidate boundary and substrate —
//! as read from the emergent structure or hand-authored — but has **no
//! authority**. Only [`FoundingProposal::ratify`], the founding members
//! co-signing, turns it into a [`PolityCharter`]. Emergence proposes; the polity
//! ratifies; (later) kith-covenant recognizes.
//!
//! # What the charter fixes and what it may change
//!
//! - The **substrate** (what the polity defers to) is *genesis-immutable*: there
//!   is no record that changes it.
//! - **Founding commitments** are genesis-immutable too (they cannot be removed).
//! - **Later commitments** follow an asymmetric rule — added at a majority,
//!   removed only at a majority **+ 1** — so weakening the polity's self-binding
//!   is always harder than strengthening it.
//! - A founding is a **singleton** per polity and needs a **majority of the
//!   roll** to ratify.
//!
//! See `doc/design.md` for the tier's full architecture.

mod authority;
mod entry;
mod record;
mod state;

#[cfg(test)]
mod tests;

pub use authority::{authority_violations, check_authority, majority};
pub use entry::{didkey_resolver, verify_all, verify_entry, CharterEntry};
pub use record::{
    Affirmation, AffirmationKind, AmendmentKind, CharterDissolution, Commitment,
    CommitmentAmendment, FoundingProposal, MembershipChange, MembershipKind, Office, OfficeChange,
    OfficeKind, PolityCharter, Substrate,
};
pub use state::{
    affirmation_state, charter_state, office_state, AffirmationState, CharterState, MemberAffirmation,
    OfficeHolding, OfficeState,
};
