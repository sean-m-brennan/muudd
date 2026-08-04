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

//! JSON dump/load for a whole archive, and the **incremental** pair a continuously
//! appended log actually needs.
//!
//! This mirrors `kc-archive`'s `dump.rs` one tier up, but generic over the entry
//! type as the rest of `en-archive` is (decision D1). Until now ethne had no
//! archive-level persistence at all — it existed only in the Python extension
//! module, which meant a Rust consumer could not persist a log and a Python one
//! re-serialized the *whole* log on every save.
//!
//! That whole-log shape is the wrong one for the observation log D16 introduced: a
//! host recording what its monitors saw appends steadily, and re-serializing the
//! entire archive after every observation is quadratic over a session. Hence
//! [`Archive::dump_json_since`] and [`Archive::append_json`] — keep a cursor, write
//! the segment past it, and fold segments back in on load.
//!
//! **A cursor is only valid within one generation** (design **D20**). Version 1 of
//! this module took a bare `usize` and treated an index past the end as "already
//! current", returning an empty segment. Combined with D17 pruning that was silent
//! corruption: after a compaction the host's index exceeds the new length, so the
//! save writes *nothing* — losing the `ObservationsPruned` record itself — while
//! earlier segments still hold the observations it dropped. Reloading resurrected
//! them, and because every CID the receipt named was back, the rebuilt log passed
//! both `verify_all` and `check_authority`. Worse, with the pruning record gone the
//! window's `pruned_before` clamp went with it and a measured absence *lengthened* —
//! the exact failure D17's clamp exists to prevent, re-entering through storage.
//!
//! So the envelope carries the `generation` and `start` a segment was cut at, and
//! both sides check them. Segments are written **compact** (they are machine-facing
//! and must be newline-free so a line-oriented store can hold one per line); the
//! whole-log dump stays pretty, because that is the one a human reads.
//!
//! The dump carries a `version`, so the format can move without silently
//! misreading old files. Loading re-`append`s every entry, which re-derives each
//! CID from the content: a tampered dump therefore lands under a *different*
//! address rather than impersonating the original.

use std::io::{Read, Write};

use en_core::{KcError, KcResult};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::store::{Archive, Cursor, Entry};

/// The current on-disk format version.
///
/// Bumped to 2 by D20, which added `generation` and `start`. Version 1 dumps still
/// *load* (they carry no generation, so they read as generation 0 — a whole-log
/// load, never a segment).
pub const DUMP_VERSION: u32 = 2;

/// The serialized envelope: a version, where in the log it was cut, and the entries.
#[derive(Serialize, Deserialize)]
struct ArchiveDump<E> {
    version: u32,
    /// The archive generation this was cut from. Absent in v1 dumps ⇒ 0.
    #[serde(default)]
    generation: u64,
    /// The index this segment starts at. Absent in v1 dumps ⇒ 0.
    #[serde(default)]
    start: usize,
    entries: Vec<E>,
}

/// Where a serialized segment sits in its log, read **without deserializing the
/// entries** — the envelope header alone.
///
/// A store needs this to answer one question cheaply: where does the log in storage
/// actually end? Reading the tail segment's header is O(tail); rebuilding the archive
/// to ask the same thing is O(whole log), which is the shape D20 already had to fix
/// once in `iter_from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentBounds {
    /// The archive generation this segment was cut from.
    pub generation: u64,
    /// The index it starts at. Zero for a whole-log base dump.
    pub start: usize,
    /// How many entries it carries.
    pub count: usize,
}

impl SegmentBounds {
    /// The cursor a log ends at once this segment has been applied.
    pub fn end(&self) -> Cursor {
        Cursor { generation: self.generation, index: self.start + self.count }
    }
}

/// Read a segment envelope's bounds without materializing its entries.
///
/// Deliberately in this module rather than in a caller: `dump.rs` owns the envelope
/// format, and a second reader of it elsewhere is two things that can disagree about
/// what a stored log means — the defect `en-store` exists to prevent, one level out.
///
/// Accepts everything [`Archive::load_json`] and [`Archive::append_json`] accept,
/// including the legacy bare array, so a store cannot refuse a file the archive
/// would have loaded.
pub fn segment_bounds(s: &str) -> KcResult<SegmentBounds> {
    // `IgnoredAny` tokenizes each entry and allocates nothing, so this costs a scan
    // of the segment's bytes and no `E` at all — which is the point: a store is
    // generic over the entry type and should not need one to find the end of a log.
    let dump = read_dump::<serde::de::IgnoredAny, _>(s.as_bytes())?;
    Ok(SegmentBounds {
        generation: dump.generation,
        start: dump.start,
        count: dump.entries.len(),
    })
}

/// The entries a segment carries, **with no positional check at all**.
///
/// Every other reader of a segment checks that it belongs where it claims to, because
/// folding one into the wrong log produces an archive that never existed. This one
/// exists for the single case where that check has already failed and the entries must
/// be recovered anyway: `en-store`'s repair of a log two writers forked. Getting the
/// entries out is exactly what lets the repair keep them instead of dropping them.
///
/// Use [`Archive::append_json`] for anything else.
pub fn segment_entries<E: DeserializeOwned>(s: &str) -> KcResult<Vec<E>> {
    Ok(read_dump::<E, _>(s.as_bytes())?.entries)
}

impl<E: Entry> Archive<E> {
    /// The whole log, as a versioned JSON envelope, written to `w`.
    pub fn dump_json<W: Write>(&self, w: W) -> KcResult<()> {
        self.dump_json_from_to(0, w, true)
    }

    /// The whole log, as a versioned JSON envelope.
    pub fn dump_json_to_string(&self) -> KcResult<String> {
        let mut buf = Vec::new();
        self.dump_json_from_to(0, &mut buf, true)?;
        String::from_utf8(buf).map_err(|e| KcError::Archive(format!("dump_json utf8: {e}")))
    }

    /// The entries after `cursor`, as a compact versioned envelope — the
    /// **incremental** dump.
    ///
    /// A host persisting an actively-appended log keeps the [`Cursor`] it last wrote,
    /// calls this with it, and appends the result to its own storage, so each save is
    /// proportional to what is *new* rather than to the whole log. When the cursor is
    /// already current the segment is empty, which is not an error — nothing has
    /// happened since.
    ///
    /// **It is an error for the cursor to be from another generation, or past the
    /// end.** Both mean the log was compacted underneath the host, and continuing to
    /// append segments from that point would lose whatever the compaction recorded
    /// while keeping what it dropped. The right response is a full rewrite, which is
    /// what `en-store` does on seeing this.
    pub fn dump_json_since(&self, cursor: Cursor) -> KcResult<String> {
        if cursor.generation != self.generation() {
            return Err(KcError::Archive(format!(
                "stale cursor: taken in generation {} but the log is at {} — it was \
                 compacted; rewrite in full rather than appending a segment",
                cursor.generation,
                self.generation()
            )));
        }
        if cursor.index > self.len() {
            return Err(KcError::Archive(format!(
                "stale cursor: index {} is past the end of a {}-entry log",
                cursor.index,
                self.len()
            )));
        }
        let mut buf = Vec::new();
        self.dump_json_from_to(cursor.index, &mut buf, false)?;
        String::from_utf8(buf).map_err(|e| KcError::Archive(format!("dump_json utf8: {e}")))
    }

    fn dump_json_from_to<W: Write>(&self, start: usize, w: W, pretty: bool) -> KcResult<()> {
        let entries: Vec<&E> = self.iter_from(start).map(|(_, e)| e).collect();
        let dump = ArchiveDump {
            version: DUMP_VERSION,
            generation: self.generation(),
            start,
            entries,
        };
        let out = if pretty {
            serde_json::to_writer_pretty(w, &dump)
        } else {
            serde_json::to_writer(w, &dump)
        };
        out.map_err(|e| KcError::Archive(format!("dump_json: {e}")))
    }
}

impl<E: Entry + DeserializeOwned> Archive<E> {
    /// Rebuild an archive from a whole-log dump, preserving append order and
    /// generation.
    ///
    /// Errors on a *segment* (one cut at a non-zero index): a segment is only
    /// meaningful applied to the log it was cut from, so loading one as if it were a
    /// whole log would silently produce a truncated archive.
    pub fn load_json<R: Read>(r: R) -> KcResult<Self> {
        let dump = read_dump(r)?;
        if dump.start != 0 {
            return Err(KcError::Archive(format!(
                "this is a segment starting at {}, not a whole log — load the base \
                 dump and apply segments with append_json",
                dump.start
            )));
        }
        let mut archive = Self::new();
        archive.set_generation(dump.generation);
        for entry in dump.entries {
            archive.append(entry)?;
        }
        Ok(archive)
    }

    /// Rebuild an archive from a dump string.
    pub fn load_json_from_str(s: &str) -> KcResult<Self> {
        Self::load_json(s.as_bytes())
    }

    /// Apply a segment produced by [`dump_json_since`] — the read side of
    /// incremental persistence. Returns how many entries were appended.
    ///
    /// **Strict on purpose.** The segment's generation must match this archive's and
    /// it must start exactly where this archive ends; otherwise the two are not the
    /// same log at the same point, and folding them together would produce an archive
    /// that never existed. Ordinary [`append`] rules still apply on top, so a segment
    /// replayed twice fails on the duplicate CID rather than doubling the log.
    ///
    /// To combine two *independent* logs into one view — a KC-side reader holding
    /// several communities' charters, say — use [`merge_json`] instead. That is a
    /// different operation and says so.
    ///
    /// [`dump_json_since`]: Archive::dump_json_since
    /// [`append`]: Archive::append
    /// [`merge_json`]: Archive::merge_json
    pub fn append_json(&mut self, s: &str) -> KcResult<usize> {
        let dump = read_dump::<E, _>(s.as_bytes())?;
        if dump.generation != self.generation() {
            return Err(KcError::Archive(format!(
                "segment is from generation {} but this log is at {} — it was \
                 compacted; reload from the base rather than applying this segment",
                dump.generation,
                self.generation()
            )));
        }
        if dump.start != self.len() {
            return Err(KcError::Archive(format!(
                "segment starts at {} but this log ends at {} — {}",
                dump.start,
                self.len(),
                if dump.start < self.len() { "already applied" } else { "a gap" }
            )));
        }
        let n = dump.entries.len();
        for entry in dump.entries {
            self.append(entry)?;
        }
        Ok(n)
    }

    /// Fold another log's entries into this one, ignoring generation and position —
    /// for building a combined *view* over independent logs, not for persistence.
    ///
    /// Duplicate CIDs are still rejected, so merging overlapping logs errors rather
    /// than quietly deduplicating; the archive stays content-addressed and
    /// append-once.
    pub fn merge_json(&mut self, s: &str) -> KcResult<usize> {
        let dump = read_dump::<E, _>(s.as_bytes())?;
        let n = dump.entries.len();
        for entry in dump.entries {
            self.append(entry)?;
        }
        Ok(n)
    }
}

/// Read a dump envelope, accepting the current version, version 1, or a bare JSON
/// array of entries.
///
/// The bare array is what ethne's Python surface emitted before this module
/// existed, and version 1 is what it wrote before D20 added the generation; reading
/// all three means dumps taken then still load. Both legacy shapes report
/// `generation: 0, start: 0` — a whole log, never a segment, which is the only thing
/// they can honestly claim to be. Only the current version is ever *written*.
fn read_dump<E: DeserializeOwned, R: Read>(mut r: R) -> KcResult<ArchiveDump<E>> {
    let mut raw = String::new();
    r.read_to_string(&mut raw).map_err(|e| KcError::Archive(format!("load_json read: {e}")))?;

    match serde_json::from_str::<ArchiveDump<E>>(&raw) {
        Ok(dump) => {
            if dump.version == 0 || dump.version > DUMP_VERSION {
                return Err(KcError::Archive(format!(
                    "unsupported archive version: {} (this build reads up to {DUMP_VERSION})",
                    dump.version
                )));
            }
            Ok(dump)
        }
        // Not an envelope — try the legacy bare array before giving up, and report
        // the *envelope* error if that fails too, since the envelope is the format
        // we document.
        Err(envelope_err) => match serde_json::from_str::<Vec<E>>(&raw) {
            Ok(entries) => Ok(ArchiveDump { version: DUMP_VERSION, generation: 0, start: 0, entries }),
            Err(_) => Err(KcError::Archive(format!("load_json: {envelope_err}"))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestRec {
        seq: u64,
        note: String,
    }

    impl Entry for TestRec {
        fn kind(&self) -> &'static str {
            "TestRec"
        }
    }

    fn rec(seq: u64) -> TestRec {
        TestRec { seq, note: format!("entry {seq}") }
    }

    fn filled(n: u64) -> Archive<TestRec> {
        let mut a = Archive::new();
        for i in 0..n {
            a.append(rec(i)).unwrap();
        }
        a
    }

    #[test]
    fn round_trips_the_whole_log_in_order() {
        let a = filled(4);
        let back: Archive<TestRec> = Archive::load_json_from_str(&a.dump_json_to_string().unwrap())
            .unwrap();
        assert_eq!(back.len(), 4);
        let seqs: Vec<u64> = back.iter().map(|(_, e)| e.seq).collect();
        assert_eq!(seqs, vec![0, 1, 2, 3]);
        // CIDs are content addresses, so a round trip lands on the same ones.
        let before: Vec<_> = a.iter().map(|(c, _)| *c).collect();
        let after: Vec<_> = back.iter().map(|(c, _)| *c).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn the_dump_carries_its_version() {
        let dumped = filled(1).dump_json_to_string().unwrap();
        assert!(dumped.contains("\"version\""));
        let bumped = dumped.replace("\"version\": 2", "\"version\": 99");
        let err = Archive::<TestRec>::load_json_from_str(&bumped).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("unsupported archive version")));
    }

    #[test]
    fn an_incremental_segment_carries_only_what_is_new() {
        let a = filled(5);
        // A host that has already persisted the first three appends only the rest.
        let segment = a.dump_json_since(Cursor { generation: 0, index: 3 }).unwrap();
        let mut restored: Archive<TestRec> =
            Archive::load_json_from_str(&a.dump_json_to_string().unwrap()).unwrap();
        assert_eq!(restored.len(), 5);

        let mut partial: Archive<TestRec> = filled(3);
        assert_eq!(partial.append_json(&segment).unwrap(), 2);
        assert_eq!(partial.len(), 5);
        let seqs: Vec<u64> = partial.iter().map(|(_, e)| e.seq).collect();
        assert_eq!(seqs, vec![0, 1, 2, 3, 4]);

        // Replaying a segment onto an already-current log is refused by position,
        // before the duplicate-CID rule is even reached.
        let err = restored.append_json(&segment).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("already applied")));
    }

    #[test]
    fn a_cursor_at_the_end_yields_an_empty_segment() {
        let a = filled(3);
        let segment = a.dump_json_since(a.cursor()).unwrap();
        let mut same: Archive<TestRec> = filled(3);
        assert_eq!(same.append_json(&segment).unwrap(), 0);
        assert_eq!(same.len(), 3);
    }

    #[test]
    fn a_cursor_past_the_end_is_refused_rather_than_read_as_current() {
        // **The regression test for D20.** Version 1 returned an empty segment here,
        // treating "past the end" as "already current". That is what made compaction
        // silently lossy: after a prune the host's index exceeds the new length, so
        // the save wrote nothing and the pruning record never reached storage, while
        // earlier segments still held the observations it dropped.
        let a = filled(3);
        let err = a.dump_json_since(Cursor { generation: 0, index: 99 }).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("past the end")));
    }

    #[test]
    fn a_cursor_from_before_a_compaction_is_refused() {
        // The same defect by its other route: an index that still *fits* but means a
        // different entry, because compaction removes from the middle.
        let a = filled(5);
        let compacted: Archive<TestRec> = {
            let mut out = Archive::next_generation(&a);
            for (_, e) in a.iter().skip(3) {
                out.append(e.clone()).unwrap();
            }
            out
        };
        assert_eq!(compacted.generation(), 1);
        assert_eq!(compacted.len(), 2);

        // The index is in range for the new log, so only the generation catches it.
        let stale = Cursor { generation: 0, index: 2 };
        let err = compacted.dump_json_since(stale).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("compacted")));

        // And the read side refuses it too, so neither half can be got wrong alone.
        let segment = compacted.dump_json_since(compacted.cursor()).unwrap();
        let mut old: Archive<TestRec> = filled(5);
        let err = old.append_json(&segment).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("generation")));
    }

    #[test]
    fn a_generation_survives_a_round_trip() {
        let a = filled(2);
        let mut next: Archive<TestRec> = Archive::next_generation(&a);
        next.append(rec(9)).unwrap();
        let back: Archive<TestRec> =
            Archive::load_json_from_str(&next.dump_json_to_string().unwrap()).unwrap();
        assert_eq!(back.generation(), 1, "a reloaded log knows it was compacted");
        assert_eq!(back.cursor(), next.cursor());
    }

    #[test]
    fn a_segment_will_not_load_as_a_whole_log() {
        let a = filled(4);
        let segment = a.dump_json_since(Cursor { generation: 0, index: 2 }).unwrap();
        let err = Archive::<TestRec>::load_json_from_str(&segment).unwrap_err();
        assert!(matches!(err, KcError::Archive(m) if m.contains("not a whole log")));
    }

    #[test]
    fn merging_independent_logs_is_its_own_operation() {
        // Folding two unrelated logs into one view is not a segment append, and says
        // so: `merge_json` ignores position, `append_json` refuses it.
        let a = filled(2);
        let mut other: Archive<TestRec> = Archive::new();
        other.append(rec(50)).unwrap();
        other.append(rec(51)).unwrap();
        let dump = other.dump_json_to_string().unwrap();

        let mut view: Archive<TestRec> = Archive::load_json_from_str(
            &a.dump_json_to_string().unwrap(),
        )
        .unwrap();
        assert!(view.append_json(&dump).is_err(), "not a segment of this log");
        assert_eq!(view.merge_json(&dump).unwrap(), 2);
        assert_eq!(view.len(), 4);
    }

    #[test]
    fn a_segment_is_compact_so_a_line_oriented_store_can_hold_one_per_line() {
        let a = filled(3);
        let segment = a.dump_json_since(Cursor { generation: 0, index: 1 }).unwrap();
        assert!(!segment.contains('\n'), "segments must be newline-free");
        // The whole-log dump stays pretty — that is the one a human reads.
        assert!(a.dump_json_to_string().unwrap().contains('\n'));
    }

    #[test]
    fn a_bare_array_still_loads() {
        // The shape ethne's Python surface emitted before this module existed.
        let legacy = serde_json::to_string(&vec![rec(0), rec(1)]).unwrap();
        let back: Archive<TestRec> = Archive::load_json_from_str(&legacy).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.iter().map(|(_, e)| e.seq).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn garbage_is_an_archive_error_not_a_panic() {
        let err = Archive::<TestRec>::load_json_from_str("{\"nope\":").unwrap_err();
        assert!(matches!(err, KcError::Archive(_)));
    }

    // --- the header-only readers D30 needs -------------------------------------

    #[test]
    fn segment_bounds_finds_the_end_without_the_entry_type() {
        let a = filled(6);
        let base = a.dump_json_to_string().unwrap();
        let b = segment_bounds(&base).unwrap();
        assert_eq!(b, SegmentBounds { generation: 0, start: 0, count: 6 });
        assert_eq!(b.end(), a.cursor(), "a whole log ends where the archive does");

        let mut grown = a.clone();
        grown.append(rec(100)).unwrap();
        grown.append(rec(101)).unwrap();
        let seg = grown.dump_json_since(a.cursor()).unwrap();
        let b = segment_bounds(&seg).unwrap();
        assert_eq!(b, SegmentBounds { generation: 0, start: 6, count: 2 });
        assert_eq!(b.end(), grown.cursor());
    }

    #[test]
    fn segment_bounds_carries_the_generation() {
        let a = filled(4);
        let mut compacted = Archive::next_generation(&a);
        for (_, e) in a.iter().skip(2) {
            compacted.append(e.clone()).unwrap();
        }
        let bounds = segment_bounds(&compacted.dump_json_to_string().unwrap()).unwrap();
        assert_eq!(bounds.generation, 1);
        assert_eq!(bounds.end(), Cursor { generation: 1, index: 2 });
    }

    #[test]
    fn segment_bounds_accepts_everything_a_load_accepts() {
        // A store must never refuse a file the archive itself would have loaded, so the
        // legacy bare array reads as generation 0 starting at 0, exactly as `read_dump`
        // treats it.
        let legacy = serde_json::to_string(&vec![rec(0), rec(1), rec(2)]).unwrap();
        assert_eq!(
            segment_bounds(&legacy).unwrap(),
            SegmentBounds { generation: 0, start: 0, count: 3 }
        );
        // And an unreadable version is refused here too, rather than read as some log.
        let future = "{\"version\":99,\"generation\":0,\"start\":0,\"entries\":[]}";
        assert!(segment_bounds(future).is_err());
        assert!(segment_bounds("{\"nope\":").is_err());
    }

    #[test]
    fn segment_entries_recovers_entries_a_segment_cannot_place() {
        // The repair path's need: a segment that will NOT apply anywhere must still give
        // up its entries, or they can only be dropped.
        let a = filled(3);
        let mut grown = a.clone();
        grown.append(rec(77)).unwrap();
        let seg = grown.dump_json_since(a.cursor()).unwrap();

        // Applying it to an empty log is refused — it starts at 3 and the log ends at 0.
        let mut empty: Archive<TestRec> = Archive::new();
        assert!(empty.append_json(&seg).is_err());

        // Reading its entries is not.
        let entries: Vec<TestRec> = segment_entries(&seg).unwrap();
        assert_eq!(entries, vec![rec(77)]);

        // And re-appending one keeps its CID, since a CID is the content's address:
        // this is what lets a repair move an entry without breaking a citation.
        let cid_before = *grown.iter().nth(3).unwrap().0;
        let cid_after = empty.append(entries[0].clone()).unwrap();
        assert_eq!(cid_before, cid_after);
    }
}
