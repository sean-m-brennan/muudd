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

//! Backend tests. The hazard D20 exists for is entirely about generations and
//! cursors, so these run over a local entry type; the end-to-end regression with a
//! real polity and a real signed pruning is in `test_store.py`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use en_archive::{Archive, Cursor, Entry};
use serde::{Deserialize, Serialize};

use crate::{ArchiveStore, FileStore, SaveOutcome};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Rec {
    seq: u64,
}

impl Entry for Rec {
    fn kind(&self) -> &'static str {
        "Rec"
    }
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A unique scratch path. No `tempfile` dependency for this.
fn scratch(name: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("en-store-{}-{}-{}", std::process::id(), n, name));
    let _ = std::fs::remove_file(&p);
    p
}

fn filled(n: u64) -> Archive<Rec> {
    let mut a = Archive::new();
    for i in 0..n {
        a.append(Rec { seq: i }).unwrap();
    }
    a
}

/// Drop entries before `keep`, as a compaction does — a new generation.
fn compact(a: &Archive<Rec>, keep: usize) -> Archive<Rec> {
    let mut out = Archive::next_generation(a);
    for (_, e) in a.iter().skip(keep) {
        out.append(e.clone()).unwrap();
    }
    out
}

fn seqs(a: &Archive<Rec>) -> Vec<u64> {
    a.iter().map(|(_, e)| e.seq).collect()
}

fn cids(a: &Archive<Rec>) -> Vec<String> {
    a.iter().map(|(c, _)| c.to_string()).collect()
}

// --- the core protocol -------------------------------------------------------

#[test]
fn an_absent_store_loads_as_an_empty_log() {
    let mut s = FileStore::new(scratch("absent"));
    let a: Archive<Rec> = s.load().unwrap();
    assert!(a.is_empty(), "a first run needs no special case");
}

#[test]
fn saves_append_and_a_reload_matches_the_live_log() {
    let path = scratch("append");
    let mut s = FileStore::new(&path);
    let mut a = filled(0);
    let mut total_appended = 0;

    for i in 0..10 {
        a.append(Rec { seq: i }).unwrap();
        let out = ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
        // The first save lays down the base line; every one after it appends, since
        // no compaction has happened.
        assert_eq!(out.rewrote, i == 0, "at i={i}");
        assert_eq!(out.appended, 1, "at i={i}");
        total_appended += out.appended;
    }
    assert_eq!(total_appended, 10);

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(cids(&back), cids(&a));
    assert_eq!(back.cursor(), a.cursor());
}

#[test]
fn saving_twice_with_nothing_new_writes_nothing() {
    let mut s = FileStore::new(scratch("noop"));
    let a = filled(3);
    assert!(ArchiveStore::<Rec>::save(&mut s, &a).unwrap().wrote_anything());
    assert_eq!(ArchiveStore::<Rec>::save(&mut s, &a).unwrap(), SaveOutcome::noop());
}

// --- the reason this crate exists --------------------------------------------

#[test]
fn a_compaction_is_rewritten_in_full_and_drops_what_it_dropped() {
    // **The D20 regression, in miniature.** Before this, the host appended a segment
    // per save and, at the compaction, wrote nothing — so storage kept every entry
    // the compaction removed and lost the record of the compaction itself.
    let path = scratch("compact");
    let mut s = FileStore::new(&path);

    let mut a = filled(0);
    for i in 0..8 {
        a.append(Rec { seq: i }).unwrap();
        ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    }

    let compacted = compact(&a, 5);
    assert_eq!(compacted.generation(), 1);
    assert_eq!(seqs(&compacted), vec![5, 6, 7]);

    let out = ArchiveStore::<Rec>::save(&mut s, &compacted).unwrap();
    assert!(out.rewrote, "a changed generation must force a full rewrite");
    assert_eq!(out.appended, 3);

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![5, 6, 7], "nothing resurrected");
    assert_eq!(back.generation(), 1, "and the log remembers it was compacted");
    assert_eq!(cids(&back), cids(&compacted));
}

#[test]
fn appending_after_a_compaction_still_reloads_correctly() {
    // The sequence that actually happens in a running host: prune, then keep going.
    let path = scratch("after-compact");
    let mut s = FileStore::new(&path);
    let mut a = filled(6);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

    a = compact(&a, 4);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    for i in 100..104 {
        a.append(Rec { seq: i }).unwrap();
        let out = ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
        assert!(!out.rewrote, "within a generation, appends stay appends");
    }

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![4, 5, 100, 101, 102, 103]);
    assert_eq!(back.generation(), 1);
}

#[test]
fn a_second_store_over_the_same_file_cannot_append_from_a_stale_cursor() {
    // What this test asserted before D30 is worth recording, because it passed for the
    // wrong reason for the whole of D20's life. Its comment claimed two writers "must
    // fail loudly", and its assertions were `save(..).unwrap()` and
    // `assert!(out.rewrote)` — i.e. it asserted the stale writer's rewrite SUCCEEDING.
    // It looked harmless only because both writers saved the *same* compacted archive,
    // so the overwrite happened to replace the log with identical content and nothing
    // was visibly lost. Give the two writers different content, as two live hosts have,
    // and the same code path is silent data loss.
    let path = scratch("two-writers");
    let mut first = FileStore::new(&path);
    let mut a = filled(4);
    ArchiveStore::<Rec>::save(&mut first, &a).unwrap();

    let mut second = FileStore::new(&path);
    let _: Archive<Rec> = second.load().unwrap();

    a = compact(&a, 2);
    ArchiveStore::<Rec>::save(&mut first, &a).unwrap();

    // `second` still believes generation 0 index 4, and storage is at generation 1
    // index 2. Both branches are wrong from here — an append forks the log, a rewrite
    // discards entries already durable — so the only answer is a refusal.
    let err = ArchiveStore::<Rec>::save(&mut second, &a).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("another writer has been here"), "{msg}");
    assert!(msg.contains("generation 0 index 4"), "names what it believed: {msg}");
    assert!(msg.contains("generation 1 index 2"), "names what is there: {msg}");

    // And the refusal changed nothing: the first writer's log is intact.
    let mut reader = FileStore::new(&path);
    assert_eq!(seqs(&reader.load().unwrap()), vec![2, 3]);

    // The way out is to look again, which is the only honest thing a stale writer can
    // do — and then it is current.
    let fresh: Archive<Rec> = second.load().unwrap();
    assert_eq!(seqs(&fresh), vec![2, 3]);
    let out = ArchiveStore::<Rec>::save(&mut second, &fresh).unwrap();
    assert!(!out.wrote_anything(), "nothing new to say");
}

// --- durability --------------------------------------------------------------

#[test]
fn a_torn_tail_is_reported_and_recovered_only_deliberately() {
    let path = scratch("torn");
    let mut s = FileStore::new(&path);
    let mut a = filled(3);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    a.append(Rec { seq: 99 }).unwrap();
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

    // Simulate a crash partway through the final append.
    let raw = std::fs::read_to_string(&path).unwrap();
    let cut = raw.len() - 12;
    std::fs::write(&path, &raw[..cut]).unwrap();

    let mut reader = FileStore::new(&path);
    let err = ArchiveStore::<Rec>::load(&mut reader).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("torn tail"), "silently dropping a signed entry is the bug: {msg}");

    // Recovery is an act, and it reports what it discarded.
    let dropped = reader.recover().unwrap();
    assert!(dropped > 0);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![0, 1, 2], "back to the last complete segment");
    assert_eq!(reader.recover().unwrap(), 0, "a complete file is left alone");
}

#[test]
fn an_interrupted_rewrite_leaves_the_previous_log_intact() {
    // The reason a rewrite goes via temp + rename. Nothing must be able to observe a
    // half-written log, so a leftover temp file is inert.
    let path = scratch("atomic");
    let mut s = FileStore::new(&path);
    let a = filled(5);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

    let mut tmp = path.clone().into_os_string();
    tmp.push(".tmp");
    std::fs::write(&tmp, "{\"version\":2,\"generation\":9,\"start\":0,\"entries\":[").unwrap();

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![0, 1, 2, 3, 4], "the crash debris is not the log");
    let _ = std::fs::remove_file(&tmp);
}

// --- the round-trip property --------------------------------------------------

/// A deterministic pseudo-random walk of appends and compactions. No `rand`
/// dependency, and no clock — the seed is fixed so a failure reproduces.
#[test]
fn any_sequence_of_appends_and_compactions_round_trips() {
    let path = scratch("property");
    let mut s = FileStore::new(&path);
    let mut a: Archive<Rec> = Archive::new();
    let mut seed: u64 = 0x5eed;
    let mut next_seq = 0u64;
    let mut compactions = 0;

    for step in 0..200 {
        // xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;

        if seed % 7 == 0 && a.len() > 2 {
            a = compact(&a, (seed as usize / 7) % a.len());
            compactions += 1;
        } else {
            a.append(Rec { seq: next_seq }).unwrap();
            next_seq += 1;
        }
        ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

        // Reload from scratch every few steps and demand exact agreement.
        if step % 5 == 0 {
            let mut reader = FileStore::new(&path);
            let back: Archive<Rec> = reader.load().unwrap();
            assert_eq!(cids(&back), cids(&a), "step {step}");
            assert_eq!(back.generation(), a.generation(), "step {step}");
            assert_eq!(back.cursor(), a.cursor(), "step {step}");
        }
    }
    assert!(compactions > 5, "the walk must actually compact ({compactions})");
}

// --- differential: the two backends must agree --------------------------------

#[cfg(feature = "sqlite")]
#[test]
fn both_backends_load_identical_archives_from_the_same_writes() {
    use crate::SqliteStore;

    let mut file = FileStore::new(scratch("differential"));
    let mut db = SqliteStore::in_memory().unwrap();
    let mut a: Archive<Rec> = Archive::new();
    let mut seed: u64 = 0xd1ff;
    let mut next_seq = 0u64;

    for _ in 0..60 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if seed % 6 == 0 && a.len() > 2 {
            a = compact(&a, (seed as usize / 6) % a.len());
        } else {
            a.append(Rec { seq: next_seq }).unwrap();
            next_seq += 1;
        }
        let f = ArchiveStore::<Rec>::save(&mut file, &a).unwrap();
        let d = ArchiveStore::<Rec>::save(&mut db, &a).unwrap();
        // Same protocol decisions, byte-for-byte the same payloads.
        assert_eq!(f.appended, d.appended);
        assert_eq!(f.rewrote, d.rewrote);
    }

    let from_file: Archive<Rec> = FileStore::new(file.path()).load().unwrap();
    let from_db: Archive<Rec> = db.load().unwrap();
    assert_eq!(cids(&from_file), cids(&from_db));
    assert_eq!(from_file.generation(), from_db.generation());
    assert_eq!(cids(&from_file), cids(&a));
}

#[cfg(feature = "sqlite")]
#[test]
fn the_sqlite_backend_survives_reopening() {
    use crate::SqliteStore;

    let path = scratch("sqlite-reopen");
    let mut a = filled(4);
    {
        let mut db = SqliteStore::open(&path).unwrap();
        ArchiveStore::<Rec>::save(&mut db, &a).unwrap();
        a = compact(&a, 2);
        a.append(Rec { seq: 42 }).unwrap();
        ArchiveStore::<Rec>::save(&mut db, &a).unwrap();
    }
    let mut db = SqliteStore::open(&path).unwrap();
    let back: Archive<Rec> = db.load().unwrap();
    assert_eq!(seqs(&back), vec![2, 3, 42]);
    assert_eq!(back.generation(), 1);
    for suffix in ["", "-wal", "-shm"] {
        let mut p = path.clone().into_os_string();
        p.push(suffix);
        let _ = std::fs::remove_file(p);
    }
}

// --- a stale cursor cannot be smuggled past the store -------------------------

#[test]
fn the_store_owns_the_cursor_so_a_caller_cannot_hold_a_stale_one() {
    let path = scratch("owns-cursor");
    let mut s = FileStore::new(&path);
    let a = filled(5);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    assert_eq!(s.cursor(), Some(Cursor { generation: 0, index: 5 }));

    let compacted = compact(&a, 3);
    ArchiveStore::<Rec>::save(&mut s, &compacted).unwrap();
    assert_eq!(s.cursor(), Some(Cursor { generation: 1, index: 2 }));
}

// --- D30: two writers ----------------------------------------------------------
//
// Every test in this section is one of the failures measured against the pre-D30 code,
// turned into the assertion that it cannot happen again. The outcomes are in the
// design record; what matters here is that each one was real.

#[test]
fn two_writers_appending_cannot_fork_the_log() {
    // MEASURED: both saves returned Ok, and the file became UNLOADABLE — "segment
    // starts at 5 but this log ends at 7". `recover()` reported 0 bytes, because the
    // file was not torn (every line was complete) but forked, so a polity's archive
    // was left permanently unreadable with no way back.
    let path = scratch("fork-appends");
    let mut seed = FileStore::new(&path);
    ArchiveStore::<Rec>::save(&mut seed, &filled(5)).unwrap();

    let mut host_a = FileStore::new(&path);
    let mut host_b = FileStore::new(&path);
    let la: Archive<Rec> = host_a.load().unwrap();
    let lb: Archive<Rec> = host_b.load().unwrap();
    assert_eq!(host_a.cursor(), host_b.cursor(), "both start from the same belief");

    // Each host does its own work, as two live hosts do.
    let mut wa = la.clone();
    wa.append(Rec { seq: 100 }).unwrap();
    wa.append(Rec { seq: 101 }).unwrap();
    let mut wb = lb.clone();
    wb.append(Rec { seq: 200 }).unwrap();

    ArchiveStore::<Rec>::save(&mut host_a, &wa).unwrap();
    let err = ArchiveStore::<Rec>::save(&mut host_b, &wb).unwrap_err();
    assert!(format!("{err}").contains("another writer has been here"), "{err}");

    // The point of the refusal: storage still loads, and holds exactly one writer's log.
    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![0, 1, 2, 3, 4, 100, 101]);
}

#[test]
fn a_stale_compaction_cannot_discard_committed_entries() {
    // MEASURED, and the one that mattered most because it was SILENT: A appended two
    // entries and fsynced them; B, which had never seen them, pruned and therefore
    // rewrote. B's save returned Ok(rewrote), the reload succeeded and was well-formed,
    // and A's entries were gone. Worse, A's cursor still read generation 0 index 7, so
    // A's next save reported `Nothing` and never re-wrote them. This is D20's own
    // failure — a compaction quietly losing signed records — across writers instead of
    // across cursors.
    let path = scratch("stale-compaction");
    let mut seed = FileStore::new(&path);
    ArchiveStore::<Rec>::save(&mut seed, &filled(5)).unwrap();

    let mut host_a = FileStore::new(&path);
    let mut host_b = FileStore::new(&path);
    let la: Archive<Rec> = host_a.load().unwrap();
    let lb: Archive<Rec> = host_b.load().unwrap();

    let mut wa = la.clone();
    wa.append(Rec { seq: 100 }).unwrap();
    wa.append(Rec { seq: 101 }).unwrap();
    ArchiveStore::<Rec>::save(&mut host_a, &wa).unwrap();

    // B prunes, which forces the rewrite branch — the one that replaces the whole file.
    let wb = compact(&lb, 2);
    let err = ArchiveStore::<Rec>::save(&mut host_b, &wb).unwrap_err();
    assert!(format!("{err}").contains("discard entries that are already durable"), "{err}");

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert!(seqs(&back).contains(&100), "the durable entries survive: {:?}", seqs(&back));
    assert!(seqs(&back).contains(&101));
}

#[test]
fn a_store_that_has_never_looked_refuses_to_overwrite_a_log() {
    // The case no compare-against-my-cursor can catch, because there is no cursor: a
    // store that has never loaded has no belief to check. Refusing on the *absence* of
    // a belief is what closes it, and the rule stands on its own — a store may not
    // overwrite a log it has never read.
    let path = scratch("never-looked");
    let mut first = FileStore::new(&path);
    ArchiveStore::<Rec>::save(&mut first, &filled(4)).unwrap();

    let mut fresh = FileStore::new(&path);
    let err = ArchiveStore::<Rec>::save(&mut fresh, &filled(2)).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("never read"), "{msg}");
    assert!(msg.contains("Call load() first"), "says what to do: {msg}");

    let mut reader = FileStore::new(&path);
    assert_eq!(seqs(&reader.load().unwrap()), vec![0, 1, 2, 3]);
}

#[test]
fn a_first_run_that_looked_before_writing_is_not_a_fork() {
    // The reference host caught this one, which is the case for having a reference host:
    // `load` on a path that does not exist yet legitimately reports a zero-entry log,
    // and the compare-and-swap must read that as agreeing with absent storage rather
    // than as storage having been emptied. Getting it wrong refused a polity its
    // founding save — the crate's stated promise that a first run needs no special case.
    let path = scratch("first-run");
    let mut s = FileStore::new(&path);
    let empty: Archive<Rec> = s.load().unwrap();
    assert!(empty.is_empty());
    assert_eq!(s.cursor(), Some(Cursor { generation: 0, index: 0 }));
    assert_eq!(s.stored_end().unwrap(), None, "and storage really is absent");

    let mut a = empty;
    a.append(Rec { seq: 0 }).unwrap();
    let out = ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    assert!(out.wrote_anything());

    // The emptying it must NOT be confused with: a store that had written a real log
    // and comes back to find nothing.
    let mut a = filled(4);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    std::fs::remove_file(&path).unwrap();
    a.append(Rec { seq: 40 }).unwrap();
    let err = ArchiveStore::<Rec>::save(&mut s, &a).unwrap_err();
    assert!(format!("{err}").contains("emptied underneath"), "{err}");
}

#[test]
fn concurrent_rewrites_do_not_share_a_temp_file() {
    // MEASURED: with a fixed `<path>.tmp`, two rewrites opened the same temporary file,
    // both wrote into it, and raced the rename. The winner's save returned
    // Err(rename: ENOENT) while its bytes were what storage held; the loser's returned
    // Ok with a byte count for a log that was gone. Both reports were wrong, in
    // opposite directions, and storage loaded cleanly so nothing found out.
    //
    // The assertion is therefore not "one of them wins" but "whoever reports success
    // describes what is actually there".
    let path = scratch("concurrent-rewrite");
    let fat: Archive<Rec> = {
        let mut a = Archive::new();
        for i in 0..2000 {
            a.append(Rec { seq: 900_000 + i }).unwrap();
        }
        a
    };
    let thin = filled(2000);

    let pa = path.clone();
    let pb = path.clone();
    let t1 = std::thread::spawn(move || ArchiveStore::<Rec>::save(&mut FileStore::new(&pa), &fat));
    let t2 = std::thread::spawn(move || ArchiveStore::<Rec>::save(&mut FileStore::new(&pb), &thin));
    let ra = t1.join().unwrap();
    let rb = t2.join().unwrap();

    // Exactly one may succeed: the other is refused, either by the lock or — if it got
    // there first — by having nothing to say about a log it never read.
    assert!(ra.is_ok() != rb.is_ok(), "one writer, not two: {ra:?} / {rb:?}");
    let winner = ra.as_ref().or(rb.as_ref()).unwrap();

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(back.len(), 2000);
    assert_eq!(
        std::fs::metadata(&path).unwrap().len() as usize,
        winner.bytes,
        "the successful save must describe what storage actually holds"
    );

    // No litter: the temporary name is unique per call and cleaned up either way.
    let dir = path.parent().unwrap();
    let stem = path.file_name().unwrap().to_string_lossy().to_string();
    let leftovers: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(&format!("{stem}.tmp")))
        .collect();
    assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
}

#[test]
fn a_claimed_store_refuses_a_second_writer() {
    // A host that claims at boot finds out then, rather than at its first save — which
    // is the refusal a supervisor can act on.
    let path = scratch("claimed");
    let mut host = FileStore::new(&path);
    host.claim().unwrap();
    assert!(host.is_claimed());
    host.claim().unwrap(); // idempotent
    ArchiveStore::<Rec>::save(&mut host, &filled(3)).unwrap();

    let mut second = FileStore::new(&path);
    let err = second.claim().unwrap_err();
    assert!(format!("{err}").contains("another writer holds"), "{err}");

    // A reader is never blocked by a claim — the log is evidence, and reading it must
    // not depend on who is writing.
    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![0, 1, 2]);

    // Releasing hands the log over without the holder having to exit.
    host.release();
    assert!(!host.is_claimed());
    second.claim().unwrap();
}

#[test]
fn a_forked_log_reconciles_without_forgetting_an_entry() {
    // The repair for a log already forked — from two writers before D30, or from a
    // restored backup, or an rsync. Build the fork by hand, since the store now refuses
    // to create one.
    let path = scratch("reconcile");
    let mut seed = FileStore::new(&path);
    let base = filled(5);
    ArchiveStore::<Rec>::save(&mut seed, &base).unwrap();

    let mut wa = base.clone();
    wa.append(Rec { seq: 100 }).unwrap();
    wa.append(Rec { seq: 101 }).unwrap();
    let mut wb = base.clone();
    wb.append(Rec { seq: 200 }).unwrap();

    let seg_a = wa.dump_json_since(base.cursor()).unwrap();
    let seg_b = wb.dump_json_since(base.cursor()).unwrap();
    let mut forked = std::fs::read_to_string(&path).unwrap();
    forked.push_str(&seg_a);
    forked.push('\n');
    forked.push_str(&seg_b);
    forked.push('\n');
    std::fs::write(&path, &forked).unwrap();

    // Confirm the fork is what a load actually cannot survive, so the repair is not
    // being demonstrated against a healthy log.
    let mut broken = FileStore::new(&path);
    let err = ArchiveStore::<Rec>::load(&mut broken).unwrap_err();
    assert!(format!("{err}").contains("already applied"), "{err}");
    assert_eq!(broken.recover().unwrap(), 0, "recover knows torn tails, not forked ones");

    let mut store = FileStore::new(&path);
    let report = ArchiveStore::<Rec>::reconcile(&mut store).unwrap();
    assert!(report.was_forked());
    assert_eq!(report.segments_orphaned, 1);
    assert_eq!(report.entries_moved, 1, "the orphaned writer's one entry");
    assert_eq!(report.entries_already_present, 0);
    assert!(report.bytes > 0);

    // NOTHING IS FORGOTTEN, and no CID changed — the entries are content-addressed, so
    // a receipt naming any of them still resolves.
    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(seqs(&back), vec![0, 1, 2, 3, 4, 100, 101, 200]);
    for cid in cids(&wa).iter().chain(cids(&wb).iter()) {
        assert!(cids(&back).contains(cid), "a CID went missing: {cid}");
    }
    assert_eq!(report.cids_moved, cids(&wb)[5..].to_vec(), "and it says which moved");

    // The log is one base segment again, so the fork is gone from the record too, and
    // the store can be written normally from here.
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
    let mut more = back.clone();
    more.append(Rec { seq: 300 }).unwrap();
    assert!(ArchiveStore::<Rec>::save(&mut store, &more).unwrap().wrote_anything());
}

#[test]
fn reconciling_a_healthy_log_moves_nothing() {
    let path = scratch("reconcile-healthy");
    let mut s = FileStore::new(&path);
    let mut a = filled(4);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    a.append(Rec { seq: 9 }).unwrap();
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

    let report = ArchiveStore::<Rec>::reconcile(&mut s).unwrap();
    assert!(!report.was_forked());
    assert_eq!(report.entries_moved, 0);
    assert!(report.cids_moved.is_empty());

    let mut reader = FileStore::new(&path);
    let back: Archive<Rec> = reader.load().unwrap();
    assert_eq!(cids(&back), cids(&a), "CID for CID, order included");
}

#[test]
fn reconciling_is_idempotent() {
    // A segment replayed twice moves nothing: the duplicate is rejected by CID.
    let path = scratch("reconcile-twice");
    let mut seed = FileStore::new(&path);
    let base = filled(3);
    ArchiveStore::<Rec>::save(&mut seed, &base).unwrap();
    let mut wb = base.clone();
    wb.append(Rec { seq: 77 }).unwrap();
    let seg = wb.dump_json_since(base.cursor()).unwrap();

    let mut raw = std::fs::read_to_string(&path).unwrap();
    raw.push_str(&format!("{seg}\n{seg}\n"));
    std::fs::write(&path, &raw).unwrap();

    let mut store = FileStore::new(&path);
    let report = ArchiveStore::<Rec>::reconcile(&mut store).unwrap();
    // The first copy applies where it claims to; the second is the orphan, and every
    // entry in it is already held under the same CID. So nothing MOVES — the whole of
    // the duplicate segment is recognised rather than re-appended.
    assert_eq!(report.segments_orphaned, 1);
    assert_eq!(report.entries_moved, 0);
    assert_eq!(report.entries_already_present, 1, "the replay is recognised, not doubled");
    let mut reader = FileStore::new(&path);
    assert_eq!(seqs(&reader.load().unwrap()), vec![0, 1, 2, 77]);

    let again = ArchiveStore::<Rec>::reconcile(&mut store).unwrap();
    assert!(!again.was_forked());
}

#[test]
fn the_tail_read_finds_the_end_without_loading_the_log() {
    let path = scratch("tail");
    let mut s = FileStore::new(&path);
    assert_eq!(s.stored_end().unwrap(), None, "an absent store ends nowhere");

    let mut a = filled(5);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    assert_eq!(s.stored_end().unwrap(), Some(Cursor { generation: 0, index: 5 }));

    a.append(Rec { seq: 50 }).unwrap();
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
    assert_eq!(s.stored_end().unwrap(), Some(Cursor { generation: 0, index: 6 }));

    let compacted = compact(&a, 4);
    ArchiveStore::<Rec>::save(&mut s, &compacted).unwrap();
    assert_eq!(s.stored_end().unwrap(), Some(Cursor { generation: 1, index: 2 }));

    // And it agrees with the store's own cursor at every step, which is the property
    // the compare-and-swap rests on.
    assert_eq!(s.stored_end().unwrap(), s.cursor());
}

#[test]
fn the_tail_read_crosses_a_chunk_boundary() {
    // The backward scan reads 8 KiB at a time; a segment longer than that must still
    // yield its header, or a busy log would silently read as ending in the wrong place.
    let path = scratch("tail-big");
    let mut s = FileStore::new(&path);
    let a = filled(3);
    ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

    let mut big = a.clone();
    for i in 0..1_500 {
        big.append(Rec { seq: 1_000 + i }).unwrap();
    }
    let out = ArchiveStore::<Rec>::save(&mut s, &big).unwrap();
    assert!(out.bytes > 8 * 1024, "the tail segment must exceed one chunk ({})", out.bytes);
    assert_eq!(s.stored_end().unwrap(), Some(Cursor { generation: 0, index: 1_503 }));
}

#[test]
fn a_torn_tail_is_named_by_the_tail_read_too() {
    // One voice for an interrupted write, wherever it is noticed.
    let path = scratch("tail-torn");
    let mut s = FileStore::new(&path);
    ArchiveStore::<Rec>::save(&mut s, &filled(3)).unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, &raw[..raw.len() - 10]).unwrap();

    let err = FileStore::new(&path).stored_end().unwrap_err();
    assert!(format!("{err}").contains("torn tail"), "{err}");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_refuses_the_same_two_writers() {
    // MEASURED before D30, and the finding that shaped the slice: SQLite's own file
    // locking prevented NEITHER failure. Interleaved appends still forked the segment
    // sequence into a log that would not load, and a stale writer's compaction still
    // discarded another writer's committed entries with every call returning Ok. So the
    // fix could not be a lock — it had to be the protocol, in the layer both backends
    // share.
    use crate::SqliteStore;

    // Interleaved appends.
    let path = scratch("sqlite-fork.db");
    {
        let mut seed = SqliteStore::open(&path).unwrap();
        ArchiveStore::<Rec>::save(&mut seed, &filled(5)).unwrap();
        let mut a = SqliteStore::open(&path).unwrap();
        let mut b = SqliteStore::open(&path).unwrap();
        let la: Archive<Rec> = a.load().unwrap();
        let lb: Archive<Rec> = b.load().unwrap();
        let mut wa = la.clone();
        wa.append(Rec { seq: 100 }).unwrap();
        let mut wb = lb.clone();
        wb.append(Rec { seq: 200 }).unwrap();
        ArchiveStore::<Rec>::save(&mut a, &wa).unwrap();
        let err = ArchiveStore::<Rec>::save(&mut b, &wb).unwrap_err();
        assert!(format!("{err}").contains("another writer has been here"), "{err}");
        let mut r = SqliteStore::open(&path).unwrap();
        let back: Archive<Rec> = r.load().unwrap();
        assert_eq!(seqs(&back), vec![0, 1, 2, 3, 4, 100], "still loads");
    }

    // A stale compaction.
    let path2 = scratch("sqlite-stale.db");
    {
        let mut seed = SqliteStore::open(&path2).unwrap();
        ArchiveStore::<Rec>::save(&mut seed, &filled(5)).unwrap();
        let mut a = SqliteStore::open(&path2).unwrap();
        let mut b = SqliteStore::open(&path2).unwrap();
        let la: Archive<Rec> = a.load().unwrap();
        let lb: Archive<Rec> = b.load().unwrap();
        let mut wa = la.clone();
        wa.append(Rec { seq: 100 }).unwrap();
        ArchiveStore::<Rec>::save(&mut a, &wa).unwrap();
        let wb = compact(&lb, 2);
        let err = ArchiveStore::<Rec>::save(&mut b, &wb).unwrap_err();
        assert!(format!("{err}").contains("already durable"), "{err}");
        let mut r = SqliteStore::open(&path2).unwrap();
        let back: Archive<Rec> = r.load().unwrap();
        assert!(seqs(&back).contains(&100), "{:?}", seqs(&back));
    }

    for p in [&path, &path2] {
        for suffix in ["", "-wal", "-shm"] {
            let mut q = p.clone().into_os_string();
            q.push(suffix);
            let _ = std::fs::remove_file(q);
        }
    }
}

#[cfg(feature = "sqlite")]
#[test]
fn both_backends_refuse_and_repair_identically() {
    // The differential property, extended to D30: a fork looks the same to both
    // backends and repairs to the same log, so neither can drift into its own idea of
    // when a write is safe.
    use crate::SqliteStore;

    let file_path = scratch("differential-fork");
    let db_path = scratch("differential-fork.db");

    let base = filled(4);
    let mut wb = base.clone();
    wb.append(Rec { seq: 55 }).unwrap();
    let orphan = wb.dump_json_since(base.cursor()).unwrap();

    // Same fork, built into each backend's storage in its own terms.
    {
        let mut f = FileStore::new(&file_path);
        ArchiveStore::<Rec>::save(&mut f, &base).unwrap();
        let mut raw = std::fs::read_to_string(&file_path).unwrap();
        raw.push_str(&format!("{orphan}\n{orphan}\n"));
        std::fs::write(&file_path, raw).unwrap();
    }
    let mut db = SqliteStore::open(&db_path).unwrap();
    ArchiveStore::<Rec>::save(&mut db, &base).unwrap();
    db.insert_segment_for_test(&orphan).unwrap();
    db.insert_segment_for_test(&orphan).unwrap();

    let mut f = FileStore::new(&file_path);
    let fr = ArchiveStore::<Rec>::reconcile(&mut f).unwrap();
    let dr = ArchiveStore::<Rec>::reconcile(&mut db).unwrap();
    assert_eq!(fr.segments_orphaned, dr.segments_orphaned);
    assert_eq!(fr.entries_moved, dr.entries_moved);
    assert_eq!(fr.entries_already_present, dr.entries_already_present);
    assert_eq!(fr.cids_moved, dr.cids_moved);

    let from_file: Archive<Rec> = FileStore::new(&file_path).load().unwrap();
    let from_db: Archive<Rec> = db.load().unwrap();
    assert_eq!(cids(&from_file), cids(&from_db));
    assert_eq!(f.stored_end().unwrap(), db.stored_end().unwrap());

    for suffix in ["", "-wal", "-shm"] {
        let mut q = db_path.clone().into_os_string();
        q.push(suffix);
        let _ = std::fs::remove_file(q);
    }
}

// --- measurement ---------------------------------------------------------------

/// Append vs full-rewrite cost, both backends. `#[ignore]`d so it never slows the
/// suite; run it with:
///
/// ```text
/// cargo test -p en-store --features sqlite -- --ignored --nocapture measure
/// ```
///
/// `Instant` here is a stopwatch over I/O, not domain logic — the no-clock rule is
/// about libraries deciding things from the time, and nothing is decided here.
#[test]
#[ignore]
fn measure_append_versus_rewrite() {
    use std::time::Instant;

    println!(
        "\n{:>8} {:>10} {:>12} {:>10} {:>12}   ratio",
        "entries", "append B", "append us", "rewrite B", "rewrite us"
    );
    for n in [100u64, 1_000, 5_000] {
        let mut a = filled(n);
        let mut s = FileStore::new(scratch(&format!("bench-{n}")));
        ArchiveStore::<Rec>::save(&mut s, &a).unwrap();

        a.append(Rec { seq: n }).unwrap();
        let t = Instant::now();
        let app = ArchiveStore::<Rec>::save(&mut s, &a).unwrap();
        let app_us = t.elapsed().as_micros();

        // Force the rewrite branch the way a compaction does.
        let compacted = compact(&a, 0);
        let t = Instant::now();
        let rew = ArchiveStore::<Rec>::save(&mut s, &compacted).unwrap();
        let rew_us = t.elapsed().as_micros();

        println!(
            "{:>8} {:>10} {:>12} {:>10} {:>12}   {:.0}x",
            n,
            app.bytes,
            app_us,
            rew.bytes,
            rew_us,
            rew.bytes as f64 / app.bytes as f64
        );
        let _ = std::fs::remove_file(s.path());
    }

    #[cfg(feature = "sqlite")]
    {
        use crate::SqliteStore;
        println!("\n  sqlite backend:");
        for n in [100u64, 1_000, 5_000] {
            let mut a = filled(n);
            let mut db = SqliteStore::in_memory().unwrap();
            ArchiveStore::<Rec>::save(&mut db, &a).unwrap();

            a.append(Rec { seq: n }).unwrap();
            let t = Instant::now();
            let app = ArchiveStore::<Rec>::save(&mut db, &a).unwrap();
            let app_us = t.elapsed().as_micros();

            let compacted = compact(&a, 0);
            let t = Instant::now();
            let rew = ArchiveStore::<Rec>::save(&mut db, &compacted).unwrap();
            let rew_us = t.elapsed().as_micros();
            println!(
                "{:>8} {:>10} {:>12} {:>10} {:>12}",
                n, app.bytes, app_us, rew.bytes, rew_us
            );
        }
    }
}
