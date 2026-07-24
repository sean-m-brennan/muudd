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
use en_core::{keys, Alg, DidKey, PrivKey, Signed, Timestamp};

use crate::*;

fn t(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("timestamp")
}

fn id() -> (PrivKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    (sk, DidKey::from_pubkey(&pk).unwrap())
}

fn commit(k: &str) -> Commitment {
    Commitment { key: k.into(), description: k.into() }
}

fn substrate() -> Substrate {
    Substrate { name: "tradition".into(), statement: "answers to X above itself".into(), anchor: None }
}

/// A fixed set of member identities plus the polity's own DID, for building
/// charter and amendment entries by member index.
struct Polity {
    polity: DidKey,
    ids: Vec<(PrivKey, DidKey)>,
}

impl Polity {
    fn new(n: usize) -> Self {
        Polity { polity: id().1, ids: (0..n).map(|_| id()).collect() }
    }

    fn roll(&self) -> Vec<DidKey> {
        self.ids.iter().map(|(_, d)| d.clone()).collect()
    }

    fn signers<'a>(&'a self, idxs: &[usize]) -> Vec<(&'a str, &'a PrivKey)> {
        idxs.iter().map(|&i| (self.ids[i].1.as_str(), &self.ids[i].0)).collect()
    }

    fn dids(&self, idxs: &[usize]) -> Vec<DidKey> {
        idxs.iter().map(|&i| self.ids[i].1.clone()).collect()
    }

    /// A charter whose ratifiers/signers are the members at `ratifiers`, signed
    /// consistently, with the given founding commitments.
    fn charter(&self, ratifiers: &[usize], founding: Vec<Commitment>) -> CharterEntry {
        let prop = FoundingProposal {
            polity: self.polity.clone(),
            substrate: substrate(),
            founding_commitments: founding,
            proposed_members: self.roll(),
            proposed_at: t(50),
        };
        let charter = prop.ratify(self.dids(ratifiers), t(100)).unwrap();
        CharterEntry::PolityChartered(Signed::build(charter, &self.signers(ratifiers)).unwrap())
    }

    fn amend(&self, kind: AmendmentKind, key: &str, deciders: &[usize], at: i64) -> CharterEntry {
        let am = CommitmentAmendment {
            polity: self.polity.clone(),
            kind,
            commitment: commit(key),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        CharterEntry::CommitmentAmended(Signed::build(am, &self.signers(deciders)).unwrap())
    }

    fn dissolve(&self, successors: &[DidKey], deciders: &[usize], at: i64) -> CharterEntry {
        let d = CharterDissolution {
            polity: self.polity.clone(),
            successors: successors.to_vec(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            dissolved_at: t(at),
        };
        CharterEntry::CharterDissolved(Signed::build(d, &self.signers(deciders)).unwrap())
    }

    /// Admit `joiner` (an identity outside the current roll): the `deciders`
    /// co-sign, and the joiner co-signs its own consent.
    fn admit(&self, joiner: &(PrivKey, DidKey), deciders: &[usize], at: i64) -> CharterEntry {
        let mc = MembershipChange {
            polity: self.polity.clone(),
            kind: MembershipKind::Admit,
            member: joiner.1.clone(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        let mut signers = self.signers(deciders);
        signers.push((joiner.1.as_str(), &joiner.0));
        CharterEntry::MembershipChanged(Signed::build(mc, &signers).unwrap())
    }

    /// Member `member` leaves of their own accord (self-signed, no deciders).
    fn depart(&self, member: usize, at: i64) -> CharterEntry {
        let mc = MembershipChange {
            polity: self.polity.clone(),
            kind: MembershipKind::DepartVoluntary,
            member: self.ids[member].1.clone(),
            rationale: "r".into(),
            deciders: vec![],
            decided_at: t(at),
        };
        CharterEntry::MembershipChanged(Signed::build(mc, &self.signers(&[member])).unwrap())
    }

    /// Expel member `target`: the `deciders` (which must exclude the target)
    /// co-sign at a majority + 1.
    fn expel(&self, target: usize, deciders: &[usize], at: i64) -> CharterEntry {
        let mc = MembershipChange {
            polity: self.polity.clone(),
            kind: MembershipKind::Expel,
            member: self.ids[target].1.clone(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        CharterEntry::MembershipChanged(Signed::build(mc, &self.signers(deciders)).unwrap())
    }

    /// Confer `office` on member `holder`: the `deciders` co-sign at a majority and
    /// the holder co-signs consent.
    fn confer(&self, office: Office, holder: usize, deciders: &[usize], at: i64) -> CharterEntry {
        let oc = OfficeChange {
            polity: self.polity.clone(),
            kind: OfficeKind::Confer,
            office,
            holder: self.ids[holder].1.clone(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        let mut signers = self.signers(deciders);
        if !deciders.contains(&holder) {
            signers.push((self.ids[holder].1.as_str(), &self.ids[holder].0));
        }
        CharterEntry::OfficeChanged(Signed::build(oc, &signers).unwrap())
    }

    /// Member `holder` resigns `office` (self-signed, no deciders).
    fn resign(&self, office: Office, holder: usize, at: i64) -> CharterEntry {
        let oc = OfficeChange {
            polity: self.polity.clone(),
            kind: OfficeKind::Resign,
            office,
            holder: self.ids[holder].1.clone(),
            rationale: "r".into(),
            deciders: vec![],
            decided_at: t(at),
        };
        CharterEntry::OfficeChanged(Signed::build(oc, &self.signers(&[holder])).unwrap())
    }

    /// Revoke `office` from member `holder`: the `deciders` (which must exclude the
    /// holder) co-sign at a majority.
    fn revoke_office(&self, office: Office, holder: usize, deciders: &[usize], at: i64) -> CharterEntry {
        let oc = OfficeChange {
            polity: self.polity.clone(),
            kind: OfficeKind::Revoke,
            office,
            holder: self.ids[holder].1.clone(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        CharterEntry::OfficeChanged(Signed::build(oc, &self.signers(deciders)).unwrap())
    }

    /// An affirmation with an explicit signer set, for the rejection cases.
    fn affirm(
        &self,
        member: &DidKey,
        kind: AffirmationKind,
        witnesses: &[usize],
        at: i64,
        signers: &[(&str, &PrivKey)],
    ) -> CharterEntry {
        let af = Affirmation {
            polity: self.polity.clone(),
            member: member.clone(),
            kind,
            witnesses: self.dids(witnesses),
            affirmed_at: t(at),
        };
        CharterEntry::MemberAffirmed(Signed::build(af, signers).unwrap())
    }

    /// Induct `member` — `&p.ids[i]` for a founding member, or an outside identity
    /// already admitted to the roll. The roll `witnesses` co-sign and the inductee
    /// co-signs its own participation.
    fn induct(&self, member: &(PrivKey, DidKey), witnesses: &[usize], at: i64) -> CharterEntry {
        let mut signers = self.signers(witnesses);
        if !signers.iter().any(|(d, _)| *d == member.1.as_str()) {
            signers.push((member.1.as_str(), &member.0));
        }
        self.affirm(&member.1, AffirmationKind::Induction, witnesses, at, &signers)
    }

    /// `member` renews their affirmation (self-signed, no witnesses).
    fn reaffirm(&self, member: &(PrivKey, DidKey), at: i64) -> CharterEntry {
        let signers = [(member.1.as_str(), &member.0)];
        self.affirm(&member.1, AffirmationKind::ReAffirmation, &[], at, &signers)
    }
}

fn reasons(archive: &Archive<CharterEntry>) -> Vec<String> {
    authority_violations(archive).into_iter().map(|(_, m)| m).collect()
}

#[test]
fn founding_ratified_by_majority_verifies_authorizes_and_derives_state() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("mutual-aid")])).unwrap();

    verify_all(&a).expect("signatures valid");
    check_authority(&a).expect("majority (2 of 3) ratified");

    let st = charter_state(&a, &p.polity, t(300));
    assert!(st.chartered);
    assert_eq!(st.members.len(), 3);
    assert_eq!(st.founded_at, Some(t(100)));
    assert!(st.substrate.is_some());
    let keys: Vec<&str> = st.commitments.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, vec!["mutual-aid"]);
}

#[test]
fn ratify_rejects_non_member_ratifier() {
    let p = Polity::new(3);
    let stranger = id().1;
    let prop = FoundingProposal {
        polity: p.polity.clone(),
        substrate: substrate(),
        founding_commitments: vec![],
        proposed_members: p.roll(),
        proposed_at: t(50),
    };
    let err = prop.ratify(vec![stranger], t(100)).unwrap_err();
    assert!(matches!(err, en_core::KcError::Authority(_)));
}

#[test]
fn sub_majority_ratification_is_unauthorized() {
    // 1 of 3 ratifies: signatures are valid but the majority bar fails.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0], vec![])).unwrap();
    verify_all(&a).expect("the single signature is valid");
    assert!(reasons(&a).iter().any(|m| m.contains("majority")));
}

#[test]
fn second_charter_for_same_polity_is_singleton_violation() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    // A distinct payload (different founding) → distinct CID → appends, but the
    // authority pass rejects the second genesis.
    a.append(p.charter(&[0, 1], vec![commit("b")])).unwrap();
    verify_all(&a).expect("both are validly signed");
    assert!(reasons(&a).iter().any(|m| m.contains("singleton")));
}

#[test]
fn charter_signers_must_be_exactly_the_ratifiers() {
    // Claim ratifiers {0,1} but co-sign with an extra member {0,1,2}.
    let p = Polity::new(3);
    let prop = FoundingProposal {
        polity: p.polity.clone(),
        substrate: substrate(),
        founding_commitments: vec![],
        proposed_members: p.roll(),
        proposed_at: t(50),
    };
    let charter = prop.ratify(p.dids(&[0, 1]), t(100)).unwrap();
    let entry = CharterEntry::PolityChartered(Signed::build(charter, &p.signers(&[0, 1, 2])).unwrap());
    let mut a = Archive::new();
    a.append(entry).unwrap();
    verify_all(&a).expect("all three signatures are individually valid");
    assert!(reasons(&a).iter().any(|m| m.contains("signers must be exactly the ratifiers")));
}

#[test]
fn add_commitment_at_majority_then_state_reflects_it() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 200)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("Add at majority is authorized");

    let mut keys: Vec<String> =
        charter_state(&a, &p.polity, t(300)).commitments.iter().map(|c| c.key.clone()).collect();
    keys.sort();
    assert_eq!(keys, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn remove_needs_more_than_a_majority() {
    // Reject: majority (2 of 3) is not enough to remove.
    let p = Polity::new(3);
    let mut reject = Archive::new();
    reject.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    reject.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 200)).unwrap();
    reject.append(p.amend(AmendmentKind::Remove, "b", &[0, 1], 300)).unwrap();
    assert!(reasons(&reject).iter().any(|m| m.contains("Remove needs more than a majority")));

    // Accept (separate archive): all three remove it.
    let mut accept = Archive::new();
    accept.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    accept.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 200)).unwrap();
    accept.append(p.amend(AmendmentKind::Remove, "b", &[0, 1, 2], 300)).unwrap();
    verify_all(&accept).unwrap();
    check_authority(&accept).expect("Remove at majority+1 is authorized");
    let keys: Vec<String> =
        charter_state(&accept, &p.polity, t(400)).commitments.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, vec!["a".to_string()]);
}

#[test]
fn founding_commitment_is_genesis_immutable() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    // Even with everyone deciding, a founding commitment cannot be removed.
    a.append(p.amend(AmendmentKind::Remove, "a", &[0, 1, 2], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("genesis-immutable")));
}

#[test]
fn add_of_in_force_and_remove_of_absent_are_rejected() {
    let p = Polity::new(3);

    let mut add_dup = Archive::new();
    add_dup.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    add_dup.append(p.amend(AmendmentKind::Add, "a", &[0, 1], 200)).unwrap();
    assert!(reasons(&add_dup).iter().any(|m| m.contains("already in-force")));

    let mut rm_absent = Archive::new();
    rm_absent.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    rm_absent.append(p.amend(AmendmentKind::Remove, "z", &[0, 1, 2], 200)).unwrap();
    assert!(reasons(&rm_absent).iter().any(|m| m.contains("absent commitment")));
}

#[test]
fn amendment_before_charter_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn charter_state_of_unknown_polity_is_empty() {
    let p = Polity::new(3);
    let other = id().1;
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    let st = charter_state(&a, &other, t(300));
    assert!(!st.chartered);
    assert!(st.substrate.is_none());
    assert!(st.commitments.is_empty());
}

#[test]
fn dissolution_by_majority_verifies_authorizes_and_winds_up_state() {
    let p = Polity::new(3);
    let succ_a = id().1;
    let succ_b = id().1;
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(p.dissolve(&[succ_a.clone(), succ_b.clone()], &[0, 1], 300)).unwrap();

    verify_all(&a).expect("signatures valid");
    check_authority(&a).expect("majority (2 of 3) dissolved");

    let st = charter_state(&a, &p.polity, t(400));
    assert!(st.chartered);
    assert!(st.dissolved);
    assert_eq!(st.dissolved_at, Some(t(300)));
    assert_eq!(st.successors, vec![succ_a, succ_b]);
}

#[test]
fn dissolution_is_invisible_before_its_time() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 300)).unwrap();
    // As of a time before the dissolution, the polity is still live.
    let st = charter_state(&a, &p.polity, t(200));
    assert!(st.chartered);
    assert!(!st.dissolved);
    assert!(st.successors.is_empty());
}

#[test]
fn sub_majority_dissolution_is_unauthorized() {
    // 1 of 3 decides: signatures are valid but the majority bar fails.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(p.dissolve(&[], &[0], 300)).unwrap();
    verify_all(&a).expect("the single signature is valid");
    assert!(reasons(&a).iter().any(|m| m.contains("dissolution needs a majority")));
}

#[test]
fn dissolution_signers_must_be_exactly_the_deciders() {
    // Claim deciders {0,1} but co-sign with an extra member {0,1,2}.
    let p = Polity::new(3);
    let d = CharterDissolution {
        polity: p.polity.clone(),
        successors: vec![],
        rationale: "r".into(),
        deciders: p.dids(&[0, 1]),
        dissolved_at: t(300),
    };
    let entry = CharterEntry::CharterDissolved(Signed::build(d, &p.signers(&[0, 1, 2])).unwrap());
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(entry).unwrap();
    verify_all(&a).expect("all three signatures are individually valid");
    assert!(reasons(&a).iter().any(|m| m.contains("dissolution signers must be exactly the deciders")));
}

#[test]
fn dissolution_before_charter_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.dissolve(&[], &[0, 1], 300)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn admit_at_majority_with_consent_grows_the_roll() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap(); // majority(3) = 2

    verify_all(&a).expect("deciders + joiner all signed");
    check_authority(&a).expect("Admit at majority with consent is authorized");

    let st = charter_state(&a, &p.polity, t(300));
    assert_eq!(st.members.len(), 4);
    assert!(st.members.contains(&joiner.1));
}

#[test]
fn admit_makes_the_roll_live_for_later_quorums() {
    // The ripple test: once a 4th member is admitted, a commitment Add that met
    // the old majority (2 of 3) now fails the grown roll's majority (3 of 4).
    let p = Polity::new(3);
    let joiner = id();

    let mut reject = Archive::new();
    reject.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    reject.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    reject.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 300)).unwrap();
    verify_all(&reject).unwrap();
    assert!(reasons(&reject).iter().any(|m| m.contains("Add needs a majority (>= 3)")));

    // With three of the now-four members, the Add is authorized.
    let mut accept = Archive::new();
    accept.append(p.charter(&[0, 1], vec![commit("a")])).unwrap();
    accept.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    accept.append(p.amend(AmendmentKind::Add, "b", &[0, 1, 2], 300)).unwrap();
    verify_all(&accept).unwrap();
    check_authority(&accept).expect("Add at the grown-roll majority is authorized");
}

#[test]
fn admit_rejections() {
    let p = Polity::new(3);
    let joiner = id();

    // Sub-majority deciders.
    let mut sub = Archive::new();
    sub.append(p.charter(&[0, 1], vec![])).unwrap();
    sub.append(p.admit(&joiner, &[0], 200)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("Admit needs a majority (>= 2)")));

    // Already a current member (pass an existing member as the "joiner").
    let mut dup = Archive::new();
    dup.append(p.charter(&[0, 1], vec![])).unwrap();
    dup.append(p.admit(&p.ids[2], &[0, 1], 200)).unwrap();
    assert!(reasons(&dup).iter().any(|m| m.contains("already-current member")));

    // Joiner did not co-sign (signers = deciders only).
    let mc = MembershipChange {
        polity: p.polity.clone(),
        kind: MembershipKind::Admit,
        member: joiner.1.clone(),
        rationale: "r".into(),
        deciders: p.dids(&[0, 1]),
        decided_at: t(200),
    };
    let no_consent = CharterEntry::MembershipChanged(Signed::build(mc, &p.signers(&[0, 1])).unwrap());
    let mut nc = Archive::new();
    nc.append(p.charter(&[0, 1], vec![])).unwrap();
    nc.append(no_consent).unwrap();
    verify_all(&nc).expect("the deciders' signatures are valid");
    assert!(reasons(&nc).iter().any(|m| m.contains("deciders plus the joiner")));
}

#[test]
fn voluntary_departure_self_signed_shrinks_the_roll() {
    // From a roll of 4, a member may leave (→ 3, which can still self-govern).
    let p = Polity::new(4);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![commit("a")])).unwrap(); // majority(4) = 3
    a.append(p.depart(3, 200)).unwrap();

    verify_all(&a).expect("the departing member's signature is valid");
    check_authority(&a).expect("voluntary departure from a roll of 4 is authorized");

    let st = charter_state(&a, &p.polity, t(300));
    assert_eq!(st.members.len(), 3);
    assert!(!st.members.contains(&p.ids[3].1));
}

#[test]
fn voluntary_departure_rejections() {
    // Self-govern floor: from a roll of 3, departure (→ 2) is refused.
    let p3 = Polity::new(3);
    let mut floor = Archive::new();
    floor.append(p3.charter(&[0, 1], vec![])).unwrap();
    floor.append(p3.depart(2, 200)).unwrap();
    assert!(reasons(&floor).iter().any(|m| m.contains("unable to self-govern")));

    // Signed by someone other than the departing member.
    let p4 = Polity::new(4);
    let mc = MembershipChange {
        polity: p4.polity.clone(),
        kind: MembershipKind::DepartVoluntary,
        member: p4.ids[3].1.clone(),
        rationale: "r".into(),
        deciders: vec![],
        decided_at: t(200),
    };
    let wrong_signer = CharterEntry::MembershipChanged(Signed::build(mc, &p4.signers(&[0])).unwrap());
    let mut ws = Archive::new();
    ws.append(p4.charter(&[0, 1, 2], vec![])).unwrap();
    ws.append(wrong_signer).unwrap();
    verify_all(&ws).expect("the signature itself is valid");
    assert!(reasons(&ws).iter().any(|m| m.contains("exactly the departing member")));
}

#[test]
fn expel_at_majority_plus_one_shrinks_the_roll() {
    // Roll of 5; majority(5) + 1 = 4 deciders, target excluded.
    let p = Polity::new(5);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.expel(4, &[0, 1, 2, 3], 200)).unwrap();

    verify_all(&a).unwrap();
    check_authority(&a).expect("Expel at majority+1 with the target excluded is authorized");

    let st = charter_state(&a, &p.polity, t(300));
    assert_eq!(st.members.len(), 4);
    assert!(!st.members.contains(&p.ids[4].1));
}

#[test]
fn expel_rejections() {
    let p = Polity::new(5);

    // Only a bare majority (3 of 5) — Expel needs majority + 1 = 4.
    let mut sub = Archive::new();
    sub.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    sub.append(p.expel(4, &[0, 1, 2], 200)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("Expel needs more than a majority (>= 4)")));

    // The target cannot be among the deciders.
    let mut selfexpel = Archive::new();
    selfexpel.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    selfexpel.append(p.expel(4, &[0, 1, 3, 4], 200)).unwrap();
    assert!(reasons(&selfexpel).iter().any(|m| m.contains("cannot be among the deciders")));
}

#[test]
fn membership_change_before_charter_is_rejected() {
    let p = Polity::new(4);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn no_membership_change_or_amendment_after_dissolution() {
    let p = Polity::new(5);
    let joiner = id();

    // Admit after dissolution is rejected.
    let mut adm = Archive::new();
    adm.append(p.charter(&[0, 1, 2], vec!["a"].into_iter().map(commit).collect())).unwrap();
    adm.append(p.dissolve(&[], &[0, 1, 2], 200)).unwrap();
    adm.append(p.admit(&joiner, &[0, 1, 2], 300)).unwrap();
    verify_all(&adm).unwrap();
    assert!(reasons(&adm).iter().any(|m| m.contains("after its dissolution")));

    // A commitment amendment after dissolution is likewise rejected (hardening
    // also closes this pre-existing gap), as is a second dissolution.
    let mut sec = Archive::new();
    sec.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    sec.append(p.dissolve(&[], &[0, 1, 2], 200)).unwrap();
    sec.append(p.dissolve(&[], &[0, 1, 2], 300)).unwrap();
    assert!(reasons(&sec).iter().any(|m| m.contains("already dissolved")));
}

#[test]
fn dissolution_majority_uses_the_live_roll() {
    // Grow the roll to 4, then a 2-of-4 dissolution is sub-majority (needs 3).
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("dissolution needs a majority of the roll (>= 3)")));
}

#[test]
fn confer_at_majority_with_consent_grants_the_office() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap(); // majority(3) = 2, holder 2 consents

    verify_all(&a).expect("deciders + holder signed");
    check_authority(&a).expect("Confer at majority with consent is authorized");

    let os = office_state(&a, &p.polity, t(300));
    assert!(os.holds(&p.ids[2].1, &Office::Convener));
    assert_eq!(os.holders(&Office::Convener), vec![&p.ids[2].1]);
    assert_eq!(os.offices_of(&p.ids[2].1), vec![&Office::Convener]);
}

#[test]
fn resign_lapses_the_office() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 2, &[0, 1], 200)).unwrap();
    a.append(p.resign(Office::CharterKeeper, 2, 300)).unwrap();

    verify_all(&a).unwrap();
    check_authority(&a).expect("a self-signed resignation is authorized");
    assert!(!office_state(&a, &p.polity, t(400)).holds(&p.ids[2].1, &Office::CharterKeeper));
}

#[test]
fn revoke_at_majority_removes_the_office() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    a.append(p.revoke_office(Office::Convener, 2, &[0, 1], 300)).unwrap(); // holder 2 excluded

    verify_all(&a).unwrap();
    check_authority(&a).expect("Revoke at majority with the holder excluded is authorized");
    assert!(!office_state(&a, &p.polity, t(400)).holds(&p.ids[2].1, &Office::Convener));
}

#[test]
fn confer_rejections() {
    let p = Polity::new(3);

    // Sub-majority deciders.
    let mut sub = Archive::new();
    sub.append(p.charter(&[0, 1], vec![])).unwrap();
    sub.append(p.confer(Office::Convener, 2, &[0], 200)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("conferral needs a majority (>= 2)")));

    // Already holds the office.
    let mut dup = Archive::new();
    dup.append(p.charter(&[0, 1], vec![])).unwrap();
    dup.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    dup.append(p.confer(Office::Convener, 2, &[0, 1], 300)).unwrap();
    assert!(reasons(&dup).iter().any(|m| m.contains("already held")));

    // Holder did not co-sign consent (signers = deciders only).
    let oc = OfficeChange {
        polity: p.polity.clone(),
        kind: OfficeKind::Confer,
        office: Office::Convener,
        holder: p.ids[2].1.clone(),
        rationale: "r".into(),
        deciders: p.dids(&[0, 1]),
        decided_at: t(200),
    };
    let no_consent = CharterEntry::OfficeChanged(Signed::build(oc, &p.signers(&[0, 1])).unwrap());
    let mut nc = Archive::new();
    nc.append(p.charter(&[0, 1], vec![])).unwrap();
    nc.append(no_consent).unwrap();
    verify_all(&nc).expect("the deciders' signatures are valid");
    assert!(reasons(&nc).iter().any(|m| m.contains("plus the holder (consent)")));
}

#[test]
fn confer_on_a_non_member_is_rejected() {
    let p = Polity::new(3);
    let outsider = id();
    let oc = OfficeChange {
        polity: p.polity.clone(),
        kind: OfficeKind::Confer,
        office: Office::Convener,
        holder: outsider.1.clone(),
        rationale: "r".into(),
        deciders: p.dids(&[0, 1]),
        decided_at: t(200),
    };
    let mut signers = p.signers(&[0, 1]);
    signers.push((outsider.1.as_str(), &outsider.0));
    let entry = CharterEntry::OfficeChanged(Signed::build(oc, &signers).unwrap());
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(entry).unwrap();
    verify_all(&a).expect("signatures valid");
    assert!(reasons(&a).iter().any(|m| m.contains("non-member")));
}

#[test]
fn resign_rejections() {
    let p = Polity::new(3);

    // Resigning an office not held.
    let mut nh = Archive::new();
    nh.append(p.charter(&[0, 1], vec![])).unwrap();
    nh.append(p.resign(Office::Convener, 2, 200)).unwrap();
    assert!(reasons(&nh).iter().any(|m| m.contains("not held")));

    // Signed by someone other than the holder.
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    let oc = OfficeChange {
        polity: p.polity.clone(),
        kind: OfficeKind::Resign,
        office: Office::Convener,
        holder: p.ids[2].1.clone(),
        rationale: "r".into(),
        deciders: vec![],
        decided_at: t(300),
    };
    let wrong = CharterEntry::OfficeChanged(Signed::build(oc, &p.signers(&[0])).unwrap());
    a.append(wrong).unwrap();
    verify_all(&a).expect("signature valid");
    assert!(reasons(&a).iter().any(|m| m.contains("exactly the resigning holder")));
}

#[test]
fn revoke_rejections() {
    let p = Polity::new(3);

    // Sub-majority.
    let mut sub = Archive::new();
    sub.append(p.charter(&[0, 1], vec![])).unwrap();
    sub.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    sub.append(p.revoke_office(Office::Convener, 2, &[0], 300)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("revocation needs a majority (>= 2)")));

    // The holder cannot be among the deciders of their own revocation.
    let mut selfrev = Archive::new();
    selfrev.append(p.charter(&[0, 1], vec![])).unwrap();
    selfrev.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    selfrev.append(p.revoke_office(Office::Convener, 2, &[0, 2], 300)).unwrap();
    assert!(reasons(&selfrev).iter().any(|m| m.contains("cannot be among the deciders")));
}

#[test]
fn an_office_lapses_when_its_holder_leaves_the_roll() {
    // Roll of 4 so the departure clears the self-govern floor.
    let p = Polity::new(4);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap(); // majority(4) = 3
    a.append(p.confer(Office::CharterKeeper, 3, &[0, 1, 2], 200)).unwrap();
    assert!(office_state(&a, &p.polity, t(250)).holds(&p.ids[3].1, &Office::CharterKeeper));

    a.append(p.depart(3, 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    // The office lapses once its holder is off the roll.
    assert!(!office_state(&a, &p.polity, t(400)).holds(&p.ids[3].1, &Office::CharterKeeper));
}

#[test]
fn multiple_holders_and_a_custom_office() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // Two co-conveners.
    a.append(p.confer(Office::Convener, 0, &[0, 1], 200)).unwrap();
    a.append(p.confer(Office::Convener, 1, &[0, 1], 210)).unwrap();
    // A polity-specific office.
    a.append(p.confer(Office::Other("quartermaster".into()), 2, &[0, 1], 220)).unwrap();

    verify_all(&a).unwrap();
    check_authority(&a).expect("all conferrals authorized");
    let os = office_state(&a, &p.polity, t(300));
    assert_eq!(os.holders(&Office::Convener).len(), 2);
    assert!(os.holds(&p.ids[2].1, &Office::Other("quartermaster".into())));
}

#[test]
fn office_change_before_charter_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn office_change_after_dissolution_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 200)).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("after its dissolution")));
}

#[test]
fn founding_members_are_inducted_at_founding() {
    // Ratifying the charter *is* their induction — no record needed.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();

    let st = affirmation_state(&a, &p.polity, t(300));
    assert_eq!(st.entries.len(), 3);
    assert!(st.entries.iter().all(|e| e.inducted));
    assert!(st.entries.iter().all(|e| e.last_affirmed_at == Some(t(100))));
    assert!(st.is_inducted(&p.ids[2].1));
    assert_eq!(st.last_affirmed(&p.ids[0].1), Some(t(100)));
}

#[test]
fn an_admitted_member_is_uninducted_until_the_roll_receives_them() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();

    // On the roll, but the substrate has not been transmitted yet.
    let before = affirmation_state(&a, &p.polity, t(250));
    assert_eq!(before.entries.len(), 4);
    assert!(!before.is_inducted(&joiner.1));
    assert_eq!(before.last_affirmed(&joiner.1), None);

    // Roll of 4 now, so induction needs 3 witnesses.
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    verify_all(&a).expect("signatures valid");
    check_authority(&a).expect("roll-witnessed induction authorized");

    let after = affirmation_state(&a, &p.polity, t(350));
    assert!(after.is_inducted(&joiner.1));
    assert_eq!(after.last_affirmed(&joiner.1), Some(t(300)));

    // A re-affirmation refreshes the timestamp without changing induction.
    a.append(p.reaffirm(&joiner, 900)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("self-signed renewal authorized");
    let renewed = affirmation_state(&a, &p.polity, t(1000));
    assert!(renewed.is_inducted(&joiner.1));
    assert_eq!(renewed.last_affirmed(&joiner.1), Some(t(900)));
}

#[test]
fn stale_members_lists_the_uninducted_and_the_lapsed() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap(); // founded_at = 100
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    // Two of the three founders renew; member 2 never does.
    a.append(p.reaffirm(&p.ids[0], 900)).unwrap();
    a.append(p.reaffirm(&p.ids[1], 900)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    // At t = 1000 with a 500-second cadence: 0 and 1 are fresh (100s ago),
    // member 2 last affirmed at founding (900s ago), and the joiner was never
    // inducted at all.
    let stale = affirmation_state(&a, &p.polity, t(1000)).stale_members(500, t(1000));
    assert!(!stale.contains(&p.ids[0].1));
    assert!(!stale.contains(&p.ids[1].1));
    assert!(stale.contains(&p.ids[2].1));
    assert!(stale.contains(&joiner.1));
    assert_eq!(stale.len(), 2);

    // A cadence long enough to cover the whole history leaves only the
    // un-inducted member stale.
    let lax = affirmation_state(&a, &p.polity, t(1000)).stale_members(5_000, t(1000));
    assert_eq!(lax, vec![joiner.1.clone()]);
}

#[test]
fn induction_rejections() {
    let p = Polity::new(3);
    let joiner = id();

    // Sub-majority witnesses: the roll is 4, so 3 must witness.
    let mut sub = Archive::new();
    sub.append(p.charter(&[0, 1], vec![])).unwrap();
    sub.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    sub.append(p.induct(&joiner, &[0, 1], 300)).unwrap();
    verify_all(&sub).expect("signatures valid");
    assert!(reasons(&sub).iter().any(|m| m.contains("majority of the roll to witness (>= 3)")));

    // Someone who was never admitted cannot be inducted.
    let mut outsider = Archive::new();
    outsider.append(p.charter(&[0, 1], vec![])).unwrap();
    outsider.append(p.induct(&joiner, &[0, 1], 300)).unwrap();
    assert!(reasons(&outsider).iter().any(|m| m.contains("affirmation by a non-member")));

    // A founding member is already inducted by ratification.
    let mut founder = Archive::new();
    founder.append(p.charter(&[0, 1], vec![])).unwrap();
    founder.append(p.induct(&p.ids[2], &[0, 1], 300)).unwrap();
    assert!(reasons(&founder).iter().any(|m| m.contains("already-inducted")));

    // Nor can a member be inducted twice.
    let mut twice = Archive::new();
    twice.append(p.charter(&[0, 1], vec![])).unwrap();
    twice.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    twice.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    twice.append(p.induct(&joiner, &[0, 1, 2], 400)).unwrap();
    assert!(reasons(&twice).iter().any(|m| m.contains("already-inducted")));

    // The inductee must co-sign: the community receives them, and they participate.
    let mut silent = Archive::new();
    silent.append(p.charter(&[0, 1], vec![])).unwrap();
    silent.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    silent.append(p.affirm(
        &joiner.1,
        AffirmationKind::Induction,
        &[0, 1, 2],
        300,
        &p.signers(&[0, 1, 2]),
    ))
    .unwrap();
    verify_all(&silent).expect("the witnesses' signatures are valid");
    assert!(reasons(&silent).iter().any(|m| m.contains("witnesses plus the inductee")));

    // A witness must be a current member.
    let mut stranger = Archive::new();
    stranger.append(p.charter(&[0, 1], vec![])).unwrap();
    stranger.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    let outside = id();
    let af = Affirmation {
        polity: p.polity.clone(),
        member: joiner.1.clone(),
        kind: AffirmationKind::Induction,
        witnesses: vec![p.ids[0].1.clone(), p.ids[1].1.clone(), outside.1.clone()],
        affirmed_at: t(300),
    };
    let mut signers = p.signers(&[0, 1]);
    signers.push((outside.1.as_str(), &outside.0));
    signers.push((joiner.1.as_str(), &joiner.0));
    stranger.append(CharterEntry::MemberAffirmed(Signed::build(af, &signers).unwrap())).unwrap();
    assert!(reasons(&stranger).iter().any(|m| m.contains("witness is not a member")));
}

#[test]
fn reaffirmation_rejections() {
    let p = Polity::new(3);
    let joiner = id();

    // Renewal presupposes transmission: an admitted-but-un-inducted member
    // has nothing to renew.
    let mut early = Archive::new();
    early.append(p.charter(&[0, 1], vec![])).unwrap();
    early.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    early.append(p.reaffirm(&joiner, 300)).unwrap();
    verify_all(&early).expect("signature valid");
    assert!(reasons(&early).iter().any(|m| m.contains("never inducted")));

    // Renewal is unwitnessed.
    let mut witnessed = Archive::new();
    witnessed.append(p.charter(&[0, 1], vec![])).unwrap();
    witnessed
        .append(p.affirm(&p.ids[0].1, AffirmationKind::ReAffirmation, &[1], 300, &p.signers(&[0])))
        .unwrap();
    assert!(reasons(&witnessed).iter().any(|m| m.contains("records no witnesses")));

    // And nobody can renew on another member's behalf.
    let mut proxy = Archive::new();
    proxy.append(p.charter(&[0, 1], vec![])).unwrap();
    proxy
        .append(p.affirm(&p.ids[0].1, AffirmationKind::ReAffirmation, &[], 300, &p.signers(&[1])))
        .unwrap();
    verify_all(&proxy).expect("the other member's signature is valid");
    assert!(reasons(&proxy).iter().any(|m| m.contains("exactly the affirming member")));
}

#[test]
fn affirmation_before_charter_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.reaffirm(&p.ids[0], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn affirmation_after_dissolution_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 200)).unwrap();
    a.append(p.reaffirm(&p.ids[0], 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("after its dissolution")));
}

#[test]
fn majority_thresholds() {
    assert_eq!(majority(1), 1);
    assert_eq!(majority(2), 2);
    assert_eq!(majority(3), 2);
    assert_eq!(majority(4), 3);
    assert_eq!(majority(5), 3);
}
