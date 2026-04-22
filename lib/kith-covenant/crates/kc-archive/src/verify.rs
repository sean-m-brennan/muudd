//! Cross-archive verification helpers. Currently just re-exports `Archive::verify`
//! through a sweep helper. Placeholder for future integrity checks (e.g., every
//! intra-rite CID referenced by an inter-ceremony must be present in the archive).

use kc_core::KcResult;

use crate::store::{Archive, InMemoryArchive};

/// Verify every entry in the archive in insertion order. Fails fast on the
/// first invalid entry.
pub fn verify_all(archive: &InMemoryArchive) -> KcResult<()> {
    for (cid, _) in archive.iter() {
        archive.verify(cid)?;
    }
    Ok(())
}
