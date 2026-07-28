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
        assert_eq!(out.appended, if i == 0 { 1 } else { 1 });
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
    // Two writers is a host error, but it must fail loudly rather than interleave
    // segments into a log that never existed.
    let path = scratch("two-writers");
    let mut first = FileStore::new(&path);
    let mut a = filled(4);
    ArchiveStore::<Rec>::save(&mut first, &a).unwrap();

    let mut second = FileStore::new(&path);
    let _: Archive<Rec> = second.load().unwrap();

    a = compact(&a, 2);
    ArchiveStore::<Rec>::save(&mut first, &a).unwrap();

    // `second` still believes generation 0; saving the compacted log rewrites rather
    // than appending, so the file is never left holding both.
    let out = ArchiveStore::<Rec>::save(&mut second, &a).unwrap();
    assert!(out.rewrote);
    let mut reader = FileStore::new(&path);
    assert_eq!(seqs(&reader.load().unwrap()), vec![2, 3]);
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
        "\n{:>8} {:>10} {:>12} {:>10} {:>12}   {}",
        "entries", "append B", "append us", "rewrite B", "rewrite us", "ratio"
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
