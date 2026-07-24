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

/// In-memory, append-only archive indexed by entry CID and generic over the
/// entry type `E`. Insertion order is preserved; a duplicate CID (identical
/// content) is rejected, because the log is content-addressed — the same content
/// is stored at most once.
#[derive(Debug, Clone)]
pub struct Archive<E: Entry> {
    order: Vec<Cid>,
    entries: BTreeMap<String, E>,
}

// Manual `Default` so `Archive::new()` does not require `E: Default`.
impl<E: Entry> Default for Archive<E> {
    fn default() -> Self {
        Self { order: Vec::new(), entries: BTreeMap::new() }
    }
}

impl<E: Entry> Archive<E> {
    pub fn new() -> Self {
        Self::default()
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
        self.order
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
    fn missing_cid_is_none() {
        let a: Archive<TestRec> = Archive::new();
        let absent = Cid::of(&rec(99, "absent")).unwrap();
        assert!(a.get(&absent).is_none());
        assert!(!a.contains(&absent));
    }
}
