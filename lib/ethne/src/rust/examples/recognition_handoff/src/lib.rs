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
}
