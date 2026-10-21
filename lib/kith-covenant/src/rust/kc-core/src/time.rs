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

//! Timestamp re-export. Uses `jiff::Timestamp` (UTC, nanosecond precision).
//!
//! Canonical CBOR serialization is via jiff's `serde` feature, which encodes
//! as ISO-8601 strings (deterministic, sortable).

pub use jiff::Timestamp;

pub fn now() -> Timestamp {
    jiff::Timestamp::now()
}
