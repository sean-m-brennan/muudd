//! Timestamp re-export. Uses `jiff::Timestamp` (UTC, nanosecond precision).
//!
//! Canonical CBOR serialization is via jiff's `serde` feature, which encodes
//! as ISO-8601 strings (deterministic, sortable).

pub use jiff::Timestamp;

pub fn now() -> Timestamp {
    jiff::Timestamp::now()
}
