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

//! `en-uplift` — the **upward interface** from the ethne polity tier to
//! kith-covenant (design decision **D4**).
//!
//! A chartered polity is a self-contained thing; kith-covenant needs nothing from
//! ethne to run (KC ⟂ ethne). But when a polity *wants* to be recognized as a KC
//! party, four things must cross the boundary, all in KC's existing vocabulary:
//!
//! 1. **Handoff** — the polity presents itself as a [`CommunityIdentity`]
//!    (the neutral primitive shared through `kc-core`), self-signed by its root
//!    key. See [`community_identity`].
//! 2. **Concern** — soft coherence erosion, graded by the
//!    [`en_cohesion`](../en_cohesion/index.html) monitor, surfaces as a KC
//!    `Concern` conduct record. See [`coherence_concern`].
//! 3. **Fission** — a hard split appears, in KC's eyes, as two new communities
//!    plus the predecessor going quiet: two successor identities, two signed
//!    successor links, and an ethne-side charter dissolution. See [`fission`].
//! 4. **Envoy** — whoever the polity has authorized to speak for it becomes a KC
//!    `SpeakerDelegation` scoped `Envoy`. This is the one border artifact an
//!    *office* produces (design **D14**): the authorization is granted inside the
//!    polity by its `EnvoyAuthorizer` or its roll, and only an authorization still
//!    in force can cross. See [`envoy_delegation`].
//! 5. **Member roll** — the polity's membership becomes a KC `MemberRoll`, which is
//!    what makes KC's §8 anonymous whistleblower channel real. KC models communities
//!    and their delegated speakers, so before this an insider holding no delegation
//!    was not in the ring at all and a polity that had delegated nobody had no
//!    channel. This tier is the thing that *does* model membership, so it is the
//!    feed. See [`member_roll`].
//!
//! # Why this crate exists (the confined coupling)
//!
//! This is the **one** ethne crate permitted to depend on kith-covenant *record*
//! crates (`kc-conduct`, and `kc-identity` for the envoy delegation). Everything
//! below the uplift boundary — the core, charter, substrate, at-adapter, and
//! cohesion crates — stays `kc-core`-only. The Concern deliberately uses `ConductBearing::Other(
//! "coherence-erosion")` rather than the covenant-specific `AntiSpiral` bearing,
//! so the neutral vocabulary is preserved and no covenant semantics leak down.
//!
//! # Emission, not appending
//!
//! ethne must not depend on `kc-archive` (it drags the whole KC graph in). So
//! this crate *emits* serialized `Signed<T>` payloads — CBOR-roundtrippable — and
//! a KC-side consumer wraps each into an `ArchiveEntry` and appends it. Nothing
//! here touches an archive.

use en_charter::{CharterDissolution, EnvoyState, PolityCharter};
use en_cohesion::{CohesionReport, ErosionLevel};
use en_core::{DidKey, KcError, KcResult, PrivKey, PubKey, Signed, Timestamp};
use kc_conduct::{ConductBearing, ConductKind, ConductRecord, DecisionClass, Severity};
use kc_core::{CommunityIdentity, SuccessorLink};
use kc_identity::{DelegationScope, MemberRoll, RollMember, SpeakerDelegation};

/// The handoff: turn a ratified [`PolityCharter`] into a self-signed
/// [`CommunityIdentity`] KC can recognize. The polity's root keypair is supplied
/// by the caller (the polity's authorized key holder); the identity is self-
/// certifying — its DID is derived from `root_pub`, and it is signed by
/// `root_priv`.
///
/// Errors if the charter's polity DID does not match the supplied root key, which
/// would mean handing off an identity for a different polity than the one
/// chartered.
pub fn community_identity(
    name: String,
    charter: &PolityCharter,
    root_priv: &PrivKey,
    root_pub: PubKey,
    created_at: Timestamp,
) -> KcResult<Signed<CommunityIdentity>> {
    let derived = DidKey::from_pubkey(&root_pub)?;
    if charter.polity != derived {
        return Err(KcError::InvalidDid(format!(
            "charter polity {} does not match the supplied root key (DID {derived})",
            charter.polity
        )));
    }
    CommunityIdentity::new_self_signed(name, root_priv, root_pub, created_at)
}

/// The envoy: turn an envoy standing the polity granted internally into KC's
/// `SpeakerDelegation`, scoped [`DelegationScope::Envoy`] and signed by the
/// polity's root key — the artifact that lets someone speak for the community in
/// KC's ceremonies.
///
/// This is where an office's authority reaches the border (design **D14**). The
/// authorization itself is an ethne-side act, granted by the `EnvoyAuthorizer` on
/// their own signature or by a majority of the roll; `envoys` is the derived
/// [`EnvoyState`] — the **read-model, not the archive**, keeping this crate free of
/// `en-archive` walks — from `en_charter::envoy_state`. An envoy whose standing has
/// lapsed (off the roll, resigned, revoked, or expired) is absent from that state,
/// so no delegation can be minted for them.
///
/// Errors if the charter's polity DID does not match the supplied root key (the
/// same guard as [`community_identity`]), or if `envoy` holds no standing in
/// `envoys`.
pub fn envoy_delegation(
    charter: &PolityCharter,
    envoys: &EnvoyState,
    envoy: &DidKey,
    root_priv: &PrivKey,
    root_pub: PubKey,
    issued_at: Timestamp,
) -> KcResult<Signed<SpeakerDelegation>> {
    let derived = DidKey::from_pubkey(&root_pub)?;
    if charter.polity != derived {
        return Err(KcError::InvalidDid(format!(
            "charter polity {} does not match the supplied root key (DID {derived})",
            charter.polity
        )));
    }
    let standing = envoys.standing(envoy).ok_or_else(|| {
        KcError::Authority(format!(
            "{envoy} holds no envoy standing for {}",
            charter.polity
        ))
    })?;
    let payload = SpeakerDelegation {
        community_did: charter.polity.clone(),
        speaker_did: standing.envoy.clone(),
        speaker_pubkey: standing.envoy_pubkey.clone(),
        scope: DelegationScope::Envoy,
        issued_at,
        // The polity's own expiry travels with the delegation, so KC's
        // `active_at` and ethne's `envoy_state` cannot disagree about when the
        // standing ends.
        expires_at: standing.expires_at,
        revoked_at: None,
    };
    Signed::build(payload, &[(charter.polity.as_str(), root_priv)])
}

/// The member roll: turn the polity's current membership into a KC [`MemberRoll`],
/// signed by the polity root.
///
/// `members` is the roll as `en_charter::charter_state` computes it — the founding
/// roll plus admissions minus departures. Every ethne member is a `did:key`, so each
/// member's verifying key comes straight out of its own DID; nothing else needs to
/// travel, and a roll cannot name a member against a key that is not theirs.
///
/// This is what gives a polity a real anonymous channel inside KC. Two consequences
/// of KC's side are worth knowing before publishing one:
///
/// - **The roll only ever widens KC's insider ring.** KC unions rolls rather than
///   superseding them, so publishing a later, shorter roll removes nobody. Departures
///   recorded here do not close the channel of whoever left. That is deliberate on
///   KC's side — the alternative would let an accused polity shrink the suspect set
///   around a member the day after they resigned.
/// - **A roll is the polity speaking about itself.** KC checks that the publisher is
///   the community named, and nothing more; it cannot audit this tier's charter. The
///   `as_of` instant is carried as provenance for exactly that reason.
///
/// Errors if the charter's polity DID does not match the supplied root key (the same
/// guard as [`community_identity`]), if `members` is empty (KC refuses an empty
/// roll), or if any member's DID does not resolve to a key.
pub fn member_roll(
    charter: &PolityCharter,
    members: &[DidKey],
    root_priv: &PrivKey,
    root_pub: PubKey,
    as_of: Timestamp,
    published_at: Timestamp,
) -> KcResult<Signed<MemberRoll>> {
    let derived = DidKey::from_pubkey(&root_pub)?;
    if charter.polity != derived {
        return Err(KcError::InvalidDid(format!(
            "charter polity {} does not match the supplied root key (DID {derived})",
            charter.polity
        )));
    }
    if members.is_empty() {
        return Err(KcError::Authority(format!(
            "polity {} has an empty roll, and an empty member roll is not a roll",
            charter.polity
        )));
    }
    let mut out = Vec::with_capacity(members.len());
    for m in members {
        // The polity itself is a member of no roll: its root key is already an insider
        // in KC's eyes, and a second entry would be a repeated ring position.
        if m == &charter.polity {
            continue;
        }
        out.push(RollMember { member_did: m.clone(), member_pubkey: m.to_pubkey()? });
    }
    if out.is_empty() {
        return Err(KcError::Authority(format!(
            "polity {} has no members besides itself",
            charter.polity
        )));
    }
    let payload =
        MemberRoll { community_did: charter.polity.clone(), members: out, as_of, published_at };
    Signed::build(payload, &[(charter.polity.as_str(), root_priv)])
}

/// The Concern: if — and only if — the cohesion `report` has reached
/// [`ErosionLevel::Concern`], emit a signed KC `Concern` conduct record about the
/// polity. Below that level the polity's strain is ethne's own business and
/// nothing crosses the boundary (`Ok(None)`).
///
/// The record is filed by `observer` (signing with `observer_priv`) with an
/// explicit `reporter_perspective` so KC's analytics discount it as a partial,
/// monitor-originated signal rather than a neutral finding. It is a `Concern`
/// (pre-indictment, no ladder rung), `Minor`, `FirstClass`, bearing the neutral
/// `Other("coherence-erosion")`.
pub fn coherence_concern(
    report: &CohesionReport,
    subject_community: DidKey,
    observer: DidKey,
    observer_priv: &PrivKey,
    at: Timestamp,
) -> KcResult<Option<Signed<ConductRecord>>> {
    if report.erosion != ErosionLevel::Concern {
        return Ok(None);
    }
    // Reputation, which since D18 is reported over the members the substrate could
    // actually RATE. With nothing rated there is no mean to state — printing "mean
    // reputation 0.00" would tell KC the polity is failing when the truth is that
    // nobody rated it — so the clause says that instead, and names the unrated
    // count whenever there is one.
    let unrated = report.observed.saturating_sub(report.rated_members);
    let reputation_clause = if report.rated_members == 0 {
        if report.observed == 0 {
            String::new()
        } else {
            format!(
                ", no reputation reading for any of the {} observed member(s)",
                report.observed
            )
        }
    } else {
        format!(
            ", mean reputation {:.2} over {} rated member(s), {} reputation-floor \
             breach(es){}",
            report.mean_reputation,
            report.rated_members,
            report.reputation_floor_breaches,
            if unrated == 0 {
                String::new()
            } else {
                format!(" ({unrated} unrated)")
            },
        )
    };
    // Every field of the report is rendered here — this narrative is all KC ever
    // sees of it, so a measure omitted here is a measure KC cannot weigh.
    let narrative = format!(
        "ethne cohesion monitor: coherence erosion at Concern — attrition {:.0}% \
         ({} of {} chartered members still observed){}{}{}{}",
        report.attrition * 100.0,
        report.observed,
        report.chartered,
        reputation_clause,
        // Attendance never carries a Concern on its own (it is capped at Notice),
        // but it colours one: whether the humans answerable for these machines
        // were present is part of what the strain means.
        if report.guardian_members == 0 {
            String::new()
        } else {
            format!(
                ", {} of {} guardian(s) not present",
                report.unattended, report.guardian_members
            )
        },
        // Transmission decay, which CAN be the reason this Concern exists at all —
        // so unlike attendance it is reported whenever there was a reading to take.
        if report.affirming_members == 0 {
            String::new()
        } else {
            format!(
                ", {} of {} member(s) lapsed on re-affirming the substrate",
                report.stale_affirmations, report.affirming_members
            )
        },
        if report.opaque_boundary {
            "; boundary partly opaque"
        } else {
            ""
        },
    );
    let record = ConductRecord {
        subject_community,
        reporters: vec![observer.clone()],
        reporter_perspective: Some("ethne cohesion monitor".into()),
        kind: ConductKind::Concern,
        bearing: ConductBearing::Other("coherence-erosion".into()),
        severity: Severity::Minor,
        ladder_rung: None,
        decision_class: DecisionClass::FirstClass,
        references: Vec::new(),
        // The KC recited object is a compact-level artifact and a cohesion Concern is
        // not about one of its commitments: this is the polity's own instrument
        // reporting on the polity, not the compact enforcing part of what it recited.
        commitments_cited: Vec::new(),
        narrative,
        observed_at: at,
        recorded_at: at,
    };
    let signed = Signed::build(record, &[(observer.as_str(), observer_priv)])?;
    Ok(Some(signed))
}

/// The bundle a fission emits: the successor identities, a signed
/// [`SuccessorLink`] from the predecessor to each — **index-aligned** with
/// `successors` — and the predecessor's [`CharterDissolution`] naming them all. A
/// KC-side consumer appends the identities and links to its recognition log; the
/// dissolution is appended to the ethne charter log.
///
/// **N-ary since design D25**, so this is the exact mirror of [`FusionOutcome`]:
/// several successors out of one predecessor, where fusion takes one out of
/// several. A three-way split needed two chained fissions before, which recorded a
/// history nobody lived — an intermediate polity that existed only to be dissolved.
#[derive(Debug, Clone)]
pub struct FissionOutcome {
    pub successors: Vec<Signed<CommunityIdentity>>,
    /// One per successor, in the same order.
    pub links: Vec<Signed<SuccessorLink>>,
    pub dissolution: Signed<CharterDissolution>,
}

/// The fission: a **ratified** split into N (≥ 2) successors (the monitor never
/// decides this — see [`en_cohesion`](../en_cohesion/index.html)). The community has
/// already ratified the successor foundings; this emits, from the predecessor root's
/// authority, a successor link to each and the predecessor's dissolution naming them
/// all. `deciders` records who ratified the split; the emitted records are signed by
/// the predecessor root — the polity's authorized key holder speaking for the
/// ratified decision.
///
/// The guards are [`fusion`]'s, in the other direction (design **D25**). Fission had
/// none, which is worth stating plainly because the omission was not benign: before
/// this it would emit a "split" naming **one** successor twice, or naming the
/// predecessor **itself** as its own successor, or — worst — sign a `SuccessorLink`
/// under the predecessor's root to an identity whose own self-signature does not
/// verify. That last one had the predecessor's key vouching for a forgery.
///
/// What none of this can do is tell a legitimate split from a faction dividing the
/// spoils. That judgement is not the tier's, and is not meant to be: the split is a
/// ratified act, and this records it.
///
/// Fusion (several polities merging into one) is the mirror image — see [`fusion`].
pub fn fission(
    predecessor: &PolityCharter,
    successors: Vec<Signed<CommunityIdentity>>,
    deciders: Vec<DidKey>,
    predecessor_root_priv: &PrivKey,
    rationale: String,
    at: Timestamp,
) -> KcResult<FissionOutcome> {
    if successors.len() < 2 {
        return Err(KcError::Authority(
            "fission needs at least two successors".into(),
        ));
    }
    let predecessor_did = &predecessor.polity;
    let mut seen = std::collections::BTreeSet::new();
    for succ in &successors {
        let did = &succ.payload.did;
        if !seen.insert(did.as_str()) {
            return Err(KcError::Authority(format!(
                "successor {did} appears twice in the fission"
            )));
        }
        if did == predecessor_did {
            return Err(KcError::InvalidDid(format!(
                "successor {did} is the predecessor itself; a polity cannot succeed itself"
            )));
        }
        // Fusion verifies the merged identity; the mirror is verifying every
        // successor. Without it the predecessor root signs a link to whatever it is
        // handed.
        CommunityIdentity::verify_self_signed(succ)?;
    }

    let root_did = predecessor.polity.as_str();
    let mut links = Vec::with_capacity(successors.len());
    for succ in &successors {
        let payload = SuccessorLink {
            previous_did: predecessor.polity.clone(),
            new_did: succ.payload.did.clone(),
            new_pubkey: succ.payload.root_pubkey.clone(),
            rotated_at: at,
        };
        links.push(Signed::build(payload, &[(root_did, predecessor_root_priv)])?);
    }

    let dissolution_payload = CharterDissolution {
        polity: predecessor.polity.clone(),
        successors: successors.iter().map(|s| s.payload.did.clone()).collect(),
        rationale,
        deciders,
        dissolved_at: at,
    };
    let dissolution = Signed::build(dissolution_payload, &[(root_did, predecessor_root_priv)])?;

    Ok(FissionOutcome { successors, links, dissolution })
}

/// One party to a fusion: a predecessor polity's charter, the root key that
/// speaks for it, and the members who decided the merger (recorded in that
/// predecessor's dissolution).
pub struct FusionPredecessor<'a> {
    pub charter: &'a PolityCharter,
    /// The predecessor's root private key — signs its successor link and its
    /// dissolution. Must correspond to `charter.polity` (the caller's
    /// responsibility, as with [`fission`]).
    pub root_priv: &'a PrivKey,
    pub deciders: Vec<DidKey>,
}

/// The bundle a fusion emits: the merged polity's identity, plus — one per
/// predecessor, index-aligned with the input slice — a [`SuccessorLink`] from
/// that predecessor to the merged polity and that predecessor's
/// [`CharterDissolution`] naming the merged polity as its sole successor. A
/// KC-side consumer appends the identity + links to its recognition log and each
/// dissolution to the corresponding predecessor's charter log.
#[derive(Debug, Clone)]
pub struct FusionOutcome {
    pub fused: Signed<CommunityIdentity>,
    pub links: Vec<Signed<SuccessorLink>>,
    pub dissolutions: Vec<Signed<CharterDissolution>>,
}

/// The fusion: a **ratified** merger (the mirror of [`fission`]). The
/// communities have already chartered the merged polity and handed it off as
/// `fused`; this emits, from each predecessor root's authority, a successor link
/// to the merged polity and that predecessor's dissolution naming it. KC sees the
/// predecessors go quiet and one new community seek recognition — no new KC
/// vocabulary.
///
/// Errors if there are fewer than two predecessors, if two predecessors share a
/// DID, if the merged polity's DID collides with a predecessor's (the merged
/// polity must be genuinely new), or if `fused` is not a valid self-signed
/// identity.
///
/// As with [`fission`], the emitted records are signed by each predecessor's root
/// — the polity's authorized key holder speaking for the ratified merger. To also
/// append a dissolution to the ethne charter log and pass its authority pass, the
/// deciders must co-sign through the charter-authoring path; that boundary is
/// unchanged here.
pub fn fusion(
    predecessors: &[FusionPredecessor],
    fused: Signed<CommunityIdentity>,
    rationale: String,
    at: Timestamp,
) -> KcResult<FusionOutcome> {
    if predecessors.len() < 2 {
        return Err(KcError::Authority(
            "fusion needs at least two predecessors".into(),
        ));
    }
    let fused_did = &fused.payload.did;
    let mut seen = std::collections::BTreeSet::new();
    for p in predecessors {
        let did = &p.charter.polity;
        if !seen.insert(did.as_str()) {
            return Err(KcError::Authority(format!(
                "predecessor {did} appears twice in the fusion"
            )));
        }
        if did == fused_did {
            return Err(KcError::InvalidDid(format!(
                "merged polity {fused_did} collides with predecessor {did}; the merged polity must be new"
            )));
        }
    }
    CommunityIdentity::verify_self_signed(&fused)?;

    let mut links = Vec::with_capacity(predecessors.len());
    let mut dissolutions = Vec::with_capacity(predecessors.len());
    for p in predecessors {
        let root_did = p.charter.polity.as_str();
        let link = SuccessorLink {
            previous_did: p.charter.polity.clone(),
            new_did: fused_did.clone(),
            new_pubkey: fused.payload.root_pubkey.clone(),
            rotated_at: at,
        };
        links.push(Signed::build(link, &[(root_did, p.root_priv)])?);

        let dissolution = CharterDissolution {
            polity: p.charter.polity.clone(),
            successors: vec![fused_did.clone()],
            rationale: rationale.clone(),
            deciders: p.deciders.clone(),
            dissolved_at: at,
        };
        dissolutions.push(Signed::build(dissolution, &[(root_did, p.root_priv)])?);
    }

    Ok(FusionOutcome {
        fused,
        links,
        dissolutions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::{FoundingProposal, Substrate};
    use en_charter::{AffirmationState, EnvoyHolding, EnvoyState};
    use en_cohesion::CohesionInputs;
    use en_cohesion::{cohesion_report, Thresholds};
    use en_core::{keys, time, Alg, PrivKey, PubKey};
    use en_substrate::{FoundingCandidate, MemberCandidate};

    fn keypair() -> (PrivKey, PubKey) {
        keys::generate(Alg::Ed25519).unwrap()
    }

    fn substrate() -> Substrate {
        Substrate { name: "tradition".into(), statement: "answers to X".into(), anchor: None }
    }

    /// A chartered polity plus its root keypair (the root DID is the polity DID).
    fn chartered_polity() -> (PolityCharter, PrivKey, PubKey) {
        let (root_sk, root_pk) = keypair();
        let polity = DidKey::from_pubkey(&root_pk).unwrap();
        // Two members ratify (the roll and ratifiers are separate identities;
        // the polity DID is the root, distinct from members).
        let members: Vec<(PrivKey, DidKey)> = (0..3)
            .map(|_| {
                let (sk, pk) = keypair();
                (sk, DidKey::from_pubkey(&pk).unwrap())
            })
            .collect();
        let roll: Vec<DidKey> = members.iter().map(|(_, d)| d.clone()).collect();
        let prop = FoundingProposal::convener_asserted(
            polity.clone(),
            substrate(),
            vec![],
            roll.clone(),
            vec![],
            time::now(),
        );
        let ratifiers = vec![roll[0].clone(), roll[1].clone()];
        let charter = prop.ratify(ratifiers, time::now()).unwrap();
        (charter, root_sk, root_pk)
    }

    fn candidate(members: &[(DidKey, f64)]) -> FoundingCandidate {
        FoundingCandidate {
            members: members
                .iter()
                .map(|(d, r)| MemberCandidate {
                    did: d.clone(),
                    reputation: Some(*r),
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
        }
    }

    #[test]
    fn handoff_produces_verifiable_self_signed_identity() {
        let (charter, root_sk, root_pk) = chartered_polity();
        let ci = community_identity(
            "Alpha".into(),
            &charter,
            &root_sk,
            root_pk.clone(),
            time::now(),
        )
        .unwrap();
        CommunityIdentity::verify_self_signed(&ci).expect("self-signed identity verifies");
        assert_eq!(ci.payload.did, charter.polity);
        assert_eq!(ci.payload.name, "Alpha");
    }

    #[test]
    fn handoff_rejects_mismatched_root_key() {
        let (charter, _root_sk, _root_pk) = chartered_polity();
        // A different root key than the one the charter's polity DID derives from.
        let (other_sk, other_pk) = keypair();
        let err = community_identity("Alpha".into(), &charter, &other_sk, other_pk, time::now())
            .unwrap_err();
        assert!(matches!(err, KcError::InvalidDid(_)));
    }

    #[test]
    fn the_narrative_carries_transmission_decay() {
        // The narrative is ALL that crosses to KC, so a component missing from it
        // is a component KC cannot weigh. Here transmission decay is the sole
        // cause of the Concern — a fully-present, well-reputed roll that has
        // simply stopped re-affirming the substrate.
        let (charter, _, _) = chartered_polity();
        let (obs_sk, obs_pk) = keypair();
        let observer = DidKey::from_pubkey(&obs_pk).unwrap();

        let roll: Vec<DidKey> = (0..4)
            .map(|_| DidKey::from_pubkey(&keypair().1).unwrap())
            .collect();
        let present: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.95)).collect();
        let long_ago = Timestamp::from_second(1_000_000).unwrap();
        let now = Timestamp::from_second(1_000_000 + 400 * 24 * 60 * 60).unwrap();
        let affirmations = AffirmationState {
            entries: roll
                .iter()
                .map(|m| en_charter::MemberAffirmation {
                    member: m.clone(),
                    inducted: true,
                    last_affirmed_at: Some(long_ago),
                })
                .collect(),
        };

        let report = cohesion_report(
            &roll,
            &candidate(&present),
            false, &CohesionInputs { affirmations: Some(&affirmations), ..Default::default() },
            now,
            &Thresholds::default(),
        );
        assert_eq!(report.erosion, ErosionLevel::Concern);
        assert_eq!(report.attrition, 0.0); // nothing else is wrong
        assert_eq!(report.stale_affirmations, 4);

        let rec = coherence_concern(&report, charter.polity.clone(), observer, &obs_sk, now)
            .unwrap()
            .expect("Concern-level erosion emits a record");
        assert!(rec
            .payload
            .narrative
            .contains("4 of 4 member(s) lapsed on re-affirming the substrate"));
    }

    #[test]
    fn the_narrative_never_tells_kc_a_polity_is_failing_when_nobody_rated_it() {
        // D18. The narrative is ALL KC ever sees of the report, so rendering "mean
        // reputation 0.00" over zero rated members would hand KC a failing polity
        // when the truth is that the substrate could not rate one. A Concern that
        // arises for some OTHER reason (attrition here) must still describe the
        // reputation state honestly.
        let (charter, _, _) = chartered_polity();
        let (obs_sk, obs_pk) = keypair();
        let observer = DidKey::from_pubkey(&obs_pk).unwrap();

        let roll: Vec<DidKey> = (0..5)
            .map(|_| DidKey::from_pubkey(&keypair().1).unwrap())
            .collect();
        // Two of five still observed (Concern from attrition), and neither is rated.
        let unrated = FoundingCandidate {
            members: roll
                .iter()
                .take(2)
                .map(|d| MemberCandidate {
                    did: d.clone(),
                    reputation: None,
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
        };
        let report = cohesion_report(
            &roll,
            &unrated,
            false,
            &CohesionInputs::default(),
            time::now(),
            &Thresholds::default(),
        );
        assert_eq!(report.erosion, ErosionLevel::Concern, "attrition carries it");
        assert_eq!(report.rated_members, 0);

        let rec = coherence_concern(&report, charter.polity.clone(), observer, &obs_sk, time::now())
            .unwrap()
            .expect("Concern-level erosion emits a record");
        let narrative = &rec.payload.narrative;
        assert!(
            narrative.contains("no reputation reading for any of the 2 observed member(s)"),
            "got: {narrative}"
        );
        assert!(
            !narrative.contains("mean reputation"),
            "must not state a mean it does not have: {narrative}"
        );
    }

    #[test]
    fn the_narrative_names_the_unrated_share_alongside_a_real_mean() {
        // The partly-rated case: a mean over the rated members, and the unrated
        // count stated so KC can see what the mean does and does not cover.
        let (charter, _, _) = chartered_polity();
        let (obs_sk, obs_pk) = keypair();
        let observer = DidKey::from_pubkey(&obs_pk).unwrap();

        let roll: Vec<DidKey> = (0..5)
            .map(|_| DidKey::from_pubkey(&keypair().1).unwrap())
            .collect();
        let mixed = FoundingCandidate {
            members: roll
                .iter()
                .take(2)
                .enumerate()
                .map(|(i, d)| MemberCandidate {
                    did: d.clone(),
                    reputation: if i == 0 { Some(0.9) } else { None },
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
        };
        let report = cohesion_report(
            &roll,
            &mixed,
            false,
            &CohesionInputs::default(),
            time::now(),
            &Thresholds::default(),
        );
        assert_eq!(report.rated_members, 1);

        let rec = coherence_concern(&report, charter.polity.clone(), observer, &obs_sk, time::now())
            .unwrap()
            .expect("Concern-level erosion emits a record");
        let narrative = &rec.payload.narrative;
        assert!(narrative.contains("mean reputation 0.90 over 1 rated member(s)"), "got: {narrative}");
        assert!(narrative.contains("(1 unrated)"), "got: {narrative}");
    }

    #[test]
    fn concern_emitted_only_at_concern_level() {
        let (charter, _, _) = chartered_polity();
        let (obs_sk, obs_pk) = keypair();
        let observer = DidKey::from_pubkey(&obs_pk).unwrap();

        // A heavily-attritioned roll → erosion Concern → a record.
        let roll: Vec<DidKey> = (0..5)
            .map(|_| DidKey::from_pubkey(&keypair().1).unwrap())
            .collect();
        let present: Vec<(DidKey, f64)> =
            roll.iter().take(2).map(|d| (d.clone(), 0.9)).collect();
        let report = cohesion_report(&roll, &candidate(&present), false, &CohesionInputs::default(), time::now(), &Thresholds::default());
        assert_eq!(report.erosion, ErosionLevel::Concern);

        let rec = coherence_concern(&report, charter.polity.clone(), observer.clone(), &obs_sk, time::now())
            .unwrap()
            .expect("Concern-level erosion emits a record");
        // Verifies under the observer's did:key.
        rec.verify(|did: &str| {
            if did == observer.as_str() {
                Some(obs_pk.clone())
            } else {
                None
            }
        })
        .expect("conduct record verifies");
        assert_eq!(rec.payload.subject_community, charter.polity);
        assert!(rec.payload.kind.is_adverse());
        assert_eq!(rec.payload.kind, ConductKind::Concern);
        assert!(matches!(rec.payload.bearing, ConductBearing::Other(ref s) if s == "coherence-erosion"));
    }

    #[test]
    fn no_concern_below_concern_level() {
        let (charter, _, _) = chartered_polity();
        let (obs_sk, obs_pk) = keypair();
        let observer = DidKey::from_pubkey(&obs_pk).unwrap();

        // A healthy roll → erosion None → no record.
        let roll: Vec<DidKey> = (0..5)
            .map(|_| DidKey::from_pubkey(&keypair().1).unwrap())
            .collect();
        let present: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.95)).collect();
        let report = cohesion_report(&roll, &candidate(&present), false, &CohesionInputs::default(), time::now(), &Thresholds::default());
        assert_eq!(report.erosion, ErosionLevel::None);

        let out =
            coherence_concern(&report, charter.polity.clone(), observer, &obs_sk, time::now()).unwrap();
        assert!(out.is_none());
    }

    /// A self-signed successor identity, plus the key that signs for it.
    fn successor(name: &str) -> Signed<CommunityIdentity> {
        let (sk, pk) = keypair();
        CommunityIdentity::new_self_signed(name.into(), &sk, pk, time::now()).unwrap()
    }

    #[test]
    fn fission_emits_a_link_per_successor_and_a_dissolution() {
        let (predecessor, root_sk, _root_pk) = chartered_polity();
        let succ_a = successor("Alpha-1");
        let succ_b = successor("Alpha-2");
        let deciders = predecessor.members.clone();

        let outcome = fission(
            &predecessor,
            vec![succ_a.clone(), succ_b.clone()],
            deciders,
            &root_sk,
            "irreconcilable substrate reading".into(),
            time::now(),
        )
        .unwrap();

        // Every successor verifies as a self-signed identity.
        for succ in &outcome.successors {
            CommunityIdentity::verify_self_signed(succ).unwrap();
        }

        // Every link points back to the predecessor and forward to its successor, in
        // order, and verifies under the predecessor root's did:key.
        let resolver = |did: &str| {
            if did == predecessor.polity.as_str() {
                predecessor.polity.to_pubkey().ok()
            } else {
                None
            }
        };
        assert_eq!(outcome.links.len(), outcome.successors.len());
        for (link, succ) in outcome.links.iter().zip(&outcome.successors) {
            assert_eq!(link.payload.previous_did, predecessor.polity);
            assert_eq!(link.payload.new_did, succ.payload.did);
            link.verify(resolver).expect("successor link verifies");
        }

        // The dissolution names every successor, in order, and verifies.
        assert_eq!(outcome.dissolution.payload.polity, predecessor.polity);
        assert_eq!(
            outcome.dissolution.payload.successors,
            vec![succ_a.payload.did.clone(), succ_b.payload.did.clone()]
        );
        outcome.dissolution.verify(resolver).expect("dissolution verifies");
    }

    #[test]
    fn a_three_way_split_is_one_act() {
        // **The D25 point.** Before this, a three-way split had to be two chained
        // fissions, which recorded a history nobody lived: an intermediate polity that
        // existed only to be dissolved again.
        let (predecessor, root_sk, _root_pk) = chartered_polity();
        let successors = vec![successor("A"), successor("B"), successor("C")];
        let dids: Vec<DidKey> = successors.iter().map(|s| s.payload.did.clone()).collect();

        let outcome = fission(
            &predecessor,
            successors,
            predecessor.members.clone(),
            &root_sk,
            "three ways".into(),
            time::now(),
        )
        .unwrap();
        assert_eq!(outcome.successors.len(), 3);
        assert_eq!(outcome.links.len(), 3);
        assert_eq!(outcome.dissolution.payload.successors, dids);
    }

    #[test]
    fn a_split_into_one_is_not_a_split() {
        let (predecessor, root_sk, _root_pk) = chartered_polity();
        let err = fission(
            &predecessor,
            vec![successor("Only")],
            predecessor.members.clone(),
            &root_sk,
            "r".into(),
            time::now(),
        )
        .unwrap_err();
        assert!(matches!(err, KcError::Authority(ref m) if m.contains("at least two")), "{err:?}");
    }

    #[test]
    fn one_successor_named_twice_is_refused() {
        // Measured before the slice: this produced a "split" whose dissolution named
        // one polity twice, with two links to the same successor.
        let (predecessor, root_sk, _root_pk) = chartered_polity();
        let twice = successor("Alpha-1");
        let err = fission(
            &predecessor,
            vec![twice.clone(), twice],
            predecessor.members.clone(),
            &root_sk,
            "r".into(),
            time::now(),
        )
        .unwrap_err();
        assert!(matches!(err, KcError::Authority(ref m) if m.contains("appears twice")), "{err:?}");
    }

    #[test]
    fn a_polity_cannot_succeed_itself() {
        // Measured before the slice: the polity dissolved into ITSELF, its own DID
        // named as a successor of its own winding-up.
        let (predecessor, root_sk, root_pk) = chartered_polity();
        let itself =
            CommunityIdentity::new_self_signed("Self".into(), &root_sk, root_pk, time::now())
                .unwrap();
        let err = fission(
            &predecessor,
            vec![itself, successor("Other")],
            predecessor.members.clone(),
            &root_sk,
            "r".into(),
            time::now(),
        )
        .unwrap_err();
        assert!(
            matches!(err, KcError::InvalidDid(ref m) if m.contains("cannot succeed itself")),
            "{err:?}"
        );
    }

    #[test]
    fn a_successor_whose_signature_does_not_verify_is_refused() {
        // **The worst of the three, measured.** `verify_self_signed` returns false for
        // this identity and fission signed a link to it anyway — the predecessor's root
        // key vouching for a forgery.
        let (predecessor, root_sk, _root_pk) = chartered_polity();
        let mut forged = successor("Forged");
        forged.payload.name = "Tampered".into();
        assert!(CommunityIdentity::verify_self_signed(&forged).is_err());

        let err = fission(
            &predecessor,
            vec![forged, successor("Honest")],
            predecessor.members.clone(),
            &root_sk,
            "r".into(),
            time::now(),
        )
        .unwrap_err();
        // Whatever the crypto layer calls it, the point is that it does not pass.
        assert!(!matches!(err, KcError::Authority(_)), "{err:?}");
    }

    #[test]
    fn fusion_emits_a_link_and_dissolution_per_predecessor() {
        // Three predecessors merge into one new polity (exercises the N-ary shape).
        let (charter_a, root_a, _) = chartered_polity();
        let (charter_b, root_b, _) = chartered_polity();
        let (charter_c, root_c, _) = chartered_polity();

        let (fused_sk, fused_pk) = keypair();
        let fused =
            CommunityIdentity::new_self_signed("Merged".into(), &fused_sk, fused_pk, time::now()).unwrap();

        let preds = [
            FusionPredecessor { charter: &charter_a, root_priv: &root_a, deciders: charter_a.members.clone() },
            FusionPredecessor { charter: &charter_b, root_priv: &root_b, deciders: charter_b.members.clone() },
            FusionPredecessor { charter: &charter_c, root_priv: &root_c, deciders: charter_c.members.clone() },
        ];
        let outcome = fusion(&preds, fused.clone(), "long-planned union".into(), time::now()).unwrap();

        CommunityIdentity::verify_self_signed(&outcome.fused).unwrap();
        assert_eq!(outcome.links.len(), 3);
        assert_eq!(outcome.dissolutions.len(), 3);

        for (i, ch) in [&charter_a, &charter_b, &charter_c].iter().enumerate() {
            let resolver = |did: &str| {
                if did == ch.polity.as_str() {
                    ch.polity.to_pubkey().ok()
                } else {
                    None
                }
            };
            // Link i: predecessor i → the fused polity, signed by predecessor i's root.
            let link = &outcome.links[i];
            assert_eq!(link.payload.previous_did, ch.polity);
            assert_eq!(link.payload.new_did, fused.payload.did);
            link.verify(resolver).expect("successor link verifies");
            // Dissolution i: predecessor i, sole successor = the fused polity.
            let diss = &outcome.dissolutions[i];
            assert_eq!(diss.payload.polity, ch.polity);
            assert_eq!(diss.payload.successors, vec![fused.payload.did.clone()]);
            diss.verify(resolver).expect("dissolution verifies");
        }
    }

    #[test]
    fn fusion_rejects_fewer_than_two_predecessors() {
        let (charter_a, root_a, _) = chartered_polity();
        let (fused_sk, fused_pk) = keypair();
        let fused =
            CommunityIdentity::new_self_signed("Merged".into(), &fused_sk, fused_pk, time::now()).unwrap();
        let preds = [FusionPredecessor {
            charter: &charter_a,
            root_priv: &root_a,
            deciders: charter_a.members.clone(),
        }];
        assert!(matches!(
            fusion(&preds, fused, "r".into(), time::now()).unwrap_err(),
            KcError::Authority(_)
        ));
    }

    #[test]
    fn fusion_rejects_a_duplicated_predecessor() {
        let (charter_a, root_a, _) = chartered_polity();
        let (fused_sk, fused_pk) = keypair();
        let fused =
            CommunityIdentity::new_self_signed("Merged".into(), &fused_sk, fused_pk, time::now()).unwrap();
        let preds = [
            FusionPredecessor { charter: &charter_a, root_priv: &root_a, deciders: charter_a.members.clone() },
            FusionPredecessor { charter: &charter_a, root_priv: &root_a, deciders: charter_a.members.clone() },
        ];
        assert!(matches!(
            fusion(&preds, fused, "r".into(), time::now()).unwrap_err(),
            KcError::Authority(_)
        ));
    }

    #[test]
    fn fusion_rejects_merged_polity_colliding_with_a_predecessor() {
        // Build the fused identity from predecessor A's own root → same DID as A.
        let (charter_a, root_a, root_a_pub) = chartered_polity();
        let (charter_b, root_b, _) = chartered_polity();
        let fused =
            CommunityIdentity::new_self_signed("Merged".into(), &root_a, root_a_pub, time::now()).unwrap();
        let preds = [
            FusionPredecessor { charter: &charter_a, root_priv: &root_a, deciders: charter_a.members.clone() },
            FusionPredecessor { charter: &charter_b, root_priv: &root_b, deciders: charter_b.members.clone() },
        ];
        assert!(matches!(
            fusion(&preds, fused, "r".into(), time::now()).unwrap_err(),
            KcError::InvalidDid(_)
        ));
    }

    #[test]
    fn fusion_rejects_a_non_self_signed_fused_identity() {
        let (charter_a, root_a, _) = chartered_polity();
        let (charter_b, root_b, _) = chartered_polity();
        let (fused_sk, fused_pk) = keypair();
        let mut fused =
            CommunityIdentity::new_self_signed("Merged".into(), &fused_sk, fused_pk, time::now()).unwrap();
        // Tamper with the payload without re-signing → the self-signed check fails.
        fused.payload.name = "Tampered".into();
        let preds = [
            FusionPredecessor { charter: &charter_a, root_priv: &root_a, deciders: charter_a.members.clone() },
            FusionPredecessor { charter: &charter_b, root_priv: &root_b, deciders: charter_b.members.clone() },
        ];
        assert!(fusion(&preds, fused, "r".into(), time::now()).is_err());
    }

    // --- the envoy artifact (D14) ---------------------------------------------
    //
    // These build an `EnvoyState` directly rather than walking an archive, which is
    // the point of taking the read-model: the border crate never touches a log.

    fn standing(expires_at: Option<Timestamp>) -> (EnvoyState, DidKey, PubKey) {
        let (_, envoy_pk) = keypair();
        let envoy = DidKey::from_pubkey(&envoy_pk).unwrap();
        let st = EnvoyState {
            standings: vec![EnvoyHolding {
                envoy: envoy.clone(),
                envoy_pubkey: envoy_pk.clone(),
                expires_at,
                authorized_at: time::now(),
            }],
        };
        (st, envoy, envoy_pk)
    }

    #[test]
    fn an_authorized_envoy_becomes_a_kc_envoy_delegation() {
        let (charter, root_sk, root_pk) = chartered_polity();
        let expiry = time::now();
        let (envoys, envoy, envoy_pk) = standing(Some(expiry));

        let signed =
            envoy_delegation(&charter, &envoys, &envoy, &root_sk, root_pk, time::now()).unwrap();

        assert_eq!(signed.payload.community_did, charter.polity);
        assert_eq!(signed.payload.speaker_did, envoy);
        assert_eq!(signed.payload.speaker_pubkey, envoy_pk);
        assert_eq!(signed.payload.scope, DelegationScope::Envoy);
        // The polity's own expiry travels with it, so the two tiers cannot disagree
        // about when the standing ends.
        assert_eq!(signed.payload.expires_at, Some(expiry));
        assert_eq!(signed.payload.revoked_at, None);
        // Signed by the polity root, which is what makes it resolvable by community.
        assert_eq!(signed.signatures.len(), 1);
        assert_eq!(signed.signatures[0].signer_did, charter.polity.as_str());
        signed.verify(en_charter::didkey_resolver).expect("root signature valid");
    }

    #[test]
    fn an_envoy_without_a_standing_gets_no_delegation() {
        // The use-time rule reaching the border: a standing that has lapsed, been
        // resigned, revoked or expired is simply absent from the state, and no
        // delegation can be minted from it.
        let (charter, root_sk, root_pk) = chartered_polity();
        let (_, envoy, _) = standing(None);

        let err = envoy_delegation(
            &charter,
            &EnvoyState::default(),
            &envoy,
            &root_sk,
            root_pk,
            time::now(),
        )
        .unwrap_err();
        assert!(matches!(err, KcError::Authority(_)), "got {err:?}");
    }

    #[test]
    fn a_delegation_cannot_be_minted_with_another_polity_root_key() {
        let (charter, _, _) = chartered_polity();
        let (other_sk, other_pk) = keypair();
        let (envoys, envoy, _) = standing(None);

        let err =
            envoy_delegation(&charter, &envoys, &envoy, &other_sk, other_pk, time::now())
                .unwrap_err();
        assert!(matches!(err, KcError::InvalidDid(_)), "got {err:?}");
    }

    // --- border artifact #6: the member roll ---------------------------------

    fn members(n: usize) -> Vec<DidKey> {
        (0..n)
            .map(|_| {
                let (_, pk) = keypair();
                DidKey::from_pubkey(&pk).unwrap()
            })
            .collect()
    }

    #[test]
    fn a_polity_roll_becomes_a_kc_member_roll() {
        let (charter, root_sk, root_pk) = chartered_polity();
        let roll = members(4);
        let as_of = time::now();

        let signed =
            member_roll(&charter, &roll, &root_sk, root_pk, as_of, as_of).unwrap();

        assert_eq!(signed.payload.community_did, charter.polity);
        assert_eq!(signed.payload.members.len(), 4);
        assert_eq!(signed.payload.as_of, as_of);
        // Every member's key is the one its own DID resolves to — nothing else has to
        // travel, and KC refuses a roll where that does not hold.
        for (m, did) in signed.payload.members.iter().zip(&roll) {
            assert_eq!(&m.member_did, did);
            assert_eq!(m.member_pubkey, did.to_pubkey().unwrap());
        }
        assert_eq!(signed.signatures.len(), 1);
        assert_eq!(signed.signatures[0].signer_did, charter.polity.as_str());
        signed.verify(en_charter::didkey_resolver).expect("root signature valid");
    }

    #[test]
    fn a_roll_cannot_be_published_with_another_polity_root_key() {
        let (charter, _, _) = chartered_polity();
        let (other_sk, other_pk) = keypair();
        let as_of = time::now();

        let err = member_roll(&charter, &members(2), &other_sk, other_pk, as_of, as_of)
            .unwrap_err();
        assert!(matches!(err, KcError::InvalidDid(_)), "got {err:?}");
    }

    #[test]
    fn an_empty_roll_does_not_cross_the_boundary() {
        // KC refuses an empty roll, so emitting one would only produce a record that
        // cannot be authorized. Refuse it here, where the error can say why.
        let (charter, root_sk, root_pk) = chartered_polity();
        let as_of = time::now();

        let err = member_roll(&charter, &[], &root_sk, root_pk, as_of, as_of).unwrap_err();
        assert!(matches!(err, KcError::Authority(_)), "got {err:?}");
    }

    #[test]
    fn the_polity_itself_is_dropped_from_its_own_roll() {
        // KC refuses a roll naming the community, because its root key is already an
        // insider and a second entry would repeat a ring position.
        let (charter, root_sk, root_pk) = chartered_polity();
        let mut roll = members(2);
        roll.push(charter.polity.clone());
        let as_of = time::now();

        let signed =
            member_roll(&charter, &roll, &root_sk, root_pk, as_of, as_of).unwrap();
        assert_eq!(signed.payload.members.len(), 2);
        assert!(!signed.payload.members.iter().any(|m| m.member_did == charter.polity));
    }

    #[test]
    fn a_roll_of_only_the_polity_itself_is_refused() {
        let (charter, root_sk, root_pk) = chartered_polity();
        let roll = vec![charter.polity.clone()];
        let as_of = time::now();

        let err =
            member_roll(&charter, &roll, &root_sk, root_pk, as_of, as_of).unwrap_err();
        assert!(matches!(err, KcError::Authority(_)), "got {err:?}");
    }
}
