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

//! The SQLite backend (feature `sqlite`): the same segments as rows.
//!
//! Stores exactly what [`FileStore`](crate::FileStore) stores — an ordered list of
//! segment envelopes — so the two are differentially testable and neither can drift
//! into its own idea of what a stored log is.
//!
//! **Where SQLite genuinely earns its place** is the compaction rewrite. The file
//! backend has to write a temporary file, fsync it, and rename it over the original
//! to make the replacement atomic; here it is `DELETE` then `INSERT` inside one
//! transaction, and the database does it. Everything else is the same protocol.
//!
//! **It earns its place a second time at D30.** [`FileStore`](crate::FileStore) needs a
//! `flock` sidecar to make "check where storage ends, then write" one critical section;
//! here the check goes inside the same `IMMEDIATE` transaction as the write, which is
//! a genuine atomic compare-and-swap with nothing bolted on. There is no `claim` and no
//! lock file, because there is nothing for them to add.
//!
//! What SQLite does **not** provide, and D30 measured it rather than assuming: its own
//! file locking prevented neither two-writer failure. Interleaved appends still forked
//! the segment sequence into a log that would not load, and a stale writer's compaction
//! still discarded another writer's committed entries with every call returning `Ok`.
//! Serializing writes is not the same as checking whether they belong.

use en_archive::{Archive, Cursor, Entry};
use en_core::{KcError, KcResult};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::de::DeserializeOwned;

use crate::{
    check_not_forked, plan_save, rebuild, reconcile_segments, whole_log, ArchiveStore, Reconciled,
    SavePlan, SaveOutcome,
};

fn db_err(what: &str, e: rusqlite::Error) -> KcError {
    KcError::Archive(format!("store: {what}: {e}"))
}

/// An archive persisted to a SQLite database, one segment per row.
pub struct SqliteStore {
    conn: Connection,
    stored: Option<Cursor>,
}

impl SqliteStore {
    /// Open (creating if needed) a store at `path`.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> KcResult<Self> {
        let conn = Connection::open(path).map_err(|e| db_err("open", e))?;
        Self::init(conn)
    }

    /// An in-memory store — for tests, and for a host that wants the same code path
    /// without a file.
    pub fn in_memory() -> KcResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| db_err("open", e))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> KcResult<Self> {
        // Full synchronous durability: this log is evidence, and a Concern can rest
        // on it. WAL so an append does not rewrite the whole page set.
        conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| db_err("pragma", e))?;
        conn.pragma_update(None, "synchronous", "FULL").map_err(|e| db_err("pragma", e))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS segments (
                 seq     INTEGER PRIMARY KEY AUTOINCREMENT,
                 payload TEXT NOT NULL
             );",
        )
        .map_err(|e| db_err("create schema", e))?;
        Ok(SqliteStore { conn, stored: None })
    }

    /// What the store believes it has persisted, if it has looked.
    pub fn cursor(&self) -> Option<Cursor> {
        self.stored
    }

    fn segments(&self) -> KcResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM segments ORDER BY seq")
            .map_err(|e| db_err("prepare", e))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| db_err("query", e))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| db_err("read row", e))
    }

    /// Append a raw segment row, bypassing the protocol — the only way to construct a
    /// forked database now that `save` refuses to create one, so the repair can be
    /// tested against a fork rather than against a healthy log.
    #[cfg(test)]
    pub(crate) fn insert_segment_for_test(&mut self, segment: &str) -> KcResult<()> {
        self.conn
            .execute("INSERT INTO segments (payload) VALUES (?1)", params![segment])
            .map_err(|e| db_err("insert segment", e))?;
        Ok(())
    }

    /// Where storage says the log ends, from the last row alone — D30's
    /// compare-and-swap read. One row, not the whole table.
    pub fn stored_end(&self) -> KcResult<Option<Cursor>> {
        Self::tail_end(&self.conn)
    }

    /// The same read, against a connection or an open transaction. Taking the handle as
    /// a parameter is what lets `save` do the check *inside* its transaction, which is
    /// the whole reason this backend needs no lock file.
    fn tail_end(conn: &Connection) -> KcResult<Option<Cursor>> {
        let row = conn.query_row(
            "SELECT payload FROM segments ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        );
        match row {
            Ok(line) => Ok(Some(en_archive::segment_bounds(&line)?.end())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err("read tail", e)),
        }
    }
}

impl<E: Entry + DeserializeOwned> ArchiveStore<E> for SqliteStore {
    fn load(&mut self) -> KcResult<Archive<E>> {
        let archive: Archive<E> = rebuild(self.segments()?.into_iter())?;
        self.stored = Some(archive.cursor());
        Ok(archive)
    }

    fn save(&mut self, archive: &Archive<E>) -> KcResult<SaveOutcome> {
        // IMMEDIATE, not deferred: the write lock is taken up front, so reading where
        // storage ends and writing to it are one atomic compare-and-swap rather than a
        // check followed by a hope. This is what the file backend needs a lock file to
        // achieve, and the reason this backend needs none.
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| db_err("begin", e))?;
        check_not_forked(self.stored, Self::tail_end(&tx)?)?;

        let outcome = match plan_save(self.stored, archive) {
            SavePlan::Nothing => SaveOutcome::noop(),
            SavePlan::Append(cursor) => {
                let segment = archive.dump_json_since(cursor)?;
                let appended = archive.len() - cursor.index;
                let bytes = segment.len();
                tx.execute("INSERT INTO segments (payload) VALUES (?1)", params![segment])
                    .map_err(|e| db_err("insert segment", e))?;
                SaveOutcome { appended, rewrote: false, bytes }
            }
            SavePlan::Rewrite => {
                let base = whole_log(archive)?;
                let bytes = base.len();
                // The other reason to use this backend: replacing the log needs no
                // temp-file dance, so there is no window in which storage holds neither
                // the old log nor the new one.
                tx.execute("DELETE FROM segments", [])
                    .map_err(|e| db_err("clear segments", e))?;
                tx.execute("INSERT INTO segments (payload) VALUES (?1)", params![base])
                    .map_err(|e| db_err("insert base", e))?;
                SaveOutcome { appended: archive.len(), rewrote: true, bytes }
            }
        };
        tx.commit().map_err(|e| db_err("commit", e))?;
        self.stored = Some(archive.cursor());
        Ok(outcome)
    }

    fn reconcile(&mut self) -> KcResult<Reconciled> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| db_err("begin", e))?;
        let segments = {
            let mut stmt = tx
                .prepare("SELECT payload FROM segments ORDER BY seq")
                .map_err(|e| db_err("prepare", e))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| db_err("query", e))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| db_err("read row", e))?
        };
        let (archive, mut report): (Archive<E>, _) = reconcile_segments(segments)?;
        // Leave storage as a single base segment, so the fork is gone from the record
        // of the log as well as from the log.
        let base = whole_log(&archive)?;
        report.bytes = base.len();
        tx.execute("DELETE FROM segments", []).map_err(|e| db_err("clear segments", e))?;
        tx.execute("INSERT INTO segments (payload) VALUES (?1)", params![base])
            .map_err(|e| db_err("insert base", e))?;
        tx.commit().map_err(|e| db_err("commit", e))?;
        self.stored = Some(archive.cursor());
        Ok(report)
    }
}
