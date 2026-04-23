//! Two-community kith-covenant ceremony demo.
//!
//! Builds two communities from scratch (identity, speaker delegation, envoy
//! delegation, witness registry), runs each community's intra-rite, runs the
//! inter-community credentialing ceremony with mutual acceptance of all five
//! anti-spiral feature claims, archives every record, verifies the archive,
//! dumps JSON, reloads, and verifies again. Exits 0 on success.

use std::collections::BTreeMap;
use std::process::ExitCode;

use jiff::ToSpan;
use kc_archive::{verify_all, Archive, ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::{
    InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext, REQUIRED_FEATURES,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_core::{keys, time, Alg, DidKey, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, Keystore, SpeakerDelegation, WitnessEntry,
    WitnessRegistry, WitnessRole,
};

struct Community {
    signed_identity: Signed<CommunityIdentity>,
    root_priv: PrivKey,
    keystore: Keystore,
    speaker_did: DidKey,
    envoy_did: DidKey,
    witness_did: DidKey,
    speaker_delegation: Signed<SpeakerDelegation>,
    envoy_delegation: Signed<SpeakerDelegation>,
    witness_registry: Signed<WitnessRegistry>,
}

fn mk_community(
    name: &str,
    movements: [&str; 5],
    language: &str,
    now: Timestamp,
) -> (Community, [String; 5]) {
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
    let signed_identity =
        CommunityIdentity::new_self_signed(name.into(), &root_priv, root_pub, now).unwrap();
    let community_did = signed_identity.payload.did.clone();
    let mut keystore = Keystore::new();
    keystore.insert(&community_did, root_priv.clone());

    let (speaker_priv, speaker_pk) = keys::generate(Alg::Ed25519).unwrap();
    let speaker_did = DidKey::from_pubkey(&speaker_pk).unwrap();
    keystore.insert(&speaker_did, speaker_priv);
    let speaker_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: speaker_did.clone(),
            speaker_pubkey: speaker_pk,
            scope: DelegationScope::Reciter,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let (envoy_priv, envoy_pk) = keys::generate(Alg::Ed25519).unwrap();
    let envoy_did = DidKey::from_pubkey(&envoy_pk).unwrap();
    keystore.insert(&envoy_did, envoy_priv);
    let envoy_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: envoy_did.clone(),
            speaker_pubkey: envoy_pk.clone(),
            scope: DelegationScope::Envoy,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let (witness_priv, witness_pk) = keys::generate(Alg::Ed25519).unwrap();
    let witness_did = DidKey::from_pubkey(&witness_pk).unwrap();
    keystore.insert(&witness_did, witness_priv);

    let witness_registry = Signed::build(
        WitnessRegistry {
            community_did: community_did.clone(),
            entries: vec![
                WitnessEntry {
                    witness_did: witness_did.clone(),
                    witness_pubkey: witness_pk,
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
                WitnessEntry {
                    witness_did: envoy_did.clone(),
                    witness_pubkey: envoy_pk,
                    role: WitnessRole::CrossBoundary,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
            ],
            issued_at: now,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let _ = language;
    let _ = name;
    let movements_owned = movements.map(str::to_string);

    (
        Community {
            signed_identity,
            root_priv,
            keystore,
            speaker_did,
            envoy_did,
            witness_did,
            speaker_delegation,
            envoy_delegation,
            witness_registry,
        },
        movements_owned,
    )
}

fn seed_archive(archive: &mut InMemoryArchive, c: &Community) {
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(
            c.signed_identity.clone(),
        ))
        .unwrap();
    archive
        .append(ArchiveEntry::SpeakerDelegated(c.speaker_delegation.clone()))
        .unwrap();
    archive
        .append(ArchiveEntry::SpeakerDelegated(c.envoy_delegation.clone()))
        .unwrap();
    archive
        .append(ArchiveEntry::WitnessRegistryUpdated(
            c.witness_registry.clone(),
        ))
        .unwrap();
}

fn run_intra_rite(
    c: &Community,
    movements: [String; 5],
    language: &str,
    now: Timestamp,
) -> Signed<IntraCommunityRiteRecord> {
    let delegations = vec![c.speaker_delegation.clone()];
    let runner =
        IntraCommunityRiteRunner::new(&c.signed_identity, &delegations, &c.witness_registry);
    let speaker_priv = c.keystore.get(&c.speaker_did).expect("speaker priv");
    let witness_priv = c.keystore.get(&c.witness_did).expect("witness priv");

    let [sub, fal, rec, vir, ult] = movements;
    let recitation = IntraCommunityRiteRunner::recite(
        &c.speaker_did,
        speaker_priv,
        Recitation {
            community_did: c.signed_identity.payload.did.clone(),
            speaker_did: c.speaker_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: sub,
            movement_fallenness: fal,
            movement_recognition: rec,
            movement_common_virtue: vir,
            movement_non_ultimacy: ult,
            language_tag: language.to_string(),
            translation_en: None,
            recited_at: now,
        },
    )
    .unwrap();
    let attestation = IntraCommunityRiteRunner::attest(
        &c.witness_did,
        witness_priv,
        recitation.payload_cid,
        now,
    )
    .unwrap();
    runner
        .seal(
            recitation,
            vec![attestation],
            &c.root_priv,
            RecitationVersion::current(),
            now,
        )
        .unwrap()
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let now = time::now();

    println!("=== kith-covenant two-community demo ===");
    println!("timestamp: {now}\n");

    // Evocative but non-specific movement text for two fictional communities.
    // Per plan note: avoids claiming representation of any real tradition.
    let (alpha, alpha_movements) = mk_community(
        "Alpha",
        [
            "We anchor above ourselves in the source we did not make.",
            "We know our members and our community are capable of harm.",
            "We recognize Beta as a community bound by its own substrate.",
            "We commit to truthfulness, restitution, and hospitality.",
            "This compact is not ultimate; something stands above it.",
        ],
        "en",
        now,
    );
    let (beta, beta_movements) = mk_community(
        "Beta",
        [
            "We stand beneath what we cannot author.",
            "We hold no illusion that any of us is without fault.",
            "We recognize Alpha as kindred in the work of dwelling together.",
            "We bind ourselves to patience, candor, and repair.",
            "Our agreements do not replace what we stand beneath.",
        ],
        "en",
        now,
    );

    println!("Alpha community:");
    println!("  DID: {}", alpha.signed_identity.payload.did);
    println!("  identity CID: {}", alpha.signed_identity.payload_cid);
    println!("  speaker: {}", alpha.speaker_did);
    println!("  envoy:   {}", alpha.envoy_did);
    println!("  witness: {}", alpha.witness_did);
    println!("Beta community:");
    println!("  DID: {}", beta.signed_identity.payload.did);
    println!("  identity CID: {}", beta.signed_identity.payload_cid);
    println!("  speaker: {}", beta.speaker_did);
    println!("  envoy:   {}", beta.envoy_did);
    println!("  witness: {}", beta.witness_did);
    println!();

    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &alpha);
    seed_archive(&mut archive, &beta);

    let alpha_rite = run_intra_rite(&alpha, alpha_movements, "en", now);
    let beta_rite = run_intra_rite(&beta, beta_movements, "en", now);
    let alpha_rite_cid = archive
        .append(ArchiveEntry::IntraCommunityRite(alpha_rite.clone()))?;
    let beta_rite_cid = archive
        .append(ArchiveEntry::IntraCommunityRite(beta_rite.clone()))?;

    println!("intra-community rites sealed:");
    println!("  Alpha  → archive CID {alpha_rite_cid}");
    println!(
        "          recitation CID {}",
        alpha_rite.payload.recitation.payload_cid
    );
    println!(
        "          internal witnesses: {}",
        alpha_rite.payload.internal_witnesses.len()
    );
    println!("  Beta   → archive CID {beta_rite_cid}");
    println!(
        "          recitation CID {}",
        beta_rite.payload.recitation.payload_cid
    );
    println!(
        "          internal witnesses: {}",
        beta_rite.payload.internal_witnesses.len()
    );
    println!();

    // Inter-community ceremony.
    let participants = vec![
        ParticipantContext {
            community: &alpha.signed_identity,
            intra_record: &alpha_rite,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: alpha
                .keystore
                .get(&alpha.envoy_did)
                .expect("alpha envoy priv"),
        },
        ParticipantContext {
            community: &beta.signed_identity,
            intra_record: &beta_rite,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: beta
                .keystore
                .get(&beta.envoy_did)
                .expect("beta envoy priv"),
        },
    ];
    let inter_runner =
        InterCredentialingRunner::new("alpha-beta-compact".into(), participants);

    let mut claims = Vec::new();
    for (cname, community_did, envoy_did, envoy_priv) in [
        (
            "Alpha",
            alpha.signed_identity.payload.did.clone(),
            &alpha.envoy_did,
            alpha.keystore.get(&alpha.envoy_did).unwrap(),
        ),
        (
            "Beta",
            beta.signed_identity.payload.did.clone(),
            &beta.envoy_did,
            beta.keystore.get(&beta.envoy_did).unwrap(),
        ),
    ] {
        println!("{cname} presents five anti-spiral feature claims:");
        for feature in REQUIRED_FEATURES {
            let claim = InterCredentialingRunner::present_claim(
                envoy_did,
                envoy_priv,
                community_did.clone(),
                feature,
                format!("{feature:?} evidenced by our sealed rite and practice."),
                vec![],
                now,
            )?;
            println!("  {feature:?} → claim CID {}", claim.payload_cid);
            claims.push(claim);
        }
    }
    println!();

    let mut acceptances = Vec::new();
    for claim in &claims {
        let (accepter_name, accepter_did_key, accepter_priv, accepter_community) =
            if claim.payload.community_did == alpha.signed_identity.payload.did {
                (
                    "Beta",
                    &beta.envoy_did,
                    beta.keystore.get(&beta.envoy_did).unwrap(),
                    beta.signed_identity.payload.did.clone(),
                )
            } else {
                (
                    "Alpha",
                    &alpha.envoy_did,
                    alpha.keystore.get(&alpha.envoy_did).unwrap(),
                    alpha.signed_identity.payload.did.clone(),
                )
            };
        let acceptance = InterCredentialingRunner::accept(
            accepter_did_key,
            accepter_priv,
            accepter_community,
            claim.payload_cid,
            true,
            None,
            now,
        )?;
        println!(
            "{accepter_name} accepts {:?} claim by {}",
            claim.payload.feature,
            if claim.payload.community_did == alpha.signed_identity.payload.did {
                "Alpha"
            } else {
                "Beta"
            }
        );
        acceptances.push(acceptance);
    }
    println!();

    let outcome = inter_runner.seal(claims, acceptances, vec![], now)?;
    let witness_record = match outcome {
        InterCredentialingOutcome::Sealed(s) => s,
        InterCredentialingOutcome::NonAdmission(n) => {
            println!("inter-community ceremony produced NON-ADMISSION (unexpected):");
            for r in &n.payload.rejections {
                println!(
                    "  {} rejection of {}'s {:?}: {}",
                    r.rejecting_community, r.presenting_community, r.feature, r.rationale
                );
            }
            return Err("non-admission".into());
        }
    };
    let witness_cid = archive.append(ArchiveEntry::InterCommunityWitness(
        witness_record.clone(),
    ))?;
    println!("inter-community witness record sealed:");
    println!("  compact: {}", witness_record.payload.compact_label);
    println!("  archive CID: {witness_cid}");
    println!(
        "  envelope signatures: {} (one per participant)",
        witness_record.signatures.len()
    );
    println!(
        "  participants: {}",
        witness_record.payload.participants.len()
    );
    println!("  claims: {}", witness_record.payload.claims.len());
    println!(
        "  acceptances: {}",
        witness_record.payload.acceptances.len()
    );
    for sr in &witness_record.payload.sequenced_rites {
        println!(
            "  sequenced rite [{}]: {} by {}",
            sr.sequence_index, sr.intra_record_cid, sr.reciting_community
        );
    }
    println!();

    println!("verifying archive ({} entries) …", archive.len());
    verify_all(&archive)?;
    println!("  all entries verify ✓");
    println!();

    println!("roundtripping archive through JSON …");
    let json = archive.dump_json_to_string()?;
    println!("  dumped {} bytes of JSON", json.len());
    let reloaded = InMemoryArchive::load_json_from_str(&json)?;
    assert_eq!(reloaded.len(), archive.len());
    verify_all(&reloaded)?;
    println!("  reloaded archive has {} entries", reloaded.len());
    println!("  all entries verify after reload ✓");
    println!();

    let mut by_kind: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (cid, entry) in archive.iter() {
        by_kind
            .entry(entry.kind())
            .or_default()
            .push(cid.to_string());
    }
    println!("archive audit trail:");
    for (kind, cids) in &by_kind {
        println!("  {kind}: {} entries", cids.len());
        for c in cids {
            println!("    {c}");
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    match run() {
        Ok(()) => {
            println!("\n✓ two_communities_demo: SUCCESS");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("\n✗ two_communities_demo: FAILED: {e}");
            ExitCode::FAILURE
        }
    }
}
