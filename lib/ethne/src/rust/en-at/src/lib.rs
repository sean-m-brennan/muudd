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

//! # ethne-at — the autonomous_trust substrate provider
//!
//! `en-at` adapts a running (or replayed) autonomous_trust view into an ethne
//! [`en_substrate::SubstrateProvider`], so a cooperating AT cluster can be
//! offered to its members for ratification into a bounded, substrate-anchored
//! polity ("ratify emergence, do not replace it").
//!
//! Locked design decisions this crate realizes:
//! - **D5** — it is a *Rust crate over AT's C interface*, not PyO3: ethne stays
//!   pure Rust. All C contact is isolated behind the [`AtEventSource`] seam.
//! - **D6** — both coupling modes (embed an AT node / attach to a running
//!   daemon's out-queue) are [`AtEventSource`] implementations, in the
//!   `at-ffi`-gated [`source::ffi`] module.
//! - **D7** — the candidate roll is the observed *gateway-reputation subtree*
//!   (AT's subtree member-roster capability); a private gateway is an opaque
//!   boundary edge, not incompleteness.
//! - **D8** — a member's guardian edge is its verified **operator** (the
//!   operator-attended signal), the human-above-machine anchor.
//!
//! Decoupling: the default build depends only on `en-*` (→ `kc-core`); it does
//! NOT link autonomous_trust. So `KC ⟂ AT` and `KC ⟂ ethne` are preserved even
//! with this adapter present — the real AT linkage is opt-in via `at-ffi`.

pub mod decode;
pub mod event;
pub mod operator;
pub mod provider;
pub mod source;

pub use decode::{ObservedReading, PeerJoin, ReputationReading};
pub use event::{signing_key_did, AtEvent, AtPeer};
pub use operator::{guardian_signer_for, operator_key_path, operator_signer};
pub use provider::AtSubstrateProvider;
pub use source::{AtEventSource, MockEventSource};
#[cfg(feature = "at-ffi")]
pub use source::ffi::{AtLogLevel, AttachedAtDaemon, EmbeddedAtNode};

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::Substrate;
    use en_core::{keys, Alg, Timestamp};
    use en_substrate::{ChangeStream, SubstrateChange, SubstrateProvider};

    /// A distinct ed25519 signing key per seed (deterministic across a run is
    /// not required — we only need distinct valid keys).
    fn key() -> Vec<u8> {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        pk.bytes
    }

    /// A fixed instant, so stream assertions never touch the wall clock.
    fn t(secs: i64) -> Timestamp {
        Timestamp::from_second(1_721_800_000 + secs).unwrap()
    }

    fn machine(rep: f64) -> (Vec<u8>, AtPeer) {
        let k = key();
        (
            k.clone(),
            AtPeer {
                signing_pubkey: k,
                reputation: Some(rep),
                rank: 0,
                operator_bound: false,
                guardian_pubkey: None,
                operator_attested_at: None,
            },
        )
    }

    /// An operator-attended node whose guardian was verified present at
    /// `attested_at` — AT's live attended-now stamp on top of the durable bound.
    fn attended_at_stamp(rep: f64, attested_at: Timestamp) -> (Vec<u8>, Vec<u8>, AtPeer) {
        let (nk, gk, mut peer) = operator_attended(rep);
        peer.operator_attested_at = Some(attested_at);
        (nk, gk, peer)
    }

    fn operator_attended(rep: f64) -> (Vec<u8>, Vec<u8>, AtPeer) {
        let node = key();
        let guardian = key();
        (
            node.clone(),
            guardian.clone(),
            AtPeer {
                signing_pubkey: node,
                reputation: Some(rep),
                rank: 5,
                operator_bound: true,
                guardian_pubkey: Some(guardian),
                operator_attested_at: None,
            },
        )
    }

    #[test]
    fn fold_builds_founding_candidate_with_guardian_edges() {
        let (_mk, m) = machine(0.9);
        let (nk, gk, op) = operator_attended(0.8);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::PeerObserved(m), AtEvent::PeerObserved(op)]);

        let mut provider = AtSubstrateProvider::new(src);
        assert_eq!(provider.refresh().unwrap(), 2);
        assert_eq!(provider.observed_len(), 2);

        let fc = provider.founding_candidate().unwrap();
        assert_eq!(fc.members.len(), 2);
        assert!(fc.substrate.is_none()); // pure emergent (Q4)

        // the operator-attended node carries a guardian edge = the operator DID;
        // the plain machine carries none (D8).
        let node_did = signing_key_did(&nk).unwrap();
        let guardian_did = signing_key_did(&gk).unwrap();
        let op_member = fc.members.iter().find(|m| m.did == node_did).unwrap();
        assert_eq!(op_member.guardian.as_ref(), Some(&guardian_did));
        assert!(fc
            .members
            .iter()
            .any(|m| m.did != node_did && m.guardian.is_none()));
    }

    #[test]
    fn departed_peer_leaves_the_roll() {
        let (mk, m) = machine(0.7);
        let (_ok, other) = machine(0.8);
        // Observe two, then depart the first — folded in arrival order within a
        // single refresh, so the departed peer is gone and the other remains.
        let mut src = MockEventSource::new();
        src.extend([
            AtEvent::PeerObserved(m),
            AtEvent::PeerObserved(other),
            AtEvent::PeerDeparted {
                signing_pubkey: mk.clone(),
            },
        ]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        assert_eq!(provider.observed_len(), 1);
        assert!(provider
            .member_reputation(&signing_key_did(&mk).unwrap())
            .is_none());
    }

    #[test]
    fn reputation_filter_trims_the_proposal() {
        // Two above the floor, one below — propose() must drop the low one.
        let (_a, a) = machine(0.9);
        let (_b, b) = machine(0.6);
        let (_c, c) = machine(0.2);
        let mut src = MockEventSource::new();
        src.extend([
            AtEvent::PeerObserved(a),
            AtEvent::PeerObserved(b),
            AtEvent::PeerObserved(c),
        ]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        let fc = provider.founding_candidate().unwrap();
        assert_eq!(fc.members.len(), 3);

        // Build a proposal with a reputation floor of 0.5 -> drops the 0.2 member.
        let (_psk, ppk) = keys::generate(Alg::Ed25519).unwrap();
        let polity = signing_key_did(&ppk.bytes).unwrap();
        let substrate = Substrate {
            name: "Test Polity".into(),
            statement: "a cohering AT cluster".into(),
            anchor: None,
        };
        let proposal = fc.propose(
            polity,
            substrate,
            vec![],
            0.5,
            vec![],
            None,
            Timestamp::from_second(0).unwrap(),
        );
        assert_eq!(proposal.proposed_members.len(), 2);
    }

    #[test]
    fn private_boundary_is_recorded_not_a_failure() {
        let (_mk, m) = machine(0.9);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::BoundaryPrivate, AtEvent::PeerObserved(m)]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();
        assert!(provider.has_private_boundary());
        // the visible member is still in the roll — privacy != incompleteness
        assert_eq!(provider.observed_len(), 1);
    }

    #[test]
    fn attendance_stamp_reaches_the_candidate() {
        // The live half of the signal (attended-now) must survive the fold, not
        // just the durable guardian edge.
        let seen = Timestamp::from_second(1_721_800_000).unwrap();
        let (nk, _gk, op) = attended_at_stamp(0.8, seen);
        let (_mk, plain) = machine(0.7);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::PeerObserved(op), AtEvent::PeerObserved(plain)]);

        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();
        let fc = provider.founding_candidate().unwrap();

        let node_did = signing_key_did(&nk).unwrap();
        let attended = fc.members.iter().find(|m| m.did == node_did).unwrap();
        assert_eq!(attended.attested_at, Some(seen));
        // A node with no guardian at all reports no attendance either.
        let unguarded = fc.members.iter().find(|m| m.did != node_did).unwrap();
        assert_eq!(unguarded.attested_at, None);
    }

    #[test]
    fn attendance_without_the_verified_bound_is_dropped() {
        // An attendance stamp on a node whose operator credential never verified
        // attests to nothing — the same refusal guardian_did() already makes.
        let seen = Timestamp::from_second(1_721_800_000).unwrap();
        let (nk, mut peer) = machine(0.9);
        peer.operator_attested_at = Some(seen);
        assert!(!peer.operator_bound);
        assert_eq!(peer.attended_at(), None);

        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(peer));
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        let fc = provider.founding_candidate().unwrap();
        let did = signing_key_did(&nk).unwrap();
        let member = fc.members.iter().find(|m| m.did == did).unwrap();
        assert_eq!(member.attested_at, None);
        assert!(member.guardian.is_none());
    }

    #[test]
    fn operator_bound_without_a_stamp_keeps_the_guardian_edge() {
        // The two signals are independent: verified hardware with nobody at the
        // console keeps its guardian and simply reports no attendance. AT sends
        // exactly this (operator_bound: true, operator_attested_at: 0).
        let (nk, gk, op) = operator_attended(0.8);
        assert_eq!(op.operator_attested_at, None);
        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(op));
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        let fc = provider.founding_candidate().unwrap();
        let member = &fc.members[0];
        assert_eq!(member.did, signing_key_did(&nk).unwrap());
        assert_eq!(member.guardian, Some(signing_key_did(&gk).unwrap()));
        assert_eq!(member.attested_at, None);
    }

    #[test]
    fn re_observation_refreshes_the_attendance_stamp() {
        // The fold is an upsert, so a later observation must carry a later stamp
        // through — otherwise attendance could never go stale OR recover.
        let first = Timestamp::from_second(1_721_800_000).unwrap();
        let later = Timestamp::from_second(1_721_800_600).unwrap();
        let (nk, _gk, early) = attended_at_stamp(0.8, first);
        let mut refreshed = early.clone();
        refreshed.operator_attested_at = Some(later);

        let mut src = MockEventSource::new();
        src.extend([
            AtEvent::PeerObserved(early),
            AtEvent::PeerObserved(refreshed),
        ]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        let fc = provider.founding_candidate().unwrap();
        let did = signing_key_did(&nk).unwrap();
        let member = fc.members.iter().find(|m| m.did == did).unwrap();
        assert_eq!(member.attested_at, Some(later));
        assert_eq!(provider.observed_len(), 1);
    }


    // --- D16: the change stream --------------------------------------------

    #[test]
    fn a_first_sighting_is_an_appearance_and_a_departure_a_vanishing() {
        let (mk, m) = machine(0.9);
        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(m));
        let mut provider = AtSubstrateProvider::new(src);

        let changes = provider.changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0], SubstrateChange::MemberAppeared(_)));
        assert_eq!(changes[0].member(), Some(&signing_key_did(&mk).unwrap()));

        // Draining is the contract: the same change is not reported twice.
        assert!(provider.changes().unwrap().is_empty());

        provider
            .source_mut()
            .push(AtEvent::PeerDeparted { signing_pubkey: mk.clone() });
        let changes = provider.changes().unwrap();
        assert_eq!(changes, vec![SubstrateChange::MemberVanished(signing_key_did(&mk).unwrap())]);

        // A second departure for someone already gone is not a change.
        provider
            .source_mut()
            .push(AtEvent::PeerDeparted { signing_pubkey: mk });
        assert!(provider.changes().unwrap().is_empty());
    }

    #[test]
    fn a_reputation_move_and_a_guardian_edge_move_are_separate_changes() {
        let (nk, gk, op) = operator_attended(0.8);
        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(op.clone()));
        let mut provider = AtSubstrateProvider::new(src);
        assert_eq!(provider.changes().unwrap().len(), 1); // the appearance

        // Same node, lower reputation and its operator binding gone.
        let mut degraded = op;
        degraded.reputation = Some(0.4);
        degraded.operator_bound = false;
        degraded.guardian_pubkey = None;
        provider.source_mut().push(AtEvent::PeerObserved(degraded));

        let changes = provider.changes().unwrap();
        let did = signing_key_did(&nk).unwrap();
        assert_eq!(
            changes,
            vec![
                SubstrateChange::ReputationChanged { member: did.clone(), from: Some(0.8), to: Some(0.4) },
                SubstrateChange::GuardianEdgeChanged { member: did, guardian: None },
            ]
        );
        let _ = gk;
    }

    #[test]
    fn an_unchanged_re_observation_reports_nothing() {
        // The point of a delta stream: a substrate that keeps saying the same thing
        // is quiet, and a quiet substrate must not fill an observation log.
        let (_nk, _gk, op) = operator_attended(0.8);
        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(op.clone()));
        let mut provider = AtSubstrateProvider::new(src);
        assert_eq!(provider.changes().unwrap().len(), 1);

        provider.source_mut().push(AtEvent::PeerObserved(op));
        assert!(provider.changes().unwrap().is_empty());
    }

    #[test]
    fn only_an_advancing_attendance_stamp_is_a_change() {
        let (nk, _gk, first) = attended_at_stamp(0.8, t(1_000));
        let mut src = MockEventSource::new();
        src.push(AtEvent::PeerObserved(first.clone()));
        let mut provider = AtSubstrateProvider::new(src);
        let did = signing_key_did(&nk).unwrap();

        // The first sighting carries both the appearance and the stamp.
        let changes = provider.changes().unwrap();
        assert_eq!(changes.len(), 2);
        assert!(matches!(changes[1], SubstrateChange::AttendanceStamped { .. }));

        // A re-attestation moves the stamp forward.
        let mut later = first.clone();
        later.operator_attested_at = Some(t(2_000));
        provider.source_mut().push(AtEvent::PeerObserved(later));
        assert_eq!(
            provider.changes().unwrap(),
            vec![SubstrateChange::AttendanceStamped { member: did, attested_at: t(2_000) }]
        );

        // The same stamp again is not a re-attestation — which is exactly the case a
        // count of readings must not mistake for presence.
        provider.source_mut().push(AtEvent::PeerObserved(first));
        assert!(provider.changes().unwrap().is_empty());
    }

    #[test]
    fn the_boundary_going_opaque_is_reported_once() {
        let mut src = MockEventSource::new();
        src.push(AtEvent::BoundaryPrivate);
        let mut provider = AtSubstrateProvider::new(src);
        assert_eq!(
            provider.changes().unwrap(),
            vec![SubstrateChange::BoundaryOpacityChanged { opaque: true }]
        );
        provider.source_mut().push(AtEvent::BoundaryPrivate);
        assert!(provider.changes().unwrap().is_empty(), "an opaque edge does not re-open");
    }

    #[test]
    fn every_at_peer_is_a_derived_party_and_the_guardian_edge_is_what_varies() {
        // D18: in AT's model there is no such thing as a human peer — a peer IS a
        // node — so derivation across this seam is structural, not a per-peer
        // reading. What varies is `operator_bound`: a node with a verified human is
        // a guarded machine, a node without one is the UNGUARDED machine D15's
        // office rule exists for. Before `MemberCandidate::derived`, that second
        // node proposed silently as a human member.
        let (nk, node) = machine(0.9);
        let (gk, guardian_key, attended) = operator_attended(0.9);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::PeerObserved(node), AtEvent::PeerObserved(attended)]);
        let mut p = AtSubstrateProvider::new(src);
        p.refresh().unwrap();

        let c = p.founding_candidate().unwrap();
        assert_eq!(c.members.len(), 2);
        assert!(c.members.iter().all(|m| m.derived), "every AT peer is derived");

        let unguarded = c
            .members
            .iter()
            .find(|m| m.did == signing_key_did(&nk).unwrap())
            .unwrap();
        assert!(unguarded.guardian.is_none(), "no operator, no guardian edge");

        let guarded = c
            .members
            .iter()
            .find(|m| m.did == signing_key_did(&gk).unwrap())
            .unwrap();
        assert_eq!(guarded.guardian, Some(signing_key_did(&guardian_key).unwrap()));
    }

    #[test]
    fn an_at_reading_crosses_as_a_rated_member() {
        // AT's scale is absolute and needs no rescaling, so the score crosses as-is.
        // `None` is not reachable from a live node yet — AT collapses "no history"
        // into PREREP_NEUTRAL at its own API boundary — which is a named AT-side
        // prerequisite, not a decoder bug. This pins today's honest behaviour.
        let (_k, node) = machine(0.77);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::PeerObserved(node)]);
        let mut p = AtSubstrateProvider::new(src);
        p.refresh().unwrap();
        let c = p.founding_candidate().unwrap();
        assert_eq!(c.members[0].reputation, Some(0.77));
    }

}

/// The AT↔ethne key join, pinned as a cross-library test vector.
///
/// Both custody paths for a guardian's co-signature (design **D15**) rest on one
/// fact: AT persists the operator's ed25519 key as a hex-encoded **32-byte seed**
/// (`load_or_create_operator_key`, libsodium via PyNaCl) and `kc_core::keys` takes a
/// 32-byte seed into `ed25519_dalek::SigningKey::from_bytes`. If those two
/// representations disagreed, the detached-signing path would produce signatures
/// ethne could not verify and the keystore loader would derive the wrong `did:key`.
///
/// So the agreement is asserted against a fixture generated by the *other* library
/// rather than by a live cross-call: the seed, public key and signature below were
/// produced by PyNaCl and are checked here by dalek. A pinned vector also catches a
/// future dependency swap, which a round trip inside one library would not.
#[cfg(test)]
mod cross_library_key_join {
    use en_core::{keys, Alg, DidKey, PubKey, Signature};

    /// Generated with PyNaCl: `SigningKey(bytes(range(32)))` over `MESSAGE`.
    const SEED_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const PUBKEY_HEX: &str = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";
    const SIGNATURE_HEX: &str = "da59bd87e71eb82df74fc2450166988bd41bced8ed1b95563a1d97ee283b82d0\
                                 4c9e68514ad6789c980b992b588edf9a41b627d6830cf5fbe86e043f72839104";
    const MESSAGE: &[u8] = b"ethne-guardian-cosign-v1 cross-library test vector";

    fn unhex(s: &str) -> Vec<u8> {
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn a_libsodium_signature_verifies_under_dalek() {
        let pk = PubKey { alg: Alg::Ed25519, bytes: unhex(PUBKEY_HEX) };
        let sig = Signature { alg: Alg::Ed25519, bytes: unhex(SIGNATURE_HEX) };
        keys::verify(&pk, MESSAGE, &sig).expect("PyNaCl's signature must verify here");
    }

    #[test]
    fn one_flipped_bit_fails() {
        // Without this the test above would pass against a verify that returned Ok
        // unconditionally, which is the whole hazard of a pinned positive vector.
        let pk = PubKey { alg: Alg::Ed25519, bytes: unhex(PUBKEY_HEX) };
        let mut bytes = unhex(SIGNATURE_HEX);
        bytes[0] ^= 0x01;
        let sig = Signature { alg: Alg::Ed25519, bytes };
        assert!(keys::verify(&pk, MESSAGE, &sig).is_err(), "a tampered signature must fail");
    }

    #[test]
    fn the_same_seed_derives_the_same_public_key_and_did() {
        // The keystore loader's premise: AT stores the seed, ethne derives the
        // did:key from the public half, and the guardian DID a substrate reading
        // reports must be the one that seed can sign for.
        let seed = unhex(SEED_HEX);
        let sk = en_core::PrivKey { alg: Alg::Ed25519, bytes: seed };
        let sig = keys::sign(&sk, MESSAGE).expect("sign with the stored seed");
        let pk = PubKey { alg: Alg::Ed25519, bytes: unhex(PUBKEY_HEX) };
        keys::verify(&pk, MESSAGE, &sig).expect("dalek's own signature verifies too");
        // Byte-identical to PyNaCl's, since ed25519 signing is deterministic.
        assert_eq!(sig.bytes, unhex(SIGNATURE_HEX), "ed25519 is deterministic across both");
        let did = DidKey::from_pubkey(&pk).expect("did:key from the operator pubkey");
        assert!(did.as_str().starts_with("did:key:z6Mk"));
    }
}
