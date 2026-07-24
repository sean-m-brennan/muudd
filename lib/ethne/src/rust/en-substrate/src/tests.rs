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

use en_archive::Archive;
use en_charter::{
    charter_state, check_authority, verify_all, CharterEntry, Commitment, Substrate,
};
use en_core::{keys, Alg, DidKey, PrivKey, Signed, Timestamp};

use crate::{FoundingCandidate, ManualFounding, MemberCandidate, SubstrateProvider};

fn t(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("timestamp")
}

fn id() -> (PrivKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    (sk, DidKey::from_pubkey(&pk).unwrap())
}

fn substrate() -> Substrate {
    Substrate { name: "tradition".into(), statement: "answers to X above itself".into(), anchor: None }
}

/// The founding path end to end through the *manual* provider — with neither
/// `autonomous_trust` nor any kith-covenant governance crate in the dependency
/// graph (`en-substrate` depends only on `en-core` + `en-charter`; the archive is
/// a dev-dependency). This is the KC ⟂ AT decoupling, demonstrated: a polity is
/// chartered and its charter verifies and authorizes with no trust substrate and
/// no federation present.
#[test]
fn manual_founding_end_to_end_without_at() {
    let members: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
    let polity = id().1;
    let roll: Vec<DidKey> = members.iter().map(|(_, d)| d.clone()).collect();

    // 1. Emergence (here: a convener) proposes a candidate boundary.
    let provider = ManualFounding::new(roll.clone(), substrate());
    let candidate = provider.founding_candidate().unwrap();
    assert_eq!(candidate.members.len(), 3);
    assert!(candidate.substrate.is_some());

    // 2. The polity draws the proposal it will ratify.
    let proposal = candidate.propose(
        polity.clone(),
        substrate(),
        vec![Commitment { key: "mutual-aid".into(), description: "aid among members".into() }],
        0.0,
        t(50),
    );
    assert_eq!(proposal.proposed_members.len(), 3);

    // 3. A majority ratifies and co-signs.
    let charter = proposal.ratify(vec![roll[0].clone(), roll[1].clone()], t(100)).unwrap();
    let signed = Signed::build(
        charter,
        &[(roll[0].as_str(), &members[0].0), (roll[1].as_str(), &members[1].0)],
    )
    .unwrap();

    // 4. The charter lands in the archive, verifies, and authorizes.
    let mut archive = Archive::new();
    archive.append(CharterEntry::PolityChartered(signed)).unwrap();
    verify_all(&archive).expect("signatures valid");
    check_authority(&archive).expect("majority-ratified singleton is authorized");

    let st = charter_state(&archive, &polity, t(300));
    assert!(st.chartered);
    assert_eq!(st.members.len(), 3);
    let keys: Vec<&str> = st.commitments.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, vec!["mutual-aid"]);
}

#[test]
fn reputation_filter_trims_low_reputation_candidates_from_the_roll() {
    let keep_a = id().1;
    let drop = id().1;
    let keep_b = id().1;
    let candidate = FoundingCandidate {
        members: vec![
            MemberCandidate { did: keep_a.clone(), reputation: 0.9, guardian: None },
            MemberCandidate { did: drop.clone(), reputation: 0.4, guardian: None },
            MemberCandidate { did: keep_b.clone(), reputation: 0.8, guardian: None },
        ],
        substrate: Some(substrate()),
    };
    let provider = ManualFounding::from_candidate(candidate);

    let proposal = provider.founding_candidate().unwrap().propose(
        id().1,
        substrate(),
        vec![],
        0.5,
        t(50),
    );
    assert_eq!(proposal.proposed_members, vec![keep_a, keep_b]);
    assert!(provider.member_reputation(&drop) == Some(0.4));
}

#[test]
fn member_reputation_reads_and_misses() {
    let a = id().1;
    let stranger = id().1;
    let provider = ManualFounding::new(vec![a.clone()], substrate());
    assert_eq!(provider.member_reputation(&a), Some(1.0));
    assert_eq!(provider.member_reputation(&stranger), None);
}
