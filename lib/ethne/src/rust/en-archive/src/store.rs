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

//! The append-only, content-addressed store, generic over the entry type.

use std::collections::BTreeMap;

use en_core::{Cid, KcError, KcResult};
use serde::Serialize;

/// A record that can live in an [`Archive`]: content-addressable (via canonical
/// CBOR + blake3, inherited from `kc-core`) and carrying a stable `kind`
/// discriminator for diagnostics. The default `cid` is the blake3 of the entry's
/// canonical encoding and should not normally be overridden.
pub trait Entry: Serialize + Clone {
    /// A short, stable name for the entry kind (diagnostics / dashboards).
    fn kind(&self) -> &'static str;

    /// Archive-level content address of the whole entry.
    fn cid(&self) -> KcResult<Cid> {
        Cid::of(self)
    }
}

/// Where a host's storage has caught up to: an index **and the generation it was
/// taken in**.
///
/// The generation is what makes a cursor safe. An index alone is meaningless across
/// a compaction — compaction removes entries from the *middle*, so post-compaction
/// index N is a different entry than pre-compaction index N, and a cursor past the
/// new end reads as "already current" rather than as the error it is. Carrying the
/// generation turns both of those into a refusal (design **D20**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// The archive generation this cursor was taken in.
    pub generation: u64,
    /// How many entries had been persisted at that point.
    pub index: usize,
}

/// In-memory, append-only archive indexed by entry CID and generic over the
/// entry type `E`. Insertion order is preserved; a duplicate CID (identical
/// content) is rejected, because the log is content-addressed — the same content
/// is stored at most once.
///
/// The archive also carries a **generation**, which advances when a derived archive
/// is built with entries *removed* (compaction — see `en_charter::apply_prunings`).
/// Appending never advances it: within one generation the log is purely additive, so
/// an index means the same thing throughout, which is exactly the condition
/// incremental persistence needs.
#[derive(Debug, Clone)]
pub struct Archive<E: Entry> {
    order: Vec<Cid>,
    entries: BTreeMap<String, E>,
    generation: u64,
}

// Manual `Default` so `Archive::new()` does not require `E: Default`.
impl<E: Entry> Default for Archive<E> {
    fn default() -> Self {
        Self { order: Vec::new(), entries: BTreeMap::new(), generation: 0 }
    }
}

impl<E: Entry> Archive<E> {
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty archive one generation *after* `prev` — how a compaction starts.
    ///
    /// Every caller that rebuilds an archive with entries dropped must start here
    /// rather than at [`new`](Archive::new), or a host's cursor from before the
    /// compaction will still look valid. There is one such caller,
    /// `en_charter::apply_prunings`.
    pub fn next_generation(prev: &Self) -> Self {
        Self { generation: prev.generation + 1, ..Self::default() }
    }

    /// Which generation this archive is in. Zero until something is compacted away.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// A cursor at the current end of the log — what a host records once it has
    /// persisted everything present.
    pub fn cursor(&self) -> Cursor {
        Cursor { generation: self.generation, index: self.len() }
    }

    /// Restore a generation read back from a dump. Crate-private: outside of loading,
    /// the generation is not something a caller sets — it is what
    /// [`next_generation`](Archive::next_generation) advances.
    pub(crate) fn set_generation(&mut self, generation: u64) {
        self.generation = generation;
    }

    /// Append `entry`, returning its CID. Errors on a duplicate CID.
    pub fn append(&mut self, entry: E) -> KcResult<Cid> {
        let cid = entry.cid()?;
        let key = cid.to_string();
        if self.entries.contains_key(&key) {
            return Err(KcError::Archive(format!("duplicate entry: {cid}")));
        }
        self.entries.insert(key, entry);
        self.order.push(cid);
        Ok(cid)
    }

    /// Fetch an entry by CID.
    pub fn get(&self, cid: &Cid) -> Option<&E> {
        self.entries.get(&cid.to_string())
    }

    /// Whether an entry with this CID is present.
    pub fn contains(&self, cid: &Cid) -> bool {
        self.entries.contains_key(&cid.to_string())
    }

    /// Iterate `(cid, entry)` pairs in insertion (append) order.
    pub fn iter(&self) -> impl Iterator<Item = (&Cid, &E)> {
        self.iter_from(0)
    }

    /// Iterate from `start` onward, in append order.
    ///
    /// Slices the order index rather than skipping over it, so an incremental dump
    /// costs what is *new* rather than walking (and index-looking-up) the whole log
    /// to reach the tail. Measured: with `skip`, appending one entry to a
    /// 5,000-entry log took ~5.4 ms of pure CPU; slicing makes it flat.
    /// `start` beyond the end yields nothing.
    pub fn iter_from(&self, start: usize) -> impl Iterator<Item = (&Cid, &E)> {
        self.order[start.min(self.order.len())..]
            .iter()
            .map(move |cid| (cid, self.entries.get(&cid.to_string()).expect("index invariant")))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestRec {
        seq: u64,
        note: String,
    }

    impl Entry for TestRec {
        fn kind(&self) -> &'static str {
            "TestRec"
        }
    }

    fn rec(seq: u64, note: &str) -> TestRec {
        TestRec { seq, note: note.into() }
    }

    #[test]
    fn append_indexes_by_cid_and_preserves_order() {
        let mut a = Archive::new();
        assert!(a.is_empty());
        let c0 = a.append(rec(0, "genesis")).unwrap();
        let _c1 = a.append(rec(1, "next")).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a.get(&c0), Some(&rec(0, "genesis")));
        let seqs: Vec<u64> = a.iter().map(|(_, e)| e.seq).collect();
        assert_eq!(seqs, vec![0, 1]);
    }

    #[test]
    fn identical_content_has_identical_cid_and_dedups() {
        let mut a = Archive::new();
        let first = a.append(rec(7, "same")).unwrap();
        // Same content → same CID → rejected (content-addressed, append-once).
        let err = a.append(rec(7, "same")).unwrap_err();
        assert!(matches!(err, KcError::Archive(_)));
        assert_eq!(a.len(), 1);
        assert!(a.contains(&first));
        // A distinct payload gets a distinct CID and appends.
        let second = a.append(rec(7, "different")).unwrap();
        assert_ne!(first, second);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn kind_and_default_cid_match_canonical_hash() {
        let r = rec(1, "x");
        assert_eq!(r.kind(), "TestRec");
        assert_eq!(r.cid().unwrap(), Cid::of(&r).unwrap());
    }

    #[test]
    fn iter_from_slices_the_tail_and_tolerates_an_over_long_start() {
        let mut a = Archive::new();
        for i in 0..4 {
            a.append(rec(i, "x")).unwrap();
        }
        assert_eq!(a.iter_from(2).map(|(_, e)| e.seq).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(a.iter_from(4).count(), 0);
        // Past the end yields nothing rather than panicking on the slice.
        assert_eq!(a.iter_from(99).count(), 0);
    }

    #[test]
    fn missing_cid_is_none() {
        let a: Archive<TestRec> = Archive::new();
        let absent = Cid::of(&rec(99, "absent")).unwrap();
        assert!(a.get(&absent).is_none());
        assert!(!a.contains(&absent));
    }
}
