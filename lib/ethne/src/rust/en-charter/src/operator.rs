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

//! The operator's own signing key — the guardian's half of a `GuardianChange`
//! (design **D15**).
//!
//! D15 requires a named guardian to co-sign the edge that makes a machine answerable
//! to it: answerability is accepted, never assigned. AT's operator-key slice gave that
//! guardian an *identity* (an ed25519 key bound to the operator's PIV credential,
//! carried to ethne as `MemberCandidate.guardian`), and this module is how the
//! matching signature is obtained.
//!
//! **Two custody paths, and the difference matters.** This one is the *co-located*
//! path: the operator runs the authoring step on their own machine, so the key can be
//! read from their keystore. It is convenient and it puts the private key in this
//! process's memory. The other path is detached signing (see
//! [`crate::detached`]), where the payload travels to the operator instead of the key
//! travelling to the archive; that one is correct for every arrangement where the
//! authoring host is not the operator's own.
//!
//! **The key belongs to the operator.** AT deliberately stores it outside the node's
//! config directory — one key per operator, stable across every node that human
//! guards, so a node that could read it could impersonate that human everywhere else
//! (ethne **D24**: per-node keys would let one person present as N guardians). The
//! path resolution here mirrors AT's `operator_key_path` exactly, and the file's
//! permissions are checked rather than assumed.

use std::fs;
use std::path::{Path, PathBuf};

use en_core::{Alg, DidKey, KcError, KcResult, PrivKey};

/// The file AT persists the operator's ed25519 key in.
const KEY_FILE: &str = "operator_ed25519.key";

/// Where the operator's key lives, mirroring AT's `operator_key_path`:
/// `$AT_OPERATOR_KEYSTORE`, else `$XDG_CONFIG_HOME/at-operator`, else
/// `~/.config/at-operator`.
///
/// An explicit `keystore_dir` overrides all of it, which is what tests and a
/// multi-operator host use.
pub fn operator_key_path(keystore_dir: Option<&Path>) -> PathBuf {
    let base = match keystore_dir {
        Some(d) => d.to_path_buf(),
        None => match std::env::var_os("AT_OPERATOR_KEYSTORE") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => {
                let cfg = std::env::var_os("XDG_CONFIG_HOME")
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        let home = std::env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|| PathBuf::from("."));
                        home.join(".config")
                    });
                cfg.join("at-operator")
            }
        },
    };
    base.join(KEY_FILE)
}

/// Load the operator's signing key, as `(did:key, private key)`.
///
/// AT writes a hex-encoded 32-byte ed25519 seed; `en_core::keys` signs from exactly
/// that representation. The two libraries' agreement is not assumed — it is pinned as
/// a cross-library test vector in [`crate::cross_library_key_join`].
///
/// Refuses a key file that anyone but its owner can read. A guardian key readable by
/// the group is a guardian key that is no longer only the guardian's, and D15's whole
/// content is that the signature means a *particular human* accepted answerability.
pub fn operator_signer(keystore_dir: Option<&Path>) -> KcResult<(DidKey, PrivKey)> {
    let path = operator_key_path(keystore_dir);
    let display = path.display();

    let hex = fs::read(&path).map_err(|e| {
        KcError::Key(format!(
            "cannot read the operator key at {display}: {e}. AT writes it when an \
             operator binds one (`--bind-operator-key`); this path can be overridden \
             with AT_OPERATOR_KEYSTORE"
        ))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path)
            .map_err(|e| KcError::Key(format!("cannot stat {display}: {e}")))?
            .permissions()
            .mode()
            & 0o777;
        if mode & 0o077 != 0 {
            return Err(KcError::Key(format!(
                "the operator key at {display} is mode {mode:04o}; it must not be \
                 readable by anyone else (expected 0600). Fix with: chmod 600 {display}"
            )));
        }
    }

    let seed = decode_hex_seed(&hex, &format!("{display}"))?;
    let sk = PrivKey { alg: Alg::Ed25519, bytes: seed };
    let pk = en_core::keys::public_of(&sk)?;
    let did = DidKey::from_pubkey(&pk)?;
    Ok((did, sk))
}

/// Load the operator's key and confirm it is the guardian the polity's substrate
/// actually reported.
///
/// The failure this prevents is an operator with several keys — or a fleet whose
/// binding was rotated — co-signing with the wrong one. Without the check the record
/// would be *authored* happily and refused by `check_authority` later, naming a signer
/// mismatch rather than the thing that is actually wrong.
pub fn guardian_signer_for(
    expected: &DidKey,
    keystore_dir: Option<&Path>,
) -> KcResult<(DidKey, PrivKey)> {
    let (did, sk) = operator_signer(keystore_dir)?;
    if &did != expected {
        return Err(KcError::Key(format!(
            "the operator key at {} is {did}, but the substrate reports this member's \
             guardian as {expected}. A guardian co-signs with the key its node was \
             bound to; check AT_OPERATOR_KEYSTORE, or rotate the edge if the operator's \
             key really changed",
            operator_key_path(keystore_dir).display()
        )));
    }
    Ok((did, sk))
}

/// AT stores the seed hex-encoded, "to match how every other key in this tree is
/// persisted". Whitespace-tolerant because the file is written with a trailing
/// newline by some tools and without by AT.
fn decode_hex_seed(raw: &[u8], where_: &str) -> KcResult<Vec<u8>> {
    let text = std::str::from_utf8(raw)
        .map_err(|_| KcError::Key(format!("{where_} is not text; expected a hex-encoded seed")))?
        .trim();
    if text.len() != 64 {
        return Err(KcError::Key(format!(
            "{where_} holds {} hex characters; an ed25519 seed is 64 (32 bytes)",
            text.len()
        )));
    }
    (0..64)
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| {
                KcError::Key(format!("{where_} is not valid hex at character {i}"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_core::keys;

    /// The fixture AT would have written: a hex seed, 0600, in a 0700 directory.
    fn keystore(dir: &Path, seed_hex: &str, mode: u32) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(KEY_FILE);
        fs::write(&path, seed_hex).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        }
        path
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("en-at-operator-{name}"));
        let _ = fs::remove_dir_all(&d);
        d
    }

    const SEED_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn a_stored_seed_loads_and_signs() {
        let dir = tmp("loads");
        keystore(&dir, SEED_HEX, 0o600);
        let (did, sk) = operator_signer(Some(&dir)).expect("the key loads");
        assert!(did.as_str().starts_with("did:key:z6Mk"));
        // The loaded key really signs for the DID it derived.
        let sig = keys::sign(&sk, b"co-sign this").unwrap();
        let pk = keys::public_of(&sk).unwrap();
        keys::verify(&pk, b"co-sign this", &sig).unwrap();
        assert_eq!(DidKey::from_pubkey(&pk).unwrap(), did);
    }

    #[test]
    fn a_group_readable_key_is_refused() {
        let dir = tmp("mode");
        keystore(&dir, SEED_HEX, 0o644);
        let err = operator_signer(Some(&dir)).expect_err("0644 must be refused");
        let msg = err.to_string();
        // The diagnostic has to name the file and the mode, and say what to do.
        assert!(msg.contains("0644"), "must name the mode: {msg}");
        assert!(msg.contains(KEY_FILE), "must name the file: {msg}");
        assert!(msg.contains("chmod 600"), "must say how to fix it: {msg}");
    }

    #[test]
    fn a_missing_key_says_where_it_looked_and_who_writes_it() {
        let dir = tmp("missing");
        let err = operator_signer(Some(&dir)).expect_err("no key, no signer");
        let msg = err.to_string();
        assert!(msg.contains(KEY_FILE) && msg.contains("bind-operator-key"), "{msg}");
    }

    #[test]
    fn a_truncated_or_non_hex_seed_is_refused() {
        let short = tmp("short");
        keystore(&short, "00010203", 0o600);
        assert!(operator_signer(Some(&short)).unwrap_err().to_string().contains("64"));

        let bad = tmp("nonhex");
        keystore(&bad, &"zz".repeat(32), 0o600);
        assert!(operator_signer(Some(&bad)).unwrap_err().to_string().contains("valid hex"));
    }

    #[test]
    fn the_wrong_key_is_caught_against_the_reported_guardian() {
        let dir = tmp("mismatch");
        keystore(&dir, SEED_HEX, 0o600);
        let (right, _) = operator_signer(Some(&dir)).unwrap();
        // Some other operator's key.
        let (_, other_pub) = keys::generate(Alg::Ed25519).unwrap();
        let other = DidKey::from_pubkey(&other_pub).unwrap();

        let err = guardian_signer_for(&other, Some(&dir)).expect_err("wrong key refused");
        assert!(err.to_string().contains("guardian as"), "{err}");
        // NEGATIVE CONTROL: the right key passes the same check.
        guardian_signer_for(&right, Some(&dir)).expect("the reported guardian's own key");
    }

    #[test]
    fn the_default_path_follows_ats_resolution() {
        // Only the shape is asserted — the env is process-global, so the precedence
        // itself is exercised through the explicit-override argument above.
        let p = operator_key_path(None);
        assert!(p.ends_with(KEY_FILE));
        assert!(p.to_string_lossy().contains("at-operator"));
    }
}
