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

//! An append-only, content-addressed log — the neutral archive **mechanism**, generic
//! over the entry type.
//!
//! Two tiers keep archives over this crate: kith-covenant's `kc-archive` over its
//! `ArchiveEntry` enum, and the ethne polity tier's `en-archive` over its own. Both
//! used to own a copy. This crate is the extraction ethne's decision **D1** named as
//! "the one extraction worth doing", and it is deliberately small: it guarantees
//! append-only ordering, content addressing (blake3 of canonical CBOR, via `kc-core`),
//! generation-safe cursors, and JSON persistence — nothing else.
//!
//! **Signature verification and DID resolution are not here.** They depend on which
//! records carry identity, which is a domain question, so they live with each tier's
//! own records: `kc_archive::InMemoryArchive::verify` for kith-covenant, and ethne's
//! `en-charter` for the polity tier. This crate therefore depends on `kc-core` only
//! for `Cid` / `KcError`, and on no governance crate in either tier.
//!
//! # Append order is the only order this crate exposes
//!
//! [`Archive`] holds a `Vec<Cid>` of append order and an unordered index. There is no
//! way to iterate the index, and that restriction is the reason the two copies were
//! worth merging rather than merely deduplicating: while each tier owned its own, a
//! domain-layer caller in the same file could reach the index directly, and one did.
//! kith-covenant's `community_identity` documented "the most recent" and read the map
//! in descending CID order — arbitrary with respect to when anything was appended. It
//! returned the *earlier* of two publications in 104 of 200 trials.
//!
//! Supersession rules ("the latest `issued_at` per community wins") are domain rules.
//! They belong above this crate, written down, over [`Archive::iter`].

mod dump;
mod store;

pub use dump::{segment_bounds, segment_entries, SegmentBounds, DUMP_VERSION};
pub use store::{Archive, Cursor, Entry};
