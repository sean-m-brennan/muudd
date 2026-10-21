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

//! JSON dump/load for the whole archive. Round-trips an `InMemoryArchive`
//! via pretty-printed JSON; useful for demos, audit exports, and regression
//! tests.
//!
//! The envelope format lives in [`kc_log`] (ethne decision **D32**); these are the
//! forwarding methods, which keep the names `kc-pyo3`'s `PyArchive` and the demos
//! already call. Two things changed with the move:
//!
//! - The format version is now `kc_log::DUMP_VERSION`, read from one place by both the
//!   writer and the reader. This module used to carry it as a bare `1` in each, with
//!   nothing linking them, so only a round-trip test stood between the two literals
//!   drifting apart.
//! - Dumps are written as version 2, which adds the `generation` and `start` fields a
//!   compaction-safe cursor needs. **Existing version-1 dumps still load**: `kc-log`'s
//!   reader accepts the `{"version": 1, "entries": [...]}` shape this module used to
//!   write, and a bare JSON array besides.

use std::io::{Read, Write};

use kc_core::KcResult;

use crate::store::InMemoryArchive;

impl InMemoryArchive {
    pub fn dump_json<W: Write>(&self, w: W) -> KcResult<()> {
        self.log().dump_json(w)
    }

    pub fn dump_json_to_string(&self) -> KcResult<String> {
        self.log().dump_json_to_string()
    }

    pub fn load_json<R: Read>(r: R) -> KcResult<Self> {
        Ok(Self::from_log(kc_log::Archive::load_json(r)?))
    }

    pub fn load_json_from_str(s: &str) -> KcResult<Self> {
        Self::load_json(s.as_bytes())
    }
}
