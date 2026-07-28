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
use en_core::{keys, Alg, Cid, DidKey, PrivKey, PubKey, Signed, Timestamp};

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
        self.charter_derived(ratifiers, founding, &[])
    }

    /// A charter with founding derivations (D15). Each entry marks the member at
    /// that index a derived party, optionally answering to the given guardian
    /// identity — which then co-signs the founding, since answerability is accepted
    /// rather than assigned.
    /// A charter whose boundary attestation is supplied — for the D21 rules, where
    /// what the charter *claims* about the boundary is the thing under test.
    fn charter_attested(
        &self,
        ratifiers: &[usize],
        attestation: BoundaryAttestation,
    ) -> CharterEntry {
        let prop = FoundingProposal {
            polity: self.polity.clone(),
            substrate: substrate(),
            founding_commitments: vec![],
            proposed_members: self.roll(),
            proposed_derivations: vec![],
            attestation,
            proposed_at: t(50),
        };
        let charter = prop.ratify(self.dids(ratifiers), t(100)).unwrap();
        CharterEntry::PolityChartered(Signed::build(charter, &self.signers(ratifiers)).unwrap())
    }

    fn charter_derived(
        &self,
        ratifiers: &[usize],
        founding: Vec<Commitment>,
        derived: &[(usize, Option<&(PrivKey, DidKey)>)],
    ) -> CharterEntry {
        let prop = FoundingProposal::convener_asserted(
            self.polity.clone(),
            substrate(),
            founding,
            self.roll(),
            derived
                .iter()
                .map(|(m, g)| Derivation {
                    member: self.ids[*m].1.clone(),
                    guardian: g.map(|g| g.1.clone()),
                })
                .collect(),
            t(50),
        );
        let charter = prop.ratify(self.dids(ratifiers), t(100)).unwrap();
        let mut signers = self.signers(ratifiers);
        for (_, g) in derived {
            if let Some(g) = g {
                if !signers.iter().any(|(d, _)| *d == g.1.as_str()) {
                    signers.push((g.1.as_str(), &g.0));
                }
            }
        }
        CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap())
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

    /// Confer `office` on an identity that may be outside `ids` (an admitted
    /// joiner), for the cases where the office holder is not a founding member.
    fn confer_on(
        &self,
        office: Office,
        holder: &(PrivKey, DidKey),
        deciders: &[usize],
        at: i64,
    ) -> CharterEntry {
        let oc = OfficeChange {
            polity: self.polity.clone(),
            kind: OfficeKind::Confer,
            office,
            holder: holder.1.clone(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        let mut signers = self.signers(deciders);
        signers.push((holder.1.as_str(), &holder.0));
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

    /// An observation by `observer` (who need not be a member), recording each
    /// `(member index, guardian, attendance stamp)` it saw.
    fn observe(
        &self,
        observer: &(PrivKey, DidKey),
        at: i64,
        readings: &[(usize, Option<&DidKey>, Option<i64>)],
    ) -> CharterEntry {
        self.observe_signed(observer, at, readings, &[(observer.1.as_str(), &observer.0)])
    }

    /// An observation with an explicit signer set, for the rejection cases.
    fn observe_signed(
        &self,
        observer: &(PrivKey, DidKey),
        at: i64,
        readings: &[(usize, Option<&DidKey>, Option<i64>)],
        signers: &[(&str, &PrivKey)],
    ) -> CharterEntry {
        let ob = Observation {
            polity: self.polity.clone(),
            observer: observer.1.clone(),
            observed_at: t(at),
            readings: readings
                .iter()
                .map(|(m, g, stamp)| MemberReading {
                    member: self.ids[*m].1.clone(),
                    reputation: 0.9,
                    guardian: g.map(|g| g.clone()),
                    attested_at: stamp.map(t),
                })
                .collect(),
            opaque_boundary: false,
        };
        CharterEntry::Observed(Signed::build(ob, signers).unwrap())
    }

    /// A pruning of `polity`'s observations before `horizon`, with a receipt
    /// computed from what the archive actually holds.
    fn prune(
        &self,
        a: &Archive<CharterEntry>,
        horizon: i64,
        deciders: &[usize],
        at: i64,
    ) -> CharterEntry {
        self.prune_claiming(a, horizon, deciders, at, None)
    }

    /// A pruning whose receipt may be overridden, for the false-receipt cases.
    fn prune_claiming(
        &self,
        a: &Archive<CharterEntry>,
        horizon: i64,
        deciders: &[usize],
        at: i64,
        claim: Option<(usize, Cid)>,
    ) -> CharterEntry {
        let dropped = prunable_observations(a, &self.polity, t(horizon));
        let (count, digest) = match claim {
            Some(c) => c,
            None => (dropped.len(), pruning_digest(&dropped).unwrap()),
        };
        let pr = ObservationPruning {
            polity: self.polity.clone(),
            horizon: t(horizon),
            dropped: count,
            digest,
            rationale: "housekeeping".into(),
            deciders: self.dids(deciders),
            pruned_at: t(at),
        };
        CharterEntry::ObservationsPruned(Signed::build(pr, &self.signers(deciders)).unwrap())
    }

    /// A guardian record with an explicit signer set, for the rejection cases.
    fn guardian_raw(
        &self,
        kind: GuardianKind,
        member: usize,
        guardian: Option<&DidKey>,
        deciders: &[usize],
        at: i64,
        signers: &[(&str, &PrivKey)],
    ) -> CharterEntry {
        let gc = GuardianChange {
            polity: self.polity.clone(),
            kind,
            member: self.ids[member].1.clone(),
            guardian: guardian.cloned(),
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        CharterEntry::GuardianChanged(Signed::build(gc, signers).unwrap())
    }

    /// Designate a member a derived party: the `deciders` co-sign at a majority and
    /// the named guardian (if any) co-signs its acceptance.
    fn designate(
        &self,
        member: usize,
        guardian: Option<&(PrivKey, DidKey)>,
        deciders: &[usize],
        at: i64,
    ) -> CharterEntry {
        let mut signers = self.signers(deciders);
        if let Some(g) = guardian {
            signers.push((g.1.as_str(), &g.0));
        }
        self.guardian_raw(
            GuardianKind::Designate,
            member,
            guardian.map(|g| &g.1),
            deciders,
            at,
            &signers,
        )
    }

    /// Rotate a designated member to a new guardian, who consents.
    fn rotate_guardian(
        &self,
        member: usize,
        incoming: &(PrivKey, DidKey),
        deciders: &[usize],
        at: i64,
    ) -> CharterEntry {
        let mut signers = self.signers(deciders);
        signers.push((incoming.1.as_str(), &incoming.0));
        self.guardian_raw(GuardianKind::Rotate, member, Some(&incoming.1), deciders, at, &signers)
    }

    /// The guardian steps away on their own signature (no deciders).
    fn release_by_guardian(&self, member: usize, guardian: &(PrivKey, DidKey), at: i64) -> CharterEntry {
        let signers = vec![(guardian.1.as_str(), &guardian.0)];
        self.guardian_raw(GuardianKind::Release, member, None, &[], at, &signers)
    }

    /// The roll records at a majority that the human is gone.
    fn release_by_roll(&self, member: usize, deciders: &[usize], at: i64) -> CharterEntry {
        let signers = self.signers(deciders);
        self.guardian_raw(GuardianKind::Release, member, None, deciders, at, &signers)
    }

    /// An envoy record with an explicit signer set and public key, for the
    /// rejection cases. `envoy_pubkey` derives from the DID unless overridden.
    fn envoy_raw(
        &self,
        kind: EnvoyKind,
        envoy: &DidKey,
        pubkey: Option<PubKey>,
        authorizers: &[usize],
        expires: Option<i64>,
        at: i64,
        signers: &[(&str, &PrivKey)],
    ) -> CharterEntry {
        let ea = EnvoyAuthorization {
            polity: self.polity.clone(),
            kind,
            envoy: envoy.clone(),
            envoy_pubkey: pubkey.unwrap_or_else(|| envoy.to_pubkey().unwrap()),
            rationale: "r".into(),
            authorizers: self.dids(authorizers),
            expires_at: expires.map(t),
            decided_at: t(at),
        };
        CharterEntry::EnvoyAuthorized(Signed::build(ea, signers).unwrap())
    }

    /// Authorize `envoy` to speak for the polity: the `authorizers` co-sign (one
    /// EnvoyAuthorizer, or a majority of the roll) and the envoy co-signs consent.
    fn authorize_envoy(
        &self,
        envoy: &(PrivKey, DidKey),
        authorizers: &[usize],
        expires: Option<i64>,
        at: i64,
    ) -> CharterEntry {
        let mut signers = self.signers(authorizers);
        if !signers.iter().any(|(d, _)| *d == envoy.1.as_str()) {
            signers.push((envoy.1.as_str(), &envoy.0));
        }
        self.envoy_raw(EnvoyKind::Authorize, &envoy.1, None, authorizers, expires, at, &signers)
    }

    /// The envoy stands down (self-signed, no authorizers).
    fn resign_envoy(&self, envoy: &(PrivKey, DidKey), at: i64) -> CharterEntry {
        let signers = vec![(envoy.1.as_str(), &envoy.0)];
        self.envoy_raw(EnvoyKind::Resign, &envoy.1, None, &[], None, at, &signers)
    }

    /// The polity withdraws the standing; the `authorizers` exclude the envoy.
    fn revoke_envoy(&self, envoy: &DidKey, authorizers: &[usize], at: i64) -> CharterEntry {
        let signers = self.signers(authorizers);
        self.envoy_raw(EnvoyKind::Revoke, envoy, None, authorizers, None, at, &signers)
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
    let prop = FoundingProposal::convener_asserted(
            p.polity.clone(),
            substrate(),
            vec![],
            p.roll(),
            vec![],
            t(50),
        );
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
    let prop = FoundingProposal::convener_asserted(
            p.polity.clone(),
            substrate(),
            vec![],
            p.roll(),
            vec![],
            t(50),
        );
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

// --- D14: the Inductor presides over the rite, defeasibly --------------------
//
// Roll of 7 throughout (6 founders + an admitted joiner): majority(7) = 4, so the
// roll's override bar is 5 and there are enough founding witnesses to reach it
// without the Inductor.

/// Charter a polity of 6, admit `joiner`, and (optionally) confer the Inductor.
fn presided(inductor: Option<usize>) -> (Polity, (PrivKey, DidKey), Archive<CharterEntry>) {
    let p = Polity::new(6);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2, 3], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1, 2, 3], 200)).unwrap();
    if let Some(i) = inductor {
        // Roll of 7 now, so a conferral needs 4 deciders.
        a.append(p.confer(Office::Inductor, i, &[0, 1, 2, 3], 250)).unwrap();
    }
    (p, joiner, a)
}

#[test]
fn an_inductor_among_the_witnesses_authorizes_at_a_plain_majority() {
    let (p, joiner, mut a) = presided(Some(0));
    a.append(p.induct(&joiner, &[0, 1, 2, 3], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the Inductor presided at a majority");
}

#[test]
fn without_the_inductor_a_plain_majority_no_longer_suffices() {
    let (p, joiner, mut a) = presided(Some(0));
    // Four witnesses is a majority of the roll of 7, and would have authorized the
    // induction before the office was conferred — see the vacant-office case below.
    a.append(p.induct(&joiner, &[1, 2, 3, 4], 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("Inductor other than the inductee")));
}

#[test]
fn the_roll_may_induct_without_the_inductor_at_its_own_higher_bar() {
    // The whole point of a *defeasible* gate: an officer raises the bar for the
    // rite and can never block it, so the roll stays sovereign (D12).
    let (p, joiner, mut a) = presided(Some(0));
    a.append(p.induct(&joiner, &[1, 2, 3, 4, 5], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("majority + 1 is the roll's override");
}

#[test]
fn a_vacant_inductor_leaves_the_majority_rule_untouched() {
    let (p, joiner, mut a) = presided(None);
    a.append(p.induct(&joiner, &[1, 2, 3, 4], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("no office held, so a majority witnesses as before");
}

#[test]
fn an_inductor_who_has_left_the_roll_does_not_preside() {
    // D12: a holding is effective only while its holder is on the roll.
    let (p, joiner, mut a) = presided(Some(5));
    a.append(p.depart(5, 260)).unwrap();
    // Roll of 6 now, so a majority is 4 — and the departed Inductor gates nothing.
    a.append(p.induct(&joiner, &[1, 2, 3, 4], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("a lapsed holding does not preside");
}

#[test]
fn any_one_of_several_inductors_suffices() {
    let (p, joiner, mut a) = presided(Some(0));
    a.append(p.confer(Office::Inductor, 5, &[0, 1, 2, 3], 260)).unwrap();
    a.append(p.induct(&joiner, &[2, 3, 4, 5], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the second Inductor presided");
}

#[test]
fn the_inductee_cannot_preside_over_their_own_reception() {
    let p = Polity::new(6);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2, 3], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1, 2, 3], 200)).unwrap();
    // The joiner is the polity's only Inductor — and the one being inducted.
    a.append(p.confer_on(Office::Inductor, &joiner, &[0, 1, 2, 3], 250)).unwrap();

    let mut own = a.clone();
    own.append(p.induct(&joiner, &[1, 2, 3, 4], 300)).unwrap();
    assert!(reasons(&own).iter().any(|m| m.contains("Inductor other than the inductee")));

    // The roll's override still gets them in.
    let mut over = a;
    over.append(p.induct(&joiner, &[1, 2, 3, 4, 5], 300)).unwrap();
    verify_all(&over).unwrap();
    check_authority(&over).expect("the roll may receive them without a presider");
}

// --- D14: the EnvoyAuthorizer's delegated authority --------------------------

/// Charter a polity of 5 (majority = 3) and (optionally) confer EnvoyAuthorizer.
fn with_authorizer(officer: Option<usize>) -> (Polity, Archive<CharterEntry>) {
    let p = Polity::new(5);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    if let Some(i) = officer {
        a.append(p.confer(Office::EnvoyAuthorizer, i, &[0, 1, 2], 200)).unwrap();
    }
    (p, a)
}

#[test]
fn an_envoy_authorizer_authorizes_on_their_own_signature() {
    // The teeth: one holder's signature stands in for the roll's majority.
    let (p, mut a) = with_authorizer(Some(0));
    a.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the officer's delegated power authorizes alone");

    let st = envoy_state(&a, &p.polity, t(400));
    assert!(st.is_active(&p.ids[3].1));
    assert_eq!(st.envoys().len(), 1);
    // The key travels with the standing: en-uplift names the speaker by key.
    assert_eq!(st.standing(&p.ids[3].1).unwrap().envoy_pubkey, p.ids[3].1.to_pubkey().unwrap());
    assert_eq!(st.standing(&p.ids[3].1).unwrap().authorized_at, t(300));
}

#[test]
fn the_roll_authorizes_an_envoy_at_a_majority_with_the_office_vacant() {
    let (p, mut a) = with_authorizer(None);
    a.append(p.authorize_envoy(&p.ids[3], &[0, 1, 2], None, 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the majority route needs no office");
    assert!(envoy_state(&a, &p.polity, t(400)).is_active(&p.ids[3].1));
}

#[test]
fn the_majority_route_stays_open_while_the_office_is_held() {
    // Filling the office must cost the roll nothing — otherwise conferring an
    // EnvoyAuthorizer would *reduce* the roll's own authority.
    let (p, mut a) = with_authorizer(Some(0));
    a.append(p.authorize_envoy(&p.ids[3], &[1, 2, 4], None, 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("a majority authorizes without the officer");
}

#[test]
fn a_lone_authorizer_holding_no_office_is_unauthorized() {
    let (p, mut a) = with_authorizer(None);
    a.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("EnvoyAuthorizer other than the envoy")));
}

#[test]
fn an_authorizer_cannot_appoint_themselves_alone() {
    // Otherwise the office would be a unilateral self-appointment as the polity's
    // voice — the capture move the consent co-signature is meant to prevent.
    let (p, a) = with_authorizer(Some(0));

    let mut solo = a.clone();
    solo.append(p.authorize_envoy(&p.ids[0], &[0], None, 300)).unwrap();
    assert!(reasons(&solo).iter().any(|m| m.contains("EnvoyAuthorizer other than the envoy")));

    // The roll may still make them its envoy.
    let mut by_roll = a;
    by_roll.append(p.authorize_envoy(&p.ids[0], &[0, 1, 2], None, 300)).unwrap();
    verify_all(&by_roll).unwrap();
    check_authority(&by_roll).expect("the roll may authorize its own officer");
}

#[test]
fn envoy_authorization_rejections() {
    let (p, base) = with_authorizer(Some(0));

    // Someone outside the roll cannot be the polity's voice.
    let outsider = id();
    let mut out = base.clone();
    out.append(p.authorize_envoy(&outsider, &[0], None, 300)).unwrap();
    assert!(reasons(&out).iter().any(|m| m.contains("not a member")));

    // A key that does not derive the DID would let an unrelated key speak.
    let (_, other_pk) = keys::generate(Alg::Ed25519).unwrap();
    let mut wrong_key = base.clone();
    let signers = vec![
        (p.ids[0].1.as_str(), &p.ids[0].0),
        (p.ids[3].1.as_str(), &p.ids[3].0),
    ];
    wrong_key
        .append(p.envoy_raw(
            EnvoyKind::Authorize,
            &p.ids[3].1,
            Some(other_pk),
            &[0],
            None,
            300,
            &signers,
        ))
        .unwrap();
    assert!(reasons(&wrong_key).iter().any(|m| m.contains("does not derive the envoy DID")));

    // Speaking for the polity is a duty: it cannot be conscripted.
    let mut silent = base.clone();
    silent
        .append(p.envoy_raw(
            EnvoyKind::Authorize,
            &p.ids[3].1,
            None,
            &[0],
            None,
            300,
            &p.signers(&[0]),
        ))
        .unwrap();
    assert!(reasons(&silent).iter().any(|m| m.contains("plus the envoy (consent)")));

    // Nor authorized twice, resigned when not held, or revoked with the envoy
    // among its own authorizers.
    let mut twice = base.clone();
    twice.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    twice.append(p.authorize_envoy(&p.ids[3], &[0], None, 400)).unwrap();
    assert!(reasons(&twice).iter().any(|m| m.contains("already authorized")));

    let mut early = base.clone();
    early.append(p.resign_envoy(&p.ids[3], 300)).unwrap();
    assert!(reasons(&early).iter().any(|m| m.contains("standing not held")));

    let mut self_revoke = base;
    self_revoke.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    self_revoke.append(p.revoke_envoy(&p.ids[3].1, &[0, 3], 400)).unwrap();
    assert!(reasons(&self_revoke).iter().any(|m| m.contains("cannot be among the authorizers")));
}

#[test]
fn a_standing_ends_by_resignation_or_revocation() {
    let (p, base) = with_authorizer(Some(0));

    let mut stood_down = base.clone();
    stood_down.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    stood_down.append(p.resign_envoy(&p.ids[3], 400)).unwrap();
    verify_all(&stood_down).unwrap();
    check_authority(&stood_down).expect("nobody is forced to keep speaking");
    assert!(!envoy_state(&stood_down, &p.polity, t(500)).is_active(&p.ids[3].1));

    // Revocation takes the same two routes as authorization — here the officer's.
    let mut withdrawn = base;
    withdrawn.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    withdrawn.append(p.revoke_envoy(&p.ids[3].1, &[0], 400)).unwrap();
    verify_all(&withdrawn).unwrap();
    check_authority(&withdrawn).expect("the officer withdraws the standing alone");
    assert!(!envoy_state(&withdrawn, &p.polity, t(500)).is_active(&p.ids[3].1));
}

#[test]
fn a_standing_lapses_off_the_roll_and_revives_on_return() {
    // Speaking for a polity one has left is exactly what must not be possible —
    // and the standing is not destroyed by the departure, only suspended (D12's
    // rule for office holdings, applied to the envoy standing).
    let (p, mut a) = with_authorizer(Some(0));
    a.append(p.authorize_envoy(&p.ids[3], &[0], None, 300)).unwrap();
    assert!(envoy_state(&a, &p.polity, t(350)).is_active(&p.ids[3].1));

    a.append(p.depart(3, 400)).unwrap();
    assert!(!envoy_state(&a, &p.polity, t(450)).is_active(&p.ids[3].1));

    a.append(p.admit(&p.ids[3], &[0, 1, 2], 500)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("departure and re-admission are both authorized");
    assert!(envoy_state(&a, &p.polity, t(550)).is_active(&p.ids[3].1));
}

#[test]
fn an_expired_standing_is_no_longer_active() {
    let (p, mut a) = with_authorizer(Some(0));
    a.append(p.authorize_envoy(&p.ids[3], &[0], Some(1000), 300)).unwrap();
    check_authority(&a).expect("an expiring authorization is authorized");

    assert!(envoy_state(&a, &p.polity, t(999)).is_active(&p.ids[3].1));
    // The boundary instant is still inside the standing.
    assert!(envoy_state(&a, &p.polity, t(1000)).is_active(&p.ids[3].1));
    assert!(!envoy_state(&a, &p.polity, t(1001)).is_active(&p.ids[3].1));
}

// --- D15: the chartered guardian edge ----------------------------------------

#[test]
fn founding_derivations_are_ratified_and_derived() {
    let p = Polity::new(3);
    let human = id();
    let mut a = Archive::new();
    // Two machines, one with a human behind it and one with nobody.
    a.append(p.charter_derived(&[0, 1], vec![], &[(1, Some(&human)), (2, None)])).unwrap();
    verify_all(&a).expect("the guardian co-signed the founding");
    check_authority(&a).expect("founding derivations authorized");

    let g = guardian_state(&a, &p.polity, t(200));
    assert!(g.is_derived(&p.ids[1].1) && g.is_derived(&p.ids[2].1));
    assert!(!g.is_derived(&p.ids[0].1), "an undesignated member is not derived");
    assert_eq!(g.guardian_of(&p.ids[1].1), Some(&Some(human.1.clone())));
    // A designated member with no guardian reads as Some(None) — chartered as a
    // machine answering to nobody — not as "not a derived party".
    assert_eq!(g.guardian_of(&p.ids[2].1), Some(&None));
    assert_eq!(g.guardian_of(&p.ids[0].1), None);
    assert_eq!(g.guarded_members(), vec![&p.ids[1].1]);
    assert_eq!(g.unguarded_members(), vec![&p.ids[2].1]);
    // Two of three: machine-majority.
    assert!(g.is_machine_majority(3));
}

#[test]
fn a_guardian_must_consent_to_the_founding() {
    // Answerability is accepted, never assigned: a charter that names a guardian
    // without their signature is unauthorized.
    let p = Polity::new(3);
    let human = id();
    let prop = FoundingProposal::convener_asserted(
            p.polity.clone(),
            substrate(),
            vec![],
            p.roll(),
            vec![Derivation {
            member: p.ids[1].1.clone(),
            guardian: Some(human.1.clone()),
        }],
            t(50),
        );
    let charter = prop.ratify(p.dids(&[0, 1]), t(100)).unwrap();
    let mut a = Archive::new();
    a.append(CharterEntry::PolityChartered(
        Signed::build(charter, &p.signers(&[0, 1])).unwrap(),
    ))
    .unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("plus every named guardian")));
}

#[test]
fn founding_derivation_rejections() {
    let p = Polity::new(3);

    // A machine cannot guard a machine, or the human-above-machine anchor loops.
    let mut looped = Archive::new();
    let prop = FoundingProposal::convener_asserted(
            p.polity.clone(),
            substrate(),
            vec![],
            p.roll(),
            vec![
            Derivation { member: p.ids[1].1.clone(), guardian: Some(p.ids[2].1.clone()) },
            Derivation { member: p.ids[2].1.clone(), guardian: None },
        ],
            t(50),
        );
    let charter = prop.ratify(p.dids(&[0, 1]), t(100)).unwrap();
    let mut signers = p.signers(&[0, 1]);
    signers.push((p.ids[2].1.as_str(), &p.ids[2].0));
    looped
        .append(CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap()))
        .unwrap();
    assert!(reasons(&looped).iter().any(|m| m.contains("cannot guard a machine")));

    // A derivation must name a member of the roll.
    let mut stranger = Archive::new();
    let outsider = id();
    let prop = FoundingProposal::convener_asserted(
            p.polity.clone(),
            substrate(),
            vec![],
            p.roll(),
            vec![Derivation { member: outsider.1.clone(), guardian: None }],
            t(50),
        );
    let charter = prop.ratify(p.dids(&[0, 1]), t(100)).unwrap();
    stranger
        .append(CharterEntry::PolityChartered(
            Signed::build(charter, &p.signers(&[0, 1])).unwrap(),
        ))
        .unwrap();
    assert!(reasons(&stranger).iter().any(|m| m.contains("not in the membership roll")));
}

#[test]
fn designate_rotate_and_release_move_the_edge() {
    let p = Polity::new(3);
    let first = id();
    let second = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();

    a.append(p.designate(2, Some(&first), &[0, 1], 200)).unwrap();
    assert_eq!(
        guardian_state(&a, &p.polity, t(250)).guardian_of(&p.ids[2].1),
        Some(&Some(first.1.clone()))
    );

    a.append(p.rotate_guardian(2, &second, &[0, 1], 300)).unwrap();
    assert_eq!(
        guardian_state(&a, &p.polity, t(350)).guardian_of(&p.ids[2].1),
        Some(&Some(second.1.clone()))
    );

    // The guardian steps away on their own signature.
    a.append(p.release_by_guardian(2, &second, 400)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("designate, rotate and release all authorized");

    let g = guardian_state(&a, &p.polity, t(450));
    // The designation survives the release: a machine does not become human by
    // losing its human.
    assert!(g.is_derived(&p.ids[2].1));
    assert_eq!(g.guardian_of(&p.ids[2].1), Some(&None));
}

#[test]
fn the_roll_may_record_that_a_guardian_is_gone() {
    let p = Polity::new(3);
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.designate(2, Some(&human), &[0, 1], 200)).unwrap();
    a.append(p.release_by_roll(2, &[0, 1], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the roll may record a vanished human at a majority");
    assert_eq!(guardian_state(&a, &p.polity, t(350)).guardian_of(&p.ids[2].1), Some(&None));
}

#[test]
fn guardian_change_rejections() {
    let p = Polity::new(3);
    let human = id();
    let other = id();
    let mut base = Archive::new();
    base.append(p.charter(&[0, 1], vec![])).unwrap();

    // The named guardian must co-sign.
    let mut unsigned = base.clone();
    unsigned
        .append(p.guardian_raw(
            GuardianKind::Designate,
            2,
            Some(&human.1),
            &[0, 1],
            200,
            &p.signers(&[0, 1]),
        ))
        .unwrap();
    assert!(reasons(&unsigned).iter().any(|m| m.contains("plus the named guardian")));

    // Nothing may guard itself.
    let mut selfish = base.clone();
    selfish
        .append(p.designate(2, Some(&(p.ids[2].0.clone(), p.ids[2].1.clone())), &[0, 1], 200))
        .unwrap();
    assert!(reasons(&selfish).iter().any(|m| m.contains("its own guardian")));

    // Rotation presupposes a designation, and must name the incoming guardian.
    let mut early = base.clone();
    early.append(p.rotate_guardian(2, &human, &[0, 1], 200)).unwrap();
    assert!(reasons(&early).iter().any(|m| m.contains("not a designated derived party")));

    // No second designation for the same member.
    let mut twice = base.clone();
    twice.append(p.designate(2, Some(&human), &[0, 1], 200)).unwrap();
    twice.append(p.designate(2, Some(&other), &[0, 1], 300)).unwrap();
    assert!(reasons(&twice).iter().any(|m| m.contains("already designated")));

    // A release of an edge that is already absent says nothing.
    let mut empty = base.clone();
    empty.append(p.designate(2, None, &[0, 1], 200)).unwrap();
    empty.append(p.release_by_roll(2, &[0, 1], 300)).unwrap();
    assert!(reasons(&empty).iter().any(|m| m.contains("already absent")));

    // A machine cannot be rotated onto another machine.
    let mut looped = base;
    looped.append(p.designate(1, None, &[0, 1], 200)).unwrap();
    looped.append(p.designate(2, Some(&human), &[0, 1], 300)).unwrap();
    looped
        .append(p.guardian_raw(
            GuardianKind::Rotate,
            2,
            Some(&p.ids[1].1),
            &[0, 1],
            400,
            &p.signers(&[0, 1]),
        ))
        .unwrap();
    assert!(reasons(&looped).iter().any(|m| m.contains("cannot guard a machine")));
}

#[test]
fn a_designation_lapses_off_the_roll_and_revives_on_return() {
    let p = Polity::new(4);
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.designate(3, Some(&human), &[0, 1, 2], 200)).unwrap();
    assert!(guardian_state(&a, &p.polity, t(250)).is_derived(&p.ids[3].1));

    a.append(p.depart(3, 300)).unwrap();
    assert!(!guardian_state(&a, &p.polity, t(350)).is_derived(&p.ids[3].1));

    a.append(p.admit(&p.ids[3], &[0, 1, 2], 400)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("departure and re-admission authorized");
    let g = guardian_state(&a, &p.polity, t(450));
    assert!(g.is_derived(&p.ids[3].1));
    assert_eq!(g.guardian_of(&p.ids[3].1), Some(&Some(human.1)));
}

// --- D15: the payoff — every office traces to a human -----------------------

#[test]
fn a_machine_majority_polity_refuses_an_office_for_an_unguarded_machine() {
    let p = Polity::new(3);
    let human = id();
    let mut a = Archive::new();
    // Two of three are machines: machine-majority. Member 2 answers to nobody.
    a.append(p.charter_derived(&[0, 1], vec![], &[(1, Some(&human)), (2, None)])).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("derived member with no guardian")));
}

#[test]
fn the_same_conferral_is_authorized_once_a_guardian_answers_for_it() {
    // The refusal is not about the machine, it is about the missing human: attach
    // one and the identical conferral goes through. An already-designated member
    // acquires a guardian by `Rotate` (a second `Designate` is refused), which is
    // what makes Rotate the operation on the *edge* rather than on the designation.
    let p = Polity::new(3);
    let first = id();
    let second = id();
    let mut a = Archive::new();
    a.append(p.charter_derived(&[0, 1], vec![], &[(1, Some(&first)), (2, None)])).unwrap();
    a.append(p.rotate_guardian(2, &second, &[0, 1], 200)).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 300)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("an unguarded machine given a human may hold office");
    assert_eq!(
        guardian_state(&a, &p.polity, t(350)).guardian_of(&p.ids[2].1),
        Some(&Some(second.1))
    );
}

#[test]
fn a_guarded_machine_may_hold_office_in_a_machine_majority_polity() {
    let p = Polity::new(3);
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter_derived(&[0, 1], vec![], &[(1, Some(&human)), (2, Some(&human))])).unwrap();
    a.append(p.confer(Office::Convener, 2, &[0, 1], 200)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("a machine with a human behind it may hold office");
}

#[test]
fn the_office_rule_is_conditional_on_machine_majority() {
    // The same unguarded machine, in a polity that is mostly human, holds office
    // freely — the rule exists because a machine-majority polity substantially *is*
    // its guardian assembly, not because machines are second class.
    let p = Polity::new(5);
    let mut a = Archive::new();
    a.append(p.charter_derived(&[0, 1, 2], vec![], &[(4, None)])).unwrap();
    a.append(p.confer(Office::Convener, 4, &[0, 1, 2], 200)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("one machine among five is not machine-majority");
    assert!(!guardian_state(&a, &p.polity, t(250)).is_machine_majority(5));
}

#[test]
fn machine_majority_is_a_majority_of_the_roll() {
    // Roll of 5, majority 3: two machines is not machine-majority, three is.
    let p = Polity::new(5);
    let mut two = Archive::new();
    two.append(p.charter_derived(&[0, 1, 2], vec![], &[(3, None), (4, None)])).unwrap();
    assert!(!guardian_state(&two, &p.polity, t(200)).is_machine_majority(5));
    two.append(p.confer(Office::Convener, 3, &[0, 1, 2], 200)).unwrap();
    check_authority(&two).expect("two of five leaves the rule dormant");

    let mut three = Archive::new();
    three
        .append(p.charter_derived(&[0, 1, 2], vec![], &[(2, None), (3, None), (4, None)]))
        .unwrap();
    assert!(guardian_state(&three, &p.polity, t(200)).is_machine_majority(5));
    three.append(p.confer(Office::Convener, 3, &[0, 1, 2], 200)).unwrap();
    assert!(reasons(&three).iter().any(|m| m.contains("derived member with no guardian")));
}

// --- D16: decay trends, read from the polity's own records --------------------

#[test]
fn decay_trends_track_the_affirmation_series_and_its_gaps() {
    let p = Polity::new(3);
    let joiners: Vec<(PrivKey, DidKey)> = (0..2).map(|_| id()).collect();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1], 200)).unwrap();
    // Roll of 4: an induction needs 3 witnesses.
    a.append(p.induct(&joiners[0], &[0, 1, 2], 300)).unwrap();
    a.append(p.admit(&joiners[1], &[0, 1, 2], 400)).unwrap();
    // Roll of 5: 3 witnesses again, but one fewer turns out than could.
    a.append(p.induct(&joiners[1], &[0, 1, 2], 900)).unwrap();
    a.append(p.reaffirm(&joiners[0], 1500)).unwrap();
    check_authority(&a).expect("the series is authorized");

    let t_ = decay_trends(&a, &p.polity, t(2000));
    assert_eq!(t_.affirmations.len(), 3);
    assert_eq!(t_.inductions().len(), 2, "a re-affirmation has no witnesses to count");
    assert_eq!(t_.gaps_seconds, vec![600, 600]);
    assert_eq!(t_.seconds_since_last, Some(500));
    assert_eq!(t_.membership_changes.len(), 2);
}

#[test]
fn a_widening_gap_needs_two_gaps_to_compare_against() {
    // KC's hedge, borrowed: with fewer than two gaps there is no cadence yet, so
    // the answer is "unknown", never "no".
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    assert_eq!(decay_trends(&a, &p.polity, t(400)).latest_gap_widening(), None);

    a.append(p.reaffirm(&joiner, 400)).unwrap();
    assert_eq!(
        decay_trends(&a, &p.polity, t(500)).latest_gap_widening(),
        None,
        "one gap is still not a cadence"
    );

    // Gaps of 100 then 1000: the latest exceeds the median of the prior gaps.
    a.append(p.reaffirm(&joiner, 1400)).unwrap();
    assert_eq!(decay_trends(&a, &p.polity, t(1500)).latest_gap_widening(), Some(true));
}

#[test]
fn a_steady_cadence_is_not_widening() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    for at in [400i64, 500, 600] {
        a.append(p.reaffirm(&joiner, at)).unwrap();
    }
    check_authority(&a).unwrap();
    let t_ = decay_trends(&a, &p.polity, t(700));
    assert_eq!(t_.gaps_seconds, vec![100, 100, 100]);
    assert_eq!(t_.latest_gap_widening(), Some(false));
}

#[test]
fn the_median_rule_absorbs_a_single_long_interval() {
    // Gaps 100, 5000, 100: the *last* gap is not above the median (100), so one
    // aberrant interval in the middle does not read as decay. Comparing against
    // the previous gap alone would have called this healthy too — but comparing
    // against the mean would have called a widening series healthy, which is why
    // the median is the right centre.
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    for at in [400i64, 5400, 5500] {
        a.append(p.reaffirm(&joiner, at)).unwrap();
    }
    let t_ = decay_trends(&a, &p.polity, t(6000));
    assert_eq!(t_.gaps_seconds, vec![100, 5000, 100]);
    assert_eq!(t_.latest_gap_widening(), Some(false));
}

#[test]
fn the_rolling_gap_reading_goes_quiet_once_the_drift_becomes_the_baseline() {
    // The shifting-baseline failure the pinned reading exists for. Gaps 100, 400,
    // 400, 400: the polity widened its cadence fourfold and stayed there. By the
    // last gap, 400 is no longer unusual *for this polity* — it IS the median — so
    // the rolling reading falls silent exactly when the decay has finished setting.
    // Pinned against the founding cadence, the slip is still plainly there.
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    // Induction at 300 is itself an affirmation point, so the first gap is 300->400.
    for at in [400i64, 800, 1200, 1600] {
        a.append(p.reaffirm(&joiner, at)).unwrap();
    }
    let t_ = decay_trends(&a, &p.polity, t(1700));
    assert_eq!(t_.gaps_seconds, vec![100, 400, 400, 400]);
    assert_eq!(
        t_.latest_gap_widening(),
        Some(false),
        "the degraded cadence has become its own reference"
    );
    assert_eq!(
        t_.gap_widening_since_founding(),
        Some(true),
        "against the founding cadence the slip is still visible"
    );
}

#[test]
fn the_pinned_gap_reading_needs_two_gaps_and_agrees_on_a_steady_cadence() {
    // Negative controls for the pinned reading: it invents nothing from one gap,
    // and a polity that held its cadence is not flagged.
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    a.append(p.reaffirm(&joiner, 400)).unwrap();
    assert_eq!(
        decay_trends(&a, &p.polity, t(500)).gap_widening_since_founding(),
        None,
        "one gap is not a cadence"
    );

    for at in [500i64, 600] {
        a.append(p.reaffirm(&joiner, at)).unwrap();
    }
    let t_ = decay_trends(&a, &p.polity, t(700));
    assert_eq!(t_.gaps_seconds, vec![100, 100, 100]);
    assert_eq!(t_.gap_widening_since_founding(), Some(false));
}

#[test]
fn a_witness_decline_needs_two_inductions() {
    let p = Polity::new(6);
    let joiners: Vec<(PrivKey, DidKey)> = (0..2).map(|_| id()).collect();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2, 3], vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1, 2, 3], 200)).unwrap();
    // Roll of 7, majority 4 — but five turn out to receive the first joiner.
    a.append(p.induct(&joiners[0], &[0, 1, 2, 3, 4], 300)).unwrap();
    assert!(
        !decay_trends(&a, &p.polity, t(400)).witness_count_declined(),
        "one induction is not a trend"
    );

    a.append(p.admit(&joiners[1], &[0, 1, 2, 3], 400)).unwrap();
    // Roll of 8, majority 5: the bare minimum turns out this time.
    a.append(p.induct(&joiners[1], &[0, 1, 2, 3, 4], 500)).unwrap();
    check_authority(&a).unwrap();
    let t_ = decay_trends(&a, &p.polity, t(600));
    assert_eq!(t_.inductions().len(), 2);
    assert!(!t_.witness_count_declined(), "five then five is not a decline");
}

#[test]
fn fewer_witnesses_than_the_first_induction_is_a_decline() {
    let p = Polity::new(6);
    let joiners: Vec<(PrivKey, DidKey)> = (0..2).map(|_| id()).collect();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2, 3], vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1, 2, 3], 200)).unwrap();
    // Six of the roll of seven receive the first joiner...
    a.append(p.induct(&joiners[0], &[0, 1, 2, 3, 4, 5], 300)).unwrap();
    a.append(p.admit(&joiners[1], &[0, 1, 2, 3], 400)).unwrap();
    // ...and only the bare majority of eight receives the second.
    a.append(p.induct(&joiners[1], &[0, 1, 2, 3, 4], 500)).unwrap();
    check_authority(&a).expect("both inductions are authorized");

    let t_ = decay_trends(&a, &p.polity, t(600));
    assert!(t_.witness_count_declined(), "6 -> 5 is a decline");
}

#[test]
fn churn_is_a_rate_over_a_window_and_unmeasured_over_nothing() {
    let p = Polity::new(4);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1, 2], 1_000)).unwrap();
    a.append(p.depart(3, 2_000)).unwrap();

    let t_ = decay_trends(&a, &p.polity, t(3_000));
    assert_eq!(t_.membership_changes.len(), 2);
    // Both changes inside the window, over a roll of 4.
    assert_eq!(t_.churn_rate(5_000, 4, t(3_000)), Some(0.5));
    // A window that reaches back only to 2_500 sees neither.
    assert_eq!(t_.churn_rate(500, 4, t(3_000)), Some(0.0));
    // A rate over nothing is not zero but unmeasured.
    assert_eq!(t_.churn_rate(5_000, 0, t(3_000)), None);
    assert_eq!(t_.churn_rate(0, 4, t(3_000)), None);
}

#[test]
fn trends_are_read_as_of_a_moment() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 300)).unwrap();
    a.append(p.reaffirm(&joiner, 900)).unwrap();

    let early = decay_trends(&a, &p.polity, t(400));
    assert_eq!(early.affirmations.len(), 1);
    assert_eq!(early.seconds_since_last, Some(100));
    let late = decay_trends(&a, &p.polity, t(1000));
    assert_eq!(late.affirmations.len(), 2);
    assert_eq!(late.seconds_since_last, Some(100));
}

// --- D16: observations, and the window they make -----------------------------

#[test]
fn an_observation_is_signed_by_its_observer_who_need_not_be_a_member() {
    let p = Polity::new(3);
    let monitor = id();
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 200, &[(2, Some(&human.1), Some(190))])).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("a monitor outside the roll may still record what it saw");

    let w = observation_window(&a, &p.polity, t(0), t(300));
    assert_eq!(w.reading_count(), 1);
    assert_eq!(w.observations[0].observer, monitor.1);
    assert_eq!(w.last_attested(&p.ids[2].1), Some(t(190)));
}

#[test]
fn nobody_may_put_an_observation_in_another_mouth() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // Signed by a member, but naming the monitor as the observer.
    a.append(p.observe_signed(&monitor, 200, &[(2, None, None)], &p.signers(&[0])))
        .unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("signed by exactly its observer")));
}

#[test]
fn an_observation_records_no_decision_and_needs_no_majority() {
    // The whole point of the record's authority rule being this thin: evidence is
    // not a decision, so a lone observer is enough — and a duplicate reading, which
    // *is* malformed, is still refused.
    let p = Polity::new(3);
    let monitor = id();
    let mut dup = Archive::new();
    dup.append(p.charter(&[0, 1], vec![])).unwrap();
    dup.append(p.observe(&monitor, 200, &[(2, None, None), (2, None, Some(150))]))
        .unwrap();
    assert!(reasons(&dup).iter().any(|m| m.contains("duplicate reading")));
}

#[test]
fn the_window_is_bounded_at_both_ends() {
    // A monitor asking "has anyone been present today" must not be answered out of
    // last year's evidence.
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    for at in [200i64, 400, 600] {
        a.append(p.observe(&monitor, at, &[(2, None, None)])).unwrap();
    }
    check_authority(&a).unwrap();

    assert_eq!(observation_window(&a, &p.polity, t(0), t(1000)).reading_count(), 3);
    assert_eq!(observation_window(&a, &p.polity, t(300), t(1000)).reading_count(), 2);
    assert_eq!(observation_window(&a, &p.polity, t(0), t(500)).reading_count(), 2);
    assert_eq!(observation_window(&a, &p.polity, t(300), t(500)).reading_count(), 1);
    // A span needs two points to exist at all.
    assert_eq!(observation_window(&a, &p.polity, t(300), t(500)).span_seconds(), None);
    assert_eq!(observation_window(&a, &p.polity, t(0), t(1000)).span_seconds(), Some(400));
}

#[test]
fn an_unattended_span_distinguishes_never_watched_from_never_present() {
    let p = Polity::new(3);
    let monitor = id();
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // Member 2 is seen twice with a human present at 190; member 1 is seen twice
    // with no human at all; member 0 is never seen.
    a.append(p.observe(&monitor, 200, &[(2, Some(&human.1), Some(190)), (1, None, None)]))
        .unwrap();
    a.append(p.observe(&monitor, 400, &[(2, Some(&human.1), Some(190)), (1, None, None)]))
        .unwrap();
    check_authority(&a).unwrap();
    let w = observation_window(&a, &p.polity, t(0), t(1000));

    // Seen with a stamp: unattended since the stamp.
    assert_eq!(w.unattended_span(&p.ids[2].1, t(1_000)), Some(810));
    // Seen, never a human: unattended across the whole interval we watched.
    assert_eq!(w.unattended_span(&p.ids[1].1, t(1_000)), Some(800));
    // Never seen: not evaluated. Nobody was watching them, which is not evidence
    // of absence.
    assert_eq!(w.unattended_span(&p.ids[0].1, t(1_000)), None);
    // And the per-member evidence of watching is per member.
    assert_eq!(w.readings_for(&p.ids[2].1), 2);
    assert_eq!(w.readings_for(&p.ids[0].1), 0);
}

#[test]
fn an_observation_after_dissolution_is_unauthorized() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 200)).unwrap();
    a.append(p.observe(&monitor, 300, &[(2, None, None)])).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("after its dissolution")));
}


// --- D17: pruning the observation log, and the receipt it leaves ------------

const DAY: i64 = 86_400;
/// Comfortably past `PRUNE_SETTLING_SECONDS`, so an officer route is available.
const SETTLED: i64 = 40 * DAY;

#[test]
fn a_majority_may_prune_and_the_receipt_matches_what_it_drops() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 100, &[(2, None, Some(90))])).unwrap();
    a.append(p.observe(&monitor, 200, &[(2, None, Some(190))])).unwrap();
    a.append(p.observe(&monitor, 900, &[(2, None, Some(890))])).unwrap();
    let pruning = p.prune(&a, 500, &[0, 1], 1000);
    a.append(pruning).unwrap();

    verify_all(&a).unwrap();
    check_authority(&a).expect("the roll may prune its own observation log");

    // The record says what it drops, before anything is dropped.
    let dropped = prunable_observations(&a, &p.polity, t(500));
    assert_eq!(dropped.len(), 2);

    // Compaction removes exactly those, and only those.
    let compacted = apply_prunings(&a, &p.polity, t(1000)).unwrap();
    assert_eq!(compacted.len(), a.len() - 2);
    let w = observation_window(&compacted, &p.polity, t(0), t(1000));
    assert_eq!(w.reading_count(), 1);
    assert_eq!(w.observations[0].at, t(900));

    // And it advances the generation (D20), which is what tells a host holding a
    // cursor into the old log that the cursor is now meaningless. Without this its
    // next incremental save writes nothing and its storage keeps what went.
    assert_eq!(a.generation(), 0);
    assert_eq!(compacted.generation(), 1);
    assert!(compacted.dump_json_since(a.cursor()).is_err());
}

#[test]
fn a_prune_that_drops_nothing_still_advances_the_generation() {
    // Conservative on purpose: `apply_prunings` cannot know a host did not already
    // persist past this point, so every compaction pass counts as one, even a no-op.
    // Cheap — the host does one extra full rewrite — where guessing wrong is silent.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    let compacted = apply_prunings(&a, &p.polity, t(1000)).unwrap();
    assert_eq!(compacted.len(), a.len());
    assert_eq!(compacted.generation(), 1);
}

#[test]
fn compaction_preserves_every_governing_record() {
    // Everything that *decides* something must survive a prune untouched — the
    // property that makes observations the one prunable class.
    let p = Polity::new(3);
    let joiner = id();
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.amend(AmendmentKind::Add, "k", &[0, 1], 100)).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 150)).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1, 2], 160)).unwrap();
    a.append(p.induct(&joiner, &[0, 1, 2], 170)).unwrap();
    a.append(p.observe(&monitor, 180, &[(2, None, Some(170))])).unwrap();
    // The roll grew to four with the admission, so a majority is three.
    a.append(p.prune(&a, 500, &[0, 1, 2], 600)).unwrap();
    check_authority(&a).unwrap();

    let before = charter_state(&a, &p.polity, t(600));
    let compacted = apply_prunings(&a, &p.polity, t(600)).unwrap();

    // Exactly one entry went, and it was the observation.
    assert_eq!(compacted.len(), a.len() - 1);
    let kinds: Vec<&str> = {
        use en_archive::Entry as _;
        compacted.iter().map(|(_, e)| e.kind()).collect()
    };
    assert!(!kinds.contains(&"Observed"));
    for governing in [
        "PolityChartered",
        "CommitmentAmended",
        "MembershipChanged",
        "OfficeChanged",
        "MemberAffirmed",
        "ObservationsPruned",
    ] {
        assert!(kinds.contains(&governing), "{governing} must survive compaction");
    }
    // And the polity's decided state is bit-for-bit what it was.
    assert_eq!(charter_state(&compacted, &p.polity, t(600)), before);
    check_authority(&compacted).expect("a compacted log still passes its own rules");
}

#[test]
fn a_charter_keeper_may_prune_alone_but_only_past_the_settling_horizon() {
    let p = Polity::new(5); // majority 3, so one signature is decisively short
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1, 2], 100)).unwrap();
    a.append(p.observe(&monitor, 200, &[(4, None, Some(190))])).unwrap();

    // Beyond the settling horizon: the custodian's delegated power applies.
    a.append(p.prune(&a, SETTLED, &[0], SETTLED + PRUNE_SETTLING_SECONDS)).unwrap();
    check_authority(&a).expect("a CharterKeeper's custody is the delegated power D17 gives it");
}

#[test]
fn a_charter_keeper_may_not_prune_yesterday() {
    let p = Polity::new(5);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1, 2], 100)).unwrap();
    a.append(p.observe(&monitor, 200, &[(4, None, Some(190))])).unwrap();
    // A horizon one day back, pruned now: inside the settling period.
    a.append(p.prune(&a, 10 * DAY, &[0], 11 * DAY)).unwrap();

    let v = authority_violations(&a);
    assert_eq!(v.len(), 1, "recent evidence is out of any single custodian's reach");
    assert!(v[0].1.contains("CharterKeeper"));
}

#[test]
fn the_roll_may_still_prune_inside_the_settling_period() {
    // The floor binds the *officer route*, not the polity: the majority route is
    // always open, so the office can stall nothing.
    let p = Polity::new(5);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.observe(&monitor, 200, &[(4, None, Some(190))])).unwrap();
    a.append(p.prune(&a, 10 * DAY, &[0, 1, 2], 11 * DAY)).unwrap();
    check_authority(&a).expect("the roll is sovereign over its own log");
}

#[test]
fn a_non_keeper_alone_may_not_prune_however_old() {
    let p = Polity::new(5);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1, 2], 100)).unwrap();
    a.append(p.observe(&monitor, 200, &[(4, None, Some(190))])).unwrap();
    // Member 1 holds no office.
    a.append(p.prune(&a, SETTLED, &[1], SETTLED + PRUNE_SETTLING_SECONDS)).unwrap();

    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("majority of the roll"));
}

#[test]
fn a_departed_keeper_no_longer_prunes() {
    // The same lapse rule every office follows: effective holdings only.
    let p = Polity::new(5);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 4, &[0, 1, 2], 100)).unwrap();
    a.append(p.observe(&monitor, 200, &[(3, None, Some(190))])).unwrap();
    a.append(p.depart(4, 300)).unwrap();
    a.append(p.prune(&a, SETTLED, &[4], SETTLED + PRUNE_SETTLING_SECONDS)).unwrap();

    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("not a member"));
}

#[test]
fn a_pruning_horizon_must_advance() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 100, &[(2, None, Some(90))])).unwrap();
    a.append(p.prune(&a, 5000, &[0, 1], 6000)).unwrap();
    check_authority(&a).unwrap();
    // A receding horizon claims to drop what the first already dropped.
    a.append(p.prune(&a, 4000, &[0, 1], 7000)).unwrap();

    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("must advance"));
}

#[test]
fn a_false_receipt_is_caught_while_the_evidence_is_still_here() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 100, &[(2, None, Some(90))])).unwrap();
    a.append(p.observe(&monitor, 200, &[(2, None, Some(190))])).unwrap();

    // Understating the count.
    let mut wrong_count = a.clone();
    let honest = pruning_digest(&prunable_observations(&a, &p.polity, t(500))).unwrap();
    wrong_count
        .append(p.prune_claiming(&a, 500, &[0, 1], 600, Some((1, honest))))
        .unwrap();
    let v = authority_violations(&wrong_count);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("claims 1 dropped"));

    // Right count, wrong digest.
    let mut wrong_digest = a.clone();
    wrong_digest
        .append(p.prune_claiming(&a, 500, &[0, 1], 600, Some((2, Cid::of(&"nonsense").unwrap()))))
        .unwrap();
    let v = authority_violations(&wrong_digest);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("digest does not match"));
}

#[test]
fn an_applied_pruning_still_passes_authority_though_its_receipt_is_gone() {
    // The honest limit: once compaction has run the dropped entries are no longer
    // here to recompute, and a rule that insisted on recomputing would report every
    // historical pruning as a violation. The receipt keeps its force for anyone
    // holding an earlier copy of the log.
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 100, &[(2, None, Some(90))])).unwrap();
    a.append(p.observe(&monitor, 200, &[(2, None, Some(190))])).unwrap();
    a.append(p.prune(&a, 500, &[0, 1], 600)).unwrap();

    let compacted = apply_prunings(&a, &p.polity, t(600)).unwrap();
    assert_eq!(compacted.len(), 2); // charter + the pruning record
    check_authority(&compacted).expect("a log that has forgotten still passes its own rules");
    verify_all(&compacted).unwrap();
}

#[test]
fn pruning_cannot_be_authored_before_a_charter_or_after_dissolution() {
    let p = Polity::new(3);
    let mut early = Archive::new();
    early.append(p.prune(&Archive::new(), 500, &[0, 1], 600)).unwrap();
    let v = authority_violations(&early);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("precedes its charter"));

    let mut late = Archive::new();
    late.append(p.charter(&[0, 1], vec![])).unwrap();
    late.append(p.dissolve(&[], &[0, 1], 400)).unwrap();
    late.append(p.prune(&late, 500, &[0, 1], 600)).unwrap();
    let v = authority_violations(&late);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("after its dissolution"));
}

// --- D17: the window stays honest about what it no longer holds -------------

#[test]
fn a_pruning_older_than_the_window_does_not_truncate_it() {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 900, &[(2, None, Some(890))])).unwrap();
    a.append(p.prune(&a, 500, &[0, 1], 1000)).unwrap();

    // The caller asked about [600, 1000]; the horizon is behind that, so it took
    // nothing they asked for.
    let w = observation_window(&a, &p.polity, t(600), t(1000));
    assert_eq!(w.pruned_before, None);
    // Ask across the horizon and the window says so.
    let wide = observation_window(&a, &p.polity, t(0), t(1000));
    assert_eq!(wide.pruned_before, Some(t(500)));
}

#[test]
fn pruning_never_lengthens_a_measured_absence() {
    // The case the clamp exists for. `last_attested` is a max over the window's
    // stamps: drop the interval holding the HIGHEST stamp and that max falls, the
    // absence starts earlier, and the span grows — pruning manufacturing a Concern.
    let p = Polity::new(3);
    let monitor = id();
    let watched = p.ids[2].1.clone();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // An observation finding the guardian present, then a later one from a monitor
    // reporting a much staler reading. Both stamps precede their own look, as the
    // authority pass requires.
    a.append(p.observe(&monitor, 5_000, &[(2, None, Some(5_000))])).unwrap();
    a.append(p.observe(&monitor, 6_000, &[(2, None, Some(1_000))])).unwrap();
    check_authority(&a).unwrap();

    let before = observation_window(&a, &p.polity, t(0), t(10_000));
    assert_eq!(before.last_attested(&watched), Some(t(5_000)));
    let span_before = before.unattended_span(&watched, t(10_000)).unwrap();
    assert_eq!(span_before, 5_000);

    // Prune the interval holding the high stamp.
    a.append(p.prune(&a, 5_500, &[0, 1], 7_000)).unwrap();
    let compacted = apply_prunings(&a, &p.polity, t(7_000)).unwrap();
    let after = observation_window(&compacted, &p.polity, t(0), t(10_000));
    assert_eq!(after.pruned_before, Some(t(5_500)));
    assert_eq!(after.last_attested(&watched), Some(t(1_000)), "the high stamp is gone");

    let span_after = after.unattended_span(&watched, t(10_000)).unwrap();
    assert!(
        span_after <= span_before,
        "pruning may only weaken a signal: {span_after} must not exceed {span_before}"
    );
    // Clamped at the horizon, not run back to the surviving stale stamp — which
    // would have read 9_000, nearly twice the true absence.
    assert_eq!(span_after, 10_000 - 5_500);
}

#[test]
fn the_clamp_leaves_an_unpruned_window_exactly_as_it_was() {
    let p = Polity::new(3);
    let monitor = id();
    let watched = p.ids[2].1.clone();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 1_000, &[(2, None, Some(900))])).unwrap();
    let w = observation_window(&a, &p.polity, t(0), t(5_000));
    assert_eq!(w.pruned_before, None);
    assert_eq!(w.unattended_span(&watched, t(5_000)), Some(4_100));
}

#[test]
fn a_member_only_ever_seen_before_the_horizon_is_not_evaluated() {
    // Pruning must not turn "we watched them and they were absent" into a longer
    // absence; it turns it into no evidence at all, which is the honest reading.
    let p = Polity::new(3);
    let monitor = id();
    let watched = p.ids[2].1.clone();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 1_000, &[(2, None, None)])).unwrap();
    a.append(p.prune(&a, 2_000, &[0, 1], 3_000)).unwrap();

    let compacted = apply_prunings(&a, &p.polity, t(3_000)).unwrap();
    let w = observation_window(&compacted, &p.polity, t(0), t(9_000));
    assert_eq!(w.reading_count(), 0);
    assert_eq!(w.unattended_span(&watched, t(9_000)), None);
    assert_eq!(w.readings_for(&watched), 0);
}

#[test]
fn an_attendance_stamp_may_not_postdate_the_observation_reporting_it() {
    // A monitor cannot have seen an attestation that had not happened yet. The rule
    // is also what makes D17's clamp sound: because every stamp precedes its own
    // look, a dropped observation can only have carried stamps from before the
    // horizon, so dropping it can never lengthen a measured absence.
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.observe(&monitor, 200, &[(2, None, Some(500))])).unwrap();

    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("postdates the observation"));

    // A stamp at the very moment of the look is fine.
    let mut ok = Archive::new();
    ok.append(p.charter(&[0, 1], vec![])).unwrap();
    ok.append(p.observe(&monitor, 200, &[(2, None, Some(200))])).unwrap();
    check_authority(&ok).expect("a stamp taken at the moment of observation is coherent");
}

// --- D19: the retention policy over the observation log ---------------------

/// A polity with `n` observations spaced `spacing` apart starting at `first`, and a
/// `CharterKeeper` so the officer route is available where the horizon allows it.
fn observed_polity(n: usize, first: i64, spacing: i64) -> (Polity, Archive<CharterEntry>) {
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    for i in 0..n {
        let at = first + spacing * i as i64;
        a.append(p.observe(&monitor, at, &[(2, None, Some(at - 10))])).unwrap();
    }
    (p, a)
}

/// A policy with floors small enough that the tests can work in short spans. The
/// witness floor is switched off here and exercised on its own below, so the other
/// tests read against the duration floor alone.
fn policy(retain: i64, min_dropped: usize) -> RetentionPolicy {
    RetentionPolicy {
        retain_seconds: retain,
        min_dropped,
        live_measure_seconds: 60,
        min_witness_readings: 0,
    }
}

/// A retention long enough that the horizon is *settled*, so the single-keeper route
/// is open.
///
/// **The relationship worth keeping straight:** D17 opens the officer route only for
/// a horizon at least `PRUNE_SETTLING_SECONDS` old, and the horizon is
/// `as_of - retain_seconds` — so the keeper route is available exactly when
/// `retain_seconds >= PRUNE_SETTLING_SECONDS`. A *shorter* retention keeps more
/// history and is therefore MORE restrictive to authorize, not less. That is what
/// makes the 30-day default the smallest keeper-authorizable retention.
fn settled(min_dropped: usize) -> RetentionPolicy {
    policy(PRUNE_SETTLING_SECONDS, min_dropped)
}

/// `base` with `min_dropped` overridden — for the pairs of assertions that differ
/// only in whether the witness floor is switched on.
fn policy_with(base: &RetentionPolicy, min_dropped: usize) -> RetentionPolicy {
    RetentionPolicy { min_dropped, ..base.clone() }
}

#[test]
fn nothing_to_drop_is_no_recommendation() {
    // Negative control: a log entirely inside the retention window is not overdue.
    let (p, a) = observed_polity(5, 1000, 100);
    let as_of = t(1500);
    assert!(prune_recommendation(&a, &p.polity, &policy(10 * DAY, 1), as_of).is_none());
}

#[test]
fn a_recommendation_names_what_it_drops_and_who_may_authorize_it() {
    // Ten observations at 100s spacing from t=1000; read far enough ahead that all
    // of them are past a 1-hour retention.
    let (p, a) = observed_polity(10, 1000, 100);
    let as_of = t(60 * DAY);
    let r = prune_recommendation(&a, &p.polity, &settled(1), as_of)
        .expect("everything has aged out");

    assert_eq!(r.dropped, 10);
    assert_eq!(r.seconds_retained, PRUNE_SETTLING_SECONDS);
    assert_eq!(r.horizon, t(60 * DAY - PRUNE_SETTLING_SECONDS));
    // Both routes are open at this horizon, cheapest first.
    assert!(r.keeper_may_authorize());
    assert!(matches!(r.routes[0], PruneRoute::CharterKeeper { .. }));
    assert!(matches!(r.routes[1], PruneRoute::RollMajority { need: 2 }));
}

#[test]
fn the_recommendation_only_ever_proposes_what_the_authority_pass_accepts() {
    // The property the whole read-model exists for. Take its horizon verbatim,
    // author the pruning a single CharterKeeper signs, and the authority pass must
    // agree — otherwise the recommendation is worse than no recommendation.
    let (p, mut a) = observed_polity(10, 1000, 100);
    let as_of = t(60 * DAY);
    let r = prune_recommendation(&a, &p.polity, &settled(1), as_of).unwrap();
    assert!(r.keeper_may_authorize());

    a.append(p.prune(&a, r.horizon.as_second(), &[0], as_of.as_second())).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).expect("the recommended horizon must be authorizable as offered");

    let compacted = apply_prunings(&a, &p.polity, as_of).unwrap();
    assert_eq!(compacted.len(), a.len() - r.dropped);
}

#[test]
fn acting_on_a_recommendation_weakens_no_live_measure() {
    // Two observations a day apart establish an unattended span; a third is recent.
    // Pruning to the recommended horizon must leave the span the polity would grade
    // exactly as it was — D17's clamp, exercised through the policy that chose the
    // horizon rather than through a hand-picked one.
    let p = Polity::new(3);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    let machine = p.ids[2].1.clone();
    // Old observations, all well before the horizon.
    for at in [100i64, 200, 300] {
        a.append(p.observe(&monitor, at, &[(2, None, Some(90))])).unwrap();
    }
    let as_of = t(60 * DAY);
    // A recent one, inside the live-measure floor.
    a.append(p.observe(&monitor, as_of.as_second() - 30, &[(2, None, Some(90))]))
        .unwrap();

    let before = observation_window(&a, &p.polity, t(0), as_of).unattended_span(&machine, as_of);
    let r = prune_recommendation(&a, &p.polity, &settled(1), as_of).unwrap();
    a.append(p.prune(&a, r.horizon.as_second(), &[0], as_of.as_second())).unwrap();
    check_authority(&a).unwrap();
    let compacted = apply_prunings(&a, &p.polity, as_of).unwrap();
    let after =
        observation_window(&compacted, &p.polity, t(0), as_of).unattended_span(&machine, as_of);

    // Never longer: pruning may only ever weaken a signal, never manufacture one.
    match (before, after) {
        (Some(b), Some(af)) => assert!(af <= b, "span grew: {b} -> {af}"),
        (_, None) => {}
        (None, Some(af)) => panic!("pruning invented a span of {af}"),
    }
}

#[test]
fn the_horizon_never_crosses_the_live_measure_floor() {
    // A retention shorter than the floor is honoured only up to it. Without this,
    // a caller tuning storage could silently tune away the evidence a Concern rests
    // on.
    let (p, a) = observed_polity(10, 1000, 100);
    let as_of = t(60 * DAY);
    let pol = RetentionPolicy {
        retain_seconds: 1,
        min_dropped: 1,
        live_measure_seconds: 7200,
        min_witness_readings: 0,
    };
    let r = prune_recommendation(&a, &p.polity, &pol, as_of).unwrap();
    assert_eq!(r.seconds_retained, 7200, "floored at the live-measure span");
    assert_eq!(r.horizon, t(60 * DAY - 7200));
}

#[test]
fn a_horizon_that_would_not_advance_is_no_recommendation() {
    // D17 rejects a receding horizon, so the recommendation must never offer one:
    // having pruned, asking again at the same instant yields nothing.
    let (p, mut a) = observed_polity(10, 1000, 100);
    let as_of = t(60 * DAY);
    let r = prune_recommendation(&a, &p.polity, &settled(1), as_of).unwrap();
    a.append(p.prune(&a, r.horizon.as_second(), &[0], as_of.as_second())).unwrap();
    check_authority(&a).unwrap();

    assert!(
        prune_recommendation(&a, &p.polity, &settled(1), as_of).is_none(),
        "the same horizon is already in force"
    );
    // Time moves on and more observations age out -> it speaks again. The horizon
    // advances because `as_of` did.
    let later = t(70 * DAY);
    let monitor = id();
    let mut a2 = a;
    for i in 0..3i64 {
        a2.append(p.observe(&monitor, 35 * DAY + i, &[(2, None, Some(35 * DAY))])).unwrap();
    }
    let r2 = prune_recommendation(&a2, &p.polity, &settled(3), later)
        .expect("three more have aged past the advanced horizon");
    assert!(r2.horizon > r.horizon);
}

#[test]
fn one_aged_observation_does_not_earn_its_own_pruning_record() {
    // `min_dropped` exists because a pruning record is UNPRUNABLE: measured, a
    // keeper-authorized one is ~2.9 KB whatever the roll size, against 3.4-8.0 KB
    // for an observation. Pruning per observation trades reclaimable bytes for
    // permanent ones, which is worse the longer the polity runs.
    let (p, a) = observed_polity(1, 1000, 100);
    let as_of = t(60 * DAY);
    assert!(prune_recommendation(&a, &p.polity, &settled(5), as_of).is_none());
    // The same log, with the floor lowered to what is actually there, does speak.
    assert!(prune_recommendation(&a, &p.polity, &settled(1), as_of).is_some());
}

#[test]
fn a_recent_horizon_offers_only_the_roll() {
    // The officer route is bounded by D17's settling horizon, so a retention shorter
    // than 30 days must NOT offer a CharterKeeper even though one is in office.
    // The direction is the easy thing to get backwards: a SHORTER retention keeps
    // MORE history, so its horizon is younger, so it is *less* authorizable. One day
    // short of the settling horizon already closes the officer route.
    let (p, a) = observed_polity(10, 1000, 100);
    let as_of = t(60 * DAY);
    assert!(prune_recommendation(&a, &p.polity, &settled(1), as_of)
        .unwrap()
        .keeper_may_authorize());

    let pol = policy(PRUNE_SETTLING_SECONDS - DAY, 1);
    let r = prune_recommendation(&a, &p.polity, &pol, as_of).unwrap();
    assert!(!r.keeper_may_authorize(), "inside the settling window, roll only");
    assert_eq!(r.routes.len(), 1);
    assert!(matches!(r.routes[0], PruneRoute::RollMajority { need: 2 }));
}

#[test]
fn the_default_retention_is_both_safe_and_keeper_authorizable() {
    // The coincidence the default rests on, asserted rather than trusted to a
    // comment: PRUNE_SETTLING_SECONDS is the smallest retention that both preserves
    // every observation-based measure AND leaves the single-keeper route open.
    let d = RetentionPolicy::default();
    assert_eq!(d.retain_seconds, PRUNE_SETTLING_SECONDS);
    assert!(d.retain_seconds >= d.live_measure_seconds);

    // A polity still being watched right up to the query, so the duration floor is
    // the binding one. (A log that simply stops is pinned by the witness floor
    // instead — see `the_witness_floor_pins_a_log_that_stopped`.)
    let as_of = t(100 * DAY);
    let (p, a) = observed_polity(200, 1000, 43_400);
    let r = prune_recommendation(&a, &p.polity, &d, as_of).expect("200 observations have aged out");
    assert!(r.keeper_may_authorize(), "the default must leave the cheap route open");
    assert_eq!(r.seconds_retained, PRUNE_SETTLING_SECONDS);
}

#[test]
fn the_witness_floor_holds_back_the_horizon_the_duration_floor_would_allow() {
    // The defect the duration floor alone does not catch. `en-cohesion` grades a
    // sustained absence on a span AND on a witness count; D17's clamp preserves the
    // span, but nothing preserved the count until this floor. Measured on the Python
    // side: the same absence, the same horizon, graded Concern at a dense cadence and
    // Notice at a sparse one.
    let as_of = t(100 * DAY);
    let (p, a) = observed_polity(5, 60 * DAY, DAY);

    // Duration alone would take the horizon to 70 days, dropping all five.
    let loose = RetentionPolicy { min_witness_readings: 0, ..RetentionPolicy::default() };
    let r = prune_recommendation(&a, &p.polity, &policy_with(&loose, 1), as_of).unwrap();
    assert_eq!(r.dropped, 5, "every reading of this member would go");

    // The witness floor stops at the third-most-recent, leaving exactly three.
    let d = RetentionPolicy { min_dropped: 1, ..RetentionPolicy::default() };
    let r = prune_recommendation(&a, &p.polity, &d, as_of).unwrap();
    assert_eq!(r.dropped, 2, "three readings must survive");
    assert_eq!(r.horizon, t(62 * DAY));
    assert!(r.seconds_retained > PRUNE_SETTLING_SECONDS, "it keeps MORE than asked");
}

#[test]
fn the_witness_floor_pins_a_log_that_stopped() {
    // A polity nobody has observed for months has exactly its last three readings as
    // evidence. Dropping any of them would erase the absence they attest, so nothing
    // is recommended — the roll can still prune by majority, but it will not be
    // advised to.
    let as_of = t(100 * DAY);
    let d = RetentionPolicy { min_dropped: 1, ..RetentionPolicy::default() };

    let (p, a) = observed_polity(3, 1000, 100);
    assert!(prune_recommendation(&a, &p.polity, &d, as_of).is_none(),
            "three readings is the floor; none may go");

    // One more and the oldest becomes spare.
    let (p, a) = observed_polity(4, 1000, 100);
    let r = prune_recommendation(&a, &p.polity, &d, as_of).unwrap();
    assert_eq!(r.dropped, 1);
}

#[test]
fn the_most_sparsely_watched_member_binds_the_witness_floor() {
    // The count is per member because `readings_for` is per member: a member the
    // monitors mention rarely has thin evidence, and thinning it further is the clip
    // this read-model refuses.
    let p = Polity::new(4);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    // Member 2 watched ten times late; member 3 only three times, and early.
    for i in 0..3 {
        a.append(p.observe(&monitor, 1000 + 100 * i, &[(3, None, Some(900))])).unwrap();
    }
    for i in 0..10 {
        a.append(p.observe(&monitor, 50 * DAY + 100 * i, &[(2, None, Some(900))])).unwrap();
    }
    let as_of = t(100 * DAY);
    let d = RetentionPolicy { min_dropped: 1, ..RetentionPolicy::default() };
    assert!(prune_recommendation(&a, &p.polity, &d, as_of).is_none(),
            "the thinly-watched member has no reading to spare, so nothing goes");

    // Switch the floor off and the duration floor would happily drop all thirteen.
    let loose = RetentionPolicy { min_witness_readings: 0, ..RetentionPolicy::default() };
    let r = prune_recommendation(&a, &p.polity, &policy_with(&loose, 1), as_of).unwrap();
    assert_eq!(r.dropped, 13);
}

#[test]
fn an_unchartered_or_dissolved_polity_is_not_advised_to_prune() {
    // Negative controls at both ends of the lifecycle.
    let p = Polity::new(3);
    let empty: Archive<CharterEntry> = Archive::new();
    assert!(prune_recommendation(&empty, &p.polity, &policy(3600, 1), t(60 * DAY)).is_none());

    let (p2, mut a) = observed_polity(10, 1000, 100);
    a.append(p2.dissolve(&[], &[0, 1], 2000)).unwrap();
    assert!(prune_recommendation(&a, &p2.polity, &policy(3600, 1), t(60 * DAY)).is_none());
}

#[test]
fn a_departed_keeper_is_not_offered_as_a_route() {
    // The routes read live holdings, so an office that has lapsed off the roll is
    // not advertised as able to authorize anything.
    let p = Polity::new(5);
    let monitor = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1, 2], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 4, &[0, 1, 2], 50)).unwrap();
    for i in 0..10 {
        a.append(p.observe(&monitor, 1000 + 100 * i, &[(3, None, Some(900))])).unwrap();
    }
    let as_of = t(60 * DAY);
    assert!(prune_recommendation(&a, &p.polity, &settled(1), as_of)
        .unwrap()
        .keeper_may_authorize());

    // The keeper departs; the office lapses with them.
    a.append(p.depart(4, 2000)).unwrap();
    let r = prune_recommendation(&a, &p.polity, &settled(1), as_of).unwrap();
    assert!(!r.keeper_may_authorize(), "a departed keeper authorizes nothing");
}

// --- D21: the ratification quorum (resolving Q1) -----------------------------
//
// Before this, `majority(members.len())` took its denominator from the proposal's
// OWN roll and `propose` discarded the candidate it drew that roll from — so the
// charter had no referent outside itself. Measured: one node chartered a thirty-node
// cluster as a polity of itself, with one signature, passing both verification
// passes, and the other twenty-nine went unrecorded.

fn excl(p: &Polity, i: usize, reason: &str) -> Exclusion {
    Exclusion { member: p.ids[i].1.clone(), reason: reason.into() }
}

#[test]
fn a_faction_can_no_longer_charter_a_cluster_in_silence() {
    // **The Q1 regression.** A one-member polity drawn from a thirty-member cluster.
    let p = Polity::new(1);
    let mut a = Archive::new();
    // Claiming a boundary of thirty while naming one member and excluding nobody
    // does not add up, and is refused.
    a.append(p.charter_attested(
        &[0],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 30,
            exclusions: vec![],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("does not account for itself"), "{}", v[0].1);
}

#[test]
fn the_same_faction_may_still_charter_if_it_says_who_it_left_out() {
    // Nothing here refuses a *narrow* founding — a genuinely separate group is
    // indistinguishable from a faction without a judgement no rule can make. What is
    // refused is the silence. So: name them, and the charter stands.
    let p = Polity::new(1);
    let others: Vec<Exclusion> = (0..29)
        .map(|i| Exclusion { member: id().1, reason: format!("not of this tradition ({i})") })
        .collect();
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 30,
            exclusions: others,
        },
    ))
    .unwrap();
    check_authority(&a).expect("a narrow founding that accounts for itself is valid");

    // And the number Q1 asks for is now derivable, permanently.
    let l = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(l.candidate_size, 30);
    assert_eq!(l.roll_size, 1);
    assert_eq!(l.excluded, 29);
    assert_eq!(l.ratifiers, 1);
    assert!((l.consent_share - 1.0 / 30.0).abs() < 1e-9, "{}", l.consent_share);
    assert_eq!(l.basis, FoundingBasis::Narrowed);
    assert!(l.narrowed() && l.corroborated() && l.fully_reasoned());
}

#[test]
fn an_exclusion_may_not_also_be_on_the_roll() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 4,
            exclusions: vec![excl(&p, 2, "left")],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("excluded and also on the roll"), "{}", v[0].1);
}

#[test]
fn a_duplicate_exclusion_is_refused() {
    let p = Polity::new(2);
    let outsider = id().1;
    let dup = Exclusion { member: outsider.clone(), reason: "twice".into() };
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 4,
            exclusions: vec![dup.clone(), dup],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("duplicate exclusion"), "{}", v[0].1);
}

#[test]
fn a_convener_asserted_founding_says_so_forever() {
    // manual_founding keeps working, and the charter never pretends it was
    // corroborated — the same honesty `guardian: None` carries (D15).
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    check_authority(&a).expect("a convener's own roll accounts for itself trivially");

    let l = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(l.source, BoundarySource::ConvenerAsserted);
    assert_eq!(l.basis, FoundingBasis::SelfAttested);
    assert!(!l.corroborated(), "nobody but the convener saw this boundary");
    assert!(!l.narrowed(), "a convener's roll is the whole of what they claim");
    assert_eq!(l.candidate_size, 3);
}

#[test]
fn a_whole_cluster_ratifying_reads_as_corroborated() {
    let p = Polity::new(5);
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1, 2],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 5,
            exclusions: vec![],
        },
    ))
    .unwrap();
    check_authority(&a).unwrap();
    let l = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(l.basis, FoundingBasis::Corroborated);
    assert!((l.consent_share - 0.6).abs() < 1e-9);
    assert!(!l.narrowed());
}

#[test]
fn an_unreasoned_exclusion_is_counted_but_not_refused() {
    // Naming someone is the accounting, and that is gated. Saying *why* is the
    // judgement, and that is graded — an empty reason is accounting without
    // justification, which the read-model reports rather than the authority pass
    // refusing.
    let p = Polity::new(2);
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 4,
            exclusions: vec![
                Exclusion { member: id().1, reason: String::new() },
                Exclusion { member: id().1, reason: "  ".into() },
            ],
        },
    ))
    .unwrap();
    check_authority(&a).expect("an unreasoned exclusion is still accounted for");
    let l = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(l.excluded, 2);
    assert_eq!(l.unreasoned_exclusions, 2, "whitespace is not a reason");
    assert!(!l.fully_reasoned());
}

#[test]
fn an_unchartered_polity_has_no_founding_to_read() {
    let p = Polity::new(3);
    let empty: Archive<CharterEntry> = Archive::new();
    assert!(founding_legitimacy(&empty, &p.polity).is_none());
}

#[test]
fn the_attestation_is_genesis_immutable_because_the_charter_is() {
    // Not a new rule — a restatement of one. There is no amendment path to the
    // charter's core, so a founding cannot later be re-described as broader than it
    // was. That is what makes the grade re-derivable forever.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // A second charter is refused by the genesis singleton rule, so there is no way
    // to append a competing account of the same polity's boundary.
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate,
            candidate_size: 3,
            exclusions: vec![],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert!(v.iter().any(|(_, m)| m.contains("genesis is a singleton")));
    // And the first account is what stands.
    let l = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(l.source, BoundarySource::ConvenerAsserted);
}
