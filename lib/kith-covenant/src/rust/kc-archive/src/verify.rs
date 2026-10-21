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

//! Cross-archive verification helpers. Currently just re-exports `Archive::verify`
//! through a sweep helper. Placeholder for future integrity checks (e.g., every
//! intra-rite CID referenced by an inter-ceremony must be present in the archive).

use kc_core::KcResult;

use crate::store::{Archive, InMemoryArchive};

/// Verify every entry in the archive in insertion order. Fails fast on the
/// first invalid entry.
pub fn verify_all(archive: &InMemoryArchive) -> KcResult<()> {
    for (cid, _) in archive.iter() {
        archive.verify(cid)?;
    }
    Ok(())
}
