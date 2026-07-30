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

//! The join, and the only place AT's wire shape becomes an [`AtEvent`].
//!
//! AT's app-facing carrier emits **two** message kinds keyed by peer uuid —
//! identity facts from the identity process, the score from the reputation
//! process — because joining them inside AT would have coupled two processes
//! that are otherwise independent. So the join is the consumer's, and this is it.
//!
//! Kept out of the `at-ffi` feature deliberately: the FFI is the part that cannot
//! be exercised without a C library, while the join is where the reasoning lives
//! and therefore where the tests need to be. The feature-gated code does nothing
//! but marshal the ABI's flat structs into [`ObservedReading`] /
//! [`ReputationReading`] and hand them here.
//!
//! See `autonomous_trust/doc/architecture/app-peer-carrier.md`.

use std::collections::BTreeMap;

use en_core::Timestamp;

use crate::event::{AtEvent, AtPeer};

/// Length of a peer uuid, matching `AT_APP_UUID_LEN`.
pub const AT_UUID_LEN: usize = 16;
/// Length of an ed25519 signing key, matching `AT_APP_SIGNING_KEY_LEN`.
pub const AT_SIGNING_KEY_LEN: usize = 32;

/// A peer uuid — AT's join key, and *not* the ethne identity (that is the
/// signing key, which only [`ObservedReading`] carries).
pub type AtUuid = [u8; AT_UUID_LEN];

/// One `AT_APP_EVENT_PEER_OBSERVED` payload, in Rust.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservedReading {
    pub peer_uuid: AtUuid,
    pub signing_pubkey: [u8; AT_SIGNING_KEY_LEN],
    pub rank: i32,
    pub operator_bound: bool,
    /// Epoch seconds; AT sends `0.0` for "nobody attending, or could not
    /// confirm", and zeroes it whenever `operator_bound` is false.
    pub operator_attested_at: f64,
}

/// One `AT_APP_EVENT_PEER_REPUTATION` payload, in Rust.
#[derive(Clone, Debug, PartialEq)]
pub struct ReputationReading {
    pub peer_uuid: AtUuid,
    /// AT's absolute `[0.0, 1.0]` score. Carries no information unless `rated`.
    pub score: f64,
    pub rated: bool,
}

/// The identity half of a peer, held until a score can be attached to it.
#[derive(Clone, Debug)]
struct Identity {
    signing_pubkey: [u8; AT_SIGNING_KEY_LEN],
    rank: i32,
    operator_bound: bool,
    attested_at: Option<Timestamp>,
}

/// Joins AT's two carrier streams into [`AtEvent::PeerObserved`].
///
/// # The rules, stated before the tests that check them
///
/// 1. **The join key is the uuid; the identity is the signing key.** A
///    reputation naming a uuid never yet observed cannot become an [`AtPeer`] —
///    there is no key to derive a `did:key` from — so it is **held**, not
///    dropped. AT's two producers are separate processes with no ordering
///    guarantee between them, so this path is real.
/// 2. **Neither kind clobbers the other's fields.** An observation carries no
///    score and must not erase one; a reputation carries no identity and must not
///    erase rank or operator state.
/// 3. **`None` covers both "AT says unrated" and "not heard yet"**, which is
///    faithful rather than lossy: ethne D18 defines `None` as *no reading at
///    all*, and its hazard is confusing `None` with *rated low* (AT's 0.2
///    `PREREP_NEUTRAL`). Both of these states honestly hold no reading. The
///    distinction that would be lossy — unrated versus 0.2 — is preserved.
/// 4. **A stamp of `0.0`, or any stamp on an unbound node, becomes `None`.**
///    Gated twice, here and at the emitter, because an attendance stamp on a node
///    whose operator credential never verified attests to nothing.
#[derive(Debug, Default)]
pub struct PeerJoin {
    identities: BTreeMap<AtUuid, Identity>,
    /// Present = a reputation was heard. `Some(x)` rated, `None` explicitly
    /// unrated. Absent = never heard. Rule 3 collapses the last two on output.
    scores: BTreeMap<AtUuid, Option<f64>>,
}

impl PeerJoin {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many reputations are held for peers not yet observed (rule 1). A
    /// steady non-zero value means reputations are arriving for peers this
    /// consumer never sees — worth surfacing rather than hiding.
    pub fn held_reputations(&self) -> usize {
        self.scores
            .keys()
            .filter(|u| !self.identities.contains_key(*u))
            .count()
    }

    /// Number of peers with both halves joined.
    pub fn joined(&self) -> usize {
        self.identities.len()
    }

    /// Fold in an observation. Always yields one event: an observation is news
    /// even when nothing changed, because the provider above diffs against its
    /// own prior view and is the right place to decide that.
    pub fn apply_observed(&mut self, r: &ObservedReading) -> Vec<AtEvent> {
        let attested_at = attended_stamp(r.operator_bound, r.operator_attested_at);
        self.identities.insert(
            r.peer_uuid,
            Identity {
                signing_pubkey: r.signing_pubkey,
                rank: r.rank,
                operator_bound: r.operator_bound,
                attested_at,
            },
        );
        // Rule 2: the score slot is untouched, so a re-observation keeps a score
        // already joined.
        self.emit(&r.peer_uuid).into_iter().collect()
    }

    /// Fold in a reputation. Yields an event only once the identity half is
    /// known (rule 1) — the score is retained either way, so the peer is
    /// reported complete the moment it is observed.
    pub fn apply_reputation(&mut self, r: &ReputationReading) -> Vec<AtEvent> {
        self.scores
            .insert(r.peer_uuid, if r.rated { Some(r.score) } else { None });
        // Rule 2: identity fields are untouched.
        self.emit(&r.peer_uuid).into_iter().collect()
    }

    fn emit(&self, uuid: &AtUuid) -> Option<AtEvent> {
        let id = self.identities.get(uuid)?;
        Some(AtEvent::PeerObserved(AtPeer {
            signing_pubkey: id.signing_pubkey.to_vec(),
            // Rule 3: absent and explicitly-unrated both read as None.
            reputation: self.scores.get(uuid).copied().flatten(),
            rank: id.rank,
            operator_bound: id.operator_bound,
            // No live source: AT has no guardian *identity*, only the fact that
            // one exists. D15's chartered node->guardian edge waits on the
            // operator-key slice; see AtPeer::guardian_pubkey.
            guardian_pubkey: None,
            operator_attested_at: id.attested_at,
        }))
    }
}

/// AT's `0.0` means "nobody attending, or could not confirm"; an unbound node's
/// stamp means nothing at all (rule 4).
fn attended_stamp(operator_bound: bool, stamp: f64) -> Option<Timestamp> {
    if !operator_bound || stamp <= 0.0 {
        return None;
    }
    // Whole seconds: AT stamps with `(double)time(NULL)`. A stamp too large for
    // the calendar is dropped rather than guessed at.
    Timestamp::from_second(stamp as i64).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(n: u8) -> AtUuid {
        [n; AT_UUID_LEN]
    }

    fn key(n: u8) -> [u8; AT_SIGNING_KEY_LEN] {
        [n; AT_SIGNING_KEY_LEN]
    }

    fn observed(n: u8) -> ObservedReading {
        ObservedReading {
            peer_uuid: uuid(n),
            signing_pubkey: key(n),
            rank: 3,
            operator_bound: false,
            operator_attested_at: 0.0,
        }
    }

    fn rated(n: u8, score: f64) -> ReputationReading {
        ReputationReading {
            peer_uuid: uuid(n),
            score,
            rated: true,
        }
    }

    fn unrated(n: u8) -> ReputationReading {
        ReputationReading {
            peer_uuid: uuid(n),
            score: 0.2, // AT's PREREP_NEUTRAL, which must not be believed
            rated: false,
        }
    }

    fn peer(events: &[AtEvent]) -> &AtPeer {
        match events.first().expect("an event") {
            AtEvent::PeerObserved(p) => p,
            other => panic!("expected PeerObserved, got {other:?}"),
        }
    }

    #[test]
    fn an_observation_alone_reports_a_peer_with_no_reading() {
        let mut j = PeerJoin::new();
        let out = j.apply_observed(&observed(1));
        assert_eq!(out.len(), 1);
        assert_eq!(peer(&out).signing_pubkey, key(1).to_vec());
        // Not heard yet is not a score, and above all is not 0.2.
        assert_eq!(peer(&out).reputation, None);
    }

    #[test]
    fn a_rated_reputation_joins_onto_a_known_peer() {
        let mut j = PeerJoin::new();
        j.apply_observed(&observed(1));
        let out = j.apply_reputation(&rated(1, 0.73));
        assert_eq!(peer(&out).reputation, Some(0.73));
        // And the identity half survived it (rule 2).
        assert_eq!(peer(&out).signing_pubkey, key(1).to_vec());
        assert_eq!(peer(&out).rank, 3);
    }

    #[test]
    fn an_explicit_unrated_reads_as_no_reading_not_as_its_carried_score() {
        let mut j = PeerJoin::new();
        j.apply_observed(&observed(1));
        let out = j.apply_reputation(&unrated(1));
        // The 0.2 in the message is AT's neutral prior, not an earned score;
        // believing it is exactly the D18 hazard.
        assert_eq!(peer(&out).reputation, None);
    }

    /// Rule 1: AT's two producers are separate processes.
    #[test]
    fn a_reputation_for_an_unseen_peer_is_held_not_dropped() {
        let mut j = PeerJoin::new();
        let out = j.apply_reputation(&rated(1, 0.66));
        assert!(out.is_empty(), "nothing to report without a signing key");
        assert_eq!(j.held_reputations(), 1);
        assert_eq!(j.joined(), 0);

        // The observation arrives second and the peer is complete at once.
        let out = j.apply_observed(&observed(1));
        assert_eq!(peer(&out).reputation, Some(0.66));
        assert_eq!(j.held_reputations(), 0);
    }

    /// Rule 2, the direction that would silently lose a score.
    #[test]
    fn a_re_observation_does_not_erase_a_joined_score() {
        let mut j = PeerJoin::new();
        j.apply_observed(&observed(1));
        j.apply_reputation(&rated(1, 0.81));
        let mut again = observed(1);
        again.rank = 9;
        let out = j.apply_observed(&again);
        assert_eq!(peer(&out).reputation, Some(0.81));
        assert_eq!(peer(&out).rank, 9);
    }

    /// Rule 2, the other direction.
    #[test]
    fn a_reputation_does_not_erase_rank_or_operator_state() {
        let mut j = PeerJoin::new();
        let mut o = observed(1);
        o.rank = 7;
        o.operator_bound = true;
        o.operator_attested_at = 1_721_800_000.0;
        j.apply_observed(&o);
        let out = j.apply_reputation(&rated(1, 0.5));
        let p = peer(&out);
        assert_eq!(p.rank, 7);
        assert!(p.operator_bound);
        assert_eq!(
            p.operator_attested_at,
            Some(Timestamp::from_second(1_721_800_000).unwrap())
        );
    }

    #[test]
    fn a_later_unrated_supersedes_an_earlier_score() {
        let mut j = PeerJoin::new();
        j.apply_observed(&observed(1));
        j.apply_reputation(&rated(1, 0.9));
        let out = j.apply_reputation(&unrated(1));
        // AT's current answer wins: it now holds no rating.
        assert_eq!(peer(&out).reputation, None);
    }

    #[test]
    fn peers_are_joined_independently() {
        let mut j = PeerJoin::new();
        j.apply_observed(&observed(1));
        j.apply_observed(&observed(2));
        j.apply_reputation(&rated(2, 0.4));
        assert_eq!(j.joined(), 2);
        // Peer 1 is still unread; peer 2's score did not leak onto it.
        let out = j.apply_observed(&observed(1));
        assert_eq!(peer(&out).reputation, None);
    }

    /// Rule 4: an attendance stamp is meaningless without the verified bound,
    /// so it is gated here as well as at the emitter.
    #[test]
    fn a_stamp_without_the_operator_bound_is_dropped() {
        let mut j = PeerJoin::new();
        let mut o = observed(1);
        o.operator_bound = false;
        o.operator_attested_at = 1_721_800_000.0; // an emitter bug, or a liar
        let out = j.apply_observed(&o);
        assert_eq!(peer(&out).operator_attested_at, None);
    }

    #[test]
    fn a_zero_stamp_on_a_bound_node_is_not_attended() {
        let mut j = PeerJoin::new();
        let mut o = observed(1);
        o.operator_bound = true;
        o.operator_attested_at = 0.0;
        let out = j.apply_observed(&o);
        let p = peer(&out);
        // Bound but unattended is a normal steady state, not an error.
        assert!(p.operator_bound);
        assert_eq!(p.operator_attested_at, None);
    }

    #[test]
    fn the_did_derives_from_the_signing_key_not_the_uuid() {
        let mut j = PeerJoin::new();
        let out = j.apply_observed(&observed(1));
        // A real ed25519 point is not required for did:key derivation (it embeds
        // the key), so this checks the mapping, not the curve.
        let did = peer(&out).did().expect("did:key from the signing key");
        assert!(did.as_str().starts_with("did:key:"));
    }
}
