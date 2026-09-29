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
    /// The polity's own signing key. Nothing in the charter path uses it — the roll
    /// signs, not the polity — but `en-uplift` signs a `CommunityIdentity` under the
    /// polity root, so the key exists, and D24 needs it to test that the polity
    /// vouching for its own reader does not count.
    polity_key: PrivKey,
    ids: Vec<(PrivKey, DidKey)>,
}

impl Polity {
    fn new(n: usize) -> Self {
        let (polity_key, polity) = id();
        Polity { polity, polity_key, ids: (0..n).map(|_| id()).collect() }
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

    /// A signed observation showing exactly `derived`'s guardian edges, taken before
    /// the founding — what D27 requires a claimed founding edge to carry.
    fn founding_observation(
        &self,
        derived: &[(usize, Option<&(PrivKey, DidKey)>)],
    ) -> Signed<Observation> {
        let (sk, observer) = id();
        let ob = Observation {
            polity: self.polity.clone(),
            observer: observer.clone(),
            observed_at: t(40),
            readings: derived
                .iter()
                .map(|(m, g)| MemberReading {
                    member: self.ids[*m].1.clone(),
                    reputation: 0.9,
                    guardian: g.map(|g| g.1.clone()),
                    attested_at: None,
                })
                .collect(),
            opaque_boundary: false,
        };
        Signed::build(ob, &[(observer.as_str(), &sk)]).unwrap()
    }

    /// Founding derivations whose claimed edges are **backed** by one observation
    /// (D27). The default for tests that are about something other than backing.
    /// Every member ratifies, so every member holds the substrate. This is what a
    /// fixture means when it says "a healthy polity of n": since **D29** ratifying is
    /// what inducts a founding member, a partial ratification leaves members on the
    /// roll who carry no deliberative weight, hold no office and witness no rite —
    /// correct, and not what most of these tests are about.
    fn all_ratify(&self) -> Vec<usize> {
        (0..self.ids.len()).collect()
    }

    /// [`Self::charter`] ratified by the whole roll.
    fn charter_seasoned(&self, founding: Vec<Commitment>) -> CharterEntry {
        self.charter(&self.all_ratify(), founding)
    }

    /// [`Self::charter_derived`] ratified by the whole roll.
    fn charter_derived_seasoned(
        &self,
        founding: Vec<Commitment>,
        derived: &[(usize, Option<&(PrivKey, DidKey)>)],
    ) -> CharterEntry {
        self.charter_derived(&self.all_ratify(), founding, derived)
    }

    fn charter_derived(
        &self,
        ratifiers: &[usize],
        founding: Vec<Commitment>,
        derived: &[(usize, Option<&(PrivKey, DidKey)>)],
    ) -> CharterEntry {
        let ob = self.founding_observation(derived);
        self.charter_derived_with(ratifiers, founding, derived, |m, g| Derivation {
            member: m,
            guardian: Some(g),
            evidence: Some(GuardianEvidence::Observed(Box::new(ob.clone()))),
        })
    }

    fn charter_derived_with(
        &self,
        ratifiers: &[usize],
        founding: Vec<Commitment>,
        derived: &[(usize, Option<&(PrivKey, DidKey)>)],
        mut guarded: impl FnMut(DidKey, DidKey) -> Derivation,
    ) -> CharterEntry {
        let prop = FoundingProposal::convener_asserted(
            self.polity.clone(),
            substrate(),
            founding,
            self.roll(),
            derived
                .iter()
                .map(|(m, g)| {
                    let member = self.ids[*m].1.clone();
                    match g {
                        Some(g) => guarded(member, g.1.clone()),
                        None => Derivation::unguarded(member),
                    }
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
                    guardian: g.cloned(),
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
    #[allow(clippy::too_many_arguments)]
    fn guardian_raw(
        &self,
        kind: GuardianKind,
        member: usize,
        guardian: Option<&DidKey>,
        deciders: &[usize],
        at: i64,
        signers: &[(&str, &PrivKey)],
        evidence: Option<GuardianEvidence>,
    ) -> CharterEntry {
        let gc = GuardianChange {
            polity: self.polity.clone(),
            kind,
            member: self.ids[member].1.clone(),
            guardian: guardian.cloned(),
            evidence,
            rationale: "r".into(),
            deciders: self.dids(deciders),
            decided_at: t(at),
        };
        CharterEntry::GuardianChanged(Signed::build(gc, signers).unwrap())
    }

    /// A signed observation showing `member` guarded by `guardian` at `at`, of the
    /// shape a guardian change carries as its substrate evidence (D15).
    fn edge_seen(
        &self,
        observer: &(PrivKey, DidKey),
        member: usize,
        guardian: Option<&DidKey>,
        at: i64,
    ) -> GuardianEvidence {
        let CharterEntry::Observed(signed) =
            self.observe(observer, at, &[(member, guardian, Some(at - 10))])
        else {
            unreachable!("observe builds an Observed entry")
        };
        GuardianEvidence::Observed(Box::new(signed))
    }

    /// The override: no monitor has reported, and the record says so.
    fn edge_unbacked(reason: &str) -> GuardianEvidence {
        GuardianEvidence::Unbacked { reason: reason.into() }
    }

    /// Designate a member a derived party: the `deciders` co-sign at a majority and
    /// the named guardian (if any) co-signs its acceptance.
    fn designate(
        &self,
        member: usize,
        guardian: Option<&(PrivKey, DidKey)>,
        deciders: &[usize],
        at: i64,
        evidence: Option<GuardianEvidence>,
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
            evidence,
        )
    }

    /// Rotate a designated member to a new guardian, who consents.
    fn rotate_guardian(
        &self,
        member: usize,
        incoming: &(PrivKey, DidKey),
        deciders: &[usize],
        at: i64,
        evidence: Option<GuardianEvidence>,
    ) -> CharterEntry {
        let mut signers = self.signers(deciders);
        signers.push((incoming.1.as_str(), &incoming.0));
        self.guardian_raw(
            GuardianKind::Rotate, member, Some(&incoming.1), deciders, at, &signers, evidence,
        )
    }

    /// The guardian steps away on their own signature (no deciders).
    fn release_by_guardian(&self, member: usize, guardian: &(PrivKey, DidKey), at: i64) -> CharterEntry {
        let signers = vec![(guardian.1.as_str(), &guardian.0)];
        self.guardian_raw(GuardianKind::Release, member, None, &[], at, &signers, None)
    }

    /// The roll records at a majority that the human is gone.
    fn release_by_roll(&self, member: usize, deciders: &[usize], at: i64) -> CharterEntry {
        let signers = self.signers(deciders);
        self.guardian_raw(GuardianKind::Release, member, None, deciders, at, &signers, None)
    }

    /// An envoy record with an explicit signer set and public key, for the
    /// rejection cases. `envoy_pubkey` derives from the DID unless overridden.
    // Seven axes because the rejection cases need seven independent things to make
    // wrong: an unauthorized kind, a pubkey that does not derive from the DID, an
    // authorizer set below quorum, an expiry already past, a decision instant, and a
    // signer set that does not match the authorizers. The ergonomic wrappers above
    // exist so ordinary tests never call this; the ones that do are here precisely to
    // vary one axis at a time, and a params struct would put a builder between the
    // test and the malformed record it is trying to state.
    #[allow(clippy::too_many_arguments)]
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
        movements: Vec<Cid>,
        at: i64,
        signers: &[(&str, &PrivKey)],
    ) -> CharterEntry {
        let af = Affirmation {
            polity: self.polity.clone(),
            member: member.clone(),
            kind,
            witnesses: self.dids(witnesses),
            movements,
            affirmed_at: t(at),
        };
        CharterEntry::MemberAffirmed(Signed::build(af, signers).unwrap())
    }

    /// Perform the five movements of `member`'s rite (design **D29**), attended by
    /// `attending`, one second apart from `start`. Returns the movement CIDs in order,
    /// which the completion must cite.
    fn rite(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        attending: &[usize],
        start: i64,
    ) -> Vec<Cid> {
        self.rite_of(a, member, attending, start, AffirmationKind::Induction)
    }

    /// As [`Self::rite`], naming the ceremony (**D34**).
    fn rite_of(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        attending: &[usize],
        start: i64,
        kind: AffirmationKind,
    ) -> Vec<Cid> {
        let mut cids = Vec::new();
        for (i, movement) in self.movements(a, start).into_iter().enumerate() {
            cids.push(self.movement_of(a, member, attending, movement, start + i as i64, kind));
        }
        cids
    }

    /// The five movements a valid rite recites, read off the polity's own charter as it
    /// stands — the substrate it names, the commitments in force, the anchor if any.
    /// A fixture that hardcoded these would be testing the coherence rules against
    /// itself.
    fn movements(&self, a: &Archive<CharterEntry>, at: i64) -> Vec<Movement> {
        let st = charter_state(a, &self.polity, t(at));
        let sub = st.substrate.clone().expect("the polity is chartered");
        vec![
            Movement::Substrate { names: sub.name.clone() },
            Movement::Fallenness,
            Movement::Recognition,
            Movement::CommonVirtue {
                commits_to: st.commitments.iter().map(|c| c.key.clone()).collect(),
            },
            Movement::NonUltimacy { names_anchor: sub.anchor.clone() },
        ]
    }

    /// Append one movement of `member`'s **induction** rite and return its CID.
    fn movement(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        attending: &[usize],
        movement: Movement,
        at: i64,
    ) -> Cid {
        self.movement_of(a, member, attending, movement, at, AffirmationKind::Induction)
    }

    /// As [`movement`](Self::movement), naming the ceremony (**D34**) — the renewal path
    /// takes `ReAffirmation`.
    fn movement_of(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        attending: &[usize],
        movement: Movement,
        at: i64,
        kind: AffirmationKind,
    ) -> Cid {
        let rm = RiteMovement {
            polity: self.polity.clone(),
            member: member.1.clone(),
            kind,
            movement,
            statement: "as we have received it".into(),
            attending: self.dids(attending),
            recited_at: t(at),
        };
        let mut signers = self.signers(attending);
        if !signers.iter().any(|(d, _)| *d == member.1.as_str()) {
            signers.push((member.1.as_str(), &member.0));
        }
        a.append(CharterEntry::RiteMovementRecorded(
            Signed::build(rm, &signers).unwrap(),
        ))
        .unwrap()
    }

    /// Induct `member` — `&p.ids[i]` for a founding member, or an outside identity
    /// already admitted to the roll: perform the rite `witnesses` attend, then append
    /// the completion citing it. Returns the completion's CID.
    ///
    /// The rite runs over the five seconds before `at`, so a caller that only cares
    /// about the completion does not have to schedule the movements.
    fn induct(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        witnesses: &[usize],
        at: i64,
    ) -> Cid {
        let movements = self.rite(a, member, witnesses, at - 5);
        self.induct_citing(a, member, witnesses, movements, at)
    }

    /// As [`Self::induct`], with the citation supplied — for the cases where what the
    /// completion claims about its rite is the thing under test.
    fn induct_citing(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        witnesses: &[usize],
        movements: Vec<Cid>,
        at: i64,
    ) -> Cid {
        let mut signers = self.signers(witnesses);
        if !signers.iter().any(|(d, _)| *d == member.1.as_str()) {
            signers.push((member.1.as_str(), &member.0));
        }
        let entry =
            self.affirm(&member.1, AffirmationKind::Induction, witnesses, movements, at, &signers);
        a.append(entry).unwrap()
    }

    /// A renewal with **no re-recitation at all** — the pre-D34 shape, which is now
    /// refused. Kept for the tests whose subject is a refusal that fires before the
    /// citation rule is reached, and for D34's own regression test.
    fn reaffirm_bare(&self, member: &(PrivKey, DidKey), at: i64) -> CharterEntry {
        let signers = [(member.1.as_str(), &member.0)];
        self.affirm(&member.1, AffirmationKind::ReAffirmation, &[], vec![], at, &signers)
    }

    /// `member` renews their affirmation on a full re-recitation (**D34**), unattended.
    ///
    /// The five movements are dated `at - 5 ..= at - 1`, so the renewal itself still
    /// lands exactly on `at` and a caller's cadence assertions are unaffected.
    fn renew(&self, a: &mut Archive<CharterEntry>, member: &(PrivKey, DidKey), at: i64) -> Cid {
        self.renew_attended(a, member, &[], at)
    }

    /// As [`Self::renew`], with attendance — which is recorded and never required.
    fn renew_attended(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        attending: &[usize],
        at: i64,
    ) -> Cid {
        let movements =
            self.rite_of(a, member, attending, at - 5, AffirmationKind::ReAffirmation);
        self.renew_citing(a, member, attending, movements, at)
    }

    /// As [`Self::renew_attended`], with the citation supplied — for the cases where what
    /// the renewal claims about its re-recitation is the thing under test.
    fn renew_citing(
        &self,
        a: &mut Archive<CharterEntry>,
        member: &(PrivKey, DidKey),
        witnesses: &[usize],
        movements: Vec<Cid>,
        at: i64,
    ) -> Cid {
        let signers = [(member.1.as_str(), &member.0)];
        let entry = self.affirm(
            &member.1,
            AffirmationKind::ReAffirmation,
            witnesses,
            movements,
            at,
            &signers,
        );
        a.append(entry).unwrap()
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
    reject.append(p.charter_seasoned(vec![commit("a")])).unwrap();
    reject.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 200)).unwrap();
    reject.append(p.amend(AmendmentKind::Remove, "b", &[0, 1], 300)).unwrap();
    assert!(reasons(&reject).iter().any(|m| m.contains("Remove needs more than a majority")));

    // Accept (separate archive): all three remove it.
    let mut accept = Archive::new();
    accept.append(p.charter_seasoned(vec![commit("a")])).unwrap();
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
fn induction_not_admission_makes_the_roll_live_for_later_quorums() {
    // The ripple test, corrected by **D29**: what moves a later quorum is the point at
    // which a new member acquires the substrate, not the point at which they join the
    // roll. Admission alone leaves the bar where it was — the joiner carries no weight
    // yet — and that is the whole difference between being on the roll and holding what
    // the polity is.
    let p = Polity::new(3);
    let joiner = id();

    // Admitted and not inducted: the bar is still a majority of the three founders.
    let mut admitted = Archive::new();
    admitted.append(p.charter_seasoned(vec![commit("a")])).unwrap();
    admitted.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    admitted.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 300)).unwrap();
    verify_all(&admitted).unwrap();
    check_authority(&admitted).expect("2 of the 3 who hold the substrate is a majority");

    // Once the joiner is inducted the weighted roll is four, so two no longer carry it.
    let mut reject = Archive::new();
    reject.append(p.charter_seasoned(vec![commit("a")])).unwrap();
    reject.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut reject, &joiner, &[0, 1], 250);
    reject.append(p.amend(AmendmentKind::Add, "b", &[0, 1], 300)).unwrap();
    verify_all(&reject).unwrap();
    assert!(
        reasons(&reject).iter().any(|m| m.contains("Add needs a majority (>= 3)")),
        "got: {:?}",
        reasons(&reject)
    );

    // With three of the now-four inducted members, the Add is authorized.
    let mut accept = Archive::new();
    accept.append(p.charter_seasoned(vec![commit("a")])).unwrap();
    accept.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut accept, &joiner, &[0, 1], 250);
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
    a.append(p.charter_seasoned(vec![commit("a")])).unwrap(); // majority(4) = 3
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
    a.append(p.charter_seasoned(vec![])).unwrap();
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
    sub.append(p.charter_seasoned(vec![])).unwrap();
    sub.append(p.expel(4, &[0, 1, 2], 200)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("Expel needs more than a majority (>= 4)")));

    // The target cannot be among the deciders.
    let mut selfexpel = Archive::new();
    selfexpel.append(p.charter_seasoned(vec![])).unwrap();
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
    // Grow the roll to 4 and induct the newcomer, then a 2-of-4 dissolution is
    // sub-majority (needs 3). Since D29 the majority is of the members who hold the
    // substrate, so the induction is what makes the fourth member count.
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut a, &joiner, &[0, 1], 250);
    a.append(p.dissolve(&[], &[0, 1], 300)).unwrap();
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("dissolution needs a majority of the roll (>= 3)")));
}

#[test]
fn confer_at_majority_with_consent_grants_the_office() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
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
    a.append(p.charter_seasoned(vec![])).unwrap();
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
    a.append(p.charter_seasoned(vec![])).unwrap();
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
    sub.append(p.charter_seasoned(vec![])).unwrap();
    sub.append(p.confer(Office::Convener, 2, &[0], 200)).unwrap();
    assert!(reasons(&sub).iter().any(|m| m.contains("conferral needs a majority (>= 2)")));

    // Already holds the office.
    let mut dup = Archive::new();
    dup.append(p.charter_seasoned(vec![])).unwrap();
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
    nc.append(p.charter_seasoned(vec![])).unwrap();
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
    a.append(p.charter_seasoned(vec![])).unwrap(); // majority(4) = 3
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
    a.append(p.charter_seasoned(vec![])).unwrap();
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

/// Ratifying the charter *is* a founding member's induction — and only the
/// ratifiers'. A founding needs a majority, so a listed member who never signed is on
/// the roll and un-inducted (**D29**). Until this was measured, the seed read the
/// founding roll and handed the substrate to whoever was listed: a roll of 7 ratified
/// by 4 reported 7 inducted, dated to an instant three of them were absent from.
#[test]
fn ratifiers_are_inducted_at_founding_and_the_rest_of_the_roll_is_not() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();

    let st = affirmation_state(&a, &p.polity, t(300));
    assert_eq!(st.entries.len(), 3, "all three are on the roll");
    assert!(st.is_inducted(&p.ids[0].1));
    assert!(st.is_inducted(&p.ids[1].1));
    assert_eq!(st.last_affirmed(&p.ids[0].1), Some(t(100)));
    assert_eq!(st.last_affirmed(&p.ids[1].1), Some(t(100)));
    // NEGATIVE CONTROL on the old arithmetic: member 2 was listed and signed
    // nothing, so it holds no substrate and carries no affirmation date.
    assert!(!st.is_inducted(&p.ids[2].1), "a non-ratifier is not inducted by fiat");
    assert_eq!(st.last_affirmed(&p.ids[2].1), None);
    assert_eq!(st.entries.iter().filter(|e| e.inducted).count(), 2);
}

#[test]
fn an_admitted_member_is_uninducted_until_the_roll_receives_them() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();

    // On the roll, but the substrate has not been transmitted yet.
    let before = affirmation_state(&a, &p.polity, t(250));
    assert_eq!(before.entries.len(), 4);
    assert!(!before.is_inducted(&joiner.1));
    assert_eq!(before.last_affirmed(&joiner.1), None);

    // Roll of 4 now, so induction needs 3 witnesses.
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    verify_all(&a).expect("signatures valid");
    check_authority(&a).expect("roll-witnessed induction authorized");

    let after = affirmation_state(&a, &p.polity, t(350));
    assert!(after.is_inducted(&joiner.1));
    assert_eq!(after.last_affirmed(&joiner.1), Some(t(300)));

    // A re-affirmation refreshes the timestamp without changing induction. Since D34 it
    // carries a full re-recitation of its own, so this is a second rite for the same
    // member — an induction they already had, then a renewal.
    p.renew(&mut a, &joiner, 900);
    verify_all(&a).unwrap();
    check_authority(&a).expect("self-signed renewal on a re-recitation authorized");
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
    p.renew(&mut a, &p.ids[0], 900);
    p.renew(&mut a, &p.ids[1], 900);
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    // At t = 1000 with a 500-second cadence: 0 and 1 are fresh (100s ago), while
    // member 2 (a listed founder who never ratified) and the joiner are both stale
    // for the *same* reason — never inducted, so nothing to be fresh about.
    let stale = affirmation_state(&a, &p.polity, t(1000)).stale_members(500, t(1000));
    assert!(!stale.contains(&p.ids[0].1));
    assert!(!stale.contains(&p.ids[1].1));
    assert!(stale.contains(&p.ids[2].1));
    assert!(stale.contains(&joiner.1));
    assert_eq!(stale.len(), 2);

    // A cadence long enough to cover the whole history leaves exactly the
    // un-inducted stale, and being un-inducted is not a cadence question: no
    // cadence, however lax, makes a member who never received the substrate fresh.
    let lax = affirmation_state(&a, &p.polity, t(1000)).stale_members(5_000, t(1000));
    assert_eq!(lax, vec![p.ids[2].1.clone(), joiner.1.clone()]);
}

#[test]
fn induction_rejections() {
    let p = Polity::new(3);
    let joiner = id();

    // Sub-majority witnesses. Three founders hold the substrate and the inductee does
    // not count toward its own reception, so two must witness and one is short.
    let mut sub = Archive::new();
    sub.append(p.charter_seasoned(vec![])).unwrap();
    sub.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut sub, &joiner, &[0], 300);
    verify_all(&sub).expect("signatures valid");
    assert!(
        reasons(&sub).iter().any(|m| m.contains("majority of the inducted roll to witness (>= 2)")),
        "got: {:?}",
        reasons(&sub)
    );

    // Someone who was never admitted cannot be inducted.
    let mut outsider = Archive::new();
    outsider.append(p.charter_seasoned(vec![])).unwrap();
    p.induct(&mut outsider, &joiner, &[0, 1], 300);
    assert!(reasons(&outsider).iter().any(|m| m.contains("affirmation by a non-member")));

    // A *ratifier* is already inducted by ratification.
    let mut founder = Archive::new();
    founder.append(p.charter_seasoned(vec![])).unwrap();
    p.induct(&mut founder, &p.ids[0], &[0, 1], 300);
    assert!(reasons(&founder).iter().any(|m| m.contains("already-inducted")));

    // But a listed founding member who never ratified MAY be inducted, and this is
    // the honest state D29 opened up: on the roll, substrate not yet received. The
    // fixture above used exactly this member to test the already-inducted rule,
    // which is how the seeding defect stayed invisible.
    let mut latecomer = Archive::new();
    // Ratified by two of three on purpose: member 2 is the listed-but-unsigned founder.
    latecomer.append(p.charter(&[0, 1], vec![])).unwrap();
    p.induct(&mut latecomer, &p.ids[2], &[0, 1], 300);
    assert!(
        reasons(&latecomer).is_empty(),
        "a non-ratifying founder is inducted like anyone else: {:?}",
        reasons(&latecomer)
    );

    // Nor can a member be inducted twice.
    let mut twice = Archive::new();
    twice.append(p.charter_seasoned(vec![])).unwrap();
    twice.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut twice, &joiner, &[0, 1, 2], 300);
    p.induct(&mut twice, &joiner, &[0, 1, 2], 400);
    assert!(reasons(&twice).iter().any(|m| m.contains("already-inducted")));

    // The inductee must co-sign: the community receives them, and they participate.
    let mut silent = Archive::new();
    silent.append(p.charter_seasoned(vec![])).unwrap();
    silent.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    let movements = p.rite(&mut silent, &joiner, &[0, 1], 250);
    silent
        .append(p.affirm(
            &joiner.1,
            AffirmationKind::Induction,
            &[0, 1],
            movements,
            300,
            &p.signers(&[0, 1]),
        ))
        .unwrap();
    verify_all(&silent).expect("the witnesses' signatures are valid");
    assert!(reasons(&silent).iter().any(|m| m.contains("witnesses plus the inductee")));

    // A witness must be a current member.
    let mut stranger = Archive::new();
    stranger.append(p.charter_seasoned(vec![])).unwrap();
    stranger.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    let outside = id();
    let movements = p.rite(&mut stranger, &joiner, &[0, 1], 250);
    let af = Affirmation {
        polity: p.polity.clone(),
        member: joiner.1.clone(),
        kind: AffirmationKind::Induction,
        witnesses: vec![p.ids[0].1.clone(), p.ids[1].1.clone(), outside.1.clone()],
        movements,
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
    early.append(p.reaffirm_bare(&joiner, 300)).unwrap();
    verify_all(&early).expect("signature valid");
    assert!(reasons(&early).iter().any(|m| m.contains("never inducted")));

    // **This assertion used to say a renewal records NO witnesses**, which was true only
    // because a renewal recorded nothing at all. Since D34 it carries a re-recitation and
    // may well have been attended — what it may not do is *choose* who attended. The
    // witnesses must be exactly the rite's attendance-throughout, so a claimed witness
    // who was not at every movement is refused.
    let mut overclaimed = Archive::new();
    overclaimed.append(p.charter_seasoned(vec![])).unwrap();
    let movements =
        p.rite_of(&mut overclaimed, &p.ids[0], &[1], 295, AffirmationKind::ReAffirmation);
    // Member 2 did not attend a single movement, and is named anyway.
    p.renew_citing(&mut overclaimed, &p.ids[0], &[1, 2], movements, 300);
    verify_all(&overclaimed).expect("signature valid");
    assert!(reasons(&overclaimed).iter().any(|m| m.contains("exactly who attended")));

    // And a renewal with no re-recitation at all is refused — the D34 regression.
    let mut bare = Archive::new();
    bare.append(p.charter_seasoned(vec![])).unwrap();
    bare.append(p.reaffirm_bare(&p.ids[0], 300)).unwrap();
    verify_all(&bare).expect("signature valid");
    assert!(reasons(&bare)
        .iter()
        .any(|m| m.contains("0 of 5 have been recorded")));

    // And nobody can renew on another member's behalf — which needs a *valid*
    // re-recitation to reach, since otherwise the citation rule refuses it first.
    let mut proxy = Archive::new();
    proxy.append(p.charter_seasoned(vec![])).unwrap();
    let recited = p.rite_of(&mut proxy, &p.ids[0], &[], 295, AffirmationKind::ReAffirmation);
    proxy
        .append(p.affirm(
            &p.ids[0].1,
            AffirmationKind::ReAffirmation,
            &[],
            recited,
            300,
            &p.signers(&[1]),
        ))
        .unwrap();
    verify_all(&proxy).expect("the other member's signature is valid");
    assert!(reasons(&proxy).iter().any(|m| m.contains("exactly the affirming member")));

    // A re-recitation completes ONE renewal: replaying it to reset freshness twice off a
    // single ceremony is refused.
    let mut replayed = Archive::new();
    replayed.append(p.charter_seasoned(vec![])).unwrap();
    let once = p.rite_of(&mut replayed, &p.ids[0], &[], 295, AffirmationKind::ReAffirmation);
    p.renew_citing(&mut replayed, &p.ids[0], &[], once.clone(), 300);
    p.renew_citing(&mut replayed, &p.ids[0], &[], once, 900);
    verify_all(&replayed).expect("both signatures valid");
    assert!(reasons(&replayed).iter().any(|m| m.contains("already been renewed on")));

    // A renewal may not pass an induction's rite off as its re-recitation. Reachable
    // only by naming that rite's own CIDs, since a renewal citing nothing is told it
    // cites nothing first — so the case is constructed exactly that way.
    let mut borrowed = Archive::new();
    borrowed.append(p.charter_seasoned(vec![])).unwrap();
    borrowed.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    let induction_rite = p.rite(&mut borrowed, &joiner, &[0, 1, 2], 295);
    p.induct_citing(&mut borrowed, &joiner, &[0, 1, 2], induction_rite.clone(), 300);
    p.renew_citing(&mut borrowed, &joiner, &[0, 1, 2], induction_rite, 400);
    verify_all(&borrowed).expect("signatures valid");
    assert!(
        reasons(&borrowed).iter().any(|m| m.contains("cites an induction's rite")),
        "got: {:?}",
        reasons(&borrowed)
    );
}

#[test]
fn affirmation_before_charter_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.reaffirm_bare(&p.ids[0], 200)).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("precedes its charter")));
}

#[test]
fn affirmation_after_dissolution_is_rejected() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 200)).unwrap();
    a.append(p.reaffirm_bare(&p.ids[0], 300)).unwrap();
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
// Roll of 7 throughout (6 founders + an admitted joiner), and since **D29** the bar is
// a majority of the *inducted* roll — the six founders, because the inductee does not
// count toward its own reception. majority(6) = 4, so the roll's override bar is 5, and
// the arithmetic these tests describe is unchanged.

/// Charter a polity of 6 (all ratifying, so all hold the substrate), admit `joiner`,
/// and (optionally) confer the Inductor.
fn presided(inductor: Option<usize>) -> (Polity, (PrivKey, DidKey), Archive<CharterEntry>) {
    let p = Polity::new(6);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
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
    p.induct(&mut a, &joiner, &[0, 1, 2, 3], 300);
    verify_all(&a).unwrap();
    check_authority(&a).expect("the Inductor presided at a majority");
}

#[test]
fn without_the_inductor_a_plain_majority_no_longer_suffices() {
    let (p, joiner, mut a) = presided(Some(0));
    // Four witnesses is a majority of the roll of 7, and would have authorized the
    // induction before the office was conferred — see the vacant-office case below.
    p.induct(&mut a, &joiner, &[1, 2, 3, 4], 300);
    verify_all(&a).unwrap();
    assert!(reasons(&a).iter().any(|m| m.contains("Inductor other than the inductee")));
}

#[test]
fn the_roll_may_induct_without_the_inductor_at_its_own_higher_bar() {
    // The whole point of a *defeasible* gate: an officer raises the bar for the
    // rite and can never block it, so the roll stays sovereign (D12).
    let (p, joiner, mut a) = presided(Some(0));
    p.induct(&mut a, &joiner, &[1, 2, 3, 4, 5], 300);
    verify_all(&a).unwrap();
    check_authority(&a).expect("majority + 1 is the roll's override");
}

#[test]
fn a_vacant_inductor_leaves_the_majority_rule_untouched() {
    let (p, joiner, mut a) = presided(None);
    p.induct(&mut a, &joiner, &[1, 2, 3, 4], 300);
    verify_all(&a).unwrap();
    check_authority(&a).expect("no office held, so a majority witnesses as before");
}

#[test]
fn an_inductor_who_has_left_the_roll_does_not_preside() {
    // D12: a holding is effective only while its holder is on the roll.
    let (p, joiner, mut a) = presided(Some(5));
    a.append(p.depart(5, 260)).unwrap();
    // Roll of 6 now, so a majority is 4 — and the departed Inductor gates nothing.
    p.induct(&mut a, &joiner, &[1, 2, 3, 4], 300);
    verify_all(&a).unwrap();
    check_authority(&a).expect("a lapsed holding does not preside");
}

#[test]
fn any_one_of_several_inductors_suffices() {
    let (p, joiner, mut a) = presided(Some(0));
    a.append(p.confer(Office::Inductor, 5, &[0, 1, 2, 3], 260)).unwrap();
    p.induct(&mut a, &joiner, &[2, 3, 4, 5], 300);
    verify_all(&a).unwrap();
    check_authority(&a).expect("the second Inductor presided");
}

/// D14 refused an inductee presiding over their own reception. **D29 makes that
/// unreachable**, and by a stronger rule: an office may only be conferred on a member
/// who holds the substrate, and an inductee by definition does not yet. So the case
/// D14's clause was written for cannot be constructed — the conferral is refused before
/// any rite begins.
///
/// The clause is kept (see `authority.rs`) rather than deleted, on the same reasoning
/// as the KC self-CID exclusion: it is the rule's own statement of what it means, and
/// a later change that allowed re-induction would need it again.
#[test]
fn the_inductee_cannot_be_made_an_inductor_at_all() {
    let p = Polity::new(6);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1, 2, 3], 200)).unwrap();
    // The polity tries to make its newest member — not yet inducted — its Inductor.
    a.append(p.confer_on(Office::Inductor, &joiner, &[0, 1, 2, 3], 250)).unwrap();
    verify_all(&a).unwrap();
    assert!(
        reasons(&a).iter().any(|m| m.contains("has not been \n                                 inducted")
            || m.contains("has not been inducted")),
        "got: {:?}",
        reasons(&a)
    );

    // And with no Inductor effectively held, the plain majority receives them.
    let mut over = a;
    p.induct(&mut over, &joiner, &[1, 2, 3, 4], 300);
    verify_all(&over).unwrap();
    let left: Vec<String> = reasons(&over)
        .into_iter()
        .filter(|m| !m.contains("inducted; an office"))
        .collect();
    assert!(left.is_empty(), "the induction itself is authorized: {left:?}");
}

// --- D14: the EnvoyAuthorizer's delegated authority --------------------------

/// Charter a polity of 5 (majority = 3) and (optionally) confer EnvoyAuthorizer.
fn with_authorizer(officer: Option<usize>) -> (Polity, Archive<CharterEntry>) {
    let p = Polity::new(5);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
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
            vec![Derivation::unbacked(
            p.ids[1].1.clone(),
            human.1.clone(),
            "not what this test is about",
        )],
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
            Derivation::unbacked(
                p.ids[1].1.clone(),
                p.ids[2].1.clone(),
                "not what this test is about",
            ),
            Derivation::unguarded(p.ids[2].1.clone()),
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
            vec![Derivation::unguarded(outsider.1.clone())],
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

/// MEASUREMENT (D15 co-signing slice, step 1) — kept as the regression test.
///
/// Why a `GuardianChange` must **carry** its substrate evidence rather than cite it.
///
/// D17 lets a polity drop observations, by a signed act with a receipt, and
/// `apply_prunings` really removes them (a new generation, not a tombstone). So an
/// authority rule that resolved a cited observation CID would hold a record
/// authorized today and unauthorized after a *legitimate* housekeeping pruning —
/// authority would stop being a property of the log and start being a property of
/// when you looked. Measured here: after pruning, the observation is gone from the
/// archive entirely, which is exactly what a citation would dangle against.
///
/// Same shape as D20's finding that D17's incremental save and its own compaction did
/// not compose. Carried evidence (the D22 pattern) has no such interaction: a copy
/// inside a record cannot be pruned without breaking the record's own signature.
#[test]
fn a_cited_observation_would_not_survive_a_legitimate_pruning() {
    let p = Polity::new(3);
    let monitor = id();
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    // An observation showing member 2's guardian — the evidence a rule would want.
    a.append(p.observe(&monitor, 100, &[(2, Some(&human.1), Some(90))])).unwrap();
    // The change carries its own copy of that evidence.
    a.append(p.designate(2, Some(&human), &[0, 1], 200,
                         Some(p.edge_seen(&monitor, 2, Some(&human.1), 100)))).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    let observations = |arch: &Archive<CharterEntry>| {
        arch.iter().filter(|(_, e)| matches!(e, CharterEntry::Observed(_))).count()
    };
    assert_eq!(observations(&a), 1, "the evidence is in the log to begin with");

    // A CharterKeeper prunes past D17's 30-day settling horizon — an authorized act.
    const SETTLED: i64 = 2_592_000 + 150 + 10;
    a.append(p.prune(&a, 150, &[0], SETTLED)).unwrap();
    check_authority(&a).expect("the pruning is itself authorized");
    let compacted = apply_prunings(&a, &p.polity, t(SETTLED)).unwrap();

    assert_eq!(
        observations(&compacted), 0,
        "MEASURED: the observation a citation would point at is gone after a \
         legitimate pruning — so the evidence must travel inside the record"
    );
    assert_eq!(observations(&a), 1, "the pre-compaction log is untouched by the read");
    // And the guardian change is still there and still authorized, which is what
    // carrying the evidence has to preserve.
    verify_all(&compacted).unwrap();
    check_authority(&compacted).expect("the guardian change survives the pruning");
}

/// MEASUREMENT (step 1) — the unbacked edge, kept as the regression test.
///
/// A guardian edge could be chartered with **nothing in the log showing it**: no
/// observation, no reading, no substrate evidence of any kind, and both passes green.
/// The corpus asserted this as correct — `designate_rotate_and_release_move_the_edge`
/// designates and rotates with no observation anywhere and says "designate, rotate
/// and release all authorized". Five tests move a guardian edge and none of them
/// records an observation.
///
/// That is one level up from the hole D15 itself turned out to have (a chartered rule
/// with no feed), and the same shape as `substrate.anchor` in D23 and KC's
/// `source_commitment`: a rule whose evidence nothing supplies, asserted correct by a
/// passing test.
#[test]
fn an_edge_the_substrate_never_showed_is_refused() {
    let p = Polity::new(3);
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    // No observation anywhere, and no declared override: nothing has ever reported
    // that `human` guards 2.
    a.append(p.designate(2, Some(&human), &[0, 1], 200, None)).unwrap();
    verify_all(&a).expect("crypto-intact either way");
    let violations = authority_violations(&a);
    assert!(
        violations.iter().any(|(_, why)| why.contains("substrate")),
        "MEASURED: authorized before the evidence rule existed. Violations: {violations:?}"
    );
}

#[test]
fn designate_rotate_and_release_move_the_edge() {
    let p = Polity::new(3);
    let monitor = id();
    let first = id();
    let second = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();

    a.append(p.designate(2, Some(&first), &[0, 1], 200,
                         Some(p.edge_seen(&monitor, 2, Some(&first.1), 150)))).unwrap();
    assert_eq!(
        guardian_state(&a, &p.polity, t(250)).guardian_of(&p.ids[2].1),
        Some(&Some(first.1.clone()))
    );

    a.append(p.rotate_guardian(2, &second, &[0, 1], 300,
                               Some(p.edge_seen(&monitor, 2, Some(&second.1), 250)))).unwrap();
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
    let monitor = id();
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.designate(2, Some(&human), &[0, 1], 200,
                         Some(p.edge_seen(&monitor, 2, Some(&human.1), 150)))).unwrap();
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
            // The override, not real evidence: these fixtures test *other* rules, and
            // the evidence check runs first, so a missing one would mask them.
            Some(Polity::edge_unbacked("rejection fixture")),
        ))
        .unwrap();
    assert!(reasons(&unsigned).iter().any(|m| m.contains("plus the named guardian")));

    // Nothing may guard itself.
    let mut selfish = base.clone();
    selfish
        .append(p.designate(2, Some(&(p.ids[2].0.clone(), p.ids[2].1.clone())), &[0, 1], 200,
                            Some(Polity::edge_unbacked("rejection fixture"))))
        .unwrap();
    assert!(reasons(&selfish).iter().any(|m| m.contains("its own guardian")));

    // Rotation presupposes a designation, and must name the incoming guardian.
    let mut early = base.clone();
    early.append(p.rotate_guardian(2, &human, &[0, 1], 200,
                                   Some(Polity::edge_unbacked("rejection fixture")))).unwrap();
    assert!(reasons(&early).iter().any(|m| m.contains("not a designated derived party")));

    // No second designation for the same member.
    let mut twice = base.clone();
    twice.append(p.designate(2, Some(&human), &[0, 1], 200,
                             Some(Polity::edge_unbacked("rejection fixture")))).unwrap();
    twice.append(p.designate(2, Some(&other), &[0, 1], 300,
                             Some(Polity::edge_unbacked("rejection fixture")))).unwrap();
    assert!(reasons(&twice).iter().any(|m| m.contains("already designated")));

    // A release of an edge that is already absent says nothing.
    let mut empty = base.clone();
    // No guardian named, so no edge is claimed and no evidence may be carried.
    empty.append(p.designate(2, None, &[0, 1], 200, None)).unwrap();
    empty.append(p.release_by_roll(2, &[0, 1], 300)).unwrap();
    assert!(reasons(&empty).iter().any(|m| m.contains("already absent")));

    // A machine cannot be rotated onto another machine.
    let mut looped = base;
    looped.append(p.designate(1, None, &[0, 1], 200, None)).unwrap();
    looped.append(p.designate(2, Some(&human), &[0, 1], 300,
                              Some(Polity::edge_unbacked("rejection fixture")))).unwrap();
    looped
        .append(p.guardian_raw(
            GuardianKind::Rotate,
            2,
            Some(&p.ids[1].1),
            &[0, 1],
            400,
            &p.signers(&[0, 1]),
            Some(Polity::edge_unbacked("rejection fixture")),
        ))
        .unwrap();
    assert!(reasons(&looped).iter().any(|m| m.contains("cannot guard a machine")));
}

/// The substrate-evidence rule (D15), case by case. Each rejection names the rule it
/// trips, and each is paired with the authorizing case — a gate that refused
/// everything would pass the rejections and mean nothing.
#[test]
fn guardian_evidence_rejections_and_the_cases_that_pass() {
    let p = Polity::new(3);
    let monitor = id();
    let human = id();
    let other = id();
    let mut base = Archive::new();
    base.append(p.charter(&[0, 1], vec![])).unwrap();

    // The authorizing case: an observation showing exactly this edge, seen before the
    // decision, signed by its observer.
    let mut good = base.clone();
    good.append(p.designate(2, Some(&human), &[0, 1], 200,
                            Some(p.edge_seen(&monitor, 2, Some(&human.1), 150)))).unwrap();
    verify_all(&good).unwrap();
    check_authority(&good).expect("evidence showing the claimed edge authorizes it");

    // The override authorizes too, and says why.
    let mut declared = base.clone();
    declared.append(p.designate(2, Some(&human), &[0, 1], 200,
                                Some(Polity::edge_unbacked("bootstrap: no monitor yet")))).unwrap();
    check_authority(&declared).expect("a declared override authorizes");

    // ...but not an unexplained one.
    let mut silent = base.clone();
    silent.append(p.designate(2, Some(&human), &[0, 1], 200,
                              Some(Polity::edge_unbacked("   ")))).unwrap();
    assert!(reasons(&silent).iter().any(|m| m.contains("must say why")));

    // Evidence showing a different guardian argues against the claim.
    let mut wrong = base.clone();
    wrong.append(p.designate(2, Some(&human), &[0, 1], 200,
                             Some(p.edge_seen(&monitor, 2, Some(&other.1), 150)))).unwrap();
    assert!(reasons(&wrong).iter().any(|m| m.contains("not the claimed")));

    // Evidence showing no guardian at all is not evidence of one.
    let mut none_seen = base.clone();
    none_seen.append(p.designate(2, Some(&human), &[0, 1], 200,
                                 Some(p.edge_seen(&monitor, 2, None, 150)))).unwrap();
    assert!(reasons(&none_seen).iter().any(|m| m.contains("with no guardian")));

    // Evidence about a different member says nothing about this one.
    let mut elsewhere = base.clone();
    elsewhere.append(p.designate(2, Some(&human), &[0, 1], 200,
                                 Some(p.edge_seen(&monitor, 1, Some(&human.1), 150)))).unwrap();
    assert!(reasons(&elsewhere).iter().any(|m| m.contains("carries no reading for")));

    // A reading cannot back a decision it postdates: evidence is what was seen
    // before the polity decided, not a justification assembled afterwards.
    let mut after = base.clone();
    after.append(p.designate(2, Some(&human), &[0, 1], 200,
                             Some(p.edge_seen(&monitor, 2, Some(&human.1), 300)))).unwrap();
    assert!(reasons(&after).iter().any(|m| m.contains("after the change decided")));

    // A release claims the edge is *gone*; evidence of one present argues the
    // opposite, and D16 says an absent reading is not evidence of absence either.
    let mut released = base.clone();
    released.append(p.designate(2, Some(&human), &[0, 1], 200,
                                Some(p.edge_seen(&monitor, 2, Some(&human.1), 150)))).unwrap();
    released
        .append(p.guardian_raw(
            GuardianKind::Release, 2, None, &[0, 1], 300, &p.signers(&[0, 1]),
            Some(p.edge_seen(&monitor, 2, Some(&human.1), 250)),
        ))
        .unwrap();
    assert!(reasons(&released).iter().any(|m| m.contains("claims the edge is gone")));

    // Carried evidence may not be put in somebody else's mouth, exactly as a
    // standalone observation may not.
    let mut forged = base;
    let CharterEntry::Observed(signed) =
        p.observe_signed(&monitor, 150, &[(2, Some(&human.1), Some(140))],
                         &[(other.1.as_str(), &other.0)])
    else {
        unreachable!()
    };
    forged
        .append(p.designate(2, Some(&human), &[0, 1], 200,
                            Some(GuardianEvidence::Observed(Box::new(signed)))))
        .unwrap();
    assert!(reasons(&forged).iter().any(|m| m.contains("signed by exactly its observer")));
}

/// **The control that proves carrying beats citing.** The observation the evidence was
/// copied from is pruned by an authorized act, `apply_prunings` removes it, and the
/// guardian change still verifies and is still authorized — because its evidence never
/// lived in the log to begin with. A citation would have dangled here; the measurement
/// that established this is `a_cited_observation_would_not_survive_a_legitimate_pruning`.
#[test]
fn carried_evidence_survives_the_pruning_of_its_source() {
    let p = Polity::new(3);
    let monitor = id();
    let human = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    a.append(p.observe(&monitor, 100, &[(2, Some(&human.1), Some(90))])).unwrap();
    a.append(p.designate(2, Some(&human), &[0, 1], 200,
                         Some(p.edge_seen(&monitor, 2, Some(&human.1), 100)))).unwrap();
    check_authority(&a).unwrap();

    const SETTLED: i64 = 2_592_000 + 150 + 10;
    a.append(p.prune(&a, 150, &[0], SETTLED)).unwrap();
    let compacted = apply_prunings(&a, &p.polity, t(SETTLED)).unwrap();
    assert_eq!(
        compacted.iter().filter(|(_, e)| matches!(e, CharterEntry::Observed(_))).count(),
        0,
        "the source observation is gone from the log"
    );
    verify_all(&compacted).expect("the carried copy still verifies");
    check_authority(&compacted).expect("and still backs the edge it was carried for");
    // The edge itself is unchanged by the housekeeping.
    assert_eq!(
        guardian_state(&compacted, &p.polity, t(SETTLED)).guardian_of(&p.ids[2].1),
        Some(&Some(human.1.clone()))
    );
}

#[test]
fn a_designation_lapses_off_the_roll_and_revives_on_return() {
    let p = Polity::new(4);
    let human = id();
    let mut a = Archive::new();
    let monitor = id();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.designate(3, Some(&human), &[0, 1, 2], 200,
                         Some(p.edge_seen(&monitor, 3, Some(&human.1), 150)))).unwrap();
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
    a.append(p.charter_derived_seasoned(vec![], &[(1, Some(&human)), (2, None)])).unwrap();
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
    let monitor = id();
    a.append(p.charter_derived_seasoned(vec![], &[(1, Some(&first)), (2, None)])).unwrap();
    a.append(p.rotate_guardian(2, &second, &[0, 1], 200,
                               Some(p.edge_seen(&monitor, 2, Some(&second.1), 150)))).unwrap();
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
    a.append(p.charter_derived_seasoned(vec![], &[(1, Some(&human)), (2, Some(&human))])).unwrap();
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
    a.append(p.charter_derived_seasoned(vec![], &[(4, None)])).unwrap();
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
    two.append(p.charter_derived_seasoned(vec![], &[(3, None), (4, None)])).unwrap();
    assert!(!guardian_state(&two, &p.polity, t(200)).is_machine_majority(5));
    two.append(p.confer(Office::Convener, 3, &[0, 1, 2], 200)).unwrap();
    check_authority(&two).expect("two of five leaves the rule dormant");

    let mut three = Archive::new();
    three
        .append(p.charter_derived_seasoned(vec![], &[(2, None), (3, None), (4, None)]))
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
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1], 200)).unwrap();
    // Roll of 4: an induction needs 3 witnesses.
    p.induct(&mut a, &joiners[0], &[0, 1, 2], 300);
    a.append(p.admit(&joiners[1], &[0, 1, 2], 400)).unwrap();
    // Roll of 5: 3 witnesses again, but one fewer turns out than could.
    p.induct(&mut a, &joiners[1], &[0, 1, 2], 900);
    p.renew(&mut a, &joiners[0], 1500);
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
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    assert_eq!(decay_trends(&a, &p.polity, t(400)).latest_gap_widening(), None);

    p.renew(&mut a, &joiner, 400);
    assert_eq!(
        decay_trends(&a, &p.polity, t(500)).latest_gap_widening(),
        None,
        "one gap is still not a cadence"
    );

    // Gaps of 100 then 1000: the latest exceeds the median of the prior gaps.
    p.renew(&mut a, &joiner, 1400);
    assert_eq!(decay_trends(&a, &p.polity, t(1500)).latest_gap_widening(), Some(true));
}

#[test]
fn a_steady_cadence_is_not_widening() {
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    for at in [400i64, 500, 600] {
        p.renew(&mut a, &joiner, at);
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
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    for at in [400i64, 5400, 5500] {
        p.renew(&mut a, &joiner, at);
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
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    // Induction at 300 is itself an affirmation point, so the first gap is 300->400.
    for at in [400i64, 800, 1200, 1600] {
        p.renew(&mut a, &joiner, at);
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
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    p.renew(&mut a, &joiner, 400);
    assert_eq!(
        decay_trends(&a, &p.polity, t(500)).gap_widening_since_founding(),
        None,
        "one gap is not a cadence"
    );

    for at in [500i64, 600] {
        p.renew(&mut a, &joiner, at);
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
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1, 2, 3], 200)).unwrap();
    // Roll of 7, majority 4 — but five turn out to receive the first joiner.
    p.induct(&mut a, &joiners[0], &[0, 1, 2, 3, 4], 300);
    assert!(
        !decay_trends(&a, &p.polity, t(400)).witness_count_declined(),
        "one induction is not a trend"
    );

    a.append(p.admit(&joiners[1], &[0, 1, 2, 3], 400)).unwrap();
    // Roll of 8, majority 5: the bare minimum turns out this time.
    p.induct(&mut a, &joiners[1], &[0, 1, 2, 3, 4], 500);
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
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiners[0], &[0, 1, 2, 3], 200)).unwrap();
    // Six of the roll of seven receive the first joiner...
    p.induct(&mut a, &joiners[0], &[0, 1, 2, 3, 4, 5], 300);
    a.append(p.admit(&joiners[1], &[0, 1, 2, 3], 400)).unwrap();
    // ...and only the bare majority of eight receives the second.
    p.induct(&mut a, &joiners[1], &[0, 1, 2, 3, 4], 500);
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
    p.induct(&mut a, &joiner, &[0, 1, 2], 300);
    p.renew(&mut a, &joiner, 900);

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
    // Two filings, one monitor — and since D28 the two numbers say different things.
    assert_eq!(w.watchers_for(&p.ids[2].1), 1);
    assert_eq!(w.watchers_for(&p.ids[0].1), 0);
}

#[test]
fn three_filings_by_one_key_are_one_watcher() {
    // **THE MEASURED HOLE, half one.** `en-cohesion` escalates a sustained absence to
    // `Concern` — the grade that crosses to kith-covenant as a conduct record — on a
    // span plus a witness count, and D19 justified that count as "evidence that
    // somebody was watching". It counted filings. One key, three filings, and a
    // stranger with no relationship to the polity manufactured the escalation for the
    // price of one keypair.
    let p = Polity::new(3);
    let solo = id();
    let g = id();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    for k in 0..3 {
        a.append(p.observe(&solo, 1000 + k * 2 * 86_400, &[(0, Some(&g.1), Some(1000))]))
            .unwrap();
    }
    check_authority(&a).unwrap();
    let w = observation_window(&a, &p.polity, t(0), t(1000 + 10 * 86_400));
    assert_eq!(w.readings_for(&p.ids[0].1), 3, "three filings, as it says");
    assert_eq!(w.watchers_for(&p.ids[0].1), 1, "and one party's word");

    // The control: the same three filings from three keys really are three watchers.
    let mut b = Archive::new();
    b.append(p.charter(&[0, 1], vec![])).unwrap();
    for k in 0..3 {
        let m = id();
        b.append(p.observe(&m, 1000 + k * 2 * 86_400, &[(0, Some(&g.1), Some(1000))]))
            .unwrap();
    }
    let w = observation_window(&b, &p.polity, t(0), t(1000 + 10 * 86_400));
    assert_eq!(w.watchers_for(&p.ids[0].1), 3);
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
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.amend(AmendmentKind::Add, "k", &[0, 1], 100)).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 150)).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1, 2], 160)).unwrap();
    p.induct(&mut a, &joiner, &[0, 1, 2], 170);
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
    a.append(p.charter_seasoned(vec![])).unwrap();
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
///
/// **Three monitors, rotating** (**D28**). This fixture used one monitor filing
/// repeatedly, which satisfied a floor whose entire justification was "somebody was
/// watching" — three of these tests were green while asserting exactly the hole D28
/// closes. The single-monitor case now has its own test, where it belongs.
fn observed_polity(n: usize, first: i64, spacing: i64) -> (Polity, Archive<CharterEntry>) {
    let monitors: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
    observed_by(&monitors, n, first, spacing)
}

/// `observed_polity`, with the watching rotated over whichever monitors are given —
/// one of them for the case where a lone actor files over and over.
fn observed_by(
    monitors: &[(PrivKey, DidKey)],
    n: usize,
    first: i64,
    spacing: i64,
) -> (Polity, Archive<CharterEntry>) {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    for i in 0..n {
        let at = first + spacing * i as i64;
        let m = &monitors[i % monitors.len()];
        a.append(p.observe(m, at, &[(2, None, Some(at - 10))])).unwrap();
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
        min_witness_watchers: 0,
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
        min_witness_watchers: 0,
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
    let loose = RetentionPolicy { min_witness_watchers: 0, ..RetentionPolicy::default() };
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
fn a_lone_monitor_filing_forever_never_frees_the_horizon() {
    // **D28, and the reason the fixture above had to change.** Two hundred filings by
    // one key used to clear a floor of three, because the floor counted filings. It
    // counts watchers now, so a lone monitor pins the horizon however long it keeps
    // going: there is exactly one party's word about this member, and dropping any of
    // it leaves the same one party's word.
    let as_of = t(100 * DAY);
    let d = RetentionPolicy { min_dropped: 1, ..RetentionPolicy::default() };
    let solo = vec![id()];
    let (p, a) = observed_by(&solo, 200, 1000, 43_400);
    assert!(
        prune_recommendation(&a, &p.polity, &d, as_of).is_none(),
        "one watcher is one watcher, 200 filings or not"
    );

    // The control: the same 200 filings spread over three monitors free it at once.
    let three: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
    let (p, a) = observed_by(&three, 200, 1000, 43_400);
    assert!(prune_recommendation(&a, &p.polity, &d, as_of).is_some());
}

#[test]
fn the_most_sparsely_watched_member_binds_the_witness_floor() {
    // The count is per member because `watchers_for` is per member: a member the
    // monitors mention rarely has thin evidence, and thinning it further is the clip
    // this read-model refuses.
    let p = Polity::new(4);
    let monitors: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
    let mut a = Archive::new();
    a.append(p.charter(&[0, 1], vec![])).unwrap();
    a.append(p.confer(Office::CharterKeeper, 0, &[0, 1], 50)).unwrap();
    // Member 3 seen by all three monitors, but only early. Member 2 watched ten times
    // late — and the *watching* is what binds, so member 3 is the sparse one only
    // because their evidence is old, not because it is thin.
    for i in 0..3 {
        a.append(p.observe(&monitors[i as usize], 1000 + 100 * i, &[(3, None, Some(900))]))
            .unwrap();
    }
    for i in 0..10 {
        let m = &monitors[(i as usize) % 3];
        a.append(p.observe(m, 50 * DAY + 100 * i, &[(2, None, Some(900))])).unwrap();
    }
    let as_of = t(100 * DAY);
    let d = RetentionPolicy { min_dropped: 1, ..RetentionPolicy::default() };
    assert!(prune_recommendation(&a, &p.polity, &d, as_of).is_none(),
            "the thinly-watched member has no reading to spare, so nothing goes");

    // Switch the floor off and the duration floor would happily drop all thirteen.
    let loose = RetentionPolicy { min_witness_watchers: 0, ..RetentionPolicy::default() };
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

/// A signed reading of `polity` showing exactly `members`, by a fresh reader
/// (design **D22**). Standing in for what a host signs with the key the provider
/// does not hold.
fn reading_of(polity: &DidKey, members: Vec<DidKey>) -> Signed<SubstrateReading> {
    let (sk, reader) = id();
    let r = SubstrateReading {
        polity: polity.clone(),
        reader: reader.clone(),
        read_at: t(40),
        members,
        opaque_boundary: false,
    };
    Signed::build(r, &[(reader.as_str(), &sk)]).unwrap()
}

/// A `Substrate` boundary of `size` showing the polity's whole roll plus everyone
/// the exclusions name — the shape D22's identity rule demands.
fn read_source(p: &Polity, exclusions: &[Exclusion]) -> BoundarySource {
    let mut members = p.roll();
    members.extend(exclusions.iter().map(|e| e.member.clone()));
    BoundarySource::Substrate { readings: vec![reading_of(&p.polity, members)] }
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
            source: BoundarySource::ConvenerAsserted,
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
            source: read_source(&p, &others),
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
            source: BoundarySource::ConvenerAsserted,
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
            source: BoundarySource::ConvenerAsserted,
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
            source: read_source(&p, &[]),
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
            source: BoundarySource::ConvenerAsserted,
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
            source: read_source(&p, &[]),
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


// ---- D22: the boundary is attributable, and the excluded answer back ---------

/// A dissent entry signed by `who` (or, for the negative case, by `signer`).
fn dissent_by(
    polity: &DidKey,
    who: &(PrivKey, DidKey),
    signer: &(PrivKey, DidKey),
    kind: DissentKind,
    at: i64,
) -> CharterEntry {
    let d = BoundaryDissent {
        polity: polity.clone(),
        dissenter: who.1.clone(),
        kind,
        rationale: "I was inside that cluster".into(),
        dissented_at: t(at),
    };
    CharterEntry::BoundaryDissented(
        Signed::build(d, &[(signer.1.as_str(), &signer.0)]).unwrap(),
    )
}

#[test]
fn a_convener_can_no_longer_claim_a_reading_they_do_not_hold() {
    // **The Q5 regression, measured before the slice.** Flipping one public field
    // promoted a convener's bare assertion to `Substrate` — and the grade with it to
    // `Corroborated` — with no provider consulted and nothing signed. The label was
    // an unsigned self-report, so it was evidence of nothing.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate { readings: vec![] },
            candidate_size: 3,
            exclusions: vec![],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("carries none"), "{}", v[0].1);
}

#[test]
fn a_reading_must_be_signed_by_the_reader_it_names() {
    // D16's rule for observations, applied to readings: nobody may put a reading in
    // somebody else's mouth.
    let p = Polity::new(3);
    let (_sk, reader) = id();
    let (impostor_sk, impostor) = id();
    let r = SubstrateReading {
        polity: p.polity.clone(),
        reader,
        read_at: t(40),
        members: p.roll(),
        opaque_boundary: false,
    };
    let forged = Signed::build(r, &[(impostor.as_str(), &impostor_sk)]).unwrap();
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate { readings: vec![forged] },
            candidate_size: 3,
            exclusions: vec![],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("not signed by it alone"), "{}", v[0].1);
}

#[test]
fn a_reading_of_one_cluster_does_not_travel_to_another_founding() {
    // Anti-replay. Without the binding, an honest reader's account of one cluster
    // founds somebody else's polity and the signature still verifies.
    let p = Polity::new(3);
    let elsewhere = id().1;
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate {
                readings: vec![reading_of(&elsewhere, p.roll())],
            },
            candidate_size: 3,
            exclusions: vec![],
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("is for polity"), "{}", v[0].1);
}

#[test]
fn a_gerrymandered_boundary_is_refused_although_its_arithmetic_is_perfect() {
    // **The teeth.** D21 could only check that the numbers added up, and it checked
    // them against the proposer's own report — so a boundary that named a wholly
    // different set of the right SIZE sailed through. Here the right-hand side is
    // signed by somebody else, so the check is identity, not arithmetic.
    let p = Polity::new(1);
    // An honest reader saw the polity's member plus two others.
    let real: Vec<DidKey> = vec![p.ids[0].1.clone(), id().1, id().1];
    // The convener excludes two people the reader never saw — the count is right and
    // every name is wrong.
    let substituted: Vec<Exclusion> = (0..2)
        .map(|i| Exclusion { member: id().1, reason: format!("not of this tradition ({i})") })
        .collect();
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0],
        BoundaryAttestation {
            source: BoundarySource::Substrate { readings: vec![reading_of(&p.polity, real)] },
            candidate_size: 3,
            exclusions: substituted,
        },
    ))
    .unwrap();
    // D21's rule is satisfied: 3 == 1 + 2.
    let att = match a.iter().next().unwrap().1 {
        CharterEntry::PolityChartered(s) => s.payload.attestation.clone(),
        _ => unreachable!(),
    };
    assert!(att.accounts_for(1), "the arithmetic is perfect");
    // D22's is not.
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("no signed reading shows the boundary"), "{}", v[0].1);
}

#[test]
fn a_reading_that_counts_differently_from_the_boundary_is_refused() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate {
                readings: vec![reading_of(&p.polity, p.roll())],
            },
            candidate_size: 30, // the reading shows 3
            exclusions: (0..27)
                .map(|i| Exclusion { member: id().1, reason: format!("({i})") })
                .collect(),
        },
    ))
    .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("but the boundary claims"), "{}", v[0].1);
}

#[test]
fn corroboration_counts_readers_who_agree_not_readings_filed() {
    // A reader who signed a DIFFERENT boundary corroborates nothing, and the gap
    // between the two counts is that reader contradicting the founding, on the
    // charter, permanently.
    let p = Polity::new(3);
    let agrees = reading_of(&p.polity, p.roll());
    let disagrees = reading_of(&p.polity, vec![p.ids[0].1.clone(), id().1, id().1]);
    let agreeing_reader = agrees.payload.reader.clone();
    let dissenting_reader = disagrees.payload.reader.clone();
    let mut a = Archive::new();
    a.append(p.charter_attested(
        &[0, 1],
        BoundaryAttestation {
            source: BoundarySource::Substrate { readings: vec![agrees, disagrees] },
            candidate_size: 3,
            exclusions: vec![],
        },
    ))
    .unwrap();
    check_authority(&a).unwrap();

    let leg = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(leg.readers.len(), 2, "both readings are on the charter");
    assert_eq!(leg.corroborating_readers, vec![agreeing_reader]);
    assert_eq!(leg.dissenting_readers(), vec![&dissenting_reader]);
    assert!(leg.corroborated());
    assert_eq!(leg.basis, FoundingBasis::Corroborated);
}

#[test]
fn a_dissent_must_be_signed_by_exactly_its_dissenter() {
    let p = Polity::new(3);
    let left_out = id();
    let mut a = Archive::new();
    a.append(p.charter_attested(&[0, 1], BoundaryAttestation::convener_asserted(3))).unwrap();
    a.append(dissent_by(&p.polity, &left_out, &p.ids[0], DissentKind::Unnamed, 200)).unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("signed by exactly its dissenter"), "{}", v[0].1);
}

#[test]
fn an_unnamed_dissent_from_someone_the_founding_names_is_refused() {
    // "The accounting never mentioned me" is contradicted by a record that does.
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_attested(&[0, 1], BoundaryAttestation::convener_asserted(3))).unwrap();
    a.append(dissent_by(&p.polity, &p.ids[2], &p.ids[2], DissentKind::Unnamed, 200)).unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("but it names them"), "{}", v[0].1);
}

#[test]
fn rejecting_an_exclusion_reason_requires_being_excluded() {
    // You may reject the reason given for you, never one given for somebody else.
    let p = Polity::new(3);
    let stranger = id();
    let mut a = Archive::new();
    a.append(p.charter_attested(&[0, 1], BoundaryAttestation::convener_asserted(3))).unwrap();
    a.append(dissent_by(&p.polity, &stranger, &stranger, DissentKind::ReasonRejected, 200))
        .unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("excludes no such member"), "{}", v[0].1);
}

#[test]
fn the_excluded_may_answer_and_the_record_reads_it_back() {
    // The two claims a dissent can make, both authorized, and the read-model that
    // reports them without adjudicating either.
    let p = Polity::new(1);
    let named = id();
    let omitted = id();
    let mut a = Archive::new();
    let excl_named =
        Exclusion { member: named.1.clone(), reason: "not of this tradition".into() };
    a.append(p.charter_attested(
        &[0],
        BoundaryAttestation {
            source: read_source(&p, std::slice::from_ref(&excl_named)),
            candidate_size: 2,
            exclusions: vec![excl_named],
        },
    ))
    .unwrap();
    a.append(dissent_by(&p.polity, &named, &named, DissentKind::ReasonRejected, 200)).unwrap();
    a.append(dissent_by(&p.polity, &omitted, &omitted, DissentKind::Unnamed, 210)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    let d = founding_dissent(&a, &p.polity, t(300));
    assert_eq!(d.entries.len(), 2);
    assert_eq!(d.dissenters().len(), 2);
    assert_eq!(d.reasons_rejected().len(), 1);
    // The one thing D21 said it could not reach: an understated `candidate_size`,
    // surfaced by the only witness available — the party it omitted.
    assert_eq!(d.unnamed().len(), 1);
    assert_eq!(d.unnamed()[0].dissenter, omitted.1);
    // And it is read as of a moment, like every other derivation here.
    assert_eq!(founding_dissent(&a, &p.polity, t(205)).entries.len(), 1);
}

#[test]
fn dissent_outlives_the_polity_it_contests() {
    // The ONE entry class a dissolution does not close. Otherwise founders silence
    // the parties they excluded by winding the polity up — D17's "destroying
    // evidence is the move a captured party makes" in a different costume.
    let p = Polity::new(3);
    let omitted = id();
    let mut a = Archive::new();
    a.append(p.charter_attested(&[0, 1], BoundaryAttestation::convener_asserted(3))).unwrap();
    a.append(p.dissolve(&[], &[0, 1], 300)).unwrap();
    a.append(dissent_by(&p.polity, &omitted, &omitted, DissentKind::Unnamed, 400)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(founding_dissent(&a, &p.polity, t(500)).unnamed().len(), 1);
}

#[test]
fn dissent_is_not_prunable() {
    // For free, and worth stating: `is_prunable` matches only observations, which
    // D17 made forgettable precisely because they are inert. A dissent is the
    // opposite of inert.
    let p = Polity::new(1);
    let omitted = id();
    let entry = dissent_by(&p.polity, &omitted, &omitted, DissentKind::Unnamed, 100);
    assert!(!is_prunable(&entry, &p.polity, t(10_000)));
}

#[test]
fn a_dissent_precedes_nothing() {
    let p = Polity::new(3);
    let omitted = id();
    let mut a = Archive::new();
    a.append(dissent_by(&p.polity, &omitted, &omitted, DissentKind::Unnamed, 10)).unwrap();
    let v = authority_violations(&a);
    assert_eq!(v.len(), 1);
    assert!(v[0].1.contains("precedes it"), "{}", v[0].1);
}

// ---------------------------------------------------------------------------
// D23 — the ground a polity stands on (resolving Q4).
// ---------------------------------------------------------------------------

/// A founding with an arbitrary substrate, every member ratifying, and `derived`
/// designating machines with no guardian. `extra` co-signs alongside the roll — how
/// a substrate anchor's consent is supplied.
fn charter_grounded(
    p: &Polity,
    sub: Substrate,
    derived: &[usize],
    extra: Option<&(PrivKey, DidKey)>,
) -> CharterEntry {
    let prop = FoundingProposal::convener_asserted(
        p.polity.clone(),
        sub,
        vec![],
        p.roll(),
        derived
            .iter()
            .map(|m| Derivation::unguarded(p.ids[*m].1.clone()))
            .collect(),
        t(50),
    );
    let all: Vec<usize> = (0..p.ids.len()).collect();
    let charter = prop.ratify(p.dids(&all), t(100)).unwrap();
    let mut signers = p.signers(&all);
    if let Some(a) = extra {
        signers.push((a.1.as_str(), &a.0));
    }
    CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap())
}

fn archive_of(entry: CharterEntry) -> Archive<CharterEntry> {
    let mut a = Archive::new();
    a.append(entry).unwrap();
    a
}

/// The one violation an archive reports, for the D23 refusals.
fn sole_violation(a: &Archive<CharterEntry>) -> String {
    let v = authority_violations(a);
    assert_eq!(v.len(), 1, "expected exactly one violation, got {v:?}");
    v[0].1.clone()
}

#[test]
fn a_polity_whose_whole_ground_is_two_empty_strings_is_refused() {
    // **The Q4 regression, measured before the slice.** An empty substrate chartered:
    // `verify_all` and `check_authority` both green, and `charter_state` reported the
    // polity founded on `Substrate { name: "", statement: "", anchor: None }`. The
    // non-ultimacy anchor was a decorative field — nothing read it anywhere.
    let p = Polity::new(3);
    let empty = Substrate { name: String::new(), statement: String::new(), anchor: None };
    let a = archive_of(charter_grounded(&p, empty, &[], None));
    // The signatures are perfectly good. It is the ground that is missing.
    verify_all(&a).unwrap();
    assert!(sole_violation(&a).contains("not articulated"));
}

#[test]
fn whitespace_is_not_an_articulated_substrate() {
    let p = Polity::new(3);
    let blank = Substrate { name: "  ".into(), statement: "\t\n".into(), anchor: None };
    let a = archive_of(charter_grounded(&p, blank, &[], None));
    assert!(sole_violation(&a).contains("not articulated"));
}

#[test]
fn a_statement_without_a_name_is_not_articulated_either() {
    let p = Polity::new(3);
    let half = Substrate { name: String::new(), statement: "answers to X".into(), anchor: None };
    let a = archive_of(charter_grounded(&p, half, &[], None));
    assert!(sole_violation(&a).contains("not articulated"));
}

#[test]
fn a_polity_cannot_be_its_own_ground() {
    // Q4 as written asks about the ABSENCE of a ground. The sharper hole, measured
    // the same way, is that a polity could name ITSELF — so a rule demanding only
    // presence would have been defeated by one assignment.
    let p = Polity::new(3);
    let circular = Substrate {
        name: "us".into(),
        statement: "we answer to ourselves".into(),
        anchor: Some(p.polity.clone()),
    };
    let a = archive_of(charter_grounded(&p, circular, &[], None));
    assert!(sole_violation(&a).contains("cannot ground itself"));
}

#[test]
fn a_ground_inside_what_it_grounds_is_no_ground() {
    // One step out from the self-anchor: the anchor is a member of the very roll it
    // is supposed to stand above. This is the guardian loop rule ("a machine cannot
    // guard a machine") applied to the substrate the rule's own comment names.
    let p = Polity::new(3);
    let inside = Substrate {
        name: "boss".into(),
        statement: "we answer to member 0".into(),
        anchor: Some(p.ids[0].1.clone()),
    };
    let a = archive_of(charter_grounded(&p, inside, &[], None));
    assert!(sole_violation(&a).contains("no ground"));
}

#[test]
fn a_roll_of_machines_answering_to_nobody_must_name_an_anchor() {
    // **Q4's exact case: a fresh AT cluster.** Every member a machine, nobody behind
    // any of them, and an articulated statement that is therefore a sentence the
    // polity wrote about itself. Measured before the slice: chartered, three
    // derivations, zero guarded, both passes green.
    let p = Polity::new(3);
    let a = archive_of(charter_grounded(&p, substrate(), &[0, 1, 2], None));
    assert!(sole_violation(&a).contains("must name a substrate anchor"));
}

#[test]
fn a_machine_roll_charters_once_a_ground_consents() {
    // And the same cluster, grounded: an anchor outside the roll that co-signs.
    let p = Polity::new(3);
    let ground = id();
    let sub = Substrate {
        name: "the operators".into(),
        statement: "answers to what it did not author".into(),
        anchor: Some(ground.1.clone()),
    };
    let a = archive_of(charter_grounded(&p, sub, &[0, 1, 2], Some(&ground)));
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    let leg = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(leg.grounding, SubstrateGrounding::Anchored);
    assert_eq!(leg.anchor.as_ref(), Some(&ground.1));
}

#[test]
fn a_ground_cannot_be_conscripted() {
    // The anchor is named and does NOT sign. Nothing else is wrong with the charter.
    // Answerability is accepted, never assigned — the rule D15 applies to a guardian,
    // reaching the ground itself.
    let p = Polity::new(3);
    let ground = id();
    let sub = Substrate {
        name: "the operators".into(),
        statement: "answers to what it did not author".into(),
        anchor: Some(ground.1.clone()),
    };
    let a = archive_of(charter_grounded(&p, sub, &[0, 1, 2], None));
    assert!(sole_violation(&a).contains("consent"));
}

#[test]
fn a_guarded_machine_roll_needs_no_anchor() {
    // A wholly machine roll where every machine answers to a named human is already
    // grounded — that is D15's model of legitimate machine participation, and the
    // guardians co-sign. Demanding an anchor on top would be demanding a second
    // ground from the only polities that already produced one.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(p.charter_derived(
        &[0, 1, 2],
        vec![],
        &[(0, Some(&g)), (1, Some(&g)), (2, Some(&g))],
    ));
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded
    );
}

#[test]
fn the_padding_dodge_does_not_work() {
    // The grounded rule counts derivations that are actually ON the roll. Otherwise a
    // wholly machine cluster could dodge it by designating a name the roll never had,
    // so that `derivations.len() != members.len()` — passing this rule and failing a
    // later one, which would make the ORDER of the checks load-bearing by accident.
    let p = Polity::new(3);
    let outsider = id();
    let prop = FoundingProposal::convener_asserted(
        p.polity.clone(),
        substrate(),
        vec![],
        p.roll(),
        (0..3)
            .map(|m| Derivation::unguarded(p.ids[m].1.clone()))
            .chain(std::iter::once(Derivation::unguarded(outsider.1.clone())))
            .collect(),
        t(50),
    );
    let charter = prop.ratify(p.dids(&[0, 1, 2]), t(100)).unwrap();
    let a = archive_of(CharterEntry::PolityChartered(
        Signed::build(charter, &p.signers(&[0, 1, 2])).unwrap(),
    ));
    assert!(sole_violation(&a).contains("must name a substrate anchor"));
}

#[test]
fn a_human_roll_holds_its_own_tradition() {
    // No anchor, no derived parties: `Held`. The statement IS the ground, and unkeyed
    // on purpose — a tradition, a rule, the dead. Permitted, and not the same picture
    // as `Bare`.
    let p = Polity::new(3);
    let a = archive_of(p.charter(&[0, 1, 2], vec![]));
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Held
    );
}

#[test]
fn a_mixed_roll_with_an_unguarded_machine_reads_bare() {
    // Legal — a human is on the roll and can hold the tradition — and reported, so
    // that "permitted" is not mistaken for "grounded".
    let p = Polity::new(3);
    let a = archive_of(charter_grounded(&p, substrate(), &[0], None));
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Bare
    );
}

// ---- the ground answering back ----

fn disavow_by(
    polity: &DidKey,
    who: &(PrivKey, DidKey),
    signer: &(PrivKey, DidKey),
    at: i64,
) -> CharterEntry {
    let d = SubstrateDisavowal {
        polity: polity.clone(),
        disavower: who.1.clone(),
        rationale: "I do not stand above these people".into(),
        disavowed_at: t(at),
    };
    CharterEntry::SubstrateDisavowed(
        Signed::build(d, &[(signer.1.as_str(), &signer.0)]).unwrap(),
    )
}

/// A machine polity grounded on a consenting anchor — the founding a disavowal
/// contests. Returns the archive and the anchor's identity.
fn anchored_polity(p: &Polity) -> (Archive<CharterEntry>, (PrivKey, DidKey)) {
    let ground = id();
    let sub = Substrate {
        name: "the operators".into(),
        statement: "answers to what it did not author".into(),
        anchor: Some(ground.1.clone()),
    };
    let a = archive_of(charter_grounded(p, sub, &[0, 1, 2], Some(&ground)));
    (a, ground)
}

#[test]
fn the_named_ground_may_withdraw() {
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &ground, &ground, 500)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    let st = substrate_disavowal(&a, &p.polity, t(600));
    assert!(st.disavowed());
    assert_eq!(st.first_disavowed_at(), Some(t(500)));
    // And it is not yet visible before it happened.
    assert!(!substrate_disavowal(&a, &p.polity, t(400)).disavowed());
}

#[test]
fn a_withdrawal_does_not_un_charter_the_polity() {
    // The substrate is genesis-immutable. A disavowal is the ground's assertion, and
    // handing it the power to void a charter would give any anchor a unilateral kill
    // switch — a worse hole than the one D23 closed. The polity stands, on a ground
    // that disowns it, and the record says so.
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &ground, &ground, 500)).unwrap();
    let st = charter_state(&a, &p.polity, t(600));
    assert!(st.chartered);
    assert!(!st.dissolved);
    assert_eq!(st.substrate.unwrap().anchor.as_ref(), Some(&ground.1));
    // The founding read-model is charter-alone, so it says exactly what it said
    // before: this polity was anchored. Whether the ground still stands there is a
    // different question, asked of a different read-model.
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Anchored
    );
}

#[test]
fn only_the_named_ground_may_withdraw() {
    // Unlike a dissent, which must admit any filer because an omitted party is by
    // definition absent from the record, the ground IS named on the charter.
    let p = Polity::new(3);
    let (mut a, _ground) = anchored_polity(&p);
    let stranger = id();
    a.append(disavow_by(&p.polity, &stranger, &stranger, 500)).unwrap();
    assert!(sole_violation(&a).contains("only the named ground may withdraw"));
}

#[test]
fn a_member_cannot_withdraw_the_polity_s_own_ground() {
    let p = Polity::new(3);
    let (mut a, _ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &p.ids[0], &p.ids[0], 500)).unwrap();
    assert!(sole_violation(&a).contains("only the named ground may withdraw"));
}

#[test]
fn a_disavowal_must_be_signed_by_exactly_its_disavower() {
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &ground, &p.ids[0], 500)).unwrap();
    assert!(sole_violation(&a).contains("signed by exactly its disavower"));
}

#[test]
fn there_is_nothing_to_withdraw_from_an_unanchored_polity() {
    let p = Polity::new(3);
    let a0 = archive_of(p.charter(&[0, 1, 2], vec![]));
    let mut a = a0;
    let stranger = id();
    a.append(disavow_by(&p.polity, &stranger, &stranger, 500)).unwrap();
    assert!(sole_violation(&a).contains("nothing to withdraw"));
}

#[test]
fn a_disavowal_precedes_nothing() {
    let p = Polity::new(3);
    let ground = id();
    let mut a = Archive::new();
    a.append(disavow_by(&p.polity, &ground, &ground, 10)).unwrap();
    assert!(sole_violation(&a).contains("precedes its founding"));
}

#[test]
fn a_disowned_polity_cannot_silence_its_ground_by_winding_up() {
    // The second entry class a dissolution does not close, for D22's reason in a new
    // costume: a ground that could be silenced by the founders' patience gave a
    // consent worth nothing.
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(p.dissolve(&[], &[0, 1, 2], 400)).unwrap();
    a.append(disavow_by(&p.polity, &ground, &ground, 500)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert!(substrate_disavowal(&a, &p.polity, t(600)).disavowed());
}

#[test]
fn a_disavowal_is_not_prunable() {
    let p = Polity::new(3);
    let ground = id();
    let entry = disavow_by(&p.polity, &ground, &ground, 100);
    assert!(!is_prunable(&entry, &p.polity, t(10_000)));
}

#[test]
fn a_ground_may_say_it_twice() {
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &ground, &ground, 500)).unwrap();
    a.append(disavow_by(&p.polity, &ground, &ground, 900)).unwrap();
    check_authority(&a).unwrap();
    let st = substrate_disavowal(&a, &p.polity, t(1000));
    assert_eq!(st.entries.len(), 2);
    // The reading is the FIRST withdrawal, not the latest — a ground that repeats
    // itself has not withdrawn more recently, it has withdrawn for longer.
    assert_eq!(st.first_disavowed_at(), Some(t(500)));
}

#[test]
fn the_polity_still_governs_itself_after_its_ground_withdraws() {
    // The other half of "a withdrawal does not un-charter": not merely that the state
    // reads chartered, but that the polity's acts still CARRY. A disavowal contributes
    // nothing to reconstructed authority state, so admitting a member the day after
    // the ground walked is exactly as valid as the day before.
    let p = Polity::new(3);
    let (mut a, ground) = anchored_polity(&p);
    a.append(disavow_by(&p.polity, &ground, &ground, 500)).unwrap();
    let joiner = id();
    a.append(p.admit(&joiner, &[0, 1], 600)).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(charter_state(&a, &p.polity, t(700)).members.len(), 4);
}

// ---------------------------------------------------------------------------
// D24 — independence attestation for readers (closing D22's named residue).
// ---------------------------------------------------------------------------

/// `n` readings of the polity's whole roll, each signed by a freshly minted reader —
/// a sockpuppet chorus, if one actor holds all `n` keys. The keypairs come back so a
/// reader can also act for itself.
fn chorus(p: &Polity, n: usize) -> (Vec<Signed<SubstrateReading>>, Vec<(PrivKey, DidKey)>) {
    let mut readings = Vec::new();
    let mut readers = Vec::new();
    for _ in 0..n {
        let (sk, reader) = id();
        let r = SubstrateReading {
            polity: p.polity.clone(),
            reader: reader.clone(),
            read_at: t(40),
            members: p.roll(),
            opaque_boundary: false,
        };
        readings.push(Signed::build(r, &[(reader.as_str(), &sk)]).unwrap());
        readers.push((sk, reader));
    }
    (readings, readers)
}

/// A founding corroborated by `n` readers, all agreeing with the whole roll.
fn charter_read_by(p: &Polity, n: usize) -> (Archive<CharterEntry>, Vec<(PrivKey, DidKey)>) {
    let (readings, readers) = chorus(p, n);
    let att = BoundaryAttestation {
        candidate_size: p.ids.len(),
        source: BoundarySource::Substrate { readings },
        exclusions: vec![],
    };
    (archive_of(p.charter_attested(&[0, 1, 2], att)), readers)
}

fn attest(
    polity: &DidKey,
    reader: &DidKey,
    attester: &(PrivKey, DidKey),
    signer: &(PrivKey, DidKey),
    kind: IndependenceKind,
    at: i64,
) -> CharterEntry {
    attest_role(polity, reader, AttestedRole::BoundaryReader, attester, signer, kind, at)
}

#[allow(clippy::too_many_arguments)]
fn attest_role(
    polity: &DidKey,
    subject: &DidKey,
    role: AttestedRole,
    attester: &(PrivKey, DidKey),
    signer: &(PrivKey, DidKey),
    kind: IndependenceKind,
    at: i64,
) -> CharterEntry {
    let a = IndependenceAttestation {
        polity: polity.clone(),
        subject: subject.clone(),
        role,
        attesting_party: attester.1.clone(),
        kind,
        basis: "distinct operator, separate funding".into(),
        attested_at: t(at),
    };
    CharterEntry::IndependenceAttested(
        Signed::build(a, &[(signer.1.as_str(), &signer.0)]).unwrap(),
    )
}

#[test]
fn a_sockpuppet_chorus_corroborates_nothing_independently() {
    // **The D24 regression, measured before the slice.** One actor, five minted
    // did:keys, five identical readings: the charter graded `Corroborated`, the highest
    // basis the tier awards, with both passes green and zero dissent. The chorus cost
    // 32 microseconds a voice.
    let p = Polity::new(3);
    let (a, readers) = charter_read_by(&p, 5);
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    // D22's reading is unchanged and still says what it always said.
    let leg = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(leg.corroborating_readers.len(), 5);
    assert!(leg.corroborated());
    assert_eq!(leg.basis, FoundingBasis::Corroborated);

    // D24's reading is the one that cannot be bought with 16 milliseconds.
    let ic = independent_corroboration(&a, &p.polity, t(500));
    assert_eq!(ic.count(), 0);
    assert!(!ic.any());
    assert!(reader_independence(&a, &p.polity, t(500)).independent_readers().is_empty());
    assert_eq!(readers.len(), 5);
}

#[test]
fn one_outsider_willing_to_be_named_turns_it() {
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 5);
    let outsider = id();
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();

    let ic = independent_corroboration(&a, &p.polity, t(600));
    assert_eq!(ic.count(), 1);
    assert!(ic.any());
    assert_eq!(ic.readers[0], readers[0].1);
    assert_eq!(ic.uncontested().len(), 1);
    // And not before it was filed.
    assert_eq!(independent_corroboration(&a, &p.polity, t(400)).count(), 0);
}

#[test]
fn a_founder_vouching_for_the_foundings_own_reader_does_not_count() {
    // The sockpuppet move one step out. Recorded — somebody signed their name to it —
    // and reported, but not credited.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &p.ids[0],
        &p.ids[0],
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();

    let ri = reader_independence(&a, &p.polity, t(600));
    assert!(ri.independent_readers().is_empty());
    assert_eq!(ri.interested_attesters[readers[0].1.as_str()].len(), 1);
    assert_eq!(independent_corroboration(&a, &p.polity, t(600)).count(), 0);
}

#[test]
fn a_reader_cannot_vouch_for_itself() {
    // A perfectly valid record, and worth nothing as independence: the whole point is
    // that the claim comes from somebody other than the party it flatters.
    let p = Polity::new(3);
    let (a, readers) = charter_read_by(&p, 3);
    let mut a = a;
    let itself = &readers[0];
    a.append(attest(
        &p.polity,
        &itself.1,
        itself,
        itself,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    let ri = reader_independence(&a, &p.polity, t(600));
    assert!(ri.independent_readers().is_empty());
    assert_eq!(ri.interested_attesters[itself.1.as_str()].len(), 1);
    assert_eq!(independent_corroboration(&a, &p.polity, t(600)).count(), 0);
}

#[test]
fn a_polity_cannot_vouch_for_its_own_reader() {
    // The most direct form of the interested attestation: the polity itself. Recorded,
    // like a founder's, and credited no more than one.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    let itself = (p.polity_key.clone(), p.polity.clone());
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &itself,
        &itself,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    let ri = reader_independence(&a, &p.polity, t(600));
    assert!(ri.independent_readers().is_empty());
    assert_eq!(ri.interested_attesters[readers[0].1.as_str()].len(), 1);
    assert_eq!(independent_corroboration(&a, &p.polity, t(600)).count(), 0);
}

#[test]
fn nobody_vouches_in_somebody_elses_name() {
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    let outsider = id();
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &outsider,
        &p.ids[0],
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    assert!(sole_violation(&a).contains("signed by exactly its attesting party"));
}

#[test]
fn there_is_nothing_to_attest_about_a_reader_the_founding_never_carried() {
    // Coherence with the record spoken about — never its truth, the same line D22's
    // dissent arm draws.
    let p = Polity::new(3);
    let (mut a, _readers) = charter_read_by(&p, 3);
    let stranger = id();
    let outsider = id();
    a.append(attest(
        &p.polity,
        &stranger.1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    assert!(sole_violation(&a).contains("is not a reader of"));
}

#[test]
fn an_attestation_precedes_nothing() {
    let p = Polity::new(3);
    let reader = id();
    let outsider = id();
    let mut a = Archive::new();
    a.append(attest(
        &p.polity,
        &reader.1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        10,
    ))
    .unwrap();
    assert!(sole_violation(&a).contains("precedes its founding"));
}

#[test]
fn independence_may_be_contested() {
    // The counter-claim, from anyone — including a founder breaking ranks, who is the
    // most informative possible source for it. A contested reader is reported as
    // contested, never as disqualified: nothing here adjudicates.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    let outsider = id();
    let sceptic = id();
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &sceptic,
        &sceptic,
        IndependenceKind::Controlled,
        600,
    ))
    .unwrap();
    check_authority(&a).unwrap();

    let ri = reader_independence(&a, &p.polity, t(700));
    assert!(ri.is_attested(readers[0].1.as_str()));
    assert!(ri.is_contested(readers[0].1.as_str()));

    let ic = independent_corroboration(&a, &p.polity, t(700));
    // Still counted — and the objection travels with it.
    assert_eq!(ic.count(), 1);
    assert_eq!(ic.contested.len(), 1);
    assert!(ic.uncontested().is_empty());
}

#[test]
fn a_founder_may_contest_even_though_it_may_not_vouch() {
    // The asymmetry is deliberate: interest disqualifies *praise* and not *warning*.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &p.ids[1],
        &p.ids[1],
        IndependenceKind::Controlled,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();
    assert!(reader_independence(&a, &p.polity, t(600)).is_contested(readers[0].1.as_str()));
}

#[test]
fn a_vouched_reader_who_read_a_different_boundary_corroborates_nothing() {
    // Agreement and independence are two different questions, and the reading is the
    // CONJUNCTION of them. An impeccably vouched-for reader who saw a different cluster
    // is not corroboration of *this* boundary — it is a reader contradicting it.
    //
    // Note the shape this test is forced into, which is itself worth knowing: D22
    // already refuses a founding that NO reader supports, so a disagreeing reader can
    // only ever appear alongside an agreeing one. The vouching goes to the disagreeing
    // one, and the count stays at zero.
    let p = Polity::new(3);
    let (agrees, _) = chorus(&p, 1);
    let (sk, dissenter) = id();
    // Same CARDINALITY, different membership — which is the only shape a disagreement
    // can take once D22 is in force: it also pins every reading's member count to
    // `candidate_size`. So the disagreement is over *who*, never how many.
    let mut seen = p.roll();
    seen.pop();
    seen.push(id().1); // somebody this reader saw in place of the third member
    let disagrees = Signed::build(
        SubstrateReading {
            polity: p.polity.clone(),
            reader: dissenter.clone(),
            read_at: t(40),
            members: seen,
            opaque_boundary: false,
        },
        &[(dissenter.as_str(), &sk)],
    )
    .unwrap();
    let att = BoundaryAttestation {
        candidate_size: 3,
        source: BoundarySource::Substrate {
            readings: vec![agrees[0].clone(), disagrees],
        },
        exclusions: vec![],
    };
    let mut a = archive_of(p.charter_attested(&[0, 1, 2], att));
    let outsider = id();
    a.append(attest(
        &p.polity,
        &dissenter,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();

    // Vouched for, and corroborating nothing — it is on the charter as a reader who
    // saw something else.
    let ri = reader_independence(&a, &p.polity, t(600));
    assert!(ri.is_attested(dissenter.as_str()));
    assert_eq!(independent_corroboration(&a, &p.polity, t(600)).count(), 0);
    let leg = founding_legitimacy(&a, &p.polity).unwrap();
    assert_eq!(leg.dissenting_readers(), vec![&dissenter]);
}

#[test]
fn an_attestation_outlives_the_polity() {
    // The THIRD entry class a dissolution does not close — and by now a category
    // rather than an exception: evidence about a founding outlives the polity, because
    // the founding's legitimacy does.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    a.append(p.dissolve(&[], &[0, 1, 2], 400)).unwrap();
    let outsider = id();
    a.append(attest(
        &p.polity,
        &readers[0].1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(independent_corroboration(&a, &p.polity, t(600)).count(), 1);
}

#[test]
fn an_attestation_is_not_prunable() {
    let p = Polity::new(3);
    let reader = id();
    let outsider = id();
    let entry = attest(
        &p.polity,
        &reader.1,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        100,
    );
    assert!(!is_prunable(&entry, &p.polity, t(10_000)));
}

#[test]
fn two_vouchers_for_one_reader_are_one_reader() {
    // Distinctness is keyed on the pair, so a prolific attester cannot inflate the
    // count and one reader with two vouchers is still one corroborating reader.
    let p = Polity::new(3);
    let (mut a, readers) = charter_read_by(&p, 3);
    let x = id();
    let y = id();
    a.append(attest(&p.polity, &readers[0].1, &x, &x, IndependenceKind::Independent, 500)).unwrap();
    a.append(attest(&p.polity, &readers[0].1, &y, &y, IndependenceKind::Independent, 600)).unwrap();
    a.append(attest(&p.polity, &readers[0].1, &x, &x, IndependenceKind::Independent, 700)).unwrap();
    check_authority(&a).unwrap();
    let ri = reader_independence(&a, &p.polity, t(800));
    assert_eq!(ri.attesters[readers[0].1.as_str()].len(), 2);
    assert_eq!(independent_corroboration(&a, &p.polity, t(800)).count(), 1);
}

#[test]
fn a_convener_asserted_founding_has_no_readers_to_attest() {
    let p = Polity::new(3);
    let a = archive_of(p.charter(&[0, 1, 2], vec![]));
    assert!(reader_independence(&a, &p.polity, t(500)).attesters.is_empty());
    assert_eq!(independent_corroboration(&a, &p.polity, t(500)).count(), 0);
}

// ---------------------------------------------------------------------------
// D25 — the successors a dissolution names.
// ---------------------------------------------------------------------------

#[test]
fn a_polity_may_not_name_itself_as_its_own_successor() {
    // Measured before the slice: green under `check_authority`, and `charter_state`
    // then reported a dissolved polity whose line of descent pointed back at itself.
    // Terminal dissolution forbids any later act, so that continuation is one the
    // archive can never honour.
    let p = Polity::new(3);
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    a.append(p.dissolve(std::slice::from_ref(&p.polity), &[0, 1, 2], 400))
        .unwrap();
    verify_all(&a).unwrap();
    assert!(sole_violation(&a).contains("cannot succeed itself"));
}

#[test]
fn a_successor_may_not_be_named_twice() {
    // Measured: `successors.len() == 2` for one real successor. After D21, a count
    // nobody checks is a count that misleads — the fan-out of a fission is a number.
    let p = Polity::new(3);
    let heir = id().1;
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    a.append(p.dissolve(&[heir.clone(), heir], &[0, 1, 2], 400)).unwrap();
    assert!(sole_violation(&a).contains("named twice as a successor"));
}

#[test]
fn a_successor_may_not_be_one_of_the_polity_s_own_members() {
    // The third of the closure family: a machine cannot guard a machine (D15), a ground
    // may not sit inside what it grounds (D23), and a successor is a polity rather than
    // somebody on the roll.
    let p = Polity::new(3);
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    a.append(p.dissolve(&[p.ids[0].1.clone()], &[0, 1, 2], 400)).unwrap();
    assert!(sole_violation(&a).contains("not a polity"));
}

#[test]
fn a_dissolution_naming_distinct_outside_successors_is_fine() {
    // The control: an honest N-way split still passes, and the state reports the line
    // of descent.
    let p = Polity::new(3);
    let heirs = vec![id().1, id().1, id().1];
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    a.append(p.dissolve(&heirs, &[0, 1, 2], 400)).unwrap();
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    let st = charter_state(&a, &p.polity, t(500));
    assert!(st.dissolved);
    assert_eq!(st.successors, heirs);
}

#[test]
fn a_dissolution_naming_nobody_is_still_fine() {
    // A polity may simply end. Naming no successor is not an error — it is the
    // difference between a fission and a death, and the tier records both.
    let p = Polity::new(3);
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    a.append(p.dissolve(&[], &[0, 1, 2], 400)).unwrap();
    check_authority(&a).unwrap();
    assert!(charter_state(&a, &p.polity, t(500)).successors.is_empty());
}


// ---- D27: the founding edge must show what stood behind it ----
//
// D15 made every *post-founding* guardian change carry evidence. Founding
// derivations carried nothing, and the measurement said that was a gate evasion
// rather than a cosmetic asymmetry: D23's grounded rule turns on whether any edge is
// guarded, so one asserted edge exempted a whole machine cluster from naming a
// ground.

/// A charter whose derivations are built by `mk`, so a test can hand in evidence
/// that is missing, forged, or about the wrong edge.
fn charter_with_derivations(
    p: &Polity,
    sub: Substrate,
    derivations: Vec<Derivation>,
    consenting: &[&(PrivKey, DidKey)],
) -> CharterEntry {
    let all: Vec<usize> = (0..p.ids.len()).collect();
    let prop = FoundingProposal::convener_asserted(
        p.polity.clone(),
        sub,
        vec![],
        p.roll(),
        derivations,
        t(50),
    );
    let charter = prop.ratify(p.dids(&all), t(100)).unwrap();
    let mut signers = p.signers(&all);
    for c in consenting {
        signers.push((c.1.as_str(), &c.0));
    }
    CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap())
}

#[test]
fn a_founding_edge_claimed_with_no_evidence_is_refused() {
    // The bare asymmetry: an edge asserted with nothing offered for it, which is
    // exactly what D15 refuses one record later in the same log.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation { member: p.ids[0].1.clone(), guardian: Some(g.1.clone()), evidence: None }],
        &[&g],
    ));
    assert!(sole_violation(&a).contains("no substrate evidence"));
}

#[test]
fn an_unbacked_founding_edge_cannot_ground_a_machine_roll() {
    // **THE MEASURED HOLE.** Three machines, two answering to nobody, one claimed
    // edge, no anchor. Before this slice: `verify_all` and `check_authority` both
    // green, `founding_legitimacy` grading `Guarded`. A polity raised its own ground
    // by writing one sentence — the shape D21 and D22 each had to delete.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![
            Derivation::unbacked(p.ids[0].1.clone(), g.1.clone(), "nobody looked"),
            Derivation::unguarded(p.ids[1].1.clone()),
            Derivation::unguarded(p.ids[2].1.clone()),
        ],
        &[&g],
    ));
    verify_all(&a).unwrap();
    let why = sole_violation(&a);
    assert!(why.contains("must name a substrate anchor"), "{why}");
    // And the refusal says *which* shortfall this is, not the no-edges-at-all one.
    assert!(why.contains("self-declared"), "{why}");
}

#[test]
fn an_unbacked_founding_edge_grades_bare_not_guarded() {
    // The grade moves with the gate. A human is on the roll here, so the charter is
    // legal and nothing is refused — which is why the grade has to carry the news.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation::unbacked(p.ids[0].1.clone(), g.1.clone(), "no monitor had run")],
        &[&g],
    ));
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Bare
    );
}

#[test]
fn an_unbacked_machine_roll_charters_once_it_names_an_anchor() {
    // The bootstrap case, and the reason the rule is accounting rather than
    // prohibition: guardians who are real but unobserved name a ground until a
    // monitor has run, then move the edge with a backed change.
    let p = Polity::new(3);
    let g = id();
    let ground = id();
    let sub = Substrate {
        name: "the operators".into(),
        statement: "answers to what it did not author".into(),
        anchor: Some(ground.1.clone()),
    };
    let all: Vec<usize> = (0..3).collect();
    let prop = FoundingProposal::convener_asserted(
        p.polity.clone(),
        sub,
        vec![],
        p.roll(),
        (0..3)
            .map(|m| Derivation::unbacked(p.ids[m].1.clone(), g.1.clone(), "no monitor had run"))
            .collect(),
        t(50),
    );
    let charter = prop.ratify(p.dids(&all), t(100)).unwrap();
    let mut signers = p.signers(&all);
    signers.push((g.1.as_str(), &g.0));
    signers.push((ground.1.as_str(), &ground.0));
    let a = archive_of(CharterEntry::PolityChartered(
        Signed::build(charter, &signers).unwrap(),
    ));
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Anchored
    );
}

#[test]
fn a_backed_founding_edge_needs_no_anchor() {
    // The positive case: a monitor ran over the cluster before the founding and the
    // charter carries what it saw. Every machine answers to a named human who
    // co-signed, so the polity is grounded the way D15 models it.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(p.charter_derived(
        &[0, 1, 2],
        vec![],
        &[(0, Some(&g)), (1, Some(&g)), (2, Some(&g))],
    ));
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded
    );
}

#[test]
fn founding_evidence_offered_for_no_edge_is_refused() {
    // A machine chartered as answering to nobody claims nothing, so there is nothing
    // for evidence to be evidence *of*. Refused rather than ignored: a field carried
    // and read by no rule is what D22's `BoundarySource` and D23's `anchor` both were.
    let p = Polity::new(3);
    let g = id();
    let ob = p.founding_observation(&[(0, Some(&g))]);
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation {
            member: p.ids[0].1.clone(),
            guardian: None,
            evidence: Some(GuardianEvidence::Observed(Box::new(ob))),
        }],
        &[],
    ));
    assert!(sole_violation(&a).contains("claims no guardian edge"));
}

#[test]
fn an_unbacked_founding_edge_must_say_why() {
    // The override is permitted because it is a *statement*. An empty reason is the
    // assertion again with the accounting stripped off.
    let p = Polity::new(3);
    let g = id();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation::unbacked(p.ids[0].1.clone(), g.1.clone(), "   ")],
        &[&g],
    ));
    assert!(sole_violation(&a).contains("must say why"));
}

#[test]
fn a_founding_observation_about_another_edge_backs_nothing() {
    // The evidence has to show *this* edge. An observation of the same member under a
    // different guardian is a real signed reading and still says nothing about the
    // claim the charter is making.
    let p = Polity::new(3);
    let claimed = id();
    let seen = id();
    let ob = p.founding_observation(&[(0, Some(&seen))]);
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation::observed(p.ids[0].1.clone(), claimed.1.clone(), ob)],
        &[&claimed],
    ));
    let why = sole_violation(&a);
    assert!(why.contains("not the claimed"), "{why}");
}

#[test]
fn a_forged_founding_guardian_observation_is_refused() {
    // The crypto pass reaches the carried observation, as it does for a post-founding
    // change: signer label says one party, the key is another's. Otherwise the D27
    // rule and the grounded rule would both be reading a forgery.
    let p = Polity::new(3);
    let g = id();
    let (attacker_sk, _) = id();
    let (_, victim) = id();
    let ob = Observation {
        polity: p.polity.clone(),
        observer: victim.clone(),
        observed_at: t(40),
        readings: vec![MemberReading {
            member: p.ids[0].1.clone(),
            reputation: 0.9,
            guardian: Some(g.1.clone()),
            attested_at: None,
        }],
        opaque_boundary: false,
    };
    let forged = Signed::build(ob, &[(victim.as_str(), &attacker_sk)]).unwrap();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        vec![Derivation::observed(p.ids[0].1.clone(), g.1.clone(), forged)],
        &[&g],
    ));
    assert!(verify_all(&a).is_err());
}

#[test]
fn a_forged_founding_boundary_reading_is_refused() {
    // **A second defect, measured while building the first.** The authority pass
    // requires a reading's signer set to equal the reader it names — but that is the
    // signer *label*, and nothing was verifying the signature. Measured: a reading
    // signed with an attacker's key and labelled with a victim's DID passed
    // `verify_all` and `check_authority` both, and graded the founding `Substrate`.
    // D22 exists because a reading nobody signed cannot be told apart from a reading
    // nobody took, and an unverified signature is exactly a reading nobody signed.
    let p = Polity::new(3);
    let (attacker_sk, _) = id();
    let (_, victim) = id();
    let r = SubstrateReading {
        polity: p.polity.clone(),
        reader: victim.clone(),
        read_at: t(40),
        members: p.roll(),
        opaque_boundary: false,
    };
    let forged = Signed::build(r, &[(victim.as_str(), &attacker_sk)]).unwrap();
    let prop = FoundingProposal {
        polity: p.polity.clone(),
        substrate: substrate(),
        founding_commitments: vec![],
        proposed_members: p.roll(),
        proposed_derivations: vec![],
        attestation: BoundaryAttestation::read(vec![forged], vec![]),
        proposed_at: t(50),
    };
    let charter = prop.ratify(p.dids(&[0, 1, 2]), t(100)).unwrap();
    let a = archive_of(CharterEntry::PolityChartered(
        Signed::build(charter, &p.signers(&[0, 1, 2])).unwrap(),
    ));
    assert!(verify_all(&a).is_err());
}

#[test]
fn a_founder_cannot_observe_its_own_polity_into_ground() {
    // **Measured while building D27, and the reason the rule is about who looked.**
    // Requiring a signed observation was not enough on its own: a member of the roll
    // signed one about the cluster's own edges, the guardian co-signed as always, and
    // the charter graded `Guarded` with no anchor, both passes green. A polity
    // vouching for its own ground — the closure D23 refused for the anchor and D25 for
    // a successor, reaching the evidence this time.
    let p = Polity::new(3);
    let g = id();
    let (sk0, did0) = (p.ids[0].0.clone(), p.ids[0].1.clone());
    let ob = Observation {
        polity: p.polity.clone(),
        observer: did0.clone(),
        observed_at: t(40),
        readings: (0..3)
            .map(|m| MemberReading {
                member: p.ids[m].1.clone(),
                reputation: 0.9,
                guardian: Some(g.1.clone()),
                attested_at: None,
            })
            .collect(),
        opaque_boundary: false,
    };
    let signed = Signed::build(ob, &[(did0.as_str(), &sk0)]).unwrap();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        (0..3)
            .map(|m| Derivation::observed(p.ids[m].1.clone(), g.1.clone(), signed.clone()))
            .collect(),
        &[&g],
    ));
    // The observation is real evidence and stays in the log — it is only barred from
    // crediting the exemption, exactly as D24 grades an interested reader.
    verify_all(&a).unwrap();
    let why = sole_violation(&a);
    assert!(why.contains("must name a substrate anchor"), "{why}");
    assert!(why.contains("3 rest on an observation taken from inside"), "{why}");
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Bare
    );
}

#[test]
fn an_outside_monitor_grounds_what_a_founder_could_not() {
    // The same charter, the same edges, and the only change is who looked. This is the
    // control on the rule above: without it the two tests would be indistinguishable.
    let p = Polity::new(3);
    let g = id();
    let (sk, outsider) = id();
    let ob = Observation {
        polity: p.polity.clone(),
        observer: outsider.clone(),
        observed_at: t(40),
        readings: (0..3)
            .map(|m| MemberReading {
                member: p.ids[m].1.clone(),
                reputation: 0.9,
                guardian: Some(g.1.clone()),
                attested_at: None,
            })
            .collect(),
        opaque_boundary: false,
    };
    let signed = Signed::build(ob, &[(outsider.as_str(), &sk)]).unwrap();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        (0..3)
            .map(|m| Derivation::observed(p.ids[m].1.clone(), g.1.clone(), signed.clone()))
            .collect(),
        &[&g],
    ));
    verify_all(&a).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded
    );
}

#[test]
fn a_guardian_may_observe_the_edge_it_accepts() {
    // The judgment call, stated as a test so it is not mistaken for an oversight. A
    // guardian is never on the roll, co-signs the founding either way, and what it
    // asserts by observing is its own answerability — a burden taken up, not a credit
    // awarded. The keyed human outside the roll IS the ground here, which is D23's
    // picture of `Guarded`.
    let p = Polity::new(3);
    let g = id();
    let ob = Observation {
        polity: p.polity.clone(),
        observer: g.1.clone(),
        observed_at: t(40),
        readings: (0..3)
            .map(|m| MemberReading {
                member: p.ids[m].1.clone(),
                reputation: 0.9,
                guardian: Some(g.1.clone()),
                attested_at: None,
            })
            .collect(),
        opaque_boundary: false,
    };
    let signed = Signed::build(ob, &[(g.1.as_str(), &g.0)]).unwrap();
    let a = archive_of(charter_with_derivations(
        &p,
        substrate(),
        (0..3)
            .map(|m| Derivation::observed(p.ids[m].1.clone(), g.1.clone(), signed.clone()))
            .collect(),
        &[&g],
    ));
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded
    );
}

// ---- D28: the watcher, not the filing ----------------------------------------
//
// D15 and D27 each closed a hole in guardian evidence and each named D24's
// independence attestation as the successor for what remained. Measured, it did not
// reach — its coherence gate demanded the subject be a reader of the boundary — and
// the same measurement found the sharper case, which points OUTWARD: a stranger with
// one keypair could manufacture the Concern that crosses to kith-covenant.

/// A polity with `n` machines all answering to `guardian`, their founding edges backed
/// by `observer`'s signed observation. Returns the archive and the observer's DID.
fn charter_watched_by(
    p: &Polity,
    observer: &(PrivKey, DidKey),
    guardian: &(PrivKey, DidKey),
) -> Archive<CharterEntry> {
    let ob = Observation {
        polity: p.polity.clone(),
        observer: observer.1.clone(),
        observed_at: t(40),
        readings: p
            .ids
            .iter()
            .map(|(_, m)| MemberReading {
                member: m.clone(),
                reputation: 0.9,
                guardian: Some(guardian.1.clone()),
                attested_at: None,
            })
            .collect(),
        opaque_boundary: false,
    };
    let ob = Signed::build(ob, &[(observer.1.as_str(), &observer.0)]).unwrap();
    archive_of(charter_with_derivations(
        p,
        substrate(),
        p.ids
            .iter()
            .map(|(_, m)| Derivation {
                member: m.clone(),
                guardian: Some(guardian.1.clone()),
                evidence: Some(GuardianEvidence::Observed(Box::new(ob.clone()))),
            })
            .collect(),
        &[guardian],
    ))
}

#[test]
fn a_monitor_the_polity_never_heard_from_cannot_be_vouched_for() {
    // **THE MEASURED HOLE, half two.** D24's record refused this outright — an
    // observer is not a reader of the boundary — so the successor D15 and D27 both
    // named could not speak about the party they were naming it for.
    let p = Polity::new(3);
    let g = id();
    let watcher = id();
    let a = charter_watched_by(&p, &watcher, &g);
    let silent = id();
    let outsider = id();

    let mut bad = a.clone();
    bad.append(attest_role(
        &p.polity,
        &silent.1,
        AttestedRole::Observer,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    verify_all(&bad).unwrap();
    let why = sole_violation(&bad);
    assert!(why.contains("is not an observer of"), "{why}");

    // And a monitor that HAS spoken can be. The pair is what makes the rule mean
    // "speak before you are vouched for" rather than "observers may not be vouched
    // for at all", which is what D24 said by accident.
    let mut ok = a.clone();
    ok.append(attest_role(
        &p.polity,
        &watcher.1,
        AttestedRole::Observer,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&ok).unwrap();
    assert!(observer_independence(&ok, &p.polity, t(600)).is_attested(watcher.1.as_str()));
}

#[test]
fn a_voucher_is_for_a_party_in_a_capacity() {
    // The same key, vouched for as an observer, is not thereby vouched for as a
    // reader — and the buckets are per role rather than per key. What an outsider can
    // speak to is a relationship, not an identity in the abstract.
    let p = Polity::new(3);
    let g = id();
    let watcher = id();
    let outsider = id();
    let mut a = charter_watched_by(&p, &watcher, &g);
    a.append(attest_role(
        &p.polity,
        &watcher.1,
        AttestedRole::Observer,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();
    assert!(observer_independence(&a, &p.polity, t(600)).is_attested(watcher.1.as_str()));
    assert!(
        !reader_independence(&a, &p.polity, t(600)).is_attested(watcher.1.as_str()),
        "an observer voucher must not leak into the reader bucket"
    );
}

#[test]
fn a_founder_vouching_for_the_monitor_does_not_count() {
    // D24's stake rule, unchanged and applied to the new role: the founders are
    // precisely the interested party. Recorded, reported, not credited.
    let p = Polity::new(3);
    let g = id();
    let watcher = id();
    let mut a = charter_watched_by(&p, &watcher, &g);
    a.append(attest_role(
        &p.polity,
        &watcher.1,
        AttestedRole::Observer,
        &p.ids[0],
        &p.ids[0],
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();
    let ind = observer_independence(&a, &p.polity, t(600));
    assert!(!ind.is_attested(watcher.1.as_str()), "a founder's praise buys nothing");
    assert_eq!(ind.interested_attesters.len(), 1, "and is recorded rather than destroyed");

    // A founder's WARNING counts, though — interest disqualifies praise, not a
    // warning, and a founder breaking ranks is the most informative source there is.
    a.append(attest_role(
        &p.polity,
        &watcher.1,
        AttestedRole::Observer,
        &p.ids[1],
        &p.ids[1],
        IndependenceKind::Controlled,
        600,
    ))
    .unwrap();
    check_authority(&a).unwrap();
    assert!(observer_independence(&a, &p.polity, t(700)).is_contested(watcher.1.as_str()));
}

#[test]
fn a_chartered_ground_reports_whether_anyone_vouched_for_its_monitor() {
    // D27 required the founding observer to be outside the polity and deliberately did
    // not ask WHO — requiring a vouched-for monitor at founding would refuse newness,
    // the line D24 drew. So the grade stays and this reports the residue.
    let p = Polity::new(3);
    let g = id();
    let watcher = id();
    let mut a = charter_watched_by(&p, &watcher, &g);
    check_authority(&a).unwrap();
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded,
        "the grade does not move — that is the whole point"
    );
    let before = independent_grounding(&a, &p.polity, t(600));
    assert_eq!(before.unvouched_edges.len(), 3);
    assert!(!before.any(), "nobody has said a word about this monitor");

    let outsider = id();
    a.append(attest_role(
        &p.polity,
        &watcher.1,
        AttestedRole::Observer,
        &outsider,
        &outsider,
        IndependenceKind::Independent,
        500,
    ))
    .unwrap();
    check_authority(&a).unwrap();
    let after = independent_grounding(&a, &p.polity, t(600));
    assert_eq!(after.vouched_edges.len(), 3);
    assert!(after.unvouched_edges.is_empty());
    assert_eq!(
        founding_legitimacy(&a, &p.polity).unwrap().grounding,
        SubstrateGrounding::Guarded,
        "and it still does not move: founding_legitimacy reads the charter alone"
    );
}

#[test]
fn a_monitor_carried_as_founding_evidence_counts_as_having_observed() {
    // The equivalence the authority pass and the read-model both draw: a monitor whose
    // observation the charter CARRIED has observed this polity as surely as one that
    // filed an entry of its own. Without it, exactly the monitors D27 made
    // load-bearing would be the ones nobody could vouch for.
    let p = Polity::new(3);
    let g = id();
    let watcher = id();
    let a = charter_watched_by(&p, &watcher, &g);
    let seen = independent_observation(&a, &p.polity, t(600));
    assert_eq!(seen.unvouched.len(), 1);
    assert_eq!(seen.unvouched[0].as_str(), watcher.1.as_str());
    assert_eq!(seen.count(), 0, "observed is not vouched for");
}

#[test]
fn a_chorus_of_minted_monitors_is_independently_vouched_for_by_nobody() {
    // D24's chorus, one tier along. Distinct keys ARE distinct watchers — that is
    // arithmetic and the count is honest — so the lens is what carries the news that
    // the distinctness may be worth nothing.
    let p = Polity::new(3);
    let g = id();
    let mut a = archive_of(p.charter(&[0, 1, 2], vec![]));
    for k in 0..5 {
        let minted = id();
        a.append(p.observe(&minted, 1000 + k * 86_400, &[(0, Some(&g.1), Some(1000))]))
            .unwrap();
    }
    check_authority(&a).unwrap();
    let w = observation_window(&a, &p.polity, t(0), t(1000 + 10 * 86_400));
    assert_eq!(w.watchers_for(&p.ids[0].1), 5, "five keys are five watchers");
    let seen = independent_observation(&a, &p.polity, t(1000 + 10 * 86_400));
    assert_eq!(seen.count(), 0, "and not one of them has an outsider's word behind it");
    assert_eq!(seen.unvouched.len(), 5);
}

/// D29 — the rite has movements, and the substrate is required of those who govern.
///
/// Both halves were measured before either was written: an induction record carried
/// witnesses and one timestamp, so a rite held over a week and one performed in a
/// second were identical records; and `inducted` was read at exactly one site, so a
/// roll of three inducted founders plus six who never were let the six confer an
/// office on one of their own, amend the commitments, expel a founder, and preside
/// over a newcomer's induction — every pass green.
mod rite {
    use super::*;

    /// A polity of 3, all ratifying (so all hold the substrate), plus an admitted
    /// joiner who does not.
    fn admitted() -> (Polity, (PrivKey, DidKey), Archive<CharterEntry>) {
        let p = Polity::new(3);
        let joiner = id();
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![])).unwrap();
        a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
        (p, joiner, a)
    }

    fn ok(a: &Archive<CharterEntry>) {
        verify_all(a).expect("signatures valid");
        check_authority(a).expect("expected authorized");
    }
    fn refused(a: &Archive<CharterEntry>, needle: &str) {
        verify_all(a).expect("signatures valid");
        let rs = reasons(a);
        assert!(rs.iter().any(|m| m.contains(needle)), "expected {needle:?}, got: {rs:?}");
    }

    #[test]
    fn a_rite_is_recited_and_then_completed() {
        let (p, joiner, mut a) = admitted();
        let cids = p.rite(&mut a, &joiner, &[0, 1], 250);
        assert_eq!(cids.len(), 5, "five movements, one record each");
        // Before the completion the rite is open and the member holds nothing.
        let open = rite_state(&a, &p.polity, t(280));
        assert_eq!(open.len(), 1);
        assert!(open[0].all_movements_recited());
        assert_eq!(open[0].completed_at, None, "an open rite is not an induction");
        assert!(!affirmation_state(&a, &p.polity, t(280)).is_inducted(&joiner.1));

        p.induct_citing(&mut a, &joiner, &[0, 1], cids, 300);
        ok(&a);
        assert!(affirmation_state(&a, &p.polity, t(310)).is_inducted(&joiner.1));
        let done = rite_state(&a, &p.polity, t(310));
        assert_eq!(done[0].completed_at, Some(t(300)));
        assert_eq!(done[0].span_seconds(), Some(4), "the rite's own cost in time");
    }

    #[test]
    fn movements_may_not_be_skipped_or_repeated() {
        let (p, joiner, mut a) = admitted();
        // Straight to movement 2 with no rite open.
        let mut skip = a.clone();
        p.movement(&mut skip, &joiner, &[0, 1], Movement::Fallenness, 250);
        refused(&skip, "with no rite open");

        // Movement 1, then movement 3.
        p.movement(&mut a, &joiner, &[0, 1], Movement::Substrate { names: substrate().name }, 250);
        let mut jump = a.clone();
        p.movement(&mut jump, &joiner, &[0, 1], Movement::Recognition, 251);
        refused(&jump, "may not be skipped or repeated");

        // Movement 2 twice.
        p.movement(&mut a, &joiner, &[0, 1], Movement::Fallenness, 251);
        let mut twice = a.clone();
        p.movement(&mut twice, &joiner, &[0, 1], Movement::Fallenness, 252);
        refused(&twice, "may not be skipped or repeated");
    }

    #[test]
    fn a_movement_is_checked_for_coherence_with_the_charter_and_never_for_content() {
        let (p, joiner, a) = admitted();
        // Movement 1 must name the chartered substrate.
        let mut wrong = a.clone();
        p.movement(
            &mut wrong,
            &joiner,
            &[0, 1],
            Movement::Substrate { names: "some other ground".into() },
            250,
        );
        refused(&wrong, "but the charter's is");

        // Movement 4 must commit to what is in force, as a set.
        let mut partial = a.clone();
        let cid1 = p.movement(
            &mut partial,
            &joiner,
            &[0, 1],
            Movement::Substrate { names: substrate().name },
            250,
        );
        let _ = cid1;
        p.movement(&mut partial, &joiner, &[0, 1], Movement::Fallenness, 251);
        p.movement(&mut partial, &joiner, &[0, 1], Movement::Recognition, 252);
        let mut extra = partial.clone();
        p.movement(
            &mut extra,
            &joiner,
            &[0, 1],
            Movement::CommonVirtue { commits_to: vec!["something-we-prefer".into()] },
            253,
        );
        refused(&extra, "commitments in force");

        // Movement 5 must name the chartered anchor — here, none.
        let mut anchored = partial.clone();
        p.movement(
            &mut anchored,
            &joiner,
            &[0, 1],
            Movement::CommonVirtue { commits_to: vec![] },
            253,
        );
        let (_, invented) = id();
        p.movement(
            &mut anchored,
            &joiner,
            &[0, 1],
            Movement::NonUltimacy { names_anchor: Some(invented) },
            254,
        );
        refused(&anchored, "does not name the anchor the charter names");

        // And the words themselves are never read — only their presence.
        let mut empty = a.clone();
        let rm = RiteMovement {
            polity: p.polity.clone(),
            member: joiner.1.clone(),
            kind: AffirmationKind::Induction,
            movement: Movement::Substrate { names: substrate().name },
            statement: "   ".into(),
            attending: p.dids(&[0, 1]),
            recited_at: t(250),
        };
        let mut signers = p.signers(&[0, 1]);
        signers.push((joiner.1.as_str(), &joiner.0));
        empty
            .append(CharterEntry::RiteMovementRecorded(
                Signed::build(rm, &signers).unwrap(),
            ))
            .unwrap();
        refused(&empty, "a movement is performed, not listed");
    }

    #[test]
    fn the_substrate_is_transmitted_by_those_who_hold_it() {
        // Two of three ratify, so member 2 is on the roll and holds nothing.
        let p = Polity::new(3);
        let joiner = id();
        let mut a = Archive::new();
        a.append(p.charter(&[0, 1], vec![])).unwrap();
        a.append(p.admit(&joiner, &[0, 1], 200)).unwrap();
        p.movement(&mut a, &joiner, &[0, 1, 2], Movement::Substrate { names: substrate().name }, 250);
        refused(&a, "is not inducted; the substrate is");
    }

    /// D28's arithmetic, one record over: five movements signed by disjoint groups is
    /// a rite nobody attended, so only those present throughout may witness.
    #[test]
    fn a_witness_must_have_attended_every_movement() {
        let p = Polity::new(5);
        let joiner = id();
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![])).unwrap();
        a.append(p.admit(&joiner, &[0, 1, 2], 200)).unwrap();
        // Members 0 and 1 attend throughout; member 2 shows up for the last movement.
        let mut cids = Vec::new();
        for (i, m) in p.movements(&a, 250).into_iter().enumerate() {
            let attending: &[usize] = if i == 4 { &[0, 1, 2] } else { &[0, 1] };
            cids.push(p.movement(&mut a, &joiner, attending, m, 250 + i as i64));
        }
        let rites = rite_state(&a, &p.polity, t(300));
        assert_eq!(rites[0].attended_throughout().len(), 2, "member 2 came late");

        // Citing member 2 as a witness is refused, even though they signed a movement.
        let mut late = a.clone();
        p.induct_citing(&mut late, &joiner, &[0, 1, 2], cids.clone(), 300);
        refused(&late, "did not attend every movement");

        // A majority of the five who hold the substrate is three, and only two were
        // there throughout — so this rite cannot be completed at all.
        p.induct_citing(&mut a, &joiner, &[0, 1], cids, 300);
        refused(&a, "majority of the inducted roll to witness");
    }

    #[test]
    fn an_induction_must_cite_its_own_rite_in_order() {
        let (p, joiner, mut a) = admitted();
        let other = id();
        a.append(p.admit(&other, &[0, 1], 210)).unwrap();
        let mine = p.rite(&mut a, &joiner, &[0, 1], 250);
        let theirs = p.rite(&mut a, &other, &[0, 1], 260);

        let mut borrowed = a.clone();
        p.induct_citing(&mut borrowed, &joiner, &[0, 1], theirs, 300);
        refused(&borrowed, "must cite the five movements of its own rite");

        let mut shuffled = a.clone();
        let mut out_of_order = mine.clone();
        out_of_order.swap(0, 4);
        p.induct_citing(&mut shuffled, &joiner, &[0, 1], out_of_order, 300);
        refused(&shuffled, "in order");

        let mut short = a.clone();
        p.induct_citing(&mut short, &joiner, &[0, 1], mine[..4].to_vec(), 300);
        refused(&short, "must cite the five movements");

        // And a completion cannot predate the rite it completes.
        let mut early = a;
        p.induct_citing(&mut early, &joiner, &[0, 1], mine, 251);
        refused(&early, "dated before the last movement");
    }

    /// A rite begun and left unfinished must not lock a member out forever, so
    /// movement 1 always opens a new one.
    #[test]
    fn an_abandoned_rite_does_not_deadlock_the_member() {
        let (p, joiner, mut a) = admitted();
        p.movement(&mut a, &joiner, &[0, 1], Movement::Substrate { names: substrate().name }, 250);
        p.movement(&mut a, &joiner, &[0, 1], Movement::Fallenness, 251);
        // Begun again from the top, months later.
        let cids = p.rite(&mut a, &joiner, &[0, 1], 900);
        p.induct_citing(&mut a, &joiner, &[0, 1], cids, 1000);
        ok(&a);

        let rites = rite_state(&a, &p.polity, t(1100));
        assert_eq!(rites.len(), 2, "the abandoned attempt is part of the record");
        assert_eq!(rites[0].movements.len(), 2);
        assert_eq!(rites[0].completed_at, None);
        assert!(rites[1].all_movements_recited());
        assert!(rites[1].completed_at.is_some());
    }

    #[test]
    fn a_rite_reports_the_attendees_it_lost() {
        let p = Polity::new(5);
        let joiner = id();
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![])).unwrap();
        a.append(p.admit(&joiner, &[0, 1, 2], 200)).unwrap();
        // Four turn out for the naming of the substrate; three by the end.
        let mut cids = Vec::new();
        for (i, m) in p.movements(&a, 250).into_iter().enumerate() {
            let attending: &[usize] = if i == 0 { &[0, 1, 2, 3] } else { &[0, 1, 2] };
            cids.push(p.movement(&mut a, &joiner, attending, m, 250 + i as i64));
        }
        p.induct_citing(&mut a, &joiner, &[0, 1, 2], cids, 300);
        ok(&a);
        let rites = rite_state(&a, &p.polity, t(310));
        assert_eq!(rites[0].attrition(), 1, "one attendee drifted away mid-rite");
        assert_eq!(rites[0].attended_throughout().len(), 3);
    }

    // --- F1: the substrate is required of those who govern --------------------

    #[test]
    fn the_uninducted_carry_no_weight() {
        // Three founders hold the substrate; three admitted members do not, and even
        // together they cannot amend, expel, or confer.
        let p = Polity::new(3);
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![commit("mutual-aid")])).unwrap();
        let joiners: Vec<(PrivKey, DidKey)> = (0..3).map(|_| id()).collect();
        for (i, j) in joiners.iter().enumerate() {
            a.append(p.admit(j, &[0, 1], 200 + i as i64)).unwrap();
        }
        ok(&a);

        let deciders: Vec<DidKey> = joiners.iter().map(|(_, d)| d.clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            joiners.iter().map(|(k, d)| (d.as_str(), k)).collect();
        let am = CommitmentAmendment {
            polity: p.polity.clone(),
            kind: AmendmentKind::Add,
            commitment: commit("whatever-we-prefer"),
            rationale: "r".into(),
            deciders,
            decided_at: t(300),
        };
        let mut bad = a.clone();
        bad.append(CharterEntry::CommitmentAmended(
            Signed::build(am, &pairs).unwrap(),
        ))
        .unwrap();
        refused(&bad, "Add needs a majority (>= 2), got 0");

        // The same act by the members who hold the substrate is authorized.
        let mut good = a;
        good.append(p.amend(AmendmentKind::Add, "shared-watch", &[0, 1], 300)).unwrap();
        ok(&good);
    }

    #[test]
    fn an_office_needs_the_substrate_and_revives_when_it_arrives() {
        let (p, joiner, mut a) = admitted();
        // Conferral on a member who holds nothing is refused outright.
        let mut early = a.clone();
        early.append(p.confer_on(Office::Convener, &joiner, &[0, 1], 250)).unwrap();
        refused(&early, "an office is the polity acting through a member");

        // Once inducted, the identical conferral goes through.
        let cids = p.rite(&mut a, &joiner, &[0, 1], 250);
        p.induct_citing(&mut a, &joiner, &[0, 1], cids, 300);
        a.append(p.confer_on(Office::Convener, &joiner, &[0, 1, 2], 350)).unwrap();
        ok(&a);
        let offices = office_state(&a, &p.polity, t(400));
        assert!(offices.holders(&Office::Convener).contains(&&joiner.1));
    }
}

/// **D34** — the renewal carries the re-recitation the doctrine asks of it.
///
/// The measurement this slice began from: a polity clicking a bare `ReAffirmation`
/// passed the authority check and held its transmission measure green forever, while a
/// polity that actually performed the five movements produced five violations and an
/// abandoned rite. So the pass rewarded the empty form and punished obedience. These
/// tests are that table, inverted, and are the regression for it.
mod renewal {
    use super::*;

    /// A polity of 3, all ratifying, seasoned enough that renewals are in scope.
    fn founded() -> (Polity, Archive<CharterEntry>) {
        let p = Polity::new(3);
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![])).unwrap();
        (p, a)
    }

    fn ok(a: &Archive<CharterEntry>) {
        verify_all(a).expect("signatures valid");
        check_authority(a).expect("expected authorized");
    }
    fn refused(a: &Archive<CharterEntry>, needle: &str) {
        verify_all(a).expect("signatures valid");
        let rs = reasons(a);
        assert!(rs.iter().any(|m| m.contains(needle)), "expected {needle:?}, got: {rs:?}");
    }

    /// Row 1 of the measured table, inverted: the empty form is now refused.
    #[test]
    fn a_bare_renewal_is_refused() {
        let (p, mut a) = founded();
        a.append(p.reaffirm_bare(&p.ids[0], 900)).unwrap();
        refused(&a, "0 of 5 have been recorded");
    }

    /// Row 2, inverted: the faithful re-recitation is authorized, completes its rite
    /// rather than abandoning it, and refreshes the member's freshness — none of which
    /// it could do before D34.
    #[test]
    fn the_full_re_recitation_is_authorized_and_refreshes_freshness() {
        let (p, mut a) = founded();
        p.renew(&mut a, &p.ids[0], 900);
        ok(&a);

        let st = affirmation_state(&a, &p.polity, t(1000));
        assert_eq!(st.last_affirmed(&p.ids[0].1), Some(t(900)));
        assert!(!st.stale_members(500, t(1000)).contains(&p.ids[0].1), "renewed 100s ago");
        assert!(st.stale_members(500, t(1000)).contains(&p.ids[1].1), "never renewed");

        // The rite is completed, not abandoned. Before D34 the completion could never
        // come, so a faithful ceremony sat in the log as an abandoned attempt.
        let rites = rite_state(&a, &p.polity, t(1000));
        assert_eq!(rites.len(), 1);
        assert_eq!(rites[0].kind, AffirmationKind::ReAffirmation);
        assert_eq!(rites[0].completed_at, Some(t(900)), "not an abandoned attempt");
    }

    /// A movement declares which ceremony it belongs to, and the claim is checked
    /// against the member's actual state rather than inferred from it.
    #[test]
    fn a_movement_s_declared_ceremony_must_match_the_member_s_state() {
        // An inducted member reciting an *induction* movement has nothing to acquire.
        let (p, mut a) = founded();
        p.rite(&mut a, &p.ids[0], &[1, 2], 850);
        refused(&a, "induction rite movement for an already-inducted member");

        // And an admitted-but-uninducted member has nothing yet to renew.
        let p2 = Polity::new(3);
        let joiner = id();
        let mut b = Archive::new();
        b.append(p2.charter_seasoned(vec![])).unwrap();
        b.append(p2.admit(&joiner, &[0, 1], 200)).unwrap();
        p2.rite_of(&mut b, &joiner, &[0, 1], 250, AffirmationKind::ReAffirmation);
        refused(&b, "nothing yet to renew");
    }

    /// Nobody attends their own ceremony. Load-bearing only for renewals: at an
    /// induction the inductee cannot attend because attendees must already hold the
    /// substrate, but a renewing member does hold it, so without this rule they would
    /// pad their own attendance. D28's arithmetic — count the people present.
    #[test]
    fn the_reciter_may_not_attend_their_own_movement() {
        let (p, mut a) = founded();
        p.rite_of(&mut a, &p.ids[0], &[0, 1], 895, AffirmationKind::ReAffirmation);
        refused(&a, "may not attend their own rite movement");
    }

    /// Attendance is **recorded, not gated**: a renewal with nobody present is a valid
    /// renewal, because it is precisely the act no one else may condition (D14).
    #[test]
    fn a_renewal_needs_no_attendance_at_all() {
        let (p, mut a) = founded();
        p.renew(&mut a, &p.ids[0], 900);
        ok(&a);
        let rites = rite_state(&a, &p.polity, t(1000));
        assert!(rites[0].attended_throughout().is_empty());
    }

    /// And where there is attendance, it is derived rather than chosen — so a member
    /// can neither over- nor under-report who was there.
    #[test]
    fn recorded_attendance_is_exactly_who_was_present_throughout() {
        // Over-reported: member 2 attended nothing and is named anyway.
        let (p, mut a) = founded();
        let movements =
            p.rite_of(&mut a, &p.ids[0], &[1], 895, AffirmationKind::ReAffirmation);
        p.renew_citing(&mut a, &p.ids[0], &[1, 2], movements, 900);
        refused(&a, "exactly who attended");

        // Under-reported: member 1 attended every movement and is left out.
        let (q, mut b) = founded();
        let movements =
            q.rite_of(&mut b, &q.ids[0], &[1], 895, AffirmationKind::ReAffirmation);
        q.renew_citing(&mut b, &q.ids[0], &[], movements, 900);
        refused(&b, "exactly who attended");

        // Reported exactly: authorized, and the witness is on the record.
        let (r, mut c) = founded();
        r.renew_attended(&mut c, &r.ids[0], &[1, 2], 900);
        ok(&c);
        let rites = rite_state(&c, &r.polity, t(1000));
        assert_eq!(rites[0].attended_throughout().len(), 2);
    }

    /// The coherence checks now bind renewals too, which is the real gain and not
    /// incidental: a polity that amends its commitments and never re-recites becomes
    /// visible, because its next renewal must commit to the amended set.
    #[test]
    fn a_renewal_must_commit_to_the_commitments_as_amended() {
        let (p, mut a) = founded();
        a.append(p.amend(AmendmentKind::Add, "hospitality", &p.all_ratify(), 800))
            .unwrap();
        ok(&a);

        // Reciting yesterday's set — the set before the amendment — is refused.
        let mut yesterday = a.clone();
        p.movement_of(
            &mut yesterday,
            &p.ids[0],
            &[],
            Movement::Substrate { names: substrate().name },
            895,
            AffirmationKind::ReAffirmation,
        );
        p.movement_of(
            &mut yesterday,
            &p.ids[0],
            &[],
            Movement::Fallenness,
            896,
            AffirmationKind::ReAffirmation,
        );
        p.movement_of(
            &mut yesterday,
            &p.ids[0],
            &[],
            Movement::Recognition,
            897,
            AffirmationKind::ReAffirmation,
        );
        p.movement_of(
            &mut yesterday,
            &p.ids[0],
            &[],
            Movement::CommonVirtue { commits_to: vec![] },
            898,
            AffirmationKind::ReAffirmation,
        );
        refused(&yesterday, "commitments in force");

        // Reciting the amended set goes through.
        let mut today = a.clone();
        p.renew(&mut today, &p.ids[0], 900);
        ok(&today);
    }

    /// A ceremony completes one affirmation. Nothing else stops a member replaying a
    /// single re-recitation to reset their freshness every cycle — the induction path
    /// is protected by the no-double-induction rule, and a renewal repeats by nature.
    #[test]
    fn one_re_recitation_completes_one_renewal() {
        let (p, mut a) = founded();
        let once = p.rite_of(&mut a, &p.ids[0], &[], 895, AffirmationKind::ReAffirmation);
        p.renew_citing(&mut a, &p.ids[0], &[], once.clone(), 900);
        p.renew_citing(&mut a, &p.ids[0], &[], once, 1_800);
        refused(&a, "already been renewed on");
    }

    /// A renewal may not pass an induction's rite off as its re-recitation.
    ///
    /// The reachable case is a renewal built on movements that were themselves refused:
    /// an inducted member's `Induction` movements are refused one by one, but a refusal
    /// does not remove them from the log, so the five are there to be cited. Where the
    /// induction rite was *validly* completed, the replay guard catches the borrow
    /// first — this rule is what covers the case the replay guard cannot see.
    #[test]
    fn a_renewal_may_not_cite_an_induction_s_rite() {
        let (p, mut a) = founded();
        // Five Induction movements by a member who already holds the substrate: each is
        // refused on its own, and each is nonetheless in the log.
        let borrowed = p.rite(&mut a, &p.ids[0], &[1, 2], 850);
        refused(&a, "induction rite movement for an already-inducted member");
        p.renew_citing(&mut a, &p.ids[0], &[1, 2], borrowed, 900);
        refused(&a, "cites an induction's rite");
    }

    /// Renewal stays the member's own act: those who attended have already signed the
    /// movements, and nothing here lets them condition the renewal itself.
    #[test]
    fn a_renewal_is_signed_by_the_member_alone() {
        let (p, mut a) = founded();
        let movements =
            p.rite_of(&mut a, &p.ids[0], &[], 895, AffirmationKind::ReAffirmation);
        a.append(p.affirm(
            &p.ids[0].1,
            AffirmationKind::ReAffirmation,
            &[],
            movements,
            900,
            &p.signers(&[1]),
        ))
        .unwrap();
        refused(&a, "exactly the affirming member");
    }
}

// ---------------------------------------------------------------------------
// D36 — the rendezvous office and the relays it lets a member declare.
// ---------------------------------------------------------------------------

const RELAY_UUID: &str = "0f1e2d3c-4b5a-4968-8776-655443322110";

impl Polity {
    /// Member `member`'s relay declaration, signed by member `signer` (normally the
    /// same member; a different one forges it).
    fn relay(&self, member: usize, signer: usize, host: &str, port: u16, at: i64, withdrawn: bool) -> CharterEntry {
        let d = RelayEndpoint {
            polity: self.polity.clone(),
            member: self.ids[member].1.clone(),
            at_uuid: RELAY_UUID.into(),
            host: host.into(),
            port,
            declared_at: t(at),
            withdrawn,
        };
        CharterEntry::RelayEndpointDeclared(Signed::build(d, &self.signers(&[signer])).unwrap())
    }
}

fn violation(a: &Archive<CharterEntry>, needle: &str) -> bool {
    authority_violations(a).iter().any(|(_, why)| why.contains(needle))
}

fn listed(a: &Archive<CharterEntry>, p: &Polity, at: i64) -> Vec<(DidKey, String, u16)> {
    rendezvous_state(a, &p.polity, t(at))
        .relays
        .into_iter()
        .map(|r| (r.member, r.host, r.port))
        .collect()
}

#[test]
fn an_operator_declares_a_relay_and_it_is_listed() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    a.append(p.relay(2, 2, "relay.example.org", 27790, 300, false)).unwrap();

    verify_all(&a).unwrap();
    check_authority(&a).expect("an operator's own declaration is authorized");
    assert_eq!(listed(&a, &p, 400), vec![(p.ids[2].1.clone(), "relay.example.org".into(), 27790)]);
    // Not before it was declared.
    assert!(listed(&a, &p, 250).is_empty());
}

#[test]
fn a_member_without_the_office_cannot_declare() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.relay(1, 1, "relay.example.org", 27790, 300, false)).unwrap();

    assert!(violation(&a, "without holding RendezvousOperator"));
    assert!(listed(&a, &p, 400).is_empty());
}

#[test]
fn a_declaration_before_the_conferral_is_refused() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.relay(2, 2, "relay.example.org", 27790, 150, false)).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    assert!(violation(&a, "without holding RendezvousOperator"));
}

#[test]
fn nobody_declares_a_relay_in_somebody_elses_name() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    // Member 0 signs a declaration that names member 2, who holds the office.
    a.append(p.relay(2, 0, "evil.example.org", 27790, 300, false)).unwrap();

    verify_all(&a).expect("the signature itself is valid");
    assert!(violation(&a, "signed by exactly its member"));
    // The read-model does not rely on the authority pass having run.
    assert!(listed(&a, &p, 400).is_empty());
}

#[test]
fn revoking_or_resigning_the_office_drops_the_relay() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    a.append(p.relay(2, 2, "relay.example.org", 27790, 300, false)).unwrap();
    a.append(p.revoke_office(Office::RendezvousOperator, 2, &[0, 1], 500)).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(listed(&a, &p, 450).len(), 1);
    assert!(listed(&a, &p, 600).is_empty(), "no further act is needed to drop it");

    let mut r = Archive::new();
    r.append(p.charter_seasoned(vec![])).unwrap();
    r.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    r.append(p.relay(2, 2, "relay.example.org", 27790, 300, false)).unwrap();
    r.append(p.resign(Office::RendezvousOperator, 2, 500)).unwrap();
    check_authority(&r).unwrap();
    assert!(listed(&r, &p, 600).is_empty());
}

#[test]
fn a_withdrawal_needs_no_office_and_outlives_a_reconferral() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    a.append(p.relay(2, 2, "relay.example.org", 27790, 300, false)).unwrap();
    a.append(p.revoke_office(Office::RendezvousOperator, 2, &[0, 1], 400)).unwrap();
    // Revoked, and still able to take back what it said.
    a.append(p.relay(2, 2, "", 0, 450, true)).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 500)).unwrap();
    check_authority(&a).expect("a withdrawal by a revoked operator is authorized");
    // Holding the office again does not revive a withdrawn endpoint.
    assert!(listed(&a, &p, 600).is_empty());
}

#[test]
fn the_latest_declaration_wins() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    a.append(p.relay(2, 2, "old.example.org", 27790, 300, false)).unwrap();
    a.append(p.relay(2, 2, "new.example.org", 27791, 400, false)).unwrap();
    check_authority(&a).unwrap();
    assert_eq!(listed(&a, &p, 350), vec![(p.ids[2].1.clone(), "old.example.org".into(), 27790)]);
    assert_eq!(listed(&a, &p, 500), vec![(p.ids[2].1.clone(), "new.example.org".into(), 27791)]);
}

#[test]
fn an_endpoint_that_cannot_be_written_as_a_pin_is_refused() {
    for (host, port) in [("a@b", 27790u16), ("", 27790), ("relay.example.org", 0), ("a b", 1)] {
        let p = Polity::new(3);
        let mut a = Archive::new();
        a.append(p.charter_seasoned(vec![])).unwrap();
        a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
        a.append(p.relay(2, 2, host, port, 300, false)).unwrap();
        assert!(violation(&a, "relay endpoint of"), "{host:?}:{port} was accepted");
        assert!(listed(&a, &p, 400).is_empty(), "{host:?}:{port} was listed");
    }
    let d = RelayEndpoint {
        polity: Polity::new(1).polity,
        member: Polity::new(1).polity,
        at_uuid: RELAY_UUID.to_uppercase(),
        host: "relay.example.org".into(),
        port: 1,
        declared_at: t(1),
        withdrawn: false,
    };
    assert!(d.check_shape().is_err(), "an uppercase uuid is not how AT writes a pin");
}

#[test]
fn a_dissolved_polity_runs_no_relays() {
    let p = Polity::new(3);
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.confer(Office::RendezvousOperator, 2, &[0, 1], 200)).unwrap();
    a.append(p.relay(2, 2, "relay.example.org", 27790, 300, false)).unwrap();
    a.append(p.dissolve(&[], &[0, 1, 2], 400)).unwrap();
    assert_eq!(listed(&a, &p, 350).len(), 1);
    assert!(listed(&a, &p, 500).is_empty());
    a.append(p.relay(2, 2, "later.example.org", 27790, 600, false)).unwrap();
    assert!(violation(&a, "after its dissolution"));
}

#[test]
fn an_operator_who_has_not_received_the_substrate_is_not_listed() {
    // D29: an office acts only through a holder who holds the substrate. A joiner
    // admitted and seated but never inducted cannot declare, and the roster must
    // not list it even though office_state alone would show the holding.
    let p = Polity::new(3);
    let joiner = id();
    let mut a = Archive::new();
    a.append(p.charter_seasoned(vec![])).unwrap();
    a.append(p.admit(&joiner, &[0, 1], 150)).unwrap();
    a.append(p.confer_on(Office::RendezvousOperator, &joiner, &[0, 1], 200)).unwrap();
    let d = RelayEndpoint {
        polity: p.polity.clone(),
        member: joiner.1.clone(),
        at_uuid: RELAY_UUID.into(),
        host: "relay.example.org".into(),
        port: 27790,
        declared_at: t(300),
        withdrawn: false,
    };
    a.append(CharterEntry::RelayEndpointDeclared(
        Signed::build(d, &[(joiner.1.as_str(), &joiner.0)]).unwrap(),
    ))
    .unwrap();
    assert!(violation(&a, "without holding RendezvousOperator"));
    assert!(office_state(&a, &p.polity, t(400)).holds(&joiner.1, &Office::RendezvousOperator),
            "precondition: the raw read-model shows the holding");
    assert!(listed(&a, &p, 400).is_empty());
}
