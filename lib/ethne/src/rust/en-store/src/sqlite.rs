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

use en_archive::{Archive, Cursor, Entry};
use en_core::{KcError, KcResult};
use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;

use crate::{plan_save, rebuild, whole_log, ArchiveStore, SavePlan, SaveOutcome};

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
}

impl<E: Entry + DeserializeOwned> ArchiveStore<E> for SqliteStore {
    fn load(&mut self) -> KcResult<Archive<E>> {
        let archive: Archive<E> = rebuild(self.segments()?.into_iter())?;
        self.stored = Some(archive.cursor());
        Ok(archive)
    }

    fn save(&mut self, archive: &Archive<E>) -> KcResult<SaveOutcome> {
        match plan_save(self.stored, archive) {
            SavePlan::Nothing => Ok(SaveOutcome::noop()),
            SavePlan::Append(cursor) => {
                let segment = archive.dump_json_since(cursor)?;
                let appended = archive.len() - cursor.index;
                let bytes = segment.len();
                self.conn
                    .execute("INSERT INTO segments (payload) VALUES (?1)", params![segment])
                    .map_err(|e| db_err("insert segment", e))?;
                self.stored = Some(archive.cursor());
                Ok(SaveOutcome { appended, rewrote: false, bytes })
            }
            SavePlan::Rewrite => {
                let base = whole_log(archive)?;
                let bytes = base.len();
                // The whole point of this backend: replacing the log is one
                // transaction, so there is no window in which storage holds neither
                // the old log nor the new one.
                let tx = self.conn.transaction().map_err(|e| db_err("begin", e))?;
                tx.execute("DELETE FROM segments", [])
                    .map_err(|e| db_err("clear segments", e))?;
                tx.execute("INSERT INTO segments (payload) VALUES (?1)", params![base])
                    .map_err(|e| db_err("insert base", e))?;
                tx.commit().map_err(|e| db_err("commit", e))?;
                self.stored = Some(archive.cursor());
                Ok(SaveOutcome { appended: archive.len(), rewrote: true, bytes })
            }
        }
    }
}
