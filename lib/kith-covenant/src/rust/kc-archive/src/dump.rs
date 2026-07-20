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

use std::io::{Read, Write};

use kc_core::{KcError, KcResult};
use serde::{Deserialize, Serialize};

use crate::entry::ArchiveEntry;
use crate::store::{Archive, InMemoryArchive};

#[derive(Serialize, Deserialize)]
struct ArchiveDump {
    version: u32,
    entries: Vec<ArchiveEntry>,
}

impl InMemoryArchive {
    pub fn dump_json<W: Write>(&self, w: W) -> KcResult<()> {
        let dump = ArchiveDump {
            version: 1,
            entries: self.iter().map(|(_, e)| e.clone()).collect(),
        };
        serde_json::to_writer_pretty(w, &dump)
            .map_err(|e| KcError::Archive(format!("dump_json: {e}")))
    }

    pub fn dump_json_to_string(&self) -> KcResult<String> {
        let mut buf = Vec::new();
        self.dump_json(&mut buf)?;
        String::from_utf8(buf).map_err(|e| KcError::Archive(format!("dump_json utf8: {e}")))
    }

    pub fn load_json<R: Read>(r: R) -> KcResult<Self> {
        let dump: ArchiveDump = serde_json::from_reader(r)
            .map_err(|e| KcError::Archive(format!("load_json: {e}")))?;
        if dump.version != 1 {
            return Err(KcError::Archive(format!(
                "unsupported archive version: {}",
                dump.version
            )));
        }
        let mut archive = Self::new();
        for entry in dump.entries {
            archive.append(entry)?;
        }
        Ok(archive)
    }

    pub fn load_json_from_str(s: &str) -> KcResult<Self> {
        Self::load_json(s.as_bytes())
    }
}
