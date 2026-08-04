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
//! # Being the only writer (design **D30**)
//!
//! D20 left exclusion to the host, and measuring that decision is what overturned it.
//! Two writers on one store do not "fail loudly": the worst case is **silent**. A
//! second writer holding a stale cursor compacts, rewrites, and discards the first
//! writer's already-fsynced entries — every layer returning `Ok`, the reloaded log
//! well-formed, `verify_all` and `check_authority` both green, and the loser's cursor
//! still reading *current* so it never re-writes them. That is the D20 failure above,
//! reappearing across **writers** rather than across **cursors**.
//!
//! So the same argument that put the cursor here puts exclusion here too: the store
//! owns the cursor *because* a caller keeping one caused the bug, and "am I the only
//! writer of this file" is the same class of belief about shared storage. It is not an
//! operational concern a careful host can discharge — `SqliteStore` had SQLite's own
//! file locking underneath it and lost the entries identically.
//!
//! Two mechanisms, because neither reaches the other's case:
//!
//! - **A compare-and-swap.** Immediately before writing, the store reads where storage
//!   actually ends and refuses if that is not where it left it ([`check_not_forked`]).
//!   This catches a stale belief even when the two writers never overlap in time —
//!   which no lock can, since there is nothing to contend for.
//! - **An exclusive lock**, held while a write happens and optionally for the store's
//!   whole life ([`FileStore::claim`]). This closes the case a CAS cannot reach: two
//!   *fresh* stores, neither of which has a belief to check.
//!
//! # What is still the host's
//!
//! The **clock and the loop**. Nothing here calls `now()`, in keeping with the rest
//! of the tier's libraries; `en-host` decides *when* and the binary that drives it
//! supplies the time. Also the host's: where the file lives, key custody, and
//! process supervision — and, D30 having moved exclusion in here, the **policy** on a
//! refusal: whether to retry, back off, or exit is the host's call, exactly as *when*
//! to save is.
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

pub(crate) mod lock;

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
    ///
    /// **Refuses when storage is not where this store left it** (D30): another writer
    /// has been here, and both branches would then be wrong — appending forks the log,
    /// rewriting discards entries that are already durable. See
    /// [`reconcile`](ArchiveStore::reconcile) for a log already forked.
    fn save(&mut self, archive: &Archive<E>) -> KcResult<SaveOutcome>;

    /// Repair a log that two writers forked, and report what the repair did.
    ///
    /// Storage can hold a segment sequence that will not fold back together: two
    /// writers each appended their own continuation of the same log, so the second
    /// segment claims a position the first already took. Loading such a log fails —
    /// correctly, since folding them anyway would produce an archive that never
    /// existed — and `recover` cannot help, because the file is not *torn*, it is
    /// *forked*: every line is complete. Before D30 that left a polity's archive
    /// permanently unloadable, which is the worst state in the tier, the log being the
    /// evidence.
    ///
    /// This lifts the orphaned entries out and re-appends them at the end of the
    /// surviving log, then rewrites storage as a single base segment. **Nothing is
    /// forgotten and no CID changes** — see [`reconcile_segments`] for why, and for the
    /// one judgment call it makes.
    ///
    /// Safe on a healthy log: it reports `segments_orphaned: 0` and moves nothing.
    fn reconcile(&mut self) -> KcResult<Reconciled>;
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

/// Refuse a write when storage is not where this store last left it — the
/// compare-and-swap half of D30.
///
/// [`plan_save`] decides from `stored`, which is a *belief* about shared storage. This
/// checks the belief against storage itself, immediately before writing, and both
/// backends call it for the same reason they share `plan_save`: two backends with two
/// ideas of when a write is safe is the defect this crate exists to prevent.
///
/// What each refusal prevents, measured (D30):
///
/// - **A stale cursor** meant a second writer's compaction rewrite silently discarded
///   the first writer's fsynced entries. Every layer reported success, the reloaded
///   log was well-formed, and the loser's cursor still read *current*, so it never
///   re-wrote them. That is D20's own failure — a compaction losing signed records
///   quietly — reappearing across writers instead of across cursors.
/// - **Never having looked** meant two fresh stores could each rewrite the same path,
///   after which the outcome each `save` returned described a log that was not in
///   storage. A store may not overwrite a log it has never read; the rule stands on
///   its own terms, not as a patch.
pub(crate) fn check_not_forked(stored: Option<Cursor>, on_disk: Option<Cursor>) -> KcResult<()> {
    // Absent storage and an empty log in generation 0 are the same fact, and must
    // compare equal: `load` on a path that does not exist yet legitimately reports
    // having seen a zero-entry log, and a first run needs no special case anywhere in
    // this crate. Not normalizing here refused the reference host its founding save.
    const EMPTY: Cursor = Cursor { generation: 0, index: 0 };
    let disk = on_disk.unwrap_or(EMPTY);

    match stored {
        // Never looked, and nothing to miss.
        None if disk == EMPTY => Ok(()),
        None => Err(KcError::Archive(format!(
            "store: refusing to write a log this store has never read — storage \
             already holds {} entries at generation {}, and a rewrite would discard \
             them. Call load() first",
            disk.index, disk.generation
        ))),
        Some(c) if c == disk => Ok(()),
        // A distinct diagnosis: not another writer's log, but no log at all where this
        // store left one.
        Some(c) if on_disk.is_none() => Err(KcError::Archive(format!(
            "store: storage was emptied underneath this store — it last wrote \
             generation {} index {}, and storage now holds nothing. Something else is \
             writing here; call load() before saving again",
            c.generation, c.index
        ))),
        Some(c) => Err(KcError::Archive(format!(
            "store: another writer has been here — this store last wrote generation \
             {} index {}, but storage is at generation {} index {}. Refusing to \
             write: appending would fork the log and rewriting would discard entries \
             that are already durable. Call load() to pick up what is there, or \
             reconcile() if the log is already forked",
            c.generation, c.index, disk.generation, disk.index
        ))),
    }
}

/// What a [`reconcile`](ArchiveStore::reconcile) actually did.
///
/// Every field is here so the repair is *reported* rather than merely performed: this
/// is the log's own account of the one operation in the crate that changes the order
/// of entries already written.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reconciled {
    /// Trailing segments that would not apply where they claimed to.
    pub segments_orphaned: usize,
    /// Entries lifted out of those segments and re-appended at the end of the log.
    pub entries_moved: usize,
    /// Entries in those segments that the log already held, by CID, and so were
    /// already safe. A reconcile of an unforked log moves nothing and reports zero.
    pub entries_already_present: usize,
    /// The CIDs that moved, in the order they were re-appended.
    pub cids_moved: Vec<String>,
    /// Bytes written by the rewrite that leaves storage as a single base segment.
    pub bytes: usize,
}

impl Reconciled {
    /// Whether the log was forked at all. A healthy store reports `false`.
    pub fn was_forked(&self) -> bool {
        self.segments_orphaned > 0
    }
}

/// Fold stored segments into an archive, re-appending anything that will not apply
/// where it claims to — the shared half of [`ArchiveStore::reconcile`].
///
/// Both backends use it, so a repaired log is the same log whichever backend held it.
///
/// **Nothing is forgotten.** An entry's CID is the hash of its content
/// (`Entry::cid` = `Cid::of`, with no chaining), so an entry re-appended at a
/// different index keeps the same address and every receipt naming it still resolves —
/// which is what let D17 carry evidence rather than cite it. A duplicate is rejected
/// by CID, so a segment replayed twice moves nothing.
///
/// **The judgment call, stated rather than buried:** when two writers appended
/// concurrently, this decides the order between them, putting the orphaned writer's
/// entries after the surviving log's. Neither order is more correct — the writes were
/// concurrent — but a state folded in order can differ between them, so the repair
/// reports exactly which CIDs it moved.
pub(crate) fn reconcile_segments<E>(segments: Vec<String>) -> KcResult<(Archive<E>, Reconciled)>
where
    E: Entry + DeserializeOwned,
{
    let mut report = Reconciled::default();
    let mut iter = segments.into_iter();
    let Some(base) = iter.next() else {
        return Ok((Archive::new(), report));
    };
    let mut archive: Archive<E> = Archive::load_json_from_str(&base)?;

    // Once one segment has been orphaned, every later one is too: it was cut from a
    // log whose end has moved, so its claimed position means nothing either.
    let mut forked = false;
    for segment in iter {
        if !forked {
            match archive.append_json(&segment) {
                Ok(_) => continue,
                Err(_) => forked = true,
            }
        }
        report.segments_orphaned += 1;
        for entry in en_archive::segment_entries::<E>(&segment)? {
            match archive.append(entry) {
                Ok(cid) => {
                    report.entries_moved += 1;
                    report.cids_moved.push(cid.to_string());
                }
                // Already present under the same address, so already safe.
                Err(_) => report.entries_already_present += 1,
            }
        }
    }
    Ok((archive, report))
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
