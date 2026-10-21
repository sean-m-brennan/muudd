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
//! authorizations, in Stele's own file formats (design **D41**); **#11**, the
//! polity's Stele profile (design **D42**); and **#12**, the polity's group anchor
//! (design **D43**).
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
//!
//! **The Stele profile** (#11) is what "safe enough" means to whoever follows the
//! polity, as the roll last decided it ([`en_charter::stele_profile_state`]):
//!
//! ```text
//! sig  = ed25519(polity root key, "stele-profile-v1|" + body)
//! body = {"v":1,"typename":"stele-profile","polity":<did>,"issuer":<hex root pubkey>,
//!         "seq":N,"set_by":<cid>,
//!         "profile":{"name":..,"replicas":N,"tier":N,"require":[..],"ignore":[..],
//!                    "severity_floor":"low"|"medium"|"high","onset_days":N,
//!                    "half_life_days":N,"team":"show"|"worst"}}
//! ```
//!
//! `seq` counts the polity's accepted amendments, so a reader that follows the
//! issuer keeps the highest one. The root key signs only what the roll decided:
//! a polity that never amended its profile has nothing to export.
//!
//! **The group anchor** (#12) is the polity's word about the AutonomousTrust group
//! its work is verified in: which nodes are in it, under which keys, how far each
//! has risen, and which checkpoints the group has signed. A reader in another
//! group cannot check an AT record against keys it does not hold; once it follows
//! the polity, the anchor is where those keys come from.
//!
//! ```text
//! sig  = ed25519(polity root key, "stele-anchor-v1|" + body)
//! body = {"v":1,"typename":"stele-group-anchor","polity":<did>,"issuer":<hex root pubkey>,
//!         "seq":N,"group_uuid":<uuid>,
//!         "members":[{"at_uuid":<uuid>,"pubkey":<hex>,"tier":0..4,"consent":<hex>}, ...],
//!         "checkpoints":[{"group_uuid":<uuid or "">,"epoch":N,"root":<hex>,
//!                         "first_index":N,"count":N}, ...]}
//! ```
//!
//! Each member's `consent` is its AT node signing
//!
//! ```text
//! "stele-anchor-member-v1|" + issuer + "|" + group_uuid + "|" + at_uuid + "|" + pubkey
//! ```
//!
//! with the key the anchor lists, so the root key cannot claim a node, or a key,
//! the node never stood behind. Unlike a team consent this one names its own
//! key, so it is checked here and by every reader. It does not change from one
//! anchor to the next. Tiers and checkpoints are the minting node's roster at the
//! instant of signing; they change by the minute, so the root key reissues the
//! anchor at a rising `seq` without a roll decision, and the roll governs it by
//! governing the root.

use en_charter::{GuardianState, PolityCharter, ReleaseListing, ReleaseState, SteleProfile, SteleProfileState};
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
/// The domain prefix Stele verifies a polity's profile under.
pub const PROFILE_DOMAIN: &str = "stele-profile-v1|";
/// The profile body's `typename`.
pub const PROFILE_TYPENAME: &str = "stele-profile";
/// The format version of the Stele files.
pub const STELE_VERSION: u32 = 1;
/// Most members Stele accepts in one roster.
pub const TEAM_MAX: usize = 256;
/// The domain prefix Stele verifies a group anchor under.
pub const ANCHOR_DOMAIN: &str = "stele-anchor-v1|";
/// The group anchor body's `typename`.
pub const ANCHOR_TYPENAME: &str = "stele-group-anchor";
/// The domain prefix a node signs its consent to an anchor under.
pub const ANCHOR_CONSENT_DOMAIN: &str = "stele-anchor-member-v1|";
/// Most members one group anchor lists.
pub const ANCHOR_MEMBERS_MAX: usize = 256;
/// Most checkpoints one group anchor lists: the newest are kept.
pub const ANCHOR_CHECKPOINTS_MAX: usize = 512;
/// The highest AT tier.
pub const AT_TIER_MAX: u8 = 4;

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
struct ProfileBody<'a> {
    v: u32,
    typename: &'a str,
    polity: String,
    issuer: String,
    seq: u64,
    set_by: String,
    profile: &'a SteleProfile,
}

/// A node as a group anchor lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnchorMember {
    /// The node's AT identity uuid, lowercase and hyphenated.
    pub at_uuid: String,
    /// Hex of the node's AT ed25519 public key.
    pub pubkey: String,
    /// The node's AT tier (0..=4) in the minting node's roster.
    pub tier: u8,
    /// Hex of the node's signature over [`anchor_consent_message`].
    pub consent: String,
}

/// A checkpoint the group signed, as the minting node's roster holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnchorCheckpoint {
    /// The chain's group uuid; empty for the primary chain, whose checkpoint
    /// designation does not bind a group (the keys do).
    pub group_uuid: String,
    pub epoch: u64,
    /// Hex of the window's Merkle root.
    pub root: String,
    pub first_index: u64,
    pub count: u64,
}

#[derive(Serialize)]
struct AnchorBody<'a> {
    v: u32,
    typename: &'a str,
    polity: String,
    issuer: String,
    seq: u64,
    group_uuid: &'a str,
    members: Vec<AnchorMember>,
    checkpoints: Vec<AnchorCheckpoint>,
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

/// The bytes a node signs to consent to being listed, under `pubkey_hex`, in the
/// group anchor the polity with root key `issuer_hex` mints for `group_uuid`.
pub fn anchor_consent_message(issuer_hex: &str, group_uuid: &str, at_uuid: &str,
                              pubkey_hex: &str) -> Vec<u8> {
    format!("{ANCHOR_CONSENT_DOMAIN}{issuer_hex}|{group_uuid}|{at_uuid}|{pubkey_hex}")
        .into_bytes()
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

/// The polity's Stele profile as Stele's signed file (border artifact **#11**).
///
/// `state` must come from [`en_charter::stele_profile_state`], so only a profile
/// the authority pass accepted is exported. Errors if the root key is not the
/// polity's, if the polity has never amended its profile (`stele-default` needs no
/// signature, and a reader already has it), or if the profile is outside Stele's
/// floors.
pub fn stele_profile(
    charter: &PolityCharter,
    state: &SteleProfileState,
    root_priv: &PrivKey,
    root_pub: &PubKey,
) -> KcResult<String> {
    root_guard(charter, root_pub)?;
    let Some(set_by) = state.set_by else {
        return Err(KcError::Authority(format!(
            "polity {} has never amended its Stele profile",
            charter.polity
        )));
    };
    state.profile.check_shape().map_err(KcError::Authority)?;
    let body = ProfileBody {
        v: STELE_VERSION,
        typename: PROFILE_TYPENAME,
        polity: charter.polity.as_str().to_string(),
        issuer: hex::encode(&root_pub.bytes),
        seq: state.seq,
        set_by: set_by.to_string(),
        profile: &state.profile,
    };
    let body_str =
        serde_json::to_string(&body).map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    sign_file(PROFILE_DOMAIN, &body_str, root_priv)
}

/// The polity's group anchor as Stele's signed file (border artifact **#12**).
///
/// `members` and `checkpoints` are the minting node's AT roster at the instant
/// the anchor speaks for. Members are listed by uuid; checkpoints by chain, epoch
/// and window, the newest [`ANCHOR_CHECKPOINTS_MAX`] kept.
///
/// Errors if the root key is not the polity's, if `group_uuid` or a member's
/// uuid, key or consent is malformed, if a node is listed twice, if a tier is past
/// [`AT_TIER_MAX`], if a consent does not verify under the key it is listed with,
/// if a checkpoint root is malformed, or past [`ANCHOR_MEMBERS_MAX`] members. An
/// anchor with no member vouches for nothing and is refused.
pub fn stele_group_anchor(
    charter: &PolityCharter,
    group_uuid: &str,
    members: &[AnchorMember],
    checkpoints: &[AnchorCheckpoint],
    root_priv: &PrivKey,
    root_pub: &PubKey,
    seq: u64,
) -> KcResult<String> {
    root_guard(charter, root_pub)?;
    let issuer = hex::encode(&root_pub.bytes);
    if !is_lower_uuid(group_uuid) {
        return Err(KcError::Authority(format!(
            "group_uuid {group_uuid:?} is not a lowercase hyphenated uuid"
        )));
    }
    if members.is_empty() || members.len() > ANCHOR_MEMBERS_MAX {
        return Err(KcError::Authority(format!(
            "a group anchor lists 1..={ANCHOR_MEMBERS_MAX} members, got {}",
            members.len()
        )));
    }
    let mut listed: Vec<AnchorMember> = Vec::with_capacity(members.len());
    for m in members {
        if !is_lower_uuid(&m.at_uuid) || !is_lower_hex(&m.pubkey, 64)
            || !is_lower_hex(&m.consent, 128)
        {
            return Err(KcError::Authority(format!(
                "member {:?}: the uuid, key or consent is malformed",
                m.at_uuid
            )));
        }
        if m.tier > AT_TIER_MAX {
            return Err(KcError::Authority(format!(
                "member {}: tier {} is not an AT tier (0..={AT_TIER_MAX})",
                m.at_uuid, m.tier
            )));
        }
        if listed.iter().any(|e| e.at_uuid == m.at_uuid || e.pubkey == m.pubkey) {
            return Err(KcError::Authority(format!(
                "node {} or its key is listed twice",
                m.at_uuid
            )));
        }
        let key = PubKey { alg: en_core::Alg::Ed25519, bytes: hex::decode(&m.pubkey)
            .map_err(|e| KcError::Authority(e.to_string()))? };
        let sig = en_core::Signature { alg: en_core::Alg::Ed25519, bytes: hex::decode(&m.consent)
            .map_err(|e| KcError::Authority(e.to_string()))? };
        let msg = anchor_consent_message(&issuer, group_uuid, &m.at_uuid, &m.pubkey);
        if keys::verify(&key, &msg, &sig).is_err() {
            return Err(KcError::Authority(format!(
                "node {} did not consent to this anchor under the key it is listed with",
                m.at_uuid
            )));
        }
        listed.push(m.clone());
    }
    listed.sort_by(|a, b| a.at_uuid.cmp(&b.at_uuid));
    let mut cks: Vec<AnchorCheckpoint> = Vec::with_capacity(checkpoints.len());
    for c in checkpoints {
        if !(c.group_uuid.is_empty() || is_lower_uuid(&c.group_uuid))
            || !is_lower_hex(&c.root, 64)
        {
            return Err(KcError::Authority(format!(
                "checkpoint at epoch {}: the chain uuid or root is malformed",
                c.epoch
            )));
        }
        if !cks.iter().any(|k| (&k.group_uuid, k.epoch, k.first_index, k.count)
            == (&c.group_uuid, c.epoch, c.first_index, c.count))
        {
            cks.push(c.clone());
        }
    }
    cks.sort_by(|a, b| (&a.group_uuid, a.epoch, a.first_index, a.count)
        .cmp(&(&b.group_uuid, b.epoch, b.first_index, b.count)));
    if cks.len() > ANCHOR_CHECKPOINTS_MAX {
        // The newest are what a reader needs; keep them across every chain.
        let mut by_age = cks.clone();
        by_age.sort_by(|a, b| b.epoch.cmp(&a.epoch));
        by_age.truncate(ANCHOR_CHECKPOINTS_MAX);
        cks.retain(|c| by_age.contains(c));
    }
    let body = AnchorBody {
        v: STELE_VERSION,
        typename: ANCHOR_TYPENAME,
        polity: charter.polity.as_str().to_string(),
        issuer,
        seq,
        group_uuid,
        members: listed,
        checkpoints: cks,
    };
    let body_str =
        serde_json::to_string(&body).map_err(|e| KcError::CanonicalEncode(e.to_string()))?;
    sign_file(ANCHOR_DOMAIN, &body_str, root_priv)
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

    fn profile_state() -> SteleProfileState {
        SteleProfileState {
            profile: SteleProfile {
                name: "acme-strict".into(),
                replicas: 3,
                tier: 3,
                require: ["gitleaks:probe".to_string(), "osv-scanner:probe".to_string()]
                    .into_iter()
                    .collect(),
                ignore: ["semgrep:probe".to_string()].into_iter().collect(),
                severity_floor: en_charter::SeverityFloor::Medium,
                onset_days: 1,
                half_life_days: 14,
                team: en_charter::TeamCoupling::Worst,
                deps: en_charter::DepsCoupling::Show,
            },
            set_by: Some(Cid::of_bytes(b"amendment")),
            seq: 2,
        }
    }

    #[test]
    fn the_profile_is_signed_by_the_root_and_carries_seq_and_set_by() {
        let (sk, pk) = fixed(7);
        let file = stele_profile(&charter_for(&pk), &profile_state(), &sk, &pk).unwrap();
        let (body, sig) = split(&file);
        assert!(verifies(&pk, PROFILE_DOMAIN, &body, &sig));
        assert!(!verifies(&pk, TEAM_DOMAIN, &body, &sig), "domain-separated from the roster");
        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["typename"], "stele-profile");
        assert_eq!(b["issuer"], hex::encode(&pk.bytes));
        assert_eq!(b["seq"], 2);
        assert_eq!(b["set_by"], Cid::of_bytes(b"amendment").to_string());
        assert_eq!(b["profile"]["severity_floor"], "medium");
        assert_eq!(b["profile"]["team"], "worst");
        assert_eq!(b["profile"]["require"], serde_json::json!(["gitleaks:probe", "osv-scanner:probe"]));
    }

    #[test]
    fn a_profile_writes_deps_only_when_it_caps() {
        // D42's bytes survive the supply-chain view: a profile that leaves `deps`
        // at show exports exactly as before the field existed (the pinned vector
        // above), and one that caps says so last.
        let (sk, pk) = fixed(7);
        let c = charter_for(&pk);
        let mut st = profile_state();
        let (shown, _) = split(&stele_profile(&c, &st, &sk, &pk).unwrap());
        assert!(!shown.contains("deps"), "{shown}");
        st.profile.deps = en_charter::DepsCoupling::Worst;
        let (worst, _) = split(&stele_profile(&c, &st, &sk, &pk).unwrap());
        assert!(worst.ends_with(r#","team":"worst","deps":"worst"}}"#), "{worst}");
        let b: serde_json::Value = serde_json::from_str(&worst).unwrap();
        let back: SteleProfile = serde_json::from_value(b["profile"].clone()).unwrap();
        assert_eq!(back, st.profile);
        let mut old = b["profile"].clone();
        old.as_object_mut().unwrap().remove("deps");
        let read: SteleProfile = serde_json::from_value(old).unwrap();
        assert_eq!(read.deps, en_charter::DepsCoupling::Show, "absent reads as show");
    }

    #[test]
    fn a_polity_with_no_profile_or_a_bad_one_exports_nothing() {
        let (sk, pk) = fixed(7);
        let c = charter_for(&pk);
        assert!(stele_profile(&c, &SteleProfileState::default(), &sk, &pk).is_err());
        let mut bad = profile_state();
        bad.profile.replicas = 0;
        assert!(stele_profile(&c, &bad, &sk, &pk).is_err());
        let (osk, opk) = fixed(8);
        assert!(matches!(
            stele_profile(&c, &profile_state(), &osk, &opk),
            Err(KcError::InvalidDid(_))
        ));
    }

    /// The exact profile file Stele's Python verifier is held to (viewer
    /// tests/test_stele_profile.py).
    #[test]
    fn the_profile_file_is_byte_pinned() {
        let (sk, pk) = fixed(7);
        let file = stele_profile(&charter_for(&pk), &profile_state(), &sk, &pk).unwrap();
        if std::env::var("EN_PRINT_STELE_VECTOR").is_ok() {
            println!("PROFILE_VECTOR={file}");
        }
        assert_eq!(file, PINNED_PROFILE_VECTOR);
    }

    const PINNED_PROFILE_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-profile\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"seq\":2,\"set_by\":\"b3:992882019a31d7694845b120d9592b7135e9f4876c5a09556cc316fd51858e38\",\"profile\":{\"name\":\"acme-strict\",\"replicas\":3,\"tier\":3,\"require\":[\"gitleaks:probe\",\"osv-scanner:probe\"],\"ignore\":[\"semgrep:probe\"],\"severity_floor\":\"medium\",\"onset_days\":1,\"half_life_days\":14,\"team\":\"worst\"}}","sig":"06104175233204cb6e45c503287ccf40dc3ab17c9f5c950c942cab30a7a78b390c7c56f939fea92a28e33481fc461dcdf2d90b8400f65a127a022860315a1708"}"#;

    /// Cross-checked 2026-10-07 against Stele's own verifiers: the C
    /// stele_team_test and the Python at-stele test_team parse both files,
    /// verify the root's and the officer's signatures, and verify each member's
    /// consent under its node key (seeds 30, 31 and 32).
    const PINNED_TEAM_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-team\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"seq\":3,\"members\":[{\"did\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"key\":\"0b513ad9b4924015ca0902ed079044d3ac5dbec2306f06948c10da8eb6e39f2d\",\"at_uuid\":\"2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a\",\"consent\":\"2aa71ad84c1bbe33f052af7053b75d920692743172936c9bf55ce38b181476689e6ab00381999f08ce4390d0c7500079fb10adf6a2e4e057a84ddf0a368f2f00\"},{\"did\":\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\",\"key\":\"43a72e714401762df66b68c26dfbdf2682aaec9f2474eca4613e424a0fbafd3c\",\"at_uuid\":\"0f1e2d3c-4b5a-4968-8776-655443322110\",\"consent\":\"9d50d0c830b505725bb63884dfa17d999d435dfdd7b8059bbad94bd9cf8ed45b3b5413172c6417b5a4bd8f0cd49598788664562ed9a7ac6530d45be974e6ac07\"},{\"did\":\"did:key:z6MkmNL6xksdEJENGk9Z1qqGsY5njKQdjU1UhnUEUvWyQUVX\",\"key\":\"66be7e332c7a453332bd9d0a7f7db055f5c5ef1a06ada66d98b39fb6810c473a\",\"at_uuid\":\"1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9\",\"consent\":\"b782515541bd3a0abb36bf41ff2d59ff3d17a85b63c312fe6fc2a35639a0ead8f490444496e4499c3675e8aae902ff2763434908542e138a9dacca6efed19f09\"}],\"release_officers\":[\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\"],\"guardians\":[{\"machine\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"guardian\":\"did:key:z6MkmNL6xksdEJENGk9Z1qqGsY5njKQdjU1UhnUEUvWyQUVX\"}]}","sig":"1a3ffdfc52410608b4cfff8a3e242b2ae9086b07808ca210aec56fea53e6b6975276aa4dddc360d39cad16e6357b74bf0815e45fa938b3d84461eeb73c985404"}"#;
    const PINNED_RELEASE_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-release\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"officer\":\"did:key:z6Mkj1MDZKcfx9AX5CeXHdysiGkRLzBbALyFuShD6wNeY1E3\",\"officer_key\":\"43a72e714401762df66b68c26dfbdf2682aaec9f2474eca4613e424a0fbafd3c\",\"repo\":\"demo\",\"url\":\"http://localhost:3000/stele/demo.git\",\"commit\":\"c3cd66ac5a89e0d1f2a3b4c5d6e7f8091a2b3c4d\",\"built_by\":\"did:key:z6MkfDSNRs2i9S6LZ5vd4RbpZ6754H7R3btDqRTeHhamqvuJ\",\"authorized_at\":400}","sig":"2d7e4af1f0f40f43f3251996825ecd749a90d3a0d879e7ecd23e64d8dcef98b9bcdc62f8b599aae6635804603f06801c48486bb7b1a90bb9bc2f8b352327d400"}"#;

    // ---- the group anchor (#12, D43) ----

    const GROUP: &str = "3c4d5e6f-7081-4293-a4b5-c6d7e8f90a1b";

    fn anchor_member(root: &PubKey, uuid: &str, node_seed: u8, tier: u8) -> AnchorMember {
        let pubkey = hex::encode(&fixed(node_seed).1.bytes);
        let msg = anchor_consent_message(&hex::encode(&root.bytes), GROUP, uuid, &pubkey);
        let consent = hex::encode(&keys::sign(&fixed(node_seed).0, &msg).unwrap().bytes);
        AnchorMember { at_uuid: uuid.into(), pubkey, tier, consent }
    }

    fn anchor_members(root: &PubKey) -> Vec<AnchorMember> {
        vec![
            anchor_member(root, UUID_C, 32, 3),
            anchor_member(root, UUID_A, 30, 3),
            anchor_member(root, UUID_B, 31, 1),
        ]
    }

    fn ck(epoch: u64, first: u64, count: u64) -> AnchorCheckpoint {
        AnchorCheckpoint { group_uuid: String::new(), epoch, root: format!("{:064x}", epoch),
                           first_index: first, count }
    }

    #[test]
    fn the_anchor_is_signed_by_the_root_and_lists_consenting_nodes() {
        let (sk, pk) = fixed(7);
        let file = stele_group_anchor(&charter_for(&pk), GROUP, &anchor_members(&pk),
                                      &[ck(2, 0, 7), ck(1, 0, 6), ck(1, 0, 6)], &sk, &pk, 4)
            .unwrap();
        let (body, sig) = split(&file);
        assert!(verifies(&pk, ANCHOR_DOMAIN, &body, &sig));
        assert!(!verifies(&pk, PROFILE_DOMAIN, &body, &sig), "domain-separated");
        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(b["typename"], "stele-group-anchor");
        assert_eq!((b["seq"].as_u64(), b["group_uuid"].as_str()), (Some(4), Some(GROUP)));
        let uuids: Vec<&str> = b["members"].as_array().unwrap().iter()
            .map(|m| m["at_uuid"].as_str().unwrap()).collect();
        assert_eq!(uuids, vec![UUID_A, UUID_B, UUID_C], "sorted by uuid");
        assert_eq!(b["members"][1]["tier"], 1);
        let epochs: Vec<u64> = b["checkpoints"].as_array().unwrap().iter()
            .map(|c| c["epoch"].as_u64().unwrap()).collect();
        assert_eq!(epochs, vec![1, 2], "deduplicated, in order");
        for m in b["members"].as_array().unwrap() {
            let key = PubKey { alg: Alg::Ed25519,
                               bytes: hex::decode(m["pubkey"].as_str().unwrap()).unwrap() };
            let msg = anchor_consent_message(&hex::encode(&pk.bytes), GROUP,
                                             m["at_uuid"].as_str().unwrap(),
                                             m["pubkey"].as_str().unwrap());
            let sig = en_core::Signature { alg: Alg::Ed25519,
                                           bytes: hex::decode(m["consent"].as_str().unwrap()).unwrap() };
            assert!(keys::verify(&key, &msg, &sig).is_ok());
        }
    }

    #[test]
    fn the_root_cannot_list_a_node_that_did_not_consent() {
        let (sk, pk) = fixed(7);
        let c = charter_for(&pk);
        let mut forged = anchor_members(&pk);
        // A consent signed by another node's key, for this node's listing.
        let other = anchor_member(&pk, UUID_A, 33, 3);
        forged[1].consent = other.consent;
        assert!(stele_group_anchor(&c, GROUP, &forged, &[], &sk, &pk, 1).is_err());
        // A consent given to another group does not carry over.
        let mut moved = anchor_members(&pk);
        let elsewhere = anchor_consent_message(&hex::encode(&pk.bytes),
                                               "4d5e6f70-8192-43a4-b5c6-d7e8f90a1b2c",
                                               UUID_A, &moved[1].pubkey);
        moved[1].consent = hex::encode(&keys::sign(&fixed(30).0, &elsewhere).unwrap().bytes);
        assert!(stele_group_anchor(&c, GROUP, &moved, &[], &sk, &pk, 1).is_err());
    }

    #[test]
    fn a_malformed_anchor_is_refused() {
        let (sk, pk) = fixed(7);
        let c = charter_for(&pk);
        let ok = anchor_members(&pk);
        assert!(stele_group_anchor(&c, GROUP, &[], &[], &sk, &pk, 1).is_err(), "no member");
        let mut twice = ok.clone();
        twice.push(ok[0].clone());
        assert!(stele_group_anchor(&c, GROUP, &twice, &[], &sk, &pk, 1).is_err());
        let mut high = ok.clone();
        high[0].tier = 5;
        assert!(stele_group_anchor(&c, GROUP, &high, &[], &sk, &pk, 1).is_err());
        assert!(stele_group_anchor(&c, "NOT-A-UUID", &ok, &[], &sk, &pk, 1).is_err());
        let mut bad_root = ck(1, 0, 6);
        bad_root.root = "xyz".into();
        assert!(stele_group_anchor(&c, GROUP, &ok, &[bad_root], &sk, &pk, 1).is_err());
        let (osk, opk) = fixed(8);
        assert!(stele_group_anchor(&c, GROUP, &ok, &[], &osk, &opk, 1).is_err(), "not the root");
    }

    #[test]
    fn an_anchor_keeps_the_newest_checkpoints() {
        let (sk, pk) = fixed(7);
        let many: Vec<AnchorCheckpoint> =
            (1..=(ANCHOR_CHECKPOINTS_MAX as u64 + 10)).map(|e| ck(e, 0, e)).collect();
        let file = stele_group_anchor(&charter_for(&pk), GROUP, &anchor_members(&pk), &many,
                                      &sk, &pk, 1).unwrap();
        let b: serde_json::Value = serde_json::from_str(&split(&file).0).unwrap();
        let cks = b["checkpoints"].as_array().unwrap();
        assert_eq!(cks.len(), ANCHOR_CHECKPOINTS_MAX);
        assert_eq!(cks[0]["epoch"].as_u64(), Some(11), "the oldest ten dropped");
    }

    #[test]
    fn the_anchor_file_is_byte_pinned() {
        let (sk, pk) = fixed(7);
        let file = stele_group_anchor(&charter_for(&pk), GROUP, &anchor_members(&pk),
                                      &[ck(1, 0, 6), ck(2, 0, 7)], &sk, &pk, 4).unwrap();
        if std::env::var("EN_PRINT_STELE_VECTOR").is_ok() {
            println!("ANCHOR_VECTOR={file}");
        }
        assert_eq!(file, PINNED_ANCHOR_VECTOR);
    }

    const PINNED_ANCHOR_VECTOR: &str = r#"{"body":"{\"v\":1,\"typename\":\"stele-group-anchor\",\"polity\":\"did:key:z6MkvDqGT54cXesYGvABpF1UapVNwjCqRcafi4Px6Thv5T3Z\",\"issuer\":\"ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c\",\"seq\":4,\"group_uuid\":\"3c4d5e6f-7081-4293-a4b5-c6d7e8f90a1b\",\"members\":[{\"at_uuid\":\"0f1e2d3c-4b5a-4968-8776-655443322110\",\"pubkey\":\"acdb0e29743f0ccb8686d0a104cb96e05abefec1538765e7595869f7dc8c49aa\",\"tier\":3,\"consent\":\"7ae992f006cb1cb44cc18ada695fd4e03e522b5feab21b4db73a520118b03508054ad59b3bd4beaebd02edcaaf23cc814ebc568b252904528647564dc19b0f0a\"},{\"at_uuid\":\"1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9\",\"pubkey\":\"43046bfe4092b3e94994eada15dcc20d8aaa07b658fd3954eb8e0efb8bdca5de\",\"tier\":1,\"consent\":\"06426d6054275e4d3bf48240ce471af44cf4aa42a4dd58415b646b5ac6d6fa6c3279173a39346e2aac3612cdf8a454fd3753b09b6dce8d56c6a56cb83b9a6001\"},{\"at_uuid\":\"2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a\",\"pubkey\":\"4ed32f63bf35f0eeefcb25f28a2e1fbdc873ae2835671b0c9460f5f12e4556a8\",\"tier\":3,\"consent\":\"bf3475307402da84aca9d15b3101dafe58eb5629f49066fbc70743268cd864602d5b5d3ed5dfaa425fa4a5d75fd66461f1dbc49ba69bb90b130c9483cfdb2202\"}],\"checkpoints\":[{\"group_uuid\":\"\",\"epoch\":1,\"root\":\"0000000000000000000000000000000000000000000000000000000000000001\",\"first_index\":0,\"count\":6},{\"group_uuid\":\"\",\"epoch\":2,\"root\":\"0000000000000000000000000000000000000000000000000000000000000002\",\"first_index\":0,\"count\":7}]}","sig":"184f26c56101d5a888472c05f01282cc53fe6e8ff3aa8bf2e3b2bab7587d100e572e925176ee79c6bab7f43e3b096b88cc6f15e21b8deb391a743e0b00b54109"}"#;

}
