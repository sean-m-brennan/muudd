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

//! ethne archive — an append-only, content-addressed log.
//!
//! This is the neutral archive *mechanism*, forked thin from `kc-archive`
//! (decision D1) and made **generic over the entry type** rather than tied to a
//! fixed record enum. It guarantees only append-only ordering and content
//! addressing (blake3 of canonical CBOR, via `kc-core`). Signature verification
//! and DID resolution are domain concerns — they depend on which records carry
//! identity — and so live with ethne's own records in later slices, not here.
//!
//! `kc-archive` today hardcodes kith-covenant's `ArchiveEntry` enum; refactoring
//! *it* onto a shared generic core like this one is the extraction deferred in
//! D1. `en-archive` gets there directly because it is new code.

mod store;

pub use store::{Archive, Entry};
