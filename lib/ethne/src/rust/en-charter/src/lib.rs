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

//! `en-charter` — the polity **founding** primitive (the keystone of the ethne
//! tier).
//!
//! A charter turns an emergent set of cooperators into a **polity**: a bounded,
//! substrate-anchored community that persists across its members. Its shape is
//! borrowed from kith-covenant's compact constitution but scoped to one
//! community, and — per design decision D1 — this crate depends only on
//! `kc-core` / `en-core` and the neutral `en-archive`, never on KC's governance
//! crates.
//!
//! # Ratify, do not replace
//!
//! The distinctive discipline of the tier is that structure is **ratified**, not
//! imposed. A [`FoundingProposal`] carries the candidate boundary and substrate —
//! as read from the emergent structure or hand-authored — but has **no
//! authority**. Only [`FoundingProposal::ratify`], the founding members
//! co-signing, turns it into a [`PolityCharter`]. Emergence proposes; the polity
//! ratifies; (later) kith-covenant recognizes.
//!
//! # What the charter fixes and what it may change
//!
//! - The **substrate** (what the polity defers to) is *genesis-immutable*: there
//!   is no record that changes it.
//! - **Founding commitments** are genesis-immutable too (they cannot be removed).
//! - **Later commitments** follow an asymmetric rule — added at a majority,
//!   removed only at a majority **+ 1** — so weakening the polity's self-binding
//!   is always harder than strengthening it.
//! - A founding is a **singleton** per polity and needs a **majority of the
//!   roll** to ratify.
//!
//! See `doc/design.md` for the tier's full architecture.

mod authority;
mod detached;
mod entry;
mod record;
mod state;

#[cfg(test)]
mod tests;

pub use authority::{authority_violations, check_authority, majority, PRUNE_SETTLING_SECONDS};
pub use detached::{assemble, describe_guardian_change, pending, Pending};
pub use entry::{didkey_resolver, verify_all, verify_entry, CharterEntry};
pub use record::{
    BoundaryAttestation, BoundaryDissent, BoundarySource, DissentKind, Exclusion,
    SubstrateReading,
    pruning_digest, Affirmation, AffirmationKind, AmendmentKind, CharterDissolution, Commitment,
    CommitmentAmendment, CommunityStanding, CompactAct, CompactActKind, COMPACT_COUNTERPARTIES_MAX,
    COMPACT_LABEL_MAX, ContentPolicy, ContentPolicyAmendment, LabelAction, check_label,
    CONTENT_LABEL_MAX, CONTENT_LABELS_MAX, Derivation, DerivedEdge, EnvoyAuthorization, EnvoyKind, FoundingProposal,
    GuardianChange, GuardianEvidence, GuardianKind, MemberReading, MembershipChange,
    MembershipKind, Observation,
    AttestedRole, IndependenceKind, ObservationPruning, Office, OfficeChange, OfficeKind,
    PolityCharter, RelayEndpoint, RELAY_AREAS_MAX, is_geohash_area,
    ReleaseAuthorization, RELEASE_REPO_MAX, RELEASE_URL_MAX,
    IndependenceAttestation, Substrate, SubstrateDisavowal,
    Movement, RiteMovement,
};
pub use state::{
    affirmation_state, apply_prunings, charter_state, decay_trends, effective_prune_horizon,
    founding_dissent, founding_legitimacy, independence, independent_corroboration,
    independent_grounding, independent_observation, observer_independence, reader_independence,
    substrate_disavowal, DisavowalEntry, DissentEntry, FoundingBasis, FoundingDissent,
    FoundingLegitimacy, GroundStanding, IndependentCorroboration, IndependentGrounding,
    IndependentObservation, ReaderIndependence, SubstrateGrounding,
    envoy_state, guardian_state, is_prunable, observation_window, office_state,
    prunable_observations, prune_recommendation, AffirmationPoint, AffirmationState, CharterState,
    ChurnPoint, DecayTrends, EnvoyHolding, EnvoyState, GuardianState, MemberAffirmation,
    ObservationPoint, ObservationWindow, OfficeHolding, OfficeState, PruneRecommendation,
    PruneRoute, RetentionPolicy,
    rite_state, MovementPoint, Rite,
    rendezvous_state, RelayListing, RendezvousState,
    content_policy_state, ContentPolicyState,
    compact_state, CompactState,
    release_state, ReleaseListing, ReleaseState,
};
