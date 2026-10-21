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
//! **This crate is now a re-export of [`kc_log`]** (decision **D32**), which is the
//! generic entry-agnostic archive base D1 named as "the one extraction worth doing
//! later". Until D32 this crate held its own copy of the mechanism, forked thin from
//! `kc-archive` and made generic; `kc-archive` has been refactored onto the same base,
//! so the tier no longer carries the duplication D1 recorded.
//!
//! The name stays because the tier's crates import `en_archive::{Archive, Cursor,
//! Entry}` and this is where a reader of the polity tier looks for its log. Nothing
//! else remains here: the mechanism, its persistence, and its tests all live in
//! `kc-log`.
//!
//! `kc-log` is neutral — an append-only log over `kc-core`'s `Cid`, with no governance
//! in it and no dependency on any `kc-*` governance crate — so depending on it widens
//! D1's rule from "`kc-core` only" to "`kc-core` and `kc-log`, both neutral" without
//! touching the ethne ⟂ KC-governance boundary. Signature verification and DID
//! resolution stay where they were: domain concerns, over ethne's own records, in
//! `en-charter`.

pub use kc_log::{
    segment_bounds, segment_entries, Archive, Cursor, Entry, SegmentBounds, DUMP_VERSION,
};
