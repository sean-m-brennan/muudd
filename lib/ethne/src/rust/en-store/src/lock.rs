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

//! The write lock for a file-backed store (design **D30**).
//!
//! # Why a sidecar and not the store file
//!
//! A compaction replaces the log by writing a temporary file and `rename`ing it over
//! the original, which **swaps the inode**. A lock taken on the store file would
//! therefore end up held on a file that no longer has a name, protecting nothing,
//! while the next writer locks the new inode and sees no contention. So the lock lives
//! on `<path>.lock`, which is created once and never renamed.
//!
//! A leftover `.lock` file is not a stale lock: `flock` is released by the kernel when
//! the descriptor closes, **including when the process dies**, so there is nothing to
//! reap and no holder PID to second-guess. That is the whole reason to use `flock`
//! rather than an exclusive-create lock file.
//!
//! # Non-blocking, and why
//!
//! `LOCK_NB` always. A store is called from the host's loop, and the tier's rule for
//! anything on that loop is that it does not block (`en_at::source` states it for
//! event sources). A refusal is a named error, and what to do about it — retry, back
//! off, exit — is the host's policy, which is the same line D20 drew around the clock.
//!
//! # Reading is never locked
//!
//! [`load`](crate::ArchiveStore::load) takes no lock at all. Its only hazard is
//! reading during an append, and that is already a *named* error — the torn tail —
//! while a rewrite's `rename` gives a reader the old log or the new one and never a
//! mixture. Locking reads would mean a writer's [`claim`](crate::FileStore::claim)
//! could block a reader, and a reader must always be able to read the evidence.
//!
//! # The honest limit
//!
//! `flock` is advisory and POSIX-only. On a non-unix target the lock is a no-op and
//! the compare-and-swap in `check_not_forked` carries alone; the same is true on a
//! filesystem whose `flock` does not work across hosts. The CAS is what keeps the
//! *log* correct in those conditions — what is lost is only the early, clean refusal.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use en_core::{KcError, KcResult};

/// The lock file beside a store.
pub(crate) fn lock_path(path: &Path) -> PathBuf {
    let mut s = path.to_path_buf().into_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// An exclusive write lock over a store, released when dropped.
///
/// Held for the duration of one `save` or `reconcile`, or for a whole store's life
/// when a host [`claim`](crate::FileStore::claim)s it.
#[derive(Debug)]
pub(crate) struct WriteLock {
    // Kept solely for its `Drop`: closing the descriptor releases the lock.
    #[allow(dead_code)]
    file: File,
}

impl WriteLock {
    /// Take the exclusive lock, or report who has it.
    ///
    /// The directory must exist; the store file itself need not.
    pub(crate) fn acquire(store_path: &Path) -> KcResult<Self> {
        let lp = lock_path(store_path);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lp)
            .map_err(|e| {
                KcError::Archive(format!("store: cannot open the lock at {}: {e}", lp.display()))
            })?;
        try_lock(&file, store_path)?;
        Ok(WriteLock { file })
    }
}

#[cfg(unix)]
fn try_lock(file: &File, store_path: &Path) -> KcResult<()> {
    use std::os::unix::io::AsRawFd;

    // SAFETY: `file` is an open descriptor for as long as this call runs, and the
    // flags are the two documented constants.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
        return Err(KcError::Archive(format!(
            "store: another writer holds {} — refusing to write. Only one process may \
             write a polity's log; if no host is running, the lock is already gone \
             (flock is released when the holder dies) and this means one is",
            store_path.display()
        )));
    }
    Err(KcError::Archive(format!(
        "store: cannot lock {}: {err}",
        store_path.display()
    )))
}

/// On a target without `flock` the lock is a no-op, as the module docs state. The
/// compare-and-swap still refuses a forked write; what is missing is the early
/// refusal.
#[cfg(not(unix))]
fn try_lock(_file: &File, _store_path: &Path) -> KcResult<()> {
    Ok(())
}
