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

//! `en-substrate` — the pluggable input to the polity tier.
//!
//! A [`SubstrateProvider`] surfaces the emergent structure ethne crystallizes
//! into a polity: a candidate membership boundary (and, where it can, a
//! substrate), which [`FoundingCandidate::propose`] turns into a
//! `FoundingProposal` for the members to ratify. `autonomous_trust` is *one*
//! provider (the `ethne-at` adapter); [`ManualFounding`] is a convener-driven
//! provider that needs no substrate service at all.
//!
//! Because this crate depends only on `en-core` and `en-charter` — never on
//! `autonomous_trust`, never on kith-covenant's governance crates — a polity can
//! be founded through the manual provider with neither in the graph. That is the
//! decoupling (KC ⟂ AT) made concrete; see the crate tests.

mod manual;
mod provider;

#[cfg(test)]
mod tests;

pub use manual::ManualFounding;
pub use provider::{
    ChangeStream, FoundingCandidate, FoundingTerms, MemberCandidate, SubstrateChange,
    SubstrateProvider,
};
