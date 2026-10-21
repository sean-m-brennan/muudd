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
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use en_archive::{Archive, Cursor, Entry};
use en_core::{KcError, KcResult};
use serde::de::DeserializeOwned;

use crate::lock::WriteLock;
use crate::{
    check_not_forked, io_err, plan_save, rebuild, reconcile_segments, whole_log, ArchiveStore,
    Reconciled, SavePlan, SaveOutcome,
};

/// Distinguishes this store's temporary file from every other store's, so two
/// rewrites cannot write into the same one. See [`FileStore::tmp_path`].
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

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
///
/// **Only one writer** (D30). Every write takes an exclusive lock on `<path>.lock` and
/// checks that storage is still where this store left it, refusing if it is not; a
/// host that wants to find out at boot rather than at its first save calls
/// [`claim`](FileStore::claim). Reading is never locked. See [`crate::lock`] for the
/// mechanism and its one honest limit.
pub struct FileStore {
    path: PathBuf,
    stored: Option<Cursor>,
    /// A lock held for this store's whole life, once `claim`ed. Writes still take the
    /// lock when this is `None`, for the duration of the write.
    claim: Option<WriteLock>,
}

impl FileStore {
    /// A store over `path`. Touches nothing until [`load`](ArchiveStore::load) or
    /// [`save`](ArchiveStore::save); a path that does not exist yet is a valid empty
    /// store, so a first run needs no special case.
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        FileStore { path: path.into(), stored: None, claim: None }
    }

    /// Take the write lock and hold it for this store's lifetime, so a second writer
    /// is refused **now** rather than at its first save.
    ///
    /// A host should call this at startup: a supervisor that has restarted a host
    /// while the old one is still alive, or a one-shot invocation run alongside a
    /// following loop, is the ordinary way two writers arrive on one log, and a
    /// refusal at boot is one a supervisor can act on. Writes are guarded either way —
    /// this only moves the discovery earlier.
    ///
    /// Idempotent. Released when the store is dropped, which includes the process
    /// dying, so nothing has to be cleaned up.
    pub fn claim(&mut self) -> KcResult<()> {
        if self.claim.is_none() {
            self.claim = Some(WriteLock::acquire(&self.path)?);
        }
        Ok(())
    }

    /// Whether this store is holding a lifetime write lock.
    pub fn is_claimed(&self) -> bool {
        self.claim.is_some()
    }

    /// Give up a lifetime [`claim`](FileStore::claim), leaving individual writes still
    /// guarded.
    ///
    /// For a process that is done writing but still wants to read, and does not want
    /// to make a second host wait on its own exit.
    pub fn release(&mut self) {
        self.claim = None;
    }

    /// Where storage says the log ends, read from the tail alone.
    ///
    /// The public half of D30's compare-and-swap: a host can ask whether storage has
    /// moved under it without loading the log. `None` means storage is absent or
    /// empty.
    ///
    /// Costs a read of the last segment, not of the log — the same distinction D20
    /// already had to fix once, when `iter().skip(start)` made an append proportional
    /// to the whole log.
    pub fn stored_end(&self) -> KcResult<Option<Cursor>> {
        let Some(line) = self.tail_line()? else {
            return Ok(None);
        };
        Ok(Some(en_archive::segment_bounds(&line)?.end()))
    }

    /// The last complete line of the file, scanning backward from the end.
    ///
    /// Returns the torn-tail error for an unfinished final line, exactly as
    /// [`read_lines`](FileStore::read_lines) does — a write interrupted mid-append is
    /// reported in one voice wherever it is noticed.
    fn tail_line(&self) -> KcResult<Option<String>> {
        const CHUNK: usize = 8 * 1024;

        let mut f = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_err("open", e)),
        };
        let len = f.metadata().map_err(|e| io_err("stat", e))?.len();
        if len == 0 {
            return Ok(None);
        }

        // Walk backward a chunk at a time until two newlines are in hand (the
        // terminator of the last line, and the one before it that starts it) or the
        // file is exhausted.
        let mut tail: Vec<u8> = Vec::new();
        let mut back = 0u64;
        loop {
            let step = CHUNK.min((len - back) as usize);
            back += step as u64;
            f.seek(SeekFrom::Start(len - back)).map_err(|e| io_err("seek", e))?;
            let mut buf = vec![0u8; step];
            f.read_exact(&mut buf).map_err(|e| io_err("read tail", e))?;
            buf.extend_from_slice(&tail);
            tail = buf;
            if back == len || tail.iter().filter(|b| **b == b'\n').count() >= 2 {
                break;
            }
        }

        let text = String::from_utf8(tail)
            .map_err(|e| KcError::Archive(format!("store: tail is not utf8: {e}")))?;
        if !text.ends_with('\n') {
            let torn = text.rsplit('\n').next().map(str::len).unwrap_or(0);
            return Err(KcError::Archive(format!(
                "store: torn tail: {torn} bytes of an unfinished final segment in {} \
                 — a write was interrupted; call recover() to drop it deliberately",
                self.path.display()
            )));
        }
        Ok(text.trim_end_matches('\n').rsplit('\n').next().map(str::to_string))
    }

    /// Hold the write lock for one operation, reusing a lifetime claim if there is one.
    fn write_lock(&self) -> KcResult<Option<WriteLock>> {
        if self.claim.is_some() {
            return Ok(None); // already held, for this store's whole life
        }
        Ok(Some(WriteLock::acquire(&self.path)?))
    }

    /// Refuse the write if storage is not where this store left it.
    fn check_current(&self) -> KcResult<()> {
        check_not_forked(self.stored, self.stored_end()?)
    }

    /// Where this store writes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What the store believes it has persisted, if it has looked.
    pub fn cursor(&self) -> Option<Cursor> {
        self.stored
    }

    /// A temporary path **unique to this call**, not a fixed `<path>.tmp`.
    ///
    /// D30 measured what the fixed name cost: two rewrites of one store opened the same
    /// temporary file, both truncating and writing into it, and then raced the rename.
    /// The winner's `save` returned `Err(rename: ENOENT)` while its bytes were what
    /// storage held, and the loser's returned `Ok` with a byte count for a log that was
    /// gone. Both outcome reports were wrong, in opposite directions, and the file
    /// loaded cleanly so nothing downstream ever found out. The lock makes that race
    /// unreachable within a process group; a distinct name means it cannot corrupt
    /// anything even where the lock is a no-op.
    fn tmp_path(&self) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(format!(".tmp.{}.{}", std::process::id(), TMP_SEQ.fetch_add(1, Ordering::SeqCst)));
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
        let _lock = self.write_lock()?;
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
        let written = (|| -> KcResult<()> {
            let mut f = File::create(&tmp).map_err(|e| io_err("create temp", e))?;
            f.write_all(contents.as_bytes()).map_err(|e| io_err("write temp", e))?;
            f.sync_all().map_err(|e| io_err("fsync temp", e))?;
            Ok(())
        })();
        // The temporary name is unique per call, so a failed write leaves litter rather
        // than something the next attempt would collide with — clear it anyway.
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        if let Err(e) = fs::rename(&tmp, &self.path) {
            let _ = fs::remove_file(&tmp);
            return Err(io_err("rename", e));
        }
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
        // Lock first, then look: reading where storage ends and writing must be one
        // critical section, or the check is only a narrower window than no check.
        let _lock = self.write_lock()?;
        self.check_current()?;
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

    fn reconcile(&mut self) -> KcResult<Reconciled> {
        let _lock = self.write_lock()?;
        let lines = self.read_lines()?.unwrap_or_default();
        let (archive, mut report): (Archive<E>, _) = reconcile_segments(lines)?;
        // Leave storage as a single base segment, so the fork is gone from the record
        // of the log as well as from the log.
        let contents = format!("{}\n", whole_log(&archive)?);
        self.atomic_write(&contents)?;
        report.bytes = contents.len();
        self.stored = Some(archive.cursor());
        Ok(report)
    }
}
