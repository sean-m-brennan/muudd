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

//! The pluggable substrate-provider interface: the source of the emergent
//! evidence a founding ratifies (and, later, the ongoing input a cohesion
//! monitor reads). `autonomous_trust` is *one* provider (`ethne-at`, a separate
//! adapter crate); the [`crate::manual::ManualFounding`] provider needs no
//! substrate at all. Ethne never depends on any particular provider — that is
//! what keeps kith-covenant free of an autonomous_trust dependency.

use std::collections::BTreeSet;

use en_charter::{
    BoundaryAttestation, BoundarySource, Commitment, Derivation, Exclusion, FoundingProposal,
    Observation, Substrate, SubstrateReading,
};
use en_core::{DidKey, KcResult, Signed, Timestamp};
use serde::{Deserialize, Serialize};

/// A candidate member the provider surfaces from the substrate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberCandidate {
    pub did: DidKey,
    /// An earned-reputation reading, normalized to `[0.0, 1.0]`, or `None` when the
    /// provider has **no reading at all** for this member. A manual convener
    /// asserts full reputation; an emergent provider computes it; a provider that
    /// has not yet observed enough to rate a member says `None`.
    ///
    /// `None` means *unrated*, and that is emphatically not the same as *rated
    /// low* (design **D18**). Collapsing the two is a live hazard rather than a
    /// theoretical one: autonomous_trust returns its `PREREP_NEUTRAL` (0.2) both
    /// for a peer it knows nothing about and as an earnable score, while
    /// `en_cohesion`'s reputation floor (0.3) means *this member is failing* — so
    /// a freshly founded polity used to grade at `Concern`, the level that crosses
    /// to kith-covenant, purely for being new. Grading reads only the members that
    /// carry a reading, and counts the rest separately.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation: Option<f64>,
    /// Whether the substrate reads this member as a **derived** (machine) party.
    ///
    /// Deliberately independent of [`Self::guardian`], because the two answer
    /// different questions and the guardian edge alone cannot express all three
    /// states the substrate can be in: a human member, a machine answering to a
    /// named human, and *a machine with nobody behind it*. That last one is a real
    /// chartered state (`en_charter::Derivation { guardian: None }`) and the hinge
    /// of D15's machine-majority office rule — without it every derived member
    /// would have a guardian by construction and the rule would be vacuous. Before
    /// this flag existed, `propose` had to infer derivation from
    /// `guardian.is_some()`, so an unguarded machine could only ever be designated
    /// deliberately after founding, never ratified from the provider's own reading.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub derived: bool,
    /// The guardian this member answers to, if it is a derived (machine) party.
    /// `None` on a `derived` member is the unguarded machine; `None` on a member
    /// the provider does not read as derived carries no claim about a human.
    pub guardian: Option<DidKey>,
    /// When this member's guardian was last verified *present* — a liveness
    /// reading, distinct from the guardian edge itself. `None` means not
    /// attended: no human there, or the provider cannot tell. A provider with no
    /// notion of attendance (a manual convener) reports `None` for everyone.
    ///
    /// Carried as a stamp rather than a bool because freshness is a judgement
    /// made at query time against a cadence — see `en_cohesion::cohesion_report`,
    /// which is where it is graded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attested_at: Option<Timestamp>,
}

/// The **soft** emergent boundary a provider proposes for a founding: a candidate
/// roll and (optionally) a substrate it could infer. "Soft" because membership at
/// the edge is uncertain — the polity draws the boundary it will actually ratify.
/// Emergence proposes; the polity ratifies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundingCandidate {
    pub members: Vec<MemberCandidate>,
    /// A substrate the provider could infer. `None` for a purely emergent group
    /// with no articulated ground — which (design open question Q4) must anchor
    /// to a substrate before it can charter at all.
    pub substrate: Option<Substrate>,
}

impl FoundingCandidate {
    /// This candidate as an **unsigned** [`SubstrateReading`] offered for `polity`
    /// (design **D22**).
    ///
    /// The provider stops here. It holds no keys and gains none: signing is the
    /// host's, exactly as D20 settled for every other authored act ("`tick` decides
    /// and computes; the host signs and does I/O"). Whoever runs this provider signs
    /// the result and hands it to [`Self::propose`].
    ///
    /// D21 carried the *kind* of boundary as a label on the candidate, which only
    /// the provider could set and nobody could check. D22 replaces the label with
    /// the evidence, so a candidate can no longer claim a provenance it cannot
    /// produce a signature for.
    pub fn reading(&self, polity: DidKey, reader: DidKey, read_at: Timestamp) -> SubstrateReading {
        SubstrateReading {
            polity,
            reader,
            read_at,
            members: self.members.iter().map(|m| m.did.clone()).collect(),
            opaque_boundary: false,
        }
    }

    /// Draw a deliberate [`FoundingProposal`] from this soft candidate — the
    /// "ratify emergence, do not replace it" translation. The convener supplies
    /// the polity DID, the substrate the polity commits to (it may adopt
    /// `self.substrate` or override it), and the founding commitments (the
    /// provider never dictates what the polity stands for). The roll is trimmed
    /// to candidates whose reputation is at least `min_reputation`.
    ///
    /// **An unrated candidate does not satisfy a demanded floor** (design **D18**).
    /// A positive `min_reputation` is a positive assertion — "only members the
    /// substrate rates this well" — and an absent reading cannot support it. A floor
    /// of `0.0` (or below) demands nothing and admits everyone, rated or not, which
    /// is what the default expresses. Note this is behaviour-preserving on both
    /// ends: a provider that used to collapse "unknown" into a low number saw those
    /// members trimmed by any floor above it, and still does.
    ///
    /// The **guardian edges travel with the boundary**: every retained candidate the
    /// substrate reads as a derived (machine) party becomes a proposed `Derivation`
    /// (design **D15**). That is the same ratify-not-replace step the roll itself
    /// undergoes — the provider *observes* which members answer to which human, and
    /// only the founding turns that reading into a chartered fact. Derivation is
    /// taken from [`MemberCandidate::derived`], **not** inferred from the presence of
    /// a guardian, so a machine the substrate reads as having nobody behind it is
    /// ratified as the unguarded `Derivation` it is rather than passing silently as a
    /// human. A member the provider never flags as derived is proposed as a plain
    /// member, which remains open question Q5 rather than something the founding can
    /// settle.
    ///
    /// **Whoever the floor trims is recorded, not dropped** (design **D21**). The
    /// proposal carries a `BoundaryAttestation` naming everyone this candidate showed
    /// that the roll does not, with the reason the floor actually gave. Before D21
    /// they simply vanished, and since the genesis quorum counted the *proposal's own
    /// roll*, a narrowed founding was indistinguishable from a whole one — measured,
    /// one node could charter a thirty-node cluster as a polity of itself and pass
    /// every check.
    ///
    /// The result is still only a *proposal*: it carries no authority until the
    /// members ratify and co-sign it (`FoundingProposal::ratify`).
    pub fn propose(
        &self,
        polity: DidKey,
        substrate: Substrate,
        founding_commitments: Vec<Commitment>,
        min_reputation: f64,
        readings: Vec<Signed<SubstrateReading>>,
        observation: Option<Signed<Observation>>,
        proposed_at: Timestamp,
    ) -> FoundingProposal {
        // An unrated candidate clears a floor of 0.0 (which demands nothing) and
        // fails any positive one, because `None` cannot support the assertion a
        // positive floor makes.
        let keeps = |m: &MemberCandidate| {
            min_reputation <= 0.0 || m.reputation.is_some_and(|r| r >= min_reputation)
        };
        let exclusions = self
            .members
            .iter()
            .filter(|m| !keeps(m))
            .map(|m| Exclusion {
                member: m.did.clone(),
                // The reason the floor actually gave, distinguishing the two ways to
                // fail it — which is exactly D18's unrated-vs-rated-low distinction,
                // said out loud where an excluded party can read it.
                reason: match m.reputation {
                    Some(r) => format!("reputation {r:.2} below the floor {min_reputation:.2}"),
                    None => format!(
                        "unrated; a floor of {min_reputation:.2} demands a reading"
                    ),
                },
            })
            .collect();
        self.build(
            polity,
            substrate,
            founding_commitments,
            keeps,
            readings,
            observation,
            exclusions,
            proposed_at,
        )
    }

    /// Draw a proposal that keeps exactly `keep`, for one stated `reason` — the
    /// **deliberate** narrowing, as against the floor-based trimming [`propose`] does.
    ///
    /// This exists so an honest small founding is a first-class act. A group of five
    /// inside a thirty-node cluster may have every right to charter alone, and if the
    /// only ergonomic path were the reputation floor, that founding would either be
    /// dressed up as a floor it is not, or done by hand with an attestation the
    /// convener writes themselves. Here the narrowing is explicit, every excluded
    /// member is named with the convener's own reason, and the accounting is correct
    /// by construction.
    ///
    /// A `did` in `keep` that this candidate never showed is simply ignored — the
    /// attestation describes *this* boundary, and cannot vouch for anyone outside it.
    ///
    /// [`propose`]: FoundingCandidate::propose
    pub fn propose_selecting(
        &self,
        polity: DidKey,
        substrate: Substrate,
        founding_commitments: Vec<Commitment>,
        keep: &[DidKey],
        reason: &str,
        readings: Vec<Signed<SubstrateReading>>,
        observation: Option<Signed<Observation>>,
        proposed_at: Timestamp,
    ) -> FoundingProposal {
        let kept: BTreeSet<&str> = keep.iter().map(DidKey::as_str).collect();
        let keeps = |m: &MemberCandidate| kept.contains(m.did.as_str());
        let exclusions = self
            .members
            .iter()
            .filter(|m| !keeps(m))
            .map(|m| Exclusion { member: m.did.clone(), reason: reason.to_string() })
            .collect();
        self.build(
            polity,
            substrate,
            founding_commitments,
            keeps,
            readings,
            observation,
            exclusions,
            proposed_at,
        )
    }

    /// The shared body of the two `propose` variants, so the roll, the derivations
    /// and the attestation cannot drift out of agreement about who was kept.
    #[allow(clippy::too_many_arguments)]
    fn build(
        &self,
        polity: DidKey,
        substrate: Substrate,
        founding_commitments: Vec<Commitment>,
        keeps: impl Fn(&MemberCandidate) -> bool,
        readings: Vec<Signed<SubstrateReading>>,
        observation: Option<Signed<Observation>>,
        exclusions: Vec<Exclusion>,
        proposed_at: Timestamp,
    ) -> FoundingProposal {
        let retained = || self.members.iter().filter(|m| keeps(m));
        let proposed_members: Vec<DidKey> = retained().map(|m| m.did.clone()).collect();
        // D27 — a founding derivation that claims a guardian edge must carry what the
        // substrate showed, or declare that nothing did. The provider holds a snapshot,
        // not a signed reading, so the observation is passed in: a monitor that ran over
        // this cluster before the founding backs the edges it actually saw, and every
        // other claimed edge says outright that nothing backed it. The honest default is
        // `Unbacked` — which since D27 does not count as ground, so a wholly machine
        // cluster proposed from an unobserved snapshot must name a substrate anchor.
        //
        // The observer must also be somebody the roll does not contain: a founder
        // observing the cluster it is founding is the polity vouching for its own
        // ground, which the charter's grounded rule will not credit. Checked here too
        // so the provider never mints an `Observed` derivation that rule then ignores
        // — an edge whose evidence does not count should say so where an operator can
        // read it, not look backed and behave unbacked.
        let on_roll: BTreeSet<&str> = proposed_members.iter().map(DidKey::as_str).collect();
        let disinterested = observation.as_ref().is_some_and(|ob| {
            let who = ob.payload.observer.as_str();
            who != polity.as_str() && !on_roll.contains(who)
        });
        let shown = |member: &DidKey, guardian: &DidKey| -> bool {
            disinterested
                && observation.as_ref().is_some_and(|ob| {
                    ob.payload
                        .readings
                        .iter()
                        .find(|r| &r.member == member)
                        .is_some_and(|r| r.guardian.as_ref() == Some(guardian))
                })
        };
        let proposed_derivations = retained()
            .filter(|m| m.derived)
            .map(|m| match &m.guardian {
                None => Derivation::unguarded(m.did.clone()),
                Some(g) if shown(&m.did, g) => Derivation::observed(
                    m.did.clone(),
                    g.clone(),
                    observation.clone().expect("shown() only true with an observation"),
                ),
                Some(g) => Derivation::unbacked(
                    m.did.clone(),
                    g.clone(),
                    match &observation {
                        None => "no observation was taken of this cluster before founding"
                            .to_string(),
                        Some(_) if !disinterested => {
                            "the observation was taken from inside the polity being founded"
                                .to_string()
                        }
                        Some(_) => format!(
                            "the observation carries no reading showing {} guarded by {g}",
                            m.did
                        ),
                    },
                ),
            })
            .collect();
        FoundingProposal {
            polity,
            substrate,
            founding_commitments,
            proposed_members,
            proposed_derivations,
            // The evidence IS the source (design **D22**): readings present make
            // this a `Substrate` boundary, their absence a convener's assertion.
            // There is no separate label to fall out of step with what can actually
            // be produced.
            attestation: BoundaryAttestation {
                candidate_size: self.members.len(),
                source: if readings.is_empty() {
                    BoundarySource::ConvenerAsserted
                } else {
                    BoundarySource::Substrate { readings }
                },
                exclusions,
            },
            proposed_at,
        }
    }
}

/// A source of the emergent structure ethne crystallizes into a polity. Kept
/// deliberately narrow: the founding path and a point reputation reading. The
/// change/anomaly stream is [`ChangeStream`], a separate trait — see there for why.
pub trait SubstrateProvider {
    /// The current candidate founding boundary read from the substrate.
    fn founding_candidate(&self) -> KcResult<FoundingCandidate>;

    /// A point earned-reputation reading for one member, if known.
    ///
    /// `None` means **no reading, for any reason**: the provider may not know the
    /// member at all, or may know them and have nothing to rate them on (design
    /// **D18**). The two are distinguished on
    /// [`MemberCandidate::reputation`], which is what grading reads; this accessor
    /// is a convenience with no grading consumer, so it is left narrow rather than
    /// widened to a nested `Option`.
    fn member_reputation(&self, member: &DidKey) -> Option<f64>;
}

/// One movement in the observed substrate, since the last drain (design **D16**).
///
/// A snapshot can say *what is*; only a stream can say *what moved*, and only a
/// record of what moved can say *for how long*. These are deliberately deltas and
/// not a diff of two candidates: the provider is the one place the prior reading is
/// already in hand, so making the caller re-derive it would be both wasteful and
/// lossy (two equal snapshots either side of a flap look like no change at all).
#[derive(Debug, Clone, PartialEq)]
pub enum SubstrateChange {
    /// A member entered the observed boundary, with its first reading.
    MemberAppeared(MemberCandidate),
    /// A member left the observed boundary. Whether that is a departure or an
    /// opaque edge closing over them is not the provider's to say.
    MemberVanished(DidKey),
    /// An earned-reputation reading moved. Either side may be `None` — a member
    /// becoming unrated, or being rated for the first time, is a movement too.
    ReputationChanged { member: DidKey, from: Option<f64>, to: Option<f64> },
    /// A member's guardian edge appeared or disappeared — the *credential*, as
    /// against the liveness stamp below.
    GuardianEdgeChanged { member: DidKey, guardian: Option<DidKey> },
    /// A member's guardian was verified present at this instant. Carried as the
    /// stamp, never as a freshness verdict: whether it is still *fresh* is a
    /// query-time judgement against a cadence.
    AttendanceStamped { member: DidKey, attested_at: Timestamp },
    /// The observed boundary became (or stopped being) partly opaque.
    BoundaryOpacityChanged { opaque: bool },
}

impl SubstrateChange {
    /// The member this change concerns, where it concerns one.
    pub fn member(&self) -> Option<&DidKey> {
        match self {
            SubstrateChange::MemberAppeared(m) => Some(&m.did),
            SubstrateChange::MemberVanished(d) => Some(d),
            SubstrateChange::ReputationChanged { member, .. }
            | SubstrateChange::GuardianEdgeChanged { member, .. }
            | SubstrateChange::AttendanceStamped { member, .. } => Some(member),
            SubstrateChange::BoundaryOpacityChanged { .. } => None,
        }
    }
}

/// A provider that can report what *moved*, not merely what is.
///
/// Kept separate from [`SubstrateProvider`] on purpose: `ManualFounding` is a
/// convener with a list, and a provider with no notion of change must not be made
/// to fake one — an empty stream and no stream are different claims, and only the
/// second is honest. A monitor that wants duration requires this trait; a founding
/// never does.
pub trait ChangeStream {
    /// Drain the changes observed since the last call. Draining is the contract:
    /// each change is reported once, so a caller that discards them loses them —
    /// which is exactly why a monitor records them (see `en_charter::Observation`)
    /// rather than holding them in memory.
    fn changes(&mut self) -> KcResult<Vec<SubstrateChange>>;
}
