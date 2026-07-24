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

//! `en-cohesion` — the polity's **coherence monitor** (a pure read-model).
//!
//! A chartered polity can erode: members drift out of the observed boundary,
//! earned reputation decays, the edge goes opaque. This crate reads that strain
//! off two snapshots — the **chartered roll** (from `en-charter`'s
//! `charter_state`) and the **current candidate** the substrate provider surfaces
//! (`SubstrateProvider::founding_candidate`) — and grades it into a
//! [`CohesionReport`]. It adds no trait method and no archive entry: it is a lens
//! over state the other crates already produce, so it stays `kc-core`-only and
//! depends on no particular provider.
//!
//! # Ratify, do not replace (design open question Q2)
//!
//! The monitor **does not decide a fission**. Algorithmically splitting a polity
//! would be exactly the "replace emergence" move the tier exists to refuse. What
//! the monitor does is raise a graded [`ErosionLevel`] — up to
//! [`ErosionLevel::Concern`], an *early warning* — which the upper tier
//! ([`en-uplift`](../en_uplift/index.html)) can surface to kith-covenant as a
//! Concern. A fission is a **deliberate re-charter**: the community ratifies two
//! successor foundings, and only then is the predecessor dissolved. `Concern` may
//! name the strain that precedes a ratified split, but the split itself is
//! ratified, never auto-verdicted.
//!
//! The three erosion levels mirror kith-covenant's own graded erosion alert
//! (`None` / `Notice` / `Alert`), one tier down.

use std::collections::{BTreeMap, BTreeSet};

use en_core::DidKey;
use en_substrate::FoundingCandidate;

/// How strained a polity's cohesion is, graded. Ordered `None < Notice <
/// Concern` (the derived order follows declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ErosionLevel {
    /// Cohesion within normal bounds.
    #[default]
    None,
    /// Early strain worth watching; not yet a Concern.
    Notice,
    /// Strain the upper tier should surface as a kith-covenant Concern.
    Concern,
}

/// The cut-points that turn raw cohesion measures into an [`ErosionLevel`]. Each
/// `*_notice` bound raises at least a Notice; each `*_concern` bound raises a
/// Concern. See [`Thresholds::default`] for the v1 values.
#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    /// A member whose earned reputation is below this counts as a floor breach.
    pub reputation_floor: f64,
    /// Attrition (fraction of the chartered roll no longer observed) at or above
    /// this raises a Notice; at or above `attrition_concern`, a Concern.
    pub attrition_notice: f64,
    pub attrition_concern: f64,
    /// Mean observed reputation at or below this raises a Notice; at or below
    /// `mean_reputation_concern`, a Concern.
    pub mean_reputation_notice: f64,
    pub mean_reputation_concern: f64,
    /// Fraction of observed members in floor breach at or above this raises a
    /// Notice; at or above `breach_concern`, a Concern.
    pub breach_notice: f64,
    pub breach_concern: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            reputation_floor: 0.3,
            attrition_notice: 0.2,
            attrition_concern: 0.4,
            mean_reputation_notice: 0.5,
            mean_reputation_concern: 0.35,
            breach_notice: 0.2,
            breach_concern: 0.4,
        }
    }
}

/// A graded read of a polity's cohesion at one moment. Every field is derived;
/// nothing here changes state.
#[derive(Debug, Clone, PartialEq)]
pub struct CohesionReport {
    /// Fraction of the chartered roll no longer present in the observed
    /// candidate, in `[0.0, 1.0]`.
    pub attrition: f64,
    /// How many still-observed members are below the reputation floor.
    pub reputation_floor_breaches: usize,
    /// Mean earned reputation over the still-observed chartered members (`0.0`
    /// when none remain observed).
    pub mean_reputation: f64,
    /// Whether the observed boundary is (partly) an intentional opaque edge. When
    /// true, apparent attrition is ambiguous — a member may have gone private
    /// rather than left — so attrition alone is not allowed to escalate past a
    /// Notice.
    pub opaque_boundary: bool,
    /// Chartered members still present in the candidate.
    pub observed: usize,
    /// Size of the chartered roll.
    pub chartered: usize,
    /// The graded erosion this report carries.
    pub erosion: ErosionLevel,
}

fn grade(value: f64, notice: f64, concern: f64) -> ErosionLevel {
    if value >= concern {
        ErosionLevel::Concern
    } else if value >= notice {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    }
}

/// Grade a *low-is-bad* measure (reputation): at or below `concern` → Concern, at
/// or below `notice` → Notice.
fn grade_low(value: f64, notice: f64, concern: f64) -> ErosionLevel {
    if value <= concern {
        ErosionLevel::Concern
    } else if value <= notice {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    }
}

/// Read a polity's cohesion by comparing its `chartered_roll` against the
/// `candidate` the substrate provider currently surfaces. `chartered_roll` is the
/// polity's **live** roll — `charter_state(...).members`, which already reflects
/// admissions and departures — so a member who formally departed is simply absent
/// from the roll and is *not* counted as attrition, while a still-enrolled member
/// the provider no longer observes *is*. `opaque_boundary` is the provider's
/// report of whether the observed
/// edge is (partly) an intentional opaque one (e.g. `AtSubstrateProvider::
/// has_private_boundary`); a provider that cannot tell should pass `false`.
///
/// The erosion level is the strongest of the component signals — attrition, mean
/// reputation, and floor-breach share — except that under an opaque boundary the
/// attrition component is capped at `Notice`, because a disappearance behind an
/// opaque edge cannot be attributed to departure.
pub fn cohesion_report(
    chartered_roll: &[DidKey],
    candidate: &FoundingCandidate,
    opaque_boundary: bool,
    thresholds: &Thresholds,
) -> CohesionReport {
    // Reputation of each currently observed candidate member, by DID.
    let observed_rep: BTreeMap<&str, f64> = candidate
        .members
        .iter()
        .map(|m| (m.did.as_str(), m.reputation))
        .collect();

    // The chartered members still present in the candidate, and their reputation.
    let mut present_rep: Vec<f64> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for did in chartered_roll {
        // Guard against a duplicated roll entry double-counting.
        if !seen.insert(did.as_str()) {
            continue;
        }
        if let Some(rep) = observed_rep.get(did.as_str()) {
            present_rep.push(*rep);
        }
    }

    let chartered = seen.len();
    let observed = present_rep.len();
    let attrition = if chartered == 0 {
        0.0
    } else {
        (chartered - observed) as f64 / chartered as f64
    };
    let mean_reputation = if present_rep.is_empty() {
        0.0
    } else {
        present_rep.iter().sum::<f64>() / present_rep.len() as f64
    };
    let reputation_floor_breaches = present_rep
        .iter()
        .filter(|r| **r < thresholds.reputation_floor)
        .count();
    let breach_share = if observed == 0 {
        0.0
    } else {
        reputation_floor_breaches as f64 / observed as f64
    };

    let attrition_level = {
        let raw = grade(attrition, thresholds.attrition_notice, thresholds.attrition_concern);
        if opaque_boundary {
            raw.min(ErosionLevel::Notice)
        } else {
            raw
        }
    };
    // Mean reputation only speaks when members remain observed; an empty polity's
    // erosion is carried by attrition, not by a spurious 0.0 mean.
    let mean_level = if observed == 0 {
        ErosionLevel::None
    } else {
        grade_low(
            mean_reputation,
            thresholds.mean_reputation_notice,
            thresholds.mean_reputation_concern,
        )
    };
    let breach_level = grade(breach_share, thresholds.breach_notice, thresholds.breach_concern);

    let erosion = attrition_level.max(mean_level).max(breach_level);

    CohesionReport {
        attrition,
        reputation_floor_breaches,
        mean_reputation,
        opaque_boundary,
        observed,
        chartered,
        erosion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::Substrate;
    use en_core::{keys, Alg, DidKey};
    use en_substrate::{FoundingCandidate, MemberCandidate};

    fn did() -> DidKey {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        DidKey::from_pubkey(&pk).unwrap()
    }

    fn substrate() -> Substrate {
        Substrate { name: "tradition".into(), statement: "answers to X".into(), anchor: None }
    }

    fn candidate(members: &[(DidKey, f64)]) -> FoundingCandidate {
        FoundingCandidate {
            members: members
                .iter()
                .map(|(d, r)| MemberCandidate { did: d.clone(), reputation: *r, guardian: None })
                .collect(),
            substrate: Some(substrate()),
        }
    }

    #[test]
    fn healthy_roll_is_none() {
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &Thresholds::default());
        assert_eq!(r.chartered, 5);
        assert_eq!(r.observed, 5);
        assert_eq!(r.attrition, 0.0);
        assert_eq!(r.reputation_floor_breaches, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn mild_attrition_is_notice() {
        // 5 chartered, 4 still observed → attrition 0.2 (== notice, < concern).
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(4).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &Thresholds::default());
        assert_eq!(r.observed, 4);
        assert!((r.attrition - 0.2).abs() < 1e-9);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn heavy_attrition_is_concern() {
        // 5 chartered, 2 still observed → attrition 0.6 (>= concern 0.4).
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(2).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &Thresholds::default());
        assert!((r.attrition - 0.6).abs() < 1e-9);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn opaque_boundary_caps_attrition_at_notice() {
        // The same 0.6 attrition, but behind an opaque edge → capped at Notice,
        // because the disappearance cannot be attributed to departure.
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(2).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), true, &Thresholds::default());
        assert!(r.opaque_boundary);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn low_mean_reputation_is_concern() {
        // Full roll observed, but reputation has decayed below the concern mean.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.3)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &Thresholds::default());
        assert_eq!(r.attrition, 0.0);
        assert!(r.mean_reputation <= Thresholds::default().mean_reputation_concern);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn floor_breaches_counted_and_graded() {
        // 4 observed, 2 below the 0.3 floor → breach share 0.5 (>= concern 0.4),
        // but the mean (≈0.5) stays above its concern bound, so the breach share
        // is what carries the Concern.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = vec![
            (roll[0].clone(), 0.8),
            (roll[1].clone(), 0.8),
            (roll[2].clone(), 0.1),
            (roll[3].clone(), 0.1),
        ];
        let r = cohesion_report(&roll, &candidate(&members), false, &Thresholds::default());
        assert_eq!(r.reputation_floor_breaches, 2);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn empty_chartered_roll_is_none() {
        let r = cohesion_report(&[], &candidate(&[]), false, &Thresholds::default());
        assert_eq!(r.chartered, 0);
        assert_eq!(r.observed, 0);
        assert_eq!(r.attrition, 0.0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }
}
