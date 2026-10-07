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

//! Border artifacts **#9** and **#10**: a polity's team roster and its release
//! authorizations, in Stele's own file formats (design **D41**).
//!
//! Like the relay roster (#7), these cross to an application in the
//! application's vocabulary, and are checked with nothing but ed25519 keys. Stele
//! never parses an ethne record.
//!
//! **The team roster** says who the team is, as the polity's root key states it:
//!
//! ```text
//! sig  = ed25519(polity root key, "stele-team-v1|" + body)
//! body = {"v":1,"typename":"stele-team","polity":<did>,"issuer":<hex root pubkey>,
//!         "seq":N,
//!         "members":[{"did":<did>,"key":<hex>,"at_uuid":<uuid>,"consent":<hex>}, ...],
//!         "release_officers":[<did>, ...],
//!         "guardians":[{"machine":<did>,"guardian":<did>}, ...]}
//! ```
//!
//! Each member's `consent` is that member's **AT node** signing
//!
//! ```text
//! "stele-team-member-v1|" + issuer + "|" + did + "|" + at_uuid
//! ```
//!
//! with its own AT key, which Stele checks against the key its AT roster holds for
//! `at_uuid`. The root key alone cannot put a node on the team: a polity that
//! could would claim an outside auditor as a teammate, and a teammate does not
//! count toward an attestation about the team. That would expel the auditor
//! without expelling anyone.
//!
//! **A higher `seq` replaces the issuer's previous roster whole**, as the relay
//! roster does. Stele keeps every release it has verified attributed regardless,
//! so a shorter roster narrows who is on the team from now on, never what the
//! team has already shipped.
//!
//! **A release authorization** is one officer's acceptance of one commit:
//!
//! ```text
//! sig  = ed25519(officer key, "stele-release-v1|" + body)
//! body = {"v":1,"typename":"stele-release","polity":<did>,"issuer":<hex root pubkey>,
//!         "officer":<did>,"officer_key":<hex>,"repo":..,"url":..,"commit":..,
//!         "built_by":<did>?,"authorized_at":<unix seconds>}
//! ```
//!
//! It is signed by the officer, not the root, because the officer is who answers
//! for it. Stele believes it while the officer is listed in a roster it holds.

use en_charter::{GuardianState, PolityCharter, ReleaseListing, ReleaseState};
use en_core::{keys, DidKey, KcError, KcResult, PrivKey, PubKey};
use serde::Serialize;

/// The domain prefix Stele verifies a team roster under.
pub const TEAM_DOMAIN: &str = "stele-team-v1|";
/// The team roster body's `typename`.
pub const TEAM_TYPENAME: &str = "stele-team";
/// The domain prefix a member's AT node signs its consent under.
pub const CONSENT_DOMAIN: &str = "stele-team-member-v1|";
/// The domain prefix Stele verifies a release authorization under.
pub const RELEASE_DOMAIN: &str = "stele-release-v1|";
/// The release authorization body's `typename`.
pub const RELEASE_TYPENAME: &str = "stele-release";
/// The format version of both files.
pub const STELE_VERSION: u32 = 1;
/// Most members Stele accepts in one roster.
pub const TEAM_MAX: usize = 256;

/// A member as the team roster lists it: the polity member, the AT node it acts
/// through, and that node's consent to being listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMember {
    pub did: DidKey,
    /// The member node's AT identity uuid, lowercase and hyphenated.
    pub at_uuid: String,
    /// Hex of the node's ed25519 signature over [`consent_message`].
    pub consent: String,
}

#[derive(Serialize)]
struct MemberEntry {
    did: String,
    key: String,
    at_uuid: String,
    consent: String,
}

#[derive(Serialize)]
struct GuardianEntry {
    machine: String,
    guardian: String,
}

#[derive(Serialize)]
struct TeamBody<'a> {
    v: u32,
    typename: &'a str,
    polity: String,
    issuer: String,
    seq: u64,
    members: Vec<MemberEntry>,
    release_officers: Vec<String>,
    guardians: Vec<GuardianEntry>,
}

#[derive(Serialize)]
struct ReleaseBody<'a> {
    v: u32,
    typename: &'a str,
    polity: String,
    issuer: String,
    officer: String,
    officer_key: String,
    repo: &'a str,
    url: &'a str,
    commit: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    built_by: Option<String>,
    authorized_at: i64,
}

#[derive(Serialize)]
struct SignedFile<'a> {
    body: &'a str,
    sig: String,
}

/// The bytes a member's AT node signs to consent to being listed on the team
/// whose root key is `issuer_hex`.
pub fn consent_message(issuer_hex: &str, did: &DidKey, at_uuid: &str) -> Vec<u8> {
    format!("{CONSENT_DOMAIN}{issuer_hex}|{}|{at_uuid}", did.as_str()).into_bytes()
}

fn is_lower_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
        })
}

fn is_lower_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn sign_file(domain: &str, body: &str, key: &PrivKey) -> KcResult<String> {
    let mut msg = Vec::with_capacity(domain.len() + body.len());
    msg.extend_from_slice(domain.as_bytes());
    msg.extend_from_slice(body.as_bytes());
    let sig = keys::sign(key, &msg)?;
    serde_json::to_string(&SignedFile { body, sig: hex::encode(&sig.bytes) })
        .map_err(|e| KcError::CanonicalEncode(e.to_string()))
}

fn root_guard(charter: &PolityCharter, root_pub: &PubKey) -> KcResult<()> {
    let derived = DidKey::from_pubkey(root_pub)?;
    if charter.polity != derived {
        return Err(KcError::InvalidDid(format!(
            "charter polity {} does not match the supplied root key (DID {derived})",
            charter.polity
        )));
    }
    Ok(())
}

/// The polity's team roster as Stele's signed file.
///
/// `members` are the members acting through AT nodes, each with its node's
/// consent. `release` is [`en_charter::release_state`] and `guardians` is
/// [`en_charter::guardian_state`], both at the instant the roster speaks for.
/// Every listed officer, and every guardian edge both of whose ends are listed,
/// is carried; an edge with an end that has no AT node cannot be scored by Stele,
/// so it is left out rather than listed against nobody.
///
/// Errors if the root key is not the polity's, if a member is listed twice or
/// has a malformed uuid or consent, if an officer has no listed node (a release
/// Stele could not attribute to anyone), or past [`TEAM_MAX`] members. Consents
/// are carried, not checked: they are signed by AT keys, which only an AT roster
/// holds.
pub fn stele_team_roster(
    charter: &PolityCharter,
    members: &[TeamMember],
    release: &ReleaseState,
    guardians: &GuardianState,
    root_priv: &PrivKey,
    root_pub: &PubKey,
    seq: u64,
) -> KcResult<String> {
    root_guard(charter, root_pub)?;
    if members.len() > TEAM_MAX {
        return Err(KcError::Authority(format!(
            "a team roster lists at most {TEAM_MAX} members, got {}",
            members.len()
        )));
    }
    let mut entries: Vec<MemberEntry> = Vec::with_capacity(members.len());
    for m in members {
        if !is_lower_uuid(&m.at_uuid) {
            return Err(KcError::Authority(format!(
                "member {}: at_uuid {:?} is not a lowercase hyphenated uuid",
                m.did, m.at_uuid
            )));
        }
        if !is_lower_hex(&m.consent, 128) {
            return Err(KcError::Authority(format!(
                "member {}: consent is not a hex ed25519 signature",
                m.did
            )));
        }
        if entries.iter().any(|e| e.did == m.did.as_str() || e.at_uuid == m.at_uuid) {
            return Err(KcError::Authority(format!(
                "member {} or node {} is listed twice",
                m.did, m.at_uuid
            )));
        }
        entries.push(MemberEntry {
            did: m.did.as_str().to_string(),
            key: hex::encode(&m.did.to_pubkey()?.bytes),
            at_uuid: m.at_uuid.clone(),
            consent: m.consent.clone(),
        });
    }
    entries.sort_by(|a, b| a.did.cmp(&b.did));
    let listed = |d: &DidKey| entries.iter().any(|e| e.did == d.as_str());
    let mut officers = Vec::with_capacity(release.officers.len());
    for o in &release.officers {
        if !listed(o) {
            return Err(KcError::Authority(format!(
                "release officer {o} has no AT node on the roster"
            )));
        }
        officers.push(o.as_str().to_string());
    }
    let mut edges: Vec<GuardianEntry> = guardians
        .derivations
        .iter()
        .filter_map(|d| {
            let g = d.guardian.as_ref()?;
            (listed(&d.member) && listed(g)).then(|| GuardianEntry {
                machine: d.member.as_str().to_string(),
                guardian: g.as_str().to_string(),
            })
        })
        .collect();
    edges.sort_by(|a, b| a.machine.cmp(&b.machine));
    let body = TeamBody {
        v: STELE_VERSION,
        typename: TEAM_TYPENAME,
        polity: charter.polity.as_str().to_string(),
        issuer: hex::encode(&root_pub.bytes),
        seq,
        members: entries,
        release_officers: officers,
        guardians: edges,
    };
    let body_str =
        serde_json::to_string(&body).map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    sign_file(TEAM_DOMAIN, &body_str, root_priv)
}

/// One release as Stele's signed authorization, signed by the officer who made
/// it.
///
/// `listing` must come from [`en_charter::release_state`], so only an
/// authorization the authority pass accepted is exported. Errors if the officer
/// key is not the listing's officer.
pub fn stele_release_authorization(
    charter: &PolityCharter,
    listing: &ReleaseListing,
    officer_priv: &PrivKey,
    officer_pub: &PubKey,
) -> KcResult<String> {
    let derived = DidKey::from_pubkey(officer_pub)?;
    if listing.officer != derived {
        return Err(KcError::InvalidDid(format!(
            "release officer {} does not match the supplied key (DID {derived})",
            listing.officer
        )));
    }
    let body = ReleaseBody {
        v: STELE_VERSION,
        typename: RELEASE_TYPENAME,
        polity: charter.polity.as_str().to_string(),
        issuer: hex::encode(&charter.polity.to_pubkey()?.bytes),
        officer: listing.officer.as_str().to_string(),
        officer_key: hex::encode(&officer_pub.bytes),
        repo: &listing.repo,
        url: &listing.url,
        commit: &listing.commit,
        built_by: listing.built_by.as_ref().map(|b| b.as_str().to_string()),
        authorized_at: listing.authorized_at.as_second(),
    };
    let body_str =
        serde_json::to_string(&body).map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    sign_file(RELEASE_DOMAIN, &body_str, officer_priv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::{DerivedEdge, FoundingProposal, Substrate};
    use en_core::{Alg, Cid, Timestamp};

    const UUID_A: &str = "0f1e2d3c-4b5a-4968-8776-655443322110";
    const UUID_B: &str = "1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9";
    const UUID_C: &str = "2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a";
    const COMMIT: &str = "c3cd66ac5a89e0d1f2a3b4c5d6e7f8091a2b3c4d";

    fn t(secs: i64) -> Timestamp {
        Timestamp::from_second(secs).unwrap()
    }

    fn fixed(seed: u8) -> (PrivKey, PubKey) {
        let sk = PrivKey { alg: Alg::Ed25519, bytes: vec![seed; 32] };
        let pk = keys::public_of(&sk).unwrap();
        (sk, pk)
    }

    fn did(seed: u8) -> DidKey {
        DidKey::from_pubkey(&fixed(seed).1).unwrap()
    }

    fn charter_for(root_pub: &PubKey) -> PolityCharter {
        let polity = DidKey::from_pubkey(root_pub).unwrap();
        let roll: Vec<DidKey> = (10u8..13).map(did).collect();
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

    /// The consent an AT node with key `node_seed` gives for `member_seed`.
    fn consent(issuer: &PubKey, member_seed: u8, uuid: &str, node_seed: u8) -> String {
        let msg = consent_message(&hex::encode(&issuer.bytes), &did(member_seed), uuid);
        hex::encode(&keys::sign(&fixed(node_seed).0, &msg).unwrap().bytes)
    }

    fn team(root: &PubKey) -> Vec<TeamMember> {
        vec![
            TeamMember { did: did(10), at_uuid: UUID_A.into(), consent: consent(root, 10, UUID_A, 30) },
            TeamMember { did: did(11), at_uuid: UUID_B.into(), consent: consent(root, 11, UUID_B, 31) },
            TeamMember { did: did(12), at_uuid: UUID_C.into(), consent: consent(root, 12, UUID_C, 32) },
        ]
    }

    fn release() -> ReleaseState {
        ReleaseState { officers: vec![did(10)], releases: vec![] }
    }

    /// Member 12 is a build machine whose guardian is member 11.
    fn guardians() -> GuardianState {
        GuardianState {
            derivations: vec![DerivedEdge { member: did(12), guardian: Some(did(11)) }],
        }
    }

    fn split(file: &str) -> (String, String) {
        let v: serde_json::Value = serde_json::from_str(file).unwrap();
        (v["body"].as_str().unwrap().to_string(), v["sig"].as_str().unwrap().to_string())
    }

    fn verifies(pk: &PubKey, domain: &str, body: &str, sig: &str) -> bool {
        let mut msg = domain.as_bytes().to_vec();
        msg.extend_from_slice(body.as_bytes());
        let sig = en_core::Signature { alg: Alg::Ed25519, bytes: hex::decode(sig).unwrap() };
        keys::verify(pk, &msg, &sig).is_ok()
    }

    fn listing() -> ReleaseListing {
        ReleaseListing {
            cid: Cid::of_bytes(b"release"),
            officer: did(10),
            repo: "demo".into(),
            url: "http://localhost:3000/stele/demo.git".into(),
            commit: COMMIT.into(),
            built_by: Some(did(12)),
            authorized_at: t(400),
        }
    }

    #[test]
    fn the_roster_is_signed_by_the_root_and_lists_officers_and_guardians() {
        let (sk, pk) = fixed(7);
        let file =
            stele_team_roster(&charter_for(&pk), &team(&pk), &release(), &guardians(), &sk, &pk, 3)
                .unwrap();
        let (body, sig) = split(&file);
        assert!(verifies(&pk, TEAM_DOMAIN, &body, &sig));
        assert!(!verifies(&pk, "at-relay-roster-v1|", &body, &sig));
        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["typename"], "stele-team");
        assert_eq!(b["polity"], did(7).as_str());
        assert_eq!(b["issuer"], hex::encode(&pk.bytes));
        assert_eq!(b["members"].as_array().unwrap().len(), 3);
        assert_eq!(b["release_officers"], serde_json::json!([did(10).as_str()]));
        assert_eq!(b["guardians"][0]["machine"], did(12).as_str());
        assert_eq!(b["guardians"][0]["guardian"], did(11).as_str());
    }

    #[test]
    fn each_consent_is_the_member_nodes_own_signature() {
        let (sk, pk) = fixed(7);
        let file =
            stele_team_roster(&charter_for(&pk), &team(&pk), &release(), &guardians(), &sk, &pk, 3)
                .unwrap();
        let b: serde_json::Value = serde_json::from_str(&split(&file).0).unwrap();
        let issuer = hex::encode(&pk.bytes);
        // Sorted by DID, so find each member's node by its uuid.
        for m in b["members"].as_array().unwrap() {
            let node = match m["at_uuid"].as_str().unwrap() {
                UUID_A => 30u8,
                UUID_B => 31,
                _ => 32,
            };
            let d: DidKey = m["did"].as_str().unwrap().parse().unwrap();
            let msg = consent_message(&issuer, &d, m["at_uuid"].as_str().unwrap());
            let sig = en_core::Signature {
                alg: Alg::Ed25519,
                bytes: hex::decode(m["consent"].as_str().unwrap()).unwrap(),
            };
            keys::verify(&fixed(node).1, &msg, &sig).expect("the node's own key");
            assert!(keys::verify(&pk, &msg, &sig).is_err(), "not the root's");
        }
    }

    #[test]
    fn an_officer_without_a_node_is_refused() {
        let (sk, pk) = fixed(7);
        let mut members = team(&pk);
        members.remove(0);
        assert!(stele_team_roster(&charter_for(&pk), &members, &release(), &guardians(), &sk, &pk, 1)
            .is_err());
    }

    #[test]
    fn a_guardian_edge_with_an_unlisted_end_is_left_out() {
        let (sk, pk) = fixed(7);
        let mut members = team(&pk);
        members.remove(1); // the guardian has no node
        let file =
            stele_team_roster(&charter_for(&pk), &members, &release(), &guardians(), &sk, &pk, 1)
                .unwrap();
        let b: serde_json::Value = serde_json::from_str(&split(&file).0).unwrap();
        assert_eq!(b["guardians"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn malformed_and_duplicate_members_are_refused() {
        let (sk, pk) = fixed(7);
        let c = charter_for(&pk);
        let mut bad_uuid = team(&pk);
        bad_uuid[0].at_uuid = UUID_A.to_uppercase();
        assert!(stele_team_roster(&c, &bad_uuid, &release(), &guardians(), &sk, &pk, 1).is_err());
        let mut bad_consent = team(&pk);
        bad_consent[0].consent = "00".into();
        assert!(stele_team_roster(&c, &bad_consent, &release(), &guardians(), &sk, &pk, 1).is_err());
        let mut twice = team(&pk);
        twice[2].at_uuid = UUID_A.into();
        assert!(stele_team_roster(&c, &twice, &release(), &guardians(), &sk, &pk, 1).is_err());
    }

    #[test]
    fn a_root_key_that_is_not_the_politys_is_refused() {
        let (_, pk) = fixed(7);
        let (osk, opk) = fixed(8);
        assert!(matches!(
            stele_team_roster(&charter_for(&pk), &team(&pk), &release(), &guardians(), &osk, &opk, 1),
            Err(KcError::InvalidDid(_))
        ));
    }

    #[test]
    fn the_release_is_signed_by_its_officer() {
        let (_, pk) = fixed(7);
        let (osk, opk) = fixed(10);
        let file = stele_release_authorization(&charter_for(&pk), &listing(), &osk, &opk).unwrap();
        let (body, sig) = split(&file);
        assert!(verifies(&opk, RELEASE_DOMAIN, &body, &sig));
        assert!(!verifies(&pk, RELEASE_DOMAIN, &body, &sig), "not the root's");
        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["officer"], did(10).as_str());
        assert_eq!(b["officer_key"], hex::encode(&opk.bytes));
        assert_eq!(b["issuer"], hex::encode(&pk.bytes));
        assert_eq!(b["commit"], COMMIT);
        assert_eq!(b["built_by"], did(12).as_str());
        assert_eq!(b["authorized_at"], 400);
        // Somebody else's key cannot export it.
        let (wsk, wpk) = fixed(11);
        assert!(stele_release_authorization(&charter_for(&pk), &listing(), &wsk, &wpk).is_err());
    }

    #[test]
    fn a_release_built_by_nobody_omits_the_field() {
        let (_, pk) = fixed(7);
        let (osk, opk) = fixed(10);
        let mut l = listing();
        l.built_by = None;
        let file = stele_release_authorization(&charter_for(&pk), &l, &osk, &opk).unwrap();
        assert!(!split(&file).0.contains("built_by"));
    }

    /// The exact bytes Stele's C and Python verifiers are held to. ed25519 is
    /// deterministic, so fixed keys give fixed files; a change here is a change to
    /// the wire format and needs Stele changed with it.
    #[test]
    fn the_files_are_byte_pinned() {
        let (sk, pk) = fixed(7);
        let roster =
            stele_team_roster(&charter_for(&pk), &team(&pk), &release(), &guardians(), &sk, &pk, 3)
                .unwrap();
        let (osk, opk) = fixed(10);
        let rel = stele_release_authorization(&charter_for(&pk), &listing(), &osk, &opk).unwrap();
        if std::env::var("EN_PRINT_STELE_VECTOR").is_ok() {
            println!("TEAM_VECTOR={roster}");
            println!("RELEASE_VECTOR={rel}");
            for s in [30u8, 31, 32] {
                println!("NODE{s}={}", hex::encode(&fixed(s).1.bytes));
            }
        }
        assert_eq!(roster, PINNED_TEAM_VECTOR);
        assert_eq!(rel, PINNED_RELEASE_VECTOR);
    }

    /// Cross-checked 2026-10-07 against Stele's own verifiers: the C
    /// stele_team_test and the Python at-stele test_team parse both files,
    /// verify the root's and the officer's signatures, and verify each member's
    /// consent under its node key (seeds 30, 31 and 32).
    const PINNED_TEAM_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-team\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"seq\":3,\"members\":[{\"did\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"key\":\"0b513ad9b4924015ca0902ed079044d3ac5dbec2306f06948c10da8eb6e39f2d\",\"at_uuid\":\"2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a\",\"consent\":\"2aa71ad84c1bbe33f052af7053b75d920692743172936c9bf55ce38b181476689e6ab00381999f08ce4390d0c7500079fb10adf6a2e4e057a84ddf0a368f2f00\"},{\"did\":\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\",\"key\":\"43a72e714401762df66b68c26dfbdf2682aaec9f2474eca4613e424a0fbafd3c\",\"at_uuid\":\"0f1e2d3c-4b5a-4968-8776-655443322110\",\"consent\":\"9d50d0c830b505725bb63884dfa17d999d435dfdd7b8059bbad94bd9cf8ed45b3b5413172c6417b5a4bd8f0cd49598788664562ed9a7ac6530d45be974e6ac07\"},{\"did\":\"did:key:z6MkmNL6xksdEJENGk9Z1qqGsY5njKQdjU1UhnUEUvWyQUVX\",\"key\":\"66be7e332c7a453332bd9d0a7f7db055f5c5ef1a06ada66d98b39fb6810c473a\",\"at_uuid\":\"1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9\",\"consent\":\"b782515541bd3a0abb36bf41ff2d59ff3d17a85b63c312fe6fc2a35639a0ead8f490444496e4499c3675e8aae902ff2763434908542e138a9dacca6efed19f09\"}],\"release_officers\":[\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\"],\"guardians\":[{\"machine\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"guardian\":\"did:key:z6MkmNL6xksdEJENGk9Z1qqGsY5njKQdjU1UhnUEUvWyQUVX\"}]}","sig":"1a3ffdfc52410608b4cfff8a3e242b2ae9086b07808ca210aec56fea53e6b6975276aa4dddc360d39cad16e6357b74bf0815e45fa938b3d84461eeb73c985404"}"#;
    const PINNED_RELEASE_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-release\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"officer\":\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\",\"officer_key\":\"43a72e714401762df66b68c26dfbdf2682aaec9f2474eca4613e424a0fbafd3c\",\"repo\":\"demo\",\"url\":\"http://localhost:3000/stele/demo.git\",\"commit\":\"c3cd66ac5a89e0d1f2a3b4c5d6e7f8091a2b3c4d\",\"built_by\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"authorized_at\":400}","sig":"2d7e4af1f0f40f43f3251996825ecd749a90d3a0d879e7ecd23e64d8dcef98b9bcdc62f8b599aae6635804603f06801c48486bb7b1a90bb9bc2f8b352327d400"}"#;
}
