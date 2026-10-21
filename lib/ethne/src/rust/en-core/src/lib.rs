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

//! ethne base crate.
//!
//! Per design decision **D1**, ethne (the polity tier) depends on the neutral
//! `kc-core` primitives *only* — never on kith-covenant's governance crates — so
//! the tiers stay decoupled and a polity can be chartered and governed without
//! ever federating. This crate re-exports the shared primitives ethne builds on
//! (content addressing, DID identity, signing, time) and is the home for
//! ethne-wide shared types as later slices add them.

pub use kc_core::{
    canonical, keys, time, Alg, Cid, DidKey, KcError, KcResult, PrivKey, PubKey, Signature,
    SignatureEntry, Signed, Timestamp,
};
