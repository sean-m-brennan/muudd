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

//! The file backend: one line-oriented file, one segment envelope per line.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use en_archive::{Archive, Cursor, Entry};
use en_core::{KcError, KcResult};
use serde::de::DeserializeOwned;

use crate::{io_err, plan_save, rebuild, whole_log, ArchiveStore, SavePlan, SaveOutcome};

/// An archive persisted to a single file, one segment per line.
///
/// Durability, stated plainly because it is the whole reason to use this rather than
/// a hand-rolled `File::create`:
///
/// - **Append** writes one line and `fsync`s it. A crash mid-append can leave a
///   partial final line, which [`load`](ArchiveStore::load) reports as a *torn tail*
///   rather than skipping — see below.
/// - **Rewrite** (what a compaction needs) writes the whole log to `<path>.tmp`,
///   fsyncs it, and `rename`s it over the original. POSIX makes that atomic, so a
///   crash at any point leaves either the old complete log or the new one, never a
///   half-written file.
///
/// **A torn tail is named, not swallowed.** Silently dropping an unparseable last
/// line would discard a signed entry without saying so, which is precisely the shape
/// of failure this crate was written to stop. Load errors instead, and
/// [`recover`](FileStore::recover) truncates the partial line as a deliberate act.
/// The log may forget, but only by saying so.
pub struct FileStore {
    path: PathBuf,
    stored: Option<Cursor>,
}

impl FileStore {
    /// A store over `path`. Touches nothing until [`load`](ArchiveStore::load) or
    /// [`save`](ArchiveStore::save); a path that does not exist yet is a valid empty
    /// store, so a first run needs no special case.
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        FileStore { path: path.into(), stored: None }
    }

    /// Where this store writes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What the store believes it has persisted, if it has looked.
    pub fn cursor(&self) -> Option<Cursor> {
        self.stored
    }

    fn tmp_path(&self) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    }

    /// The stored lines, or `None` when the file does not exist.
    fn read_lines(&self) -> KcResult<Option<Vec<String>>> {
        let mut f = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_err("open", e)),
        };
        let mut raw = String::new();
        f.read_to_string(&mut raw).map_err(|e| io_err("read", e))?;
        if raw.is_empty() {
            return Ok(Some(Vec::new()));
        }
        // A complete file ends in a newline. Anything after the last one is a line
        // that never finished being written.
        let complete = raw.ends_with('\n');
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        if !complete {
            let torn = lines.pop().map(|l| l.len()).unwrap_or(0);
            return Err(KcError::Archive(format!(
                "store: torn tail: {torn} bytes of an unfinished final segment in {} \
                 — a write was interrupted; call recover() to drop it deliberately",
                self.path.display()
            )));
        }
        Ok(Some(lines))
    }

    /// Drop an unfinished final segment, returning the bytes discarded.
    ///
    /// A deliberate, reported act — the counterpart to loading refusing to do it
    /// quietly. Only ever removes a trailing partial line; a complete file is left
    /// untouched and reports zero.
    pub fn recover(&mut self) -> KcResult<usize> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(io_err("read", e)),
        };
        if raw.is_empty() || raw.ends_with('\n') {
            return Ok(0);
        }
        let keep = raw.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let dropped = raw.len() - keep;
        self.atomic_write(&raw[..keep])?;
        self.stored = None; // whatever we thought we had persisted is no longer true
        Ok(dropped)
    }

    /// Write `contents` to a temporary file, fsync, and rename over the target.
    fn atomic_write(&self, contents: &str) -> KcResult<()> {
        let tmp = self.tmp_path();
        {
            let mut f = File::create(&tmp).map_err(|e| io_err("create temp", e))?;
            f.write_all(contents.as_bytes()).map_err(|e| io_err("write temp", e))?;
            f.sync_all().map_err(|e| io_err("fsync temp", e))?;
        }
        fs::rename(&tmp, &self.path).map_err(|e| io_err("rename", e))?;
        // Fsync the directory too, or the rename itself may not survive a crash.
        if let Some(dir) = self.path.parent() {
            if let Ok(d) = File::open(dir) {
                let _ = d.sync_all();
            }
        }
        Ok(())
    }

    fn append_line(&self, line: &str) -> KcResult<usize> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| io_err("open for append", e))?;
        let bytes = line.len() + 1;
        f.write_all(line.as_bytes()).map_err(|e| io_err("append", e))?;
        f.write_all(b"\n").map_err(|e| io_err("append", e))?;
        f.sync_all().map_err(|e| io_err("fsync", e))?;
        Ok(bytes)
    }
}

impl<E: Entry + DeserializeOwned> ArchiveStore<E> for FileStore {
    fn load(&mut self) -> KcResult<Archive<E>> {
        let archive: Archive<E> = match self.read_lines()? {
            None => Archive::new(),
            Some(lines) => rebuild(lines.into_iter())?,
        };
        self.stored = Some(archive.cursor());
        Ok(archive)
    }

    fn save(&mut self, archive: &Archive<E>) -> KcResult<SaveOutcome> {
        match plan_save(self.stored, archive) {
            SavePlan::Nothing => Ok(SaveOutcome::noop()),
            SavePlan::Append(cursor) => {
                let segment = archive.dump_json_since(cursor)?;
                let appended = archive.len() - cursor.index;
                let bytes = self.append_line(&segment)?;
                self.stored = Some(archive.cursor());
                Ok(SaveOutcome { appended, rewrote: false, bytes })
            }
            SavePlan::Rewrite => {
                let base = whole_log(archive)?;
                let contents = format!("{base}\n");
                self.atomic_write(&contents)?;
                self.stored = Some(archive.cursor());
                Ok(SaveOutcome { appended: archive.len(), rewrote: true, bytes: contents.len() })
            }
        }
    }
}
