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

//! Border artifact **#7**: the polity's relay roster, in AT's own file format
//! (design **D36**).
//!
//! The six artifacts above this one cross the border to kith-covenant, in KC's
//! vocabulary. This one crosses to the trust substrate instead, and in the
//! substrate's vocabulary: AutonomousTrust defines an `at-relay-roster` file, and
//! AT verifies it with nothing but the ed25519 key its operator pinned. AT never
//! parses an ethne record, which is what keeps `ethne ⟂ AT` intact and lets a plain
//! AT deployment ignore polities entirely. `en-at` stays read-only: this module
//! emits a file, and the host decides where it goes.
//!
//! The file is `{"body": <exact JSON string>, "sig": <hex>}`, where
//!
//! ```text
//! sig  = ed25519(polity root key, "at-relay-roster-v1|" + body)
//! body = {"v":1,"typename":"at-relay-roster","issuer":<hex root pubkey>,
//!         "seq":N,"relays":["relay://<uuid>:<fp>@host:port", ...]}
//! ```
//!
//! and `fp` is the first 16 bytes of SHA-256 over the relay's raw ed25519 signing
//! key, in hex, exactly as AT's `relay.key_fingerprint` computes it.
//!
//! **A higher `seq` replaces the issuer's previous roster whole.** That is the
//! opposite of the member roll's union rule (D26), deliberately: a roll that could
//! shrink would let a polity narrow a suspect set, but a roster that could not
//! shrink would leave a revoked operator's relay in every client's list for good.

use en_charter::{PolityCharter, RendezvousState};
use en_core::{keys, DidKey, KcError, KcResult, PrivKey, PubKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// The domain prefix AT verifies the roster signature under.
pub const ROSTER_DOMAIN: &str = "at-relay-roster-v1|";
/// The body's `typename`.
pub const ROSTER_TYPENAME: &str = "at-relay-roster";
/// The body's format version.
pub const ROSTER_VERSION: u32 = 1;
/// Most relays AT accepts in one roster; past it AT refuses the whole file, so
/// this refuses to emit one.
pub const ROSTER_MAX: usize = 64;
/// Bytes of SHA-256 an AT relay pin carries (AT's `AT_RELAY_FP_BYTES`).
pub const FP_BYTES: usize = 16;

#[derive(Serialize)]
struct RosterBody<'a> {
    v: u32,
    typename: &'a str,
    issuer: String,
    seq: u64,
    relays: Vec<String>,
}

#[derive(Serialize)]
struct RosterFile<'a> {
    body: &'a str,
    sig: String,
}

/// The pin fingerprint AT computes for a relay whose signing key is `pubkey`.
pub fn relay_fingerprint(pubkey: &PubKey) -> KcResult<String> {
    if pubkey.bytes.len() != 32 {
        return Err(KcError::Key(format!(
            "a relay pin needs a 32-byte ed25519 key, got {} bytes",
            pubkey.bytes.len()
        )));
    }
    Ok(hex::encode(&Sha256::digest(&pubkey.bytes)[..FP_BYTES]))
}

/// The AT hint pinning the relay at `host:port` to `at_uuid` and `member`'s key.
/// An IPv6 host is bracketed, as AT's `hint_for` writes one.
pub fn relay_hint(member: &DidKey, at_uuid: &str, host: &str, port: u16) -> KcResult<String> {
    let fp = relay_fingerprint(&member.to_pubkey()?)?;
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    Ok(format!("relay://{at_uuid}:{fp}@{host}:{port}"))
}

/// The polity's relay roster as AT's signed file, ready for the host to write
/// where its AT nodes read rosters.
///
/// `state` is [`en_charter::rendezvous_state`] at the instant the roster speaks
/// for: the members who hold `RendezvousOperator` then and have a live endpoint.
/// Each relay is pinned to its operator's own `did:key`, so the roster cannot name
/// a relay under a key that is not its operator's.
///
/// `seq` must exceed every seq this polity has published before, since AT keeps
/// the highest it has seen per issuer and refuses anything at or below it. The
/// seconds of the state's instant are a natural choice.
///
/// An **empty** roster is valid and is how a polity says it runs no relays now;
/// it replaces the previous one. Errors if the charter's polity DID does not match
/// the supplied root key (the guard every border artifact shares), if there are
/// more than [`ROSTER_MAX`] relays, or if a listing cannot be written as a pin.
pub fn rendezvous_roster(
    charter: &PolityCharter,
    state: &RendezvousState,
    root_priv: &PrivKey,
    root_pub: &PubKey,
    seq: u64,
) -> KcResult<String> {
    let derived = DidKey::from_pubkey(root_pub)?;
    if charter.polity != derived {
        return Err(KcError::InvalidDid(format!(
            "charter polity {} does not match the supplied root key (DID {derived})",
            charter.polity
        )));
    }
    if state.relays.len() > ROSTER_MAX {
        return Err(KcError::Authority(format!(
            "polity {} lists {} relays, and AT accepts at most {ROSTER_MAX} in one roster",
            charter.polity,
            state.relays.len()
        )));
    }
    let mut relays = Vec::with_capacity(state.relays.len());
    for r in &state.relays {
        relays.push(relay_hint(&r.member, &r.at_uuid, &r.host, r.port)?);
    }
    let body = RosterBody {
        v: ROSTER_VERSION,
        typename: ROSTER_TYPENAME,
        issuer: hex::encode(&root_pub.bytes),
        seq,
        relays,
    };
    let body_str = serde_json::to_string(&body)
        .map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    let mut msg = Vec::with_capacity(ROSTER_DOMAIN.len() + body_str.len());
    msg.extend_from_slice(ROSTER_DOMAIN.as_bytes());
    msg.extend_from_slice(body_str.as_bytes());
    let sig = keys::sign(root_priv, &msg)?;
    serde_json::to_string(&RosterFile { body: &body_str, sig: hex::encode(&sig.bytes) })
        .map_err(|e| KcError::CanonicalEncode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::{FoundingProposal, RelayListing, Substrate};
    use en_core::{keys, Alg, Timestamp};

    const UUID_A: &str = "0f1e2d3c-4b5a-4968-8776-655443322110";
    const UUID_B: &str = "1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9";

    fn t(secs: i64) -> Timestamp {
        Timestamp::from_second(secs).unwrap()
    }

    /// A fixed ed25519 key from a one-byte seed, so the file is byte-stable.
    fn fixed(seed: u8) -> (PrivKey, PubKey) {
        let sk = PrivKey { alg: Alg::Ed25519, bytes: vec![seed; 32] };
        let pk = keys::public_of(&sk).unwrap();
        (sk, pk)
    }

    fn charter_for(root_pub: &PubKey) -> PolityCharter {
        let polity = DidKey::from_pubkey(root_pub).unwrap();
        let roll: Vec<DidKey> = (10u8..13).map(|s| DidKey::from_pubkey(&fixed(s).1).unwrap()).collect();
        let prop = FoundingProposal::convener_asserted(
            polity,
            Substrate { name: "tradition".into(), statement: "answers to X".into(), anchor: None },
            vec![],
            roll.clone(),
            vec![],
            t(50),
        );
        prop.ratify(vec![roll[0].clone(), roll[1].clone()], t(100)).unwrap()
    }

    fn listing(seed: u8, uuid: &str, host: &str, port: u16) -> RelayListing {
        RelayListing {
            member: DidKey::from_pubkey(&fixed(seed).1).unwrap(),
            at_uuid: uuid.into(),
            host: host.into(),
            port,
            declared_at: t(300),
        }
    }

    fn split(file: &str) -> (String, String) {
        let v: serde_json::Value = serde_json::from_str(file).unwrap();
        (v["body"].as_str().unwrap().to_string(), v["sig"].as_str().unwrap().to_string())
    }

    fn two_relays() -> RendezvousState {
        RendezvousState {
            relays: vec![
                listing(20, UUID_A, "relay.example.org", 27790),
                listing(21, UUID_B, "2001:db8::7", 27791),
            ],
        }
    }

    #[test]
    fn the_roster_is_signed_by_the_polity_root_under_ats_domain() {
        let (sk, pk) = fixed(7);
        let file = rendezvous_roster(&charter_for(&pk), &two_relays(), &sk, &pk, 5).unwrap();
        let (body, sig) = split(&file);
        let mut msg = ROSTER_DOMAIN.as_bytes().to_vec();
        msg.extend_from_slice(body.as_bytes());
        let sig = en_core::Signature { alg: Alg::Ed25519, bytes: hex::decode(sig).unwrap() };
        keys::verify(&pk, &msg, &sig).expect("AT's domain + the exact body bytes");
        // Not under the seed list's domain, which a different signer owns.
        let mut wrong = b"at-seeds-v1|".to_vec();
        wrong.extend_from_slice(body.as_bytes());
        assert!(keys::verify(&pk, &wrong, &sig).is_err());

        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["v"], 1);
        assert_eq!(b["typename"], "at-relay-roster");
        assert_eq!(b["issuer"], hex::encode(&pk.bytes));
        assert_eq!(b["seq"], 5);
        assert_eq!(b["relays"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn each_relay_is_pinned_to_its_operators_own_key() {
        let (_, member_pk) = fixed(20);
        let fp = hex::encode(&Sha256::digest(&member_pk.bytes)[..16]);
        let did = DidKey::from_pubkey(&member_pk).unwrap();
        assert_eq!(relay_fingerprint(&member_pk).unwrap(), fp);
        assert_eq!(
            relay_hint(&did, UUID_A, "relay.example.org", 27790).unwrap(),
            format!("relay://{UUID_A}:{fp}@relay.example.org:27790")
        );
        // IPv6 is bracketed, as AT's hint_for writes it.
        assert!(relay_hint(&did, UUID_A, "2001:db8::7", 1).unwrap().ends_with("@[2001:db8::7]:1"));
    }

    #[test]
    fn a_root_key_that_is_not_the_politys_is_refused() {
        let (_, pk) = fixed(7);
        let (other_sk, other_pk) = fixed(8);
        let err = rendezvous_roster(&charter_for(&pk), &two_relays(), &other_sk, &other_pk, 1)
            .unwrap_err();
        assert!(matches!(err, KcError::InvalidDid(_)));
    }

    #[test]
    fn an_empty_roster_is_how_a_polity_says_it_runs_none() {
        let (sk, pk) = fixed(7);
        let file = rendezvous_roster(&charter_for(&pk), &RendezvousState::default(), &sk, &pk, 9).unwrap();
        let (body, _) = split(&file);
        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["relays"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn more_relays_than_at_accepts_is_refused_not_truncated() {
        let (sk, pk) = fixed(7);
        let state = RendezvousState {
            relays: (0..=ROSTER_MAX).map(|i| listing(20, UUID_A, "relay.example.org", 1000 + i as u16)).collect(),
        };
        assert!(rendezvous_roster(&charter_for(&pk), &state, &sk, &pk, 1).is_err());
    }

    /// The exact bytes AT's consumer (stage 2) is held to. ed25519 is
    /// deterministic, so fixed keys give a fixed file; a change here is a change
    /// to the wire format and needs AT's side changed with it.
    #[test]
    fn the_file_is_byte_pinned() {
        let (sk, pk) = fixed(7);
        let file = rendezvous_roster(&charter_for(&pk), &two_relays(), &sk, &pk, 5).unwrap();
        if std::env::var("EN_PRINT_ROSTER_VECTOR").is_ok() {
            println!("ROSTER_VECTOR={file}");
            println!("ISSUER={}", hex::encode(&pk.bytes));
            println!("MEMBER20={}", hex::encode(&fixed(20).1.bytes));
            println!("MEMBER21={}", hex::encode(&fixed(21).1.bytes));
        }
        assert_eq!(file, PINNED_VECTOR);
    }

    /// Cross-checked 2026-09-29 against AT's own Python: the signature verifies
    /// under `at-relay-roster-v1|` with the issuer key, `relay.parse_hint` reads
    /// both pins, `relay.key_fingerprint` of each member key gives the pinned fp,
    /// and `relay.hint_for` writes both hints back byte for byte.
    const PINNED_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"at-relay-roster\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"seq\":5,\"relays\":[\"relay://0f1e2d3c-4b5a-4968-8776-655443322110:7300c0ae1429cd153252736a781b78f4@relay.example.org:27790\",\"relay://1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9:d7e1e084be213b01e506852af8198b99@[2001:db8::7]:27791\"]}","sig":"491f75ed4f7472d3c07f204df7de22354eb8a2ba23632af27507822fb94790ebe6b20ac55e69de391b274e6ba850ed148970d7a384c2e8ffe63b578416cd620f"}"#;
}
