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

//! Durable storage for an ethne log — the backend D17 and D19 both left owed
//! (design **D20**).
//!
//! # Why this is the tier's job and not the host's
//!
//! D17 shipped an incremental save protocol (keep a cursor, write the segment past
//! it) and D17/D19 shipped compaction (`apply_prunings` returns a *new* archive with
//! observations removed). **Those two do not compose, and getting it wrong is
//! silent.** After a compaction the host's cursor is meaningless — entries went from
//! the middle, so the same index is a different entry, and an index past the new end
//! read as "already current". A host doing the obvious thing therefore wrote *no*
//! segment for the compaction, losing the `ObservationsPruned` record, while its
//! earlier segments still held every observation that record dropped. Reloading
//! resurrected them; the rebuilt log passed `verify_all` and `check_authority`,
//! because every CID the receipt named was back; and with the pruning record gone
//! the window's clamp went too, so a measured absence *lengthened* — pruning
//! manufacturing a Concern, which is the failure D17's clamp exists to prevent.
//!
//! So the branch that matters is one line, and this crate exists to own it:
//!
//! > **same generation ⇒ append a segment; generation changed ⇒ rewrite in full.**
//!
//! The store holds the cursor, so a host cannot hold a stale one.
//!
//! # What is still the host's
//!
//! The **clock and the loop**. Nothing here calls `now()`, in keeping with the rest
//! of the tier's libraries; `en-host` decides *when* and the binary that drives it
//! supplies the time. Also the host's: where the file lives, key custody, and
//! process supervision.
//!
//! # Backends
//!
//! Both store the same thing — an ordered list of segment envelopes, exactly as
//! [`Archive::dump_json_since`] emits them — and differ only in how a write is made
//! durable. That is deliberate: it keeps the interesting logic in one place and makes
//! the two backends differentially testable against each other.
//!
//! - [`FileStore`] — one line-oriented file, one segment per line. Append is
//!   `write` + `fsync`; a full rewrite goes to a temporary file, is fsynced, and is
//!   `rename`d into place, which POSIX makes atomic.
//! - [`SqliteStore`] (feature `sqlite`) — the same segments as rows. Append and
//!   rewrite are both single transactions, which is where SQLite genuinely earns its
//!   place: the rewrite needs no temp-file dance.

use en_archive::{Archive, Cursor, Entry};
use en_core::{KcError, KcResult};
use serde::de::DeserializeOwned;

mod file;
pub use file::FileStore;

#[cfg(test)]
mod tests;

#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;

/// What a [`save`](ArchiveStore::save) actually did.
///
/// `rewrote` is the one worth watching: it means the log was compacted and the whole
/// thing went back to storage. A host logging these sees its compactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveOutcome {
    /// Entries newly written (the whole log, when `rewrote`).
    pub appended: usize,
    /// Whether this was a full rewrite rather than an append.
    pub rewrote: bool,
    /// Bytes written.
    pub bytes: usize,
}

impl SaveOutcome {
    /// Nothing had changed since the last save.
    pub fn noop() -> Self {
        SaveOutcome { appended: 0, rewrote: false, bytes: 0 }
    }

    /// Whether anything reached storage.
    pub fn wrote_anything(&self) -> bool {
        self.bytes > 0
    }
}

/// Durable storage for an [`Archive`], generic over the entry type as the rest of
/// the archive machinery is (decision D1).
///
/// Implementations own the cursor. That is the point: the compaction hazard above is
/// exactly what happens when a *caller* keeps one.
pub trait ArchiveStore<E: Entry + DeserializeOwned> {
    /// Read the whole log back, at the generation it was stored in.
    ///
    /// An absent store is an empty archive, not an error — a host starting fresh
    /// should not have to special-case its first run.
    fn load(&mut self) -> KcResult<Archive<E>>;

    /// Persist whatever is not yet stored.
    ///
    /// Appends a segment when the archive is in the generation this store last saw,
    /// and rewrites in full when it is not — because a changed generation means
    /// entries were removed, and appending past that point would keep what the
    /// compaction dropped while losing what it recorded.
    fn save(&mut self, archive: &Archive<E>) -> KcResult<SaveOutcome>;
}

/// Rebuild an archive from stored segments: the first is the base, the rest are
/// applied in order.
///
/// Shared by both backends, so they cannot disagree about what a stored log means.
/// The strictness of `append_json` — generation and position both checked — is doing
/// the work here: a segment sequence that was written wrongly fails to load rather
/// than loading as some archive that never existed.
pub(crate) fn rebuild<E, I>(mut segments: I) -> KcResult<Archive<E>>
where
    E: Entry + DeserializeOwned,
    I: Iterator<Item = String>,
{
    let Some(base) = segments.next() else {
        return Ok(Archive::new());
    };
    let mut archive = Archive::load_json_from_str(&base)?;
    for segment in segments {
        archive.append_json(&segment)?;
    }
    Ok(archive)
}

/// The whole log as one compact segment envelope — the form a base record takes.
///
/// A whole log is just a segment starting at zero, so there is one serialization
/// path rather than two that could drift.
pub(crate) fn whole_log<E: Entry>(archive: &Archive<E>) -> KcResult<String> {
    archive.dump_json_since(Cursor { generation: archive.generation(), index: 0 })
}

/// Decide what `save` must do, given what the store last persisted.
///
/// Split out so the decision is testable without touching a disk, and so both
/// backends make it identically. **This is the whole of D20's storage knowledge.**
pub(crate) fn plan_save<E: Entry>(stored: Option<Cursor>, archive: &Archive<E>) -> SavePlan {
    match stored {
        // Never persisted: everything goes.
        None => SavePlan::Rewrite,
        // Compacted underneath us — entries were removed, so no cursor into the old
        // log means anything. The only safe answer is the whole thing.
        Some(c) if c.generation != archive.generation() => SavePlan::Rewrite,
        // A cursor past the end within one generation should be impossible (a
        // generation is append-only by construction), but if it happens the log is
        // not what this store thinks it is, and guessing is what caused D20.
        Some(c) if c.index > archive.len() => SavePlan::Rewrite,
        Some(c) if c.index == archive.len() => SavePlan::Nothing,
        Some(c) => SavePlan::Append(c),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SavePlan {
    Nothing,
    Append(Cursor),
    Rewrite,
}

/// Map an I/O failure into the archive error the rest of the tier speaks.
pub(crate) fn io_err(what: &str, e: std::io::Error) -> KcError {
    KcError::Archive(format!("store: {what}: {e}"))
}

#[cfg(test)]
mod plan_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestRec {
        seq: u64,
    }

    impl Entry for TestRec {
        fn kind(&self) -> &'static str {
            "TestRec"
        }
    }

    fn filled(n: u64) -> Archive<TestRec> {
        let mut a = Archive::new();
        for i in 0..n {
            a.append(TestRec { seq: i }).unwrap();
        }
        a
    }

    #[test]
    fn the_save_decision_is_the_whole_of_it() {
        let a = filled(5);
        assert_eq!(plan_save(None, &a), SavePlan::Rewrite, "never persisted");
        assert_eq!(plan_save(Some(a.cursor()), &a), SavePlan::Nothing, "already current");
        assert_eq!(
            plan_save(Some(Cursor { generation: 0, index: 2 }), &a),
            SavePlan::Append(Cursor { generation: 0, index: 2 }),
        );
        // The case D20 exists for: the log was compacted, so nothing may be appended.
        assert_eq!(
            plan_save(Some(Cursor { generation: 0, index: 2 }), &Archive::<TestRec>::next_generation(&a)),
            SavePlan::Rewrite,
        );
        // And the shouldn't-happen case still refuses to guess.
        assert_eq!(plan_save(Some(Cursor { generation: 0, index: 99 }), &a), SavePlan::Rewrite);
    }

    #[test]
    fn a_whole_log_is_a_segment_from_zero() {
        let a = filled(3);
        let base = whole_log(&a).unwrap();
        let back: Archive<TestRec> = Archive::load_json_from_str(&base).unwrap();
        assert_eq!(back.cursor(), a.cursor());
        assert!(!base.contains('\n'), "must fit one line");
    }

    #[test]
    fn rebuilding_no_segments_is_an_empty_log() {
        let back: Archive<TestRec> = rebuild(std::iter::empty()).unwrap();
        assert!(back.is_empty());
    }
}
