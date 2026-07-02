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

//! Content identifiers. 32-byte Blake3 hashes of canonical-CBOR payloads.
//!
//! Displayed as `b3:<hex>`. CIDs are the primitive referenced throughout the
//! archive to point at frozen records.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canonical;
use crate::error::{KcError, KcResult};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cid([u8; 32]);

impl Cid {
    pub const BYTES: usize = 32;

    pub fn of<T: Serialize>(value: &T) -> KcResult<Self> {
        let bytes = canonical::to_canonical_cbor(value)?;
        Ok(Self::of_bytes(&bytes))
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> KcResult<Self> {
        let v = hex::decode(s).map_err(|e| KcError::InvalidDid(format!("cid hex: {e}")))?;
        let arr: [u8; 32] = v.try_into().map_err(|v: Vec<u8>| {
            KcError::InvalidDid(format!("cid hex: expected 32 bytes, got {}", v.len()))
        })?;
        Ok(Self(arr))
    }
}

impl fmt::Debug for Cid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cid({self})")
    }
}

impl fmt::Display for Cid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b3:{}", self.to_hex())
    }
}

// Serialize as a CBOR byte string (not an array of numbers). Decode accepts
// 32-byte byte-strings; other sizes or types are errors.
impl Serialize for Cid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(&self.0).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Cid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let buf: serde_bytes::ByteBuf = Deserialize::deserialize(d)?;
        let v = buf.into_vec();
        let arr: [u8; 32] = v.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::invalid_length(v.len(), &"32 bytes for Cid")
        })?;
        Ok(Self(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Payload {
        a: u64,
        b: String,
    }

    #[test]
    fn equal_payloads_have_equal_cids() {
        let a = Cid::of(&Payload { a: 1, b: "x".into() }).unwrap();
        let b = Cid::of(&Payload { a: 1, b: "x".into() }).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_payloads_have_distinct_cids() {
        let a = Cid::of(&Payload { a: 1, b: "x".into() }).unwrap();
        let b = Cid::of(&Payload { a: 2, b: "x".into() }).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn display_prefix() {
        let c = Cid::of_bytes(b"hello");
        assert!(c.to_string().starts_with("b3:"));
        assert_eq!(c.to_string().len(), 3 + 64);
    }

    #[test]
    fn hex_roundtrip() {
        let c = Cid::of_bytes(b"hello");
        let s = c.to_hex();
        let decoded = Cid::from_hex(&s).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn serde_roundtrip_via_cbor() {
        let original = Cid::of_bytes(b"payload");
        let bytes = canonical::to_canonical_cbor(&original).unwrap();
        let decoded: Cid = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
