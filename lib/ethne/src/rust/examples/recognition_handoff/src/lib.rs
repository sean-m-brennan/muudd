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

//! The border: a chartered polity carried into kith-covenant's rites.
//!
//! Ethne emits the artifacts and KC holds the ceremonies, and until now nothing had
//! ever put the two together. This crate is where that composition is measured, and it
//! is an **example** deliberately: no `en-*` library depends on `kc-archive` or
//! `kc-ceremony`, so the confinement rule that keeps KC record deps inside `en-uplift`
//! is untouched and the default build's `cargo tree` is unchanged.

use en_archive::Archive;
use en_charter::{
    affirmation_state, charter_state, office_state, rite_state, Affirmation, AffirmationKind,
    CharterEntry, Commitment, EnvoyAuthorization, EnvoyKind, FoundingProposal, Movement, Office,
    OfficeChange, OfficeKind, RiteMovement, Substrate,
};
use en_core::{keys, Alg, DidKey, KcResult, PrivKey, PubKey, Signed, Timestamp};
// `kc_archive::Archive` is the KC store trait; `en_archive::Archive` is ethne's log.
// Two tiers, one word, so KC's is aliased here rather than shadowing ethne's.
use kc_archive::{Archive as KcStore, ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::{
    InterCommunityWitnessRecord, InterCredentialingOutcome, InterCredentialingRunner,
    ParticipantContext, REQUIRED_FEATURES,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::inter::{
    AntiSpiralFeatureClaim, ExaminerEntry, ExaminerRegistry, FeatureAcceptance,
    NonAdmissionRecord, SponsorshipRecord,
};
use kc_conduct::{
    ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity,
};

/// A founded polity with keys the caller can still sign with.
pub struct Polity {
    pub did: DidKey,
    pub root_priv: PrivKey,
    pub root_pub: PubKey,
    pub members: Vec<(DidKey, PrivKey, PubKey)>,
    pub envoy: (DidKey, PrivKey, PubKey),
    /// The member holding `Office::Inductor`, who recites the polity's KC rite.
    pub inductor: DidKey,
    pub archive: Archive<CharterEntry>,
    pub founded_at: Timestamp,
}

impl Polity {
    /// The private key of one of this polity's members, for a fixture that signs on
    /// their behalf. A real host holds only its own.
    pub fn key_of(&self, did: &DidKey) -> Option<&PrivKey> {
        self.members.iter().find(|(d, _, _)| d == did).map(|(_, sk, _)| sk)
    }
}

fn keypair() -> (DidKey, PrivKey, PubKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).expect("keygen");
    let did = DidKey::from_pubkey(&pk).expect("did");
    (did, sk, pk)
}

/// Found a polity of `n` members with `commitments` in force, authorize an envoy, and
/// return everything a border emitter needs.
pub fn found_polity(name: &str, n: usize, commitments: &[(&str, &str)], at: Timestamp)
    -> KcResult<Polity>
{
    let (polity_did, root_priv, root_pub) = keypair();
    let members: Vec<(DidKey, PrivKey, PubKey)> = (0..n).map(|_| keypair()).collect();
    let roll: Vec<DidKey> = members.iter().map(|(d, _, _)| d.clone()).collect();
    let mut archive = Archive::<CharterEntry>::new();

    let proposal = FoundingProposal::convener_asserted(
        polity_did.clone(),
        Substrate {
            name: format!("{name}-substrate"),
            statement: "the ground this polity did not make".into(),
            anchor: None,
        },
        commitments
            .iter()
            .map(|(k, d)| Commitment { key: (*k).into(), description: (*d).into() })
            .collect(),
        roll.clone(),
        vec![],
        at,
    );
    let charter = proposal.ratify(roll.clone(), at)?;
    let signers: Vec<(&str, &PrivKey)> =
        members.iter().map(|(d, sk, _)| (d.as_str(), sk)).collect();
    archive.append(CharterEntry::PolityChartered(Signed::build(charter, &signers)?))?;

    // The envoy is a MEMBER the roll authorizes to speak for the polity beyond it.
    // Measured: a non-member holds no standing, so no delegation can be minted for one.
    let (envoy_did, envoy_priv, envoy_pub) = (
        members[0].0.clone(),
        members[0].1.clone(),
        members[0].2.clone(),
    );
    let authorization = EnvoyAuthorization {
        polity: polity_did.clone(),
        kind: EnvoyKind::Authorize,
        envoy: envoy_did.clone(),
        envoy_pubkey: envoy_pub.clone(),
        rationale: "to carry the polity's word to the compact".into(),
        authorizers: roll.clone(),
        expires_at: None,
        decided_at: at,
    };
    archive.append(CharterEntry::EnvoyAuthorized(Signed::build(
        authorization,
        &signers,
    )?))?;

    // The Inductor: the office that presides over the polity's rite, and therefore the
    // office that carries one across the border.
    let inductor = members[1].0.clone();
    let conferral = OfficeChange {
        polity: polity_did.clone(),
        kind: OfficeKind::Confer,
        office: Office::Inductor,
        holder: inductor.clone(),
        rationale: "to preside over the rite".into(),
        deciders: roll.clone(),
        decided_at: at,
    };
    archive.append(CharterEntry::OfficeChanged(Signed::build(conferral, &signers)?))?;

    // The rite itself: every member re-recites the five movements with the rest of the
    // roll attending, and the renewal completes it. Without this nothing crosses: a KC
    // rite is not mintable from a ceremony nobody performed.
    let commit_keys: Vec<String> = commitments.iter().map(|(k, _)| (*k).to_string()).collect();
    let substrate_name = format!("{name}-substrate");
    for (member, sk, _) in &members {
        let attending: Vec<DidKey> = roll.iter().filter(|d| *d != member).cloned().collect();
        let movements = [
            Movement::Substrate { names: substrate_name.clone() },
            Movement::Fallenness,
            Movement::Recognition,
            Movement::CommonVirtue { commits_to: commit_keys.clone() },
            Movement::NonUltimacy { names_anchor: None },
        ];
        let mut cited = Vec::new();
        for (i, movement) in movements.into_iter().enumerate() {
            let recited_at = Timestamp::from_second(at.as_second() + 1 + i as i64)
                .map_err(|e| en_core::KcError::Archive(e.to_string()))?;
            let record = RiteMovement {
                polity: polity_did.clone(),
                member: member.clone(),
                kind: AffirmationKind::ReAffirmation,
                movement,
                statement: "as we have received it".into(),
                attending: attending.clone(),
                recited_at,
            };
            let mut rite_signers: Vec<(&str, &PrivKey)> = vec![(member.as_str(), sk)];
            rite_signers.extend(
                members.iter().filter(|(d, _, _)| d != member).map(|(d, k, _)| (d.as_str(), k)),
            );
            cited.push(archive.append(CharterEntry::RiteMovementRecorded(Signed::build(
                record,
                &rite_signers,
            )?))?);
        }
        let affirmed_at = Timestamp::from_second(at.as_second() + 10)
            .map_err(|e| en_core::KcError::Archive(e.to_string()))?;
        let affirmation = Affirmation {
            polity: polity_did.clone(),
            member: member.clone(),
            kind: AffirmationKind::ReAffirmation,
            witnesses: attending,
            movements: cited,
            affirmed_at,
        };
        archive.append(CharterEntry::MemberAffirmed(Signed::build(
            affirmation,
            &[(member.as_str(), sk)],
        )?))?;
    }

    Ok(Polity {
        did: polity_did,
        root_priv,
        root_pub,
        members,
        envoy: (envoy_did, envoy_priv, envoy_pub),
        inductor,
        archive,
        founded_at: at,
    })
}

/// What the border hands KC, and what KC did with it.
#[derive(Debug, Default)]
pub struct HandoffReading {
    /// Border artifacts appended to the KC archive.
    pub appended: Vec<String>,
    /// Refusals, in the order they were met, each naming the tier that owns it.
    pub refusals: Vec<String>,
    /// The polity's in-force commitment keys, as the charter holds them.
    pub commitments: Vec<String>,
}

/// Hand `polity` to a KC archive and report what composes.
pub fn hand_off(polity: &Polity, kc: &mut InMemoryArchive, at: Timestamp)
    -> KcResult<HandoffReading>
{
    let mut out = HandoffReading::default();
    let state = charter_state(&polity.archive, &polity.did, at);
    out.commitments = state.commitments.iter().map(|c| c.key.clone()).collect();

    let identity = en_uplift::community_identity(
        "polity".into(),
        &charter_of(&polity.archive)?,
        &polity.root_priv,
        polity.root_pub.clone(),
        polity.founded_at,
    )?;
    kc.append(ArchiveEntry::CommunityIdentityPublished(identity))?;
    out.appended.push("CommunityIdentity".into());

    let envoys = en_charter::envoy_state(&polity.archive, &polity.did, at);
    let delegation = en_uplift::envoy_delegation(
        &charter_of(&polity.archive)?,
        &envoys,
        &polity.envoy.0,
        &polity.root_priv,
        polity.root_pub.clone(),
        at,
    )?;
    kc.append(ArchiveEntry::SpeakerDelegated(delegation))?;
    out.appended.push("SpeakerDelegation(Envoy)".into());

    let roll = en_uplift::member_roll(
        &charter_of(&polity.archive)?,
        &state.members,
        &polity.root_priv,
        polity.root_pub.clone(),
        at,
        at,
    )?;
    kc.append(ArchiveEntry::MemberRollPublished(roll))?;
    out.appended.push("MemberRoll".into());

    Ok(out)
}

/// Seal the polity's KC rite: the border emits the authorizations and the draft, and
/// the people who hold the keys sign.
///
/// The split is the point. `en_uplift::intra_rite` holds nobody's key but the polity
/// root's, so the reciter signs the recitation and each witness signs its own
/// attestation, exactly as `IntraCommunityRiteRunner` requires of any KC community.
pub fn seal_rite(
    polity: &Polity,
    kc: &mut InMemoryArchive,
    at: Timestamp,
) -> KcResult<Signed<IntraCommunityRiteRecord>> {
    let state = charter_state(&polity.archive, &polity.did, at);
    let offices = office_state(&polity.archive, &polity.did, at);
    let affirmations = affirmation_state(&polity.archive, &polity.did, at);
    let rites = rite_state(&polity.archive, &polity.did, at);

    let handoff = en_uplift::intra_rite(
        &charter_of(&polity.archive)?,
        &state.commitments,
        &offices,
        &affirmations,
        &rites,
        &polity.inductor,
        &polity.root_priv,
        polity.root_pub.clone(),
        at,
    )?;

    kc.append(ArchiveEntry::SpeakerDelegated(handoff.reciter.clone()))?;
    kc.append(ArchiveEntry::WitnessRegistryUpdated(handoff.witnesses.clone()))?;

    let identity = identity_in(kc, &polity.did)?;
    let delegations = vec![handoff.reciter.clone()];
    let runner =
        IntraCommunityRiteRunner::new(&identity, &delegations, &handoff.witnesses);

    let inductor_priv = polity
        .key_of(&polity.inductor)
        .expect("the inductor is a member of this fixture");
    let recitation =
        IntraCommunityRiteRunner::recite(&polity.inductor, inductor_priv, handoff.recitation)?;

    let mut attestations = Vec::new();
    for entry in &handoff.witnesses.payload.entries {
        let key = polity
            .key_of(&entry.witness_did)
            .expect("every witness is a member of this fixture");
        attestations.push(IntraCommunityRiteRunner::attest(
            &entry.witness_did,
            key,
            recitation.payload_cid,
            at,
        )?);
    }
    let sealed = runner.seal(recitation, attestations, &polity.root_priv, at)?;
    kc.append(ArchiveEntry::IntraCommunityRite(sealed.clone()))?;
    Ok(sealed)
}

/// The compact: two polities admitted to one, through KC's inter-community ceremony.
///
/// The envoy is the cross-boundary signer, which is what the `Envoy` scope was for all
/// along, and each polity's sealed rite is what it brings. Both halves of the border
/// are load-bearing here: without the identity the ceremony has no participant, and
/// without the rite it has nothing to sequence.
pub fn admit_to_compact(
    label: &str,
    polities: &[(&Polity, &Signed<IntraCommunityRiteRecord>)],
    kc: &mut InMemoryArchive,
    at: Timestamp,
) -> KcResult<Signed<InterCommunityWitnessRecord>> {
    let identities: Vec<_> = polities
        .iter()
        .map(|(p, _)| identity_in(kc, &p.did))
        .collect::<KcResult<Vec<_>>>()?;
    let participants: Vec<ParticipantContext> = polities
        .iter()
        .zip(identities.iter())
        .map(|((p, rite), identity)| ParticipantContext {
            community: identity,
            intra_record: rite,
            cross_boundary_signer_did: p.envoy.0.clone(),
            cross_boundary_signer_priv: &p.envoy.1,
        })
        .collect();
    let runner = InterCredentialingRunner::new(label.into(), participants);

    let mut claims = Vec::new();
    for (p, _) in polities {
        for feature in REQUIRED_FEATURES.iter().copied() {
            claims.push(InterCredentialingRunner::present_claim(
                &p.envoy.0,
                &p.envoy.1,
                p.did.clone(),
                feature,
                format!("{feature:?} as the polity's charter binds it"),
                vec![],
                at,
            )?);
        }
    }
    // Each polity accepts the other's claims: recognition is mutual or it is not
    // recognition.
    let mut acceptances = Vec::new();
    for claim in &claims {
        for (p, _) in polities {
            if p.did == claim.payload.community_did {
                continue;
            }
            acceptances.push(InterCredentialingRunner::accept(
                &p.envoy.0,
                &p.envoy.1,
                p.did.clone(),
                claim.payload_cid,
                true,
                None,
                at,
            )?);
        }
    }

    match runner.seal(claims, acceptances, vec![], at)? {
        InterCredentialingOutcome::Sealed(record) => {
            kc.append(ArchiveEntry::InterCommunityWitness(record.clone()))?;
            Ok(record)
        }
        InterCredentialingOutcome::NonAdmission(record) => {
            kc.append(ArchiveEntry::InterCommunityNonAdmission(record))?;
            Err(en_core::KcError::Authority(
                "the ceremony produced a non-admission".into(),
            ))
        }
    }
}

/// The polity's published identity, read back out of the KC archive.
pub fn identity_in(
    kc: &InMemoryArchive,
    polity: &DidKey,
) -> KcResult<Signed<kc_core::CommunityIdentity>> {
    kc.iter()
        .find_map(|(_, e)| match e {
            ArchiveEntry::CommunityIdentityPublished(s) if &s.payload.did == polity => {
                Some(s.clone())
            }
            _ => None,
        })
        .ok_or_else(|| en_core::KcError::Archive(format!("{polity} is not published in KC")))
}

/// The ratified charter, read back out of the polity's own log.
pub fn charter_of(archive: &Archive<CharterEntry>) -> KcResult<en_charter::PolityCharter> {
    for (_, entry) in archive.iter() {
        if let CharterEntry::PolityChartered(s) = entry {
            return Ok(s.payload.clone());
        }
    }
    Err(en_core::KcError::Archive("the polity is not chartered".into()))
}

// --- Across nodes: admission into a standing compact (Agora Phase 5) -------------
//
// Everything above runs every key in one process. Below is the shape the Agora app
// uses when each community's keys live on its own device: every record is signed by
// whoever holds the key, and the one record too large to carry, the seal, is never
// carried at all. Each node rebuilds it from records it already holds and signs the
// rebuilt bytes.

/// Seat `seats` (DID and public key) as `polity`'s examiners from `from` until `until`.
/// Root-signed, as KC requires of a roster.
pub fn seat_examiners(
    polity: &Polity,
    seats: &[(DidKey, PubKey)],
    kc: &mut InMemoryArchive,
    from: Timestamp,
    until: Timestamp,
) -> KcResult<Signed<ExaminerRegistry>> {
    let registry = ExaminerRegistry {
        nominating_community: polity.did.clone(),
        entries: seats
            .iter()
            .map(|(did, pk)| ExaminerEntry {
                examiner_did: did.clone(),
                examiner_pubkey: pk.clone(),
                valid_from: from,
                valid_until: Some(until),
            })
            .collect(),
        issued_at: from,
    };
    let signed = Signed::build(registry, &[(polity.did.as_str(), &polity.root_priv)])?;
    kc.append(ArchiveEntry::ExaminerRegistryUpdated(signed.clone()))?;
    Ok(signed)
}

/// `sponsor` vouches for `entrant`, staking its standing until `until`. Envoy-signed.
pub fn sponsor(
    sponsor: &Polity,
    entrant: &DidKey,
    kc: &mut InMemoryArchive,
    at: Timestamp,
    until: Timestamp,
) -> KcResult<Signed<SponsorshipRecord>> {
    let record = SponsorshipRecord {
        sponsor_community: sponsor.did.clone(),
        entrant_community: entrant.clone(),
        admission_cid: None,
        probation_until: until,
        statement: "we answer for them while they find their footing".into(),
        sponsored_at: at,
    };
    let signed = Signed::build(record, &[(sponsor.envoy.0.as_str(), &sponsor.envoy.1)])?;
    kc.append(ArchiveEntry::SponsorshipVouched(signed.clone()))?;
    Ok(signed)
}

/// The claims every participant presents and the verdicts every other participant
/// gives, each signed by that community's own envoy. `verdict(accepter, presenter)`
/// says whether `accepter` accepts `presenter`'s claims.
#[allow(clippy::type_complexity)]
pub fn ceremony_parts(
    polities: &[&Polity],
    at: Timestamp,
    verdict: impl Fn(&DidKey, &DidKey) -> bool,
) -> KcResult<(Vec<Signed<AntiSpiralFeatureClaim>>, Vec<Signed<FeatureAcceptance>>)> {
    let mut claims = Vec::new();
    for p in polities {
        for feature in REQUIRED_FEATURES.iter().copied() {
            claims.push(InterCredentialingRunner::present_claim(
                &p.envoy.0,
                &p.envoy.1,
                p.did.clone(),
                feature,
                format!("{feature:?} as the polity's charter binds it"),
                vec![],
                at,
            )?);
        }
    }
    let mut acceptances = Vec::new();
    for claim in &claims {
        for p in polities {
            if p.did == claim.payload.community_did {
                continue;
            }
            let ok = verdict(&p.did, &claim.payload.community_did);
            acceptances.push(InterCredentialingRunner::accept(
                &p.envoy.0,
                &p.envoy.1,
                p.did.clone(),
                claim.payload_cid,
                ok,
                (!ok).then(|| "not yet".to_string()),
                at,
            )?);
        }
    }
    Ok((claims, acceptances))
}

/// What a seal would be, built with throwaway keys so that KC's own runner still
/// enforces one claim per feature per participant and one verdict per claim from
/// every other participant. The envelope is discarded; only the payload is kept.
pub enum Draft {
    Sealed(InterCommunityWitnessRecord),
    NonAdmission(NonAdmissionRecord),
}

pub fn draft_seal(
    label: &str,
    participants: &[(&Polity, &Signed<IntraCommunityRiteRecord>)],
    kc: &InMemoryArchive,
    claims: Vec<Signed<AntiSpiralFeatureClaim>>,
    acceptances: Vec<Signed<FeatureAcceptance>>,
    at: Timestamp,
) -> KcResult<Draft> {
    let identities: Vec<_> = participants
        .iter()
        .map(|(p, _)| identity_in(kc, &p.did))
        .collect::<KcResult<Vec<_>>>()?;
    let throwaway: Vec<PrivKey> = participants.iter().map(|_| keypair().1).collect();
    let contexts: Vec<ParticipantContext> = participants
        .iter()
        .zip(identities.iter())
        .zip(throwaway.iter())
        .map(|(((p, rite), identity), sk)| ParticipantContext {
            community: identity,
            intra_record: rite,
            cross_boundary_signer_did: p.envoy.0.clone(),
            cross_boundary_signer_priv: sk,
        })
        .collect();
    let runner = InterCredentialingRunner::new(label.into(), contexts);
    Ok(match runner.seal(claims, acceptances, vec![], at)? {
        InterCredentialingOutcome::Sealed(s) => Draft::Sealed(s.payload),
        InterCredentialingOutcome::NonAdmission(s) => Draft::NonAdmission(s.payload),
    })
}

/// Sign `payload` the way the Agora nodes do: each signer signs the canonical bytes
/// on its own device, and the signatures are assembled onto the payload. Returns the
/// envelope and the size, in bytes, of what each signer was handed.
pub fn detached<T: serde::Serialize>(
    payload: T,
    signers: &[(&DidKey, &PrivKey)],
) -> KcResult<(Signed<T>, usize)> {
    let pending = en_charter::pending(payload, "a KC record".into())?;
    let mut sigs = Vec::new();
    for (did, sk) in signers {
        sigs.push((did.as_str().to_string(), keys::sign(sk, &pending.bytes)?.bytes));
    }
    let n = pending.bytes.len();
    Ok((en_charter::assemble(pending.payload, &sigs)?, n))
}

/// A second-class anathema on `subject`, reported by every community in `reporters`,
/// each signing through its envoy. KC counts the reporters as the quorum only when
/// the record says `SecondClass`; Agora always does.
pub fn anathematize(
    reporters: &[&Polity],
    subject: &DidKey,
    kc: &mut InMemoryArchive,
    at: Timestamp,
) -> KcResult<Signed<ConductRecord>> {
    let record = ConductRecord {
        subject_community: subject.clone(),
        reporters: reporters.iter().map(|p| p.did.clone()).collect(),
        reporter_perspective: None,
        kind: ConductKind::DesignationAnathema,
        bearing: ConductBearing::CommonsObligation,
        severity: Severity::Grave,
        ladder_rung: Some(LadderRung::Expulsion),
        decision_class: DecisionClass::SecondClass,
        references: vec![],
        commitments_cited: vec![],
        narrative: "the compact has found it cannot stand with them".into(),
        observed_at: at,
        recorded_at: at,
    };
    let signers: Vec<(&DidKey, &PrivKey)> =
        reporters.iter().map(|p| (&p.envoy.0, &p.envoy.1)).collect();
    let (signed, _) = detached(record, &signers)?;
    kc.append(ArchiveEntry::ConductAttested(signed.clone()))?;
    Ok(signed)
}

/// The manifest a seal ask carries in place of the seal: the label, the instant, and
/// the content addresses of everything the seal is built from. Every replica holds
/// those records, so every signer can rebuild the seal and check it is this one.
///
/// Carried as canonical CBOR, CIDs as their binary form: as JSON with text CIDs a
/// three-participant manifest measured 7286 hex characters, over the carrier's 6144.
#[derive(serde::Serialize)]
pub struct SealManifest {
    pub label: String,
    pub participants: Vec<DidKey>,
    pub rites: Vec<kc_core::Cid>,
    pub claims: Vec<kc_core::Cid>,
    pub acceptances: Vec<kc_core::Cid>,
    pub witnessed_at: Timestamp,
}

impl SealManifest {
    pub fn of(rec: &InterCommunityWitnessRecord) -> Self {
        Self {
            label: rec.compact_label.clone(),
            participants: rec.participants.clone(),
            rites: rec.sequenced_rites.iter().map(|r| r.intra_record_cid).collect(),
            claims: rec.claims.iter().map(|c| c.payload_cid).collect(),
            acceptances: rec.acceptances.iter().map(|a| a.payload_cid).collect(),
            witnessed_at: rec.witnessed_at,
        }
    }

    /// Its size on the co-sign carrier, in hex characters.
    pub fn hex_len(&self) -> usize {
        2 * kc_core::canonical::to_canonical_cbor(self).map(|b| b.len()).unwrap_or(0)
    }
}

/// The size, in hex characters (what the co-sign carrier counts), of a signed record
/// as Agora delivers it: its canonical CBOR envelope.
pub fn hex_len<T: serde::Serialize>(signed: &Signed<T>) -> usize {
    2 * kc_core::canonical::to_canonical_cbor(signed).map(|b| b.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_archive::check_authority;
    use kc_reputation::{commitment_coverage, compact_object};

    const T0: i64 = 1_700_000_000;

    fn ts(n: i64) -> Timestamp {
        Timestamp::from_second(n).expect("timestamp")
    }

    /// A compact of two polities, each having handed KC its identity, envoy, roll and
    /// sealed rite. Returns the KC archive and the instant everything is read at.
    fn a_compact_of_polities() -> (InMemoryArchive, Polity, Polity, Timestamp) {
        let at = ts(T0);
        let later = ts(T0 + 100);
        let alpha = found_polity(
            "Alpha",
            3,
            &[("mutual-aid", "we answer for one another"),
              ("truthfulness", "we tell the truth about what we have done")],
            at,
        )
        .expect("Alpha founds");
        let beta = found_polity(
            "Beta",
            3,
            &[("mutual-aid", "we answer for one another"),
              ("hospitality", "we receive the stranger")],
            at,
        )
        .expect("Beta founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&alpha, &mut kc, at).expect("Alpha's border artifacts");
        hand_off(&beta, &mut kc, at).expect("Beta's border artifacts");
        let a_rite = seal_rite(&alpha, &mut kc, later).expect("Alpha's rite");
        let b_rite = seal_rite(&beta, &mut kc, later).expect("Beta's rite");
        admit_to_compact(
            "polity-compact",
            &[(&alpha, &a_rite), (&beta, &b_rite)],
            &mut kc,
            later,
        )
        .expect("the compact seals");
        (kc, alpha, beta, later)
    }

    fn authorized(kc: &InMemoryArchive) -> kc_core::KcResult<()> {
        check_authority(kc, 2, 3, Default::default(), Default::default(), Default::default())
    }

    // --- the identity half, which composed before this crate existed ----------

    #[test]
    fn the_border_artifacts_authorize_in_kc() {
        let at = ts(T0);
        let p = found_polity("Alpha", 3, &[("mutual-aid", "we answer")], at).expect("founds");
        let mut kc = InMemoryArchive::new();
        let reading = hand_off(&p, &mut kc, at).expect("the border emits");
        assert_eq!(reading.appended.len(), 3);
        kc_archive::verify_all(&kc).expect("the artifacts verify");
        authorized(&kc).expect("and they authorize");
    }

    // --- the rite half, which is what this slice built ------------------------

    #[test]
    fn the_rite_carries_the_polity_s_own_commitments() {
        // The whole point of the border. KC's Movement 4 is the polity's in-force
        // commitment list, key for key, because the two tiers model a commitment the
        // same way: `{ key, description }` here and `{ key, text }` there.
        let at = ts(T0);
        let later = ts(T0 + 100);
        let p = found_polity(
            "Alpha",
            3,
            &[("mutual-aid", "we answer for one another"),
              ("truthfulness", "we tell the truth about what we have done")],
            at,
        )
        .expect("founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&p, &mut kc, at).expect("border");
        let rite = seal_rite(&p, &mut kc, later).expect("the rite seals");

        let movement = &rite.payload.recitation.payload.movement_common_virtue;
        assert_eq!(movement.keys(), vec!["mutual-aid", "truthfulness"]);
        assert!(rite.payload.recitation.payload.movements_all_present());
        authorized(&kc).expect("a sealed polity rite authorizes in KC");
    }

    #[test]
    fn the_inductor_recites_and_the_attendees_witness() {
        let at = ts(T0);
        let later = ts(T0 + 100);
        let p = found_polity("Alpha", 3, &[("mutual-aid", "we answer")], at).expect("founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&p, &mut kc, at).expect("border");
        let rite = seal_rite(&p, &mut kc, later).expect("the rite seals");

        assert_eq!(rite.payload.recitation.payload.speaker_did, p.inductor);
        // The attendees of the polity's last complete rite, minus the reciter. Two
        // members attended it and one of them is the Inductor, so one witness crosses.
        // Attendance is a fact about people, not a count the border invents.
        assert_eq!(rite.payload.internal_witnesses.len(), 1);
        for att in &rite.payload.internal_witnesses {
            assert_ne!(att.payload.witness_did, p.inductor, "a reciter is not its own witness");
        }
    }

    // --- the compact ----------------------------------------------------------

    #[test]
    fn two_polities_are_admitted_to_one_compact() {
        let (kc, alpha, beta, at) = a_compact_of_polities();
        let sealed = kc
            .iter()
            .find_map(|(_, e)| match e {
                ArchiveEntry::InterCommunityWitness(s) => Some(s.clone()),
                _ => None,
            })
            .expect("the compact is sealed");
        assert_eq!(sealed.payload.participants.len(), 2);
        assert_eq!(sealed.payload.sequenced_rites.len(), 2);
        assert_eq!(sealed.payload.claims.len(), 10, "five features each");
        assert!(sealed.payload.participants.contains(&alpha.did));
        assert!(sealed.payload.participants.contains(&beta.did));
        kc_archive::verify_all(&kc).expect("the whole archive verifies");
        authorized(&kc).expect("and authorizes");
        let _ = at;
    }

    #[test]
    fn the_compact_s_recited_object_is_the_polities_own_commitments() {
        // What the two tiers compose *into*. KC's recited-object reading grades a
        // compact against the union of what its members undertook, and for a compact of
        // polities that union is their charters', carried across by the rite.
        let (kc, _alpha, _beta, at) = a_compact_of_polities();
        let object = compact_object(&kc, Some(at));
        assert_eq!(object.keys, vec!["hospitality", "mutual-aid", "truthfulness"]);
        assert_eq!(object.per_community.len(), 2);

        // Nothing has been enforced, so the reading is untestable rather than a
        // narrowing: a compact that has just formed has not abandoned anything.
        let coverage = commitment_coverage(&kc, Some(at), 5 * 365 * 86_400);
        assert_eq!(coverage.object_size, 3);
        assert_eq!(coverage.acts, 0);
        assert!(!coverage.testable(&Default::default()));
    }

    // --- the gates ------------------------------------------------------------

    #[test]
    fn a_rite_is_not_mintable_from_a_ceremony_nobody_performed() {
        // The gate the user's ruling asked for. Read at the founding instant the rite
        // has not happened yet, and the border refuses to speak for it.
        let at = ts(T0);
        let p = found_polity("Alpha", 3, &[("mutual-aid", "we answer")], at).expect("founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&p, &mut kc, at).expect("border");
        let err = seal_rite(&p, &mut kc, at).expect_err("no rite stands yet");
        assert!(format!("{err}").contains("no complete rite stands"), "{err}");
    }

    #[test]
    fn a_member_who_holds_no_inductor_office_may_not_recite() {
        let at = ts(T0);
        let later = ts(T0 + 100);
        let mut p = found_polity("Alpha", 3, &[("mutual-aid", "we answer")], at).expect("founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&p, &mut kc, at).expect("border");
        // The envoy speaks for the polity outward and does not preside over its rite.
        p.inductor = p.envoy.0.clone();
        let err = seal_rite(&p, &mut kc, later).expect_err("the envoy is not the Inductor");
        assert!(format!("{err}").contains("holds no Inductor office"), "{err}");
    }

    #[test]
    fn a_polity_with_no_commitments_recites_no_movement_four() {
        let at = ts(T0);
        let later = ts(T0 + 100);
        let p = found_polity("Alpha", 3, &[], at).expect("founds");
        let mut kc = InMemoryArchive::new();
        hand_off(&p, &mut kc, at).expect("border");
        let err = seal_rite(&p, &mut kc, later).expect_err("nothing to commit to");
        assert!(format!("{err}").contains("no commitments in force"), "{err}");
    }

    // --- across nodes: admission into a standing compact (Agora Phase 5) ------

    use kc_reputation::{lookup, RecognitionStanding};

    /// Alpha and Beta's compact, plus Gamma ready at the border with its rite sealed,
    /// and the examiners seated: Alpha's envoy and one more Alpha member, Beta's envoy.
    fn a_compact_and_an_entrant() -> (InMemoryArchive, Polity, Polity, Polity,
                                      Signed<IntraCommunityRiteRecord>, Timestamp) {
        let (mut kc, alpha, beta, later) = a_compact_of_polities();
        let gamma = found_polity("Gamma", 3, &[("hospitality", "we receive the stranger")],
                                 ts(T0)).expect("Gamma founds");
        hand_off(&gamma, &mut kc, ts(T0)).expect("Gamma's border");
        let g_rite = seal_rite(&gamma, &mut kc, later).expect("Gamma's rite");
        let until = ts(T0 + 100 + 180 * 86_400);
        let a_second = (alpha.members[2].0.clone(), alpha.members[2].2.clone());
        seat_examiners(&alpha, &[(alpha.envoy.0.clone(), alpha.envoy.2.clone()), a_second],
                       &mut kc, later, until).expect("Alpha seats two");
        seat_examiners(&beta, &[(beta.envoy.0.clone(), beta.envoy.2.clone())],
                       &mut kc, later, until).expect("Beta seats one");
        (kc, alpha, beta, gamma, g_rite, ts(T0 + 200))
    }

    fn rite_of(kc: &InMemoryArchive, did: &DidKey) -> Signed<IntraCommunityRiteRecord> {
        kc.iter()
            .find_map(|(_, e)| match e {
                ArchiveEntry::IntraCommunityRite(s)
                    if &s.payload.recitation.payload.community_did == did => Some(s.clone()),
                _ => None,
            })
            .expect("the rite is in KC")
    }

    /// Admit Gamma into the compact. `with_sponsor` and `examiners` vary the gates.
    fn admit_gamma(kc: &mut InMemoryArchive, alpha: &Polity, beta: &Polity, gamma: &Polity,
                   g_rite: &Signed<IntraCommunityRiteRecord>, at: Timestamp,
                   with_sponsor: bool, examiners: usize)
                   -> KcResult<Signed<InterCommunityWitnessRecord>> {
        if with_sponsor {
            sponsor(alpha, &gamma.did, kc, at, ts(at.as_second() + 90 * 86_400))?;
        }
        let (claims, acceptances) = ceremony_parts(&[alpha, beta, gamma], at, |_, _| true)?;
        let a_rite = rite_of(kc, &alpha.did);
        let b_rite = rite_of(kc, &beta.did);
        let draft = draft_seal("polity-compact",
                               &[(alpha, &a_rite), (beta, &b_rite), (gamma, g_rite)],
                               kc, claims, acceptances, at)?;
        let Draft::Sealed(rec) = draft else { panic!("everyone accepted") };
        let mut signers: Vec<(&DidKey, &PrivKey)> = vec![
            (&alpha.envoy.0, &alpha.envoy.1),
            (&beta.envoy.0, &beta.envoy.1),
            (&gamma.envoy.0, &gamma.envoy.1),
        ];
        if examiners >= 3 {
            signers.push((&alpha.members[2].0, &alpha.members[2].1));
        }
        let (signed, _) = detached(rec, &signers)?;
        kc.append(ArchiveEntry::InterCommunityWitness(signed.clone()))?;
        Ok(signed)
    }

    #[test]
    fn a_third_community_is_admitted_with_a_sponsor_and_a_panel_of_three() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, true, 3)
            .expect("the seal assembles from detached signatures");
        kc_archive::verify_all(&kc).expect("verifies");
        authorized(&kc).expect("sponsor, quorum of two sitting, panel of three from two");
        let view = lookup(&kc, &gamma.did, Some(ts(at.as_second() + 1)));
        assert_eq!(view.standing, RecognitionStanding::Probationary);
        let alpha_view = lookup(&kc, &alpha.did, Some(ts(at.as_second() + 1)));
        assert_eq!(alpha_view.standing, RecognitionStanding::InCompact);
    }

    #[test]
    fn without_a_sponsor_the_admission_does_not_authorize() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, false, 3).expect("appends");
        let err = authorized(&kc).expect_err("no sponsor");
        assert!(format!("{err}").contains("no sponsor"), "{err}");
    }

    #[test]
    fn a_panel_of_two_does_not_authorize() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, true, 2).expect("appends");
        let err = authorized(&kc).expect_err("two examiners");
        assert!(format!("{err}").contains("seated examiner"), "{err}");
    }

    #[test]
    fn a_refused_ceremony_leaves_the_entrant_denied() {
        let (mut kc, alpha, beta, _gamma, _g_rite, at) = a_compact_and_an_entrant();
        let delta = found_polity("Delta", 3, &[("valour", "we stand")], ts(T0)).expect("founds");
        hand_off(&delta, &mut kc, ts(T0)).expect("border");
        let d_rite = seal_rite(&delta, &mut kc, ts(T0 + 100)).expect("rite");
        sponsor(&alpha, &delta.did, &mut kc, at, ts(at.as_second() + 90 * 86_400)).expect("vouch");
        let beta_did = beta.did.clone();
        let delta_did = delta.did.clone();
        let (claims, acceptances) = ceremony_parts(&[&alpha, &beta, &delta], at, |acc, pres| {
            !(acc == &beta_did && pres == &delta_did)
        })
        .expect("parts");
        let a_rite = rite_of(&kc, &alpha.did);
        let b_rite = rite_of(&kc, &beta.did);
        let draft = draft_seal("polity-compact",
                               &[(&alpha, &a_rite), (&beta, &b_rite), (&delta, &d_rite)],
                               &kc, claims, acceptances, at).expect("drafts");
        let Draft::NonAdmission(rec) = draft else { panic!("Beta refused") };
        let (signed, _) = detached(rec, &[(&alpha.envoy.0, &alpha.envoy.1),
                                          (&beta.envoy.0, &beta.envoy.1),
                                          (&delta.envoy.0, &delta.envoy.1)])
            .expect("signed by all three, the refused one too");
        kc.append(ArchiveEntry::InterCommunityNonAdmission(signed)).expect("append");
        authorized(&kc).expect("authorizes");
        let view = lookup(&kc, &delta.did, Some(ts(at.as_second() + 1)));
        assert_eq!(view.standing, RecognitionStanding::Denied);
    }

    #[test]
    fn two_communities_anathematize_a_third() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, true, 3).expect("admitted");
        let later = ts(at.as_second() + 1000);
        anathematize(&[&alpha, &beta], &gamma.did, &mut kc, later).expect("second class");
        kc_archive::verify_all(&kc).expect("verifies");
        authorized(&kc).expect("two reporters make the quorum");
        let view = lookup(&kc, &gamma.did, Some(ts(later.as_second() + 1)));
        assert_eq!(view.standing, RecognitionStanding::Anathema);
    }

    #[test]
    fn one_reporter_does_not_make_an_anathema() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, true, 3).expect("admitted");
        anathematize(&[&alpha], &gamma.did, &mut kc, ts(at.as_second() + 1000)).expect("appends");
        assert!(authorized(&kc).is_err(), "a second-class act needs a quorum of two");
    }

    /// Phase 5 sizing (Agora): what each record costs on the co-sign carrier, which
    /// counts hex characters of canonical CBOR and holds at most 6144.
    #[test]
    fn record_sizes_on_the_co_sign_carrier() {
        let (mut kc, alpha, beta, gamma, g_rite, at) = a_compact_and_an_entrant();
        let founding = kc.iter().find_map(|(_, e)| match e {
            ArchiveEntry::InterCommunityWitness(s) => Some(s.clone()), _ => None,
        }).expect("the founding seal");
        let admitted = admit_gamma(&mut kc, &alpha, &beta, &gamma, &g_rite, at, true, 3)
            .expect("admitted");
        let anathema = anathematize(&[&alpha, &beta], &gamma.did, &mut kc,
                                    ts(at.as_second() + 1000)).expect("anathema");
        let mut max_small = 0usize;
        println!("record (hex chars of canonical CBOR):");
        for (_, e) in kc.iter() {
            let (kind, n) = match e {
                ArchiveEntry::CommunityIdentityPublished(s) => ("CommunityIdentity", hex_len(s)),
                ArchiveEntry::SpeakerDelegated(s) => ("SpeakerDelegation", hex_len(s)),
                ArchiveEntry::MemberRollPublished(s) => ("MemberRoll", hex_len(s)),
                ArchiveEntry::WitnessRegistryUpdated(s) => ("WitnessRegistry", hex_len(s)),
                ArchiveEntry::IntraCommunityRite(s) => ("IntraCommunityRite", hex_len(s)),
                ArchiveEntry::ExaminerRegistryUpdated(s) => ("ExaminerRegistry", hex_len(s)),
                ArchiveEntry::SponsorshipVouched(s) => ("Sponsorship", hex_len(s)),
                ArchiveEntry::InterCommunityWitness(s) => ("InterCommunityWitness", hex_len(s)),
                ArchiveEntry::ConductAttested(s) => ("ConductAttested", hex_len(s)),
                _ => continue,
            };
            println!("  {kind:<24} {n}");
            if !matches!(e, ArchiveEntry::InterCommunityWitness(_)
                            | ArchiveEntry::IntraCommunityRite(_)) {
                max_small = max_small.max(n);
            }
        }
        let claim = &admitted.payload.claims[0];
        let acceptance = &admitted.payload.acceptances[0];
        println!("  {:<24} {}", "AntiSpiralFeatureClaim", hex_len(claim));
        println!("  {:<24} {}", "FeatureAcceptance", hex_len(acceptance));
        let m2 = SealManifest::of(&founding.payload).hex_len();
        let m3 = SealManifest::of(&admitted.payload).hex_len();
        println!("  {:<24} {m2} (2 participants), {m3} (3)", "SealManifest");
        println!("  {:<24} {}", "anathema (2 reporters)", hex_len(&anathema));
        assert!(hex_len(claim) <= 6144 && hex_len(acceptance) <= 6144);
        assert!(max_small <= 6144, "every border record but the rite and seal fits one ask");
        assert!(m3 <= 6144, "a three-participant seal manifest fits one ask");
    }
}
