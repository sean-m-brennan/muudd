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

//! Canonical CBOR encoding for signed payloads.
//!
//! Output is deterministic for a given Rust value under the ciborium encoder:
//! stable struct-field order (declaration order), smallest-form integers, and
//! BTreeMap keys encoded in sorted order. This is sufficient for Blake3
//! content-addressing across Rust nodes running identical code.
//!
//! Strict RFC 8949 §4.2 canonical form (lexicographic key sort across encoders)
//! is NOT implemented — required only for cross-language interop, which the MVP
//! does not need.
//!
//! Domain types must not use `std::collections::HashMap` — iteration order is
//! non-deterministic. Use `BTreeMap` or `Vec<(K, V)>` with a stable sort.

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{KcError, KcResult};

pub fn to_canonical_cbor<T: Serialize>(value: &T) -> KcResult<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf)
        .map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    Ok(buf)
}

pub fn from_canonical_cbor<T: DeserializeOwned>(bytes: &[u8]) -> KcResult<T> {
    ciborium::de::from_reader(bytes).map_err(|e| KcError::CanonicalDecode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct Sample {
        name: String,
        count: u64,
        tags: Vec<String>,
        data: Vec<u8>,
        attrs: BTreeMap<String, i64>,
    }

    proptest! {
        #[test]
        fn encoding_is_deterministic(
            name in any::<String>(),
            count in any::<u64>(),
            tags in prop::collection::vec(any::<String>(), 0..8),
            data in prop::collection::vec(any::<u8>(), 0..64),
            attrs in prop::collection::btree_map(any::<String>(), any::<i64>(), 0..8),
        ) {
            let s = Sample { name, count, tags, data, attrs };
            let a = to_canonical_cbor(&s).unwrap();
            let b = to_canonical_cbor(&s).unwrap();
            prop_assert_eq!(a, b);
        }

        #[test]
        fn roundtrip(
            name in any::<String>(),
            count in any::<u64>(),
            tags in prop::collection::vec(any::<String>(), 0..8),
            data in prop::collection::vec(any::<u8>(), 0..64),
            attrs in prop::collection::btree_map(any::<String>(), any::<i64>(), 0..8),
        ) {
            let original = Sample { name, count, tags, data, attrs };
            let bytes = to_canonical_cbor(&original).unwrap();
            let decoded: Sample = from_canonical_cbor(&bytes).unwrap();
            prop_assert_eq!(original, decoded);
        }
    }

    #[test]
    fn btreemap_order_independent() {
        // Same BTreeMap content via different insertion orders must hash identically.
        let mut a = BTreeMap::new();
        a.insert("zebra".to_string(), 1i64);
        a.insert("apple".to_string(), 2i64);
        a.insert("mango".to_string(), 3i64);

        let mut b = BTreeMap::new();
        b.insert("apple".to_string(), 2i64);
        b.insert("mango".to_string(), 3i64);
        b.insert("zebra".to_string(), 1i64);

        assert_eq!(to_canonical_cbor(&a).unwrap(), to_canonical_cbor(&b).unwrap());
    }
}
