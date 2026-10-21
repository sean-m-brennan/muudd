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

//! The §7 commons payloads and their classifying enums.

use kc_conduct::DecisionClass;
use kc_core::{
    canonical, BoundProof, Cid, Commitment, DidKey, KcResult, RevealedShare, ThresholdCustody,
    Timestamp,
};
use serde::{Deserialize, Serialize};

/// The five compact-level commons (commons_federation.md §"The five space
/// commons"). Each is non-excludable at the margin, rivalrous under stress, and
/// weaponizable by withholding or pollution, so each is held by the compact
/// rather than owned by any single member community.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommonsKind {
    /// Atmosphere generation, water recycling, thermal regulation.
    LifeSupport,
    /// Electromagnetic spectrum and relays for intra-system communication.
    Communication,
    /// Ephemerides, conjunction warnings, debris tracking.
    Navigation,
    /// The accumulated knowledge of the compact ("inheritance of the future").
    KnowledgeArchive,
    /// Distress beacons, rescue vessels, medical facilities.
    Rescue,
}

impl CommonsKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LifeSupport => "LifeSupport",
            Self::Communication => "Communication",
            Self::Navigation => "Navigation",
            Self::KnowledgeArchive => "KnowledgeArchive",
            Self::Rescue => "Rescue",
        }
    }

    pub fn all() -> [CommonsKind; 5] {
        [
            Self::LifeSupport,
            Self::Communication,
            Self::Navigation,
            Self::KnowledgeArchive,
            Self::Rescue,
        ]
    }
}

/// Which nesting level a commons is held at (Ostrom principle 8).
///
/// `commons_federation.md` gives three layers: the habitat, governed by each community's
/// own institutions; the compact, governed by kith-covenant's; and the inter-compact
/// level, "commons shared between multiple compacts, if compacts federate". The first is
/// out of this tier's scope entirely. These are the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AppropriatorLevel {
    /// Appropriators are member communities. The ordinary case, and what every commons
    /// registered before the level existed is read as.
    #[default]
    Community,
    /// Appropriators are whole compacts, each named by a `CompactCharter`. The higher
    /// nesting level: it governs compacts, and reaches their members only through them.
    Compact,
}

impl AppropriatorLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Community => "Community",
            Self::Compact => "Compact",
        }
    }
}

/// Registration of a compact-level commons. Ostrom principle 1 (clearly defined
/// boundaries: who the appropriators are and what the resource is) and principle
/// 2 (proportional provision). A compact-level act: **second-class**, co-signed
/// by the registering communities under a distinct-community quorum. The commons
/// is thereafter referenced by this record's archive CID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonsRegistration {
    pub kind: CommonsKind,
    pub name: String,
    pub description: String,
    /// The parties with defined rights to draw on this commons (Ostrom's boundary).
    /// Usage and sanction records are meaningful only for a party in this set.
    ///
    /// Member communities at the [`AppropriatorLevel::Community`] level, and whole
    /// compacts at the [`AppropriatorLevel::Compact`] level.
    pub appropriators: Vec<DidKey>,
    /// Which nesting level this commons is held at. Absent in archives written before
    /// the level existed, and read as `Community` there, which is what they were.
    #[serde(default)]
    pub level: AppropriatorLevel,
    /// The rule tying benefit to contribution (principle 2), free-form for the
    /// reader; not machine-interpreted.
    pub provision_rule: String,
    /// The per-period cap on any one appropriator's draw, in the commons' own units.
    ///
    /// [`provision_rule`](Self::provision_rule)'s machine-readable half, and the only
    /// part of Ostrom's proportionality that a rule can act on. Until it existed the
    /// stated rule bound nothing: a record drawing ten times the written cap was
    /// authorized exactly like one that honoured it, and the two were indistinguishable
    /// to the archive.
    ///
    /// It is also what a [`SealedUsage`] proves against. `None` means the commons states
    /// no machine-readable cap, which is the pre-existing behaviour and which therefore
    /// cannot host sealed usage: there would be nothing to prove.
    pub draw_allowance: Option<u64>,
    /// The registering communities (each co-signs). Must be a non-empty subset of
    /// `appropriators` and satisfy the second-class quorum.
    pub registered_by: Vec<DidKey>,
    pub registered_at: Timestamp,
}

/// The figures a [`SealedUsage`] holds under custody: the actual draw and return, and
/// the blindings that open their commitments.
///
/// Sealed as canonical CBOR, so a custodian quorum recovers exactly what was committed
/// to and can be held to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageFigures {
    pub consumed: u64,
    pub contributed: u64,
    pub consumed_blinding: [u8; 32],
    pub contributed_blinding: [u8; 32],
}

/// A usage record's figures, committed rather than published, with a proof that the
/// draw is within the commons' allowance.
///
/// §7 asks for a **surveillance threshold**: "configurable disclosure levels for
/// internal health indicators", with large or powerful communities disclosing more by
/// default. A community that must publish every figure to account honestly is disclosing
/// its capacity, its consumption pattern and its reserves to every other member, which
/// is what the threshold exists to bound. What the compact actually needs to know is not
/// the number but whether the draw is within bounds, and that is what
/// [`within_allowance`](Self::within_allowance) shows without the number.
///
/// The figures are not lost, only withheld: they sit under threshold custody, and a
/// [`UsageOpened`] recovers them for an audit or a dispute. This is the same
/// [`ThresholdCustody`] holding §8 source identities and §4 sealed proceedings, at its
/// third use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedUsage {
    pub consumed_commitment: Commitment,
    pub contributed_commitment: Commitment,
    /// Proof that the committed `consumed` is at most the commons' `draw_allowance`,
    /// revealing nothing else about it.
    pub within_allowance: BoundProof,
    /// The [`UsageFigures`], sealed so that a quorum of custodians and only a quorum can
    /// recover them.
    pub custody: ThresholdCustody,
}

/// A community's self-reported accounting of what it drew from and returned to a
/// commons over a period. First-class, self-signed; feeds free-rider detection
/// (Ostrom principle 4 monitoring). `consumed`/`contributed` are dimensionless
/// units in the commons' own terms (bandwidth-hours, rescue-standby-hours,
/// archive-bytes, …), interpreted only through the free-rider ratio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// The registered commons this reports on (its `CommonsRegistered` CID).
    pub commons_cid: Cid,
    pub community: DidKey,
    /// The draw, in the clear. **Zero and unread when [`sealed`](Self::sealed) is
    /// present**, so a record carries its figures once and in one place.
    pub consumed: u64,
    /// The return, in the clear. Zero and unread when sealed, as above.
    pub contributed: u64,
    /// The committed form, when this community accounts under the surveillance
    /// threshold rather than in public.
    pub sealed: Option<SealedUsage>,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub recorded_at: Timestamp,
}

/// A custodian quorum opening a sealed usage record's figures (§7).
///
/// Mirrors §4's `DecisionUnsealed` and §8's `SourceReveal`, and for the same reason: a
/// rule about who signs is not a guarantee about what is disclosed. The opened figures
/// must open the record's own commitments, so a quorum cannot publish a flattering
/// number in place of the one the community committed to.
///
/// A sealed record is openable from the moment it is filed, unlike a sealed decision.
/// The threshold here bounds *routine* disclosure, not disclosure for cause: an audit or
/// a dispute is exactly the occasion for opening, and requiring the compact to wait
/// would put the free-rider question beyond reach for as long as the seal held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageOpened {
    /// The `UsageAccounted` entry being opened.
    pub usage_cid: Cid,
    /// The custodians opening it, each of whom co-signs. **Derived, not chosen**: the
    /// authority pass requires this to be exactly the co-signers.
    pub custodians: Vec<DidKey>,
    /// Why it is being opened. Required non-empty, and never read for content.
    pub cause: String,
    /// The figures recovered, which must open the sealed commitments.
    pub consumed: u64,
    pub contributed: u64,
    pub shares: Vec<RevealedShare>,
    pub opened_at: Timestamp,
}

impl SealedUsage {
    /// The canonical bytes a sealed record's custody holds.
    pub fn figure_bytes(figures: &UsageFigures) -> KcResult<Vec<u8>> {
        canonical::to_canonical_cbor(figures)
    }

    /// Recover the figures from `plaintext` a custody quorum produced.
    pub fn figures_from(plaintext: &[u8]) -> KcResult<UsageFigures> {
        canonical::from_canonical_cbor(plaintext)
    }
}

/// Is this commons one the compact keeps running whatever else stops?
///
/// Life support and rescue are the two the structured fast does not pause and that a
/// habitat cannot do without for the length of a deliberation. Abusing them is
/// categorically graver than overdrawing bandwidth or archive space, and the commons
/// escalation trigger weighs them accordingly.
pub fn is_life_critical(kind: CommonsKind) -> bool {
    matches!(kind, CommonsKind::LifeSupport | CommonsKind::Rescue)
}

/// The commons equivalent of the anathema designation: a finding that a community's
/// abuse of a shared resource has passed beyond what graduated sanctions can answer.
///
/// `commons_federation.md` leaves this open in as many words: "What is the **commons
/// equivalent of the anathema designation**: when does a community's abuse of a shared
/// commons become grounds for escalation beyond graduated sanctions? The escalation
/// ladder provides the mechanism, but the trigger threshold for commons-specific
/// violations needs specification." Both halves existed and nothing joined them: §7's
/// [`SanctionLevel`] ladder ran to `Exclusion` and stopped, and §5's designation track
/// had no commons-side ground to stand on.
///
/// The threshold this specifies has four conditions, in the shape §5's own
/// four-condition anathema test already established:
///
/// 1. **The ladder is exhausted.** The subject is at `Exclusion` on this commons.
///    Graduated sanctions are the doctrine's answer until they have demonstrably failed,
///    and a community still climbing is still being answered.
/// 2. **Defiance rather than compliance.** The subject has reached `Exclusion` more than
///    once, which is the distinction `commons_federation.md` draws when it warns that
///    disproportionate response "produces defiance rather than compliance". One offence,
///    however grave, is what the ladder is for; a community that returns to exclusion
///    after being restored is not failing to understand the rule.
/// 3. **Restoration was available and not taken.** A minimum dwell at `Exclusion`, so
///    the trigger cannot fire on a community that is mid-restoration.
/// 4. **Life-criticality weights the count.** For life support and rescue the cycle
///    requirement is lower, per [`is_life_critical`].
///
/// **This gates nothing by itself.** It is what a §5 `Designation` may cite as its
/// commons-side basis, so §5 is untouched and no consequence follows automatically. The
/// threshold lives in the refusal to *record* it: a compact cannot say a community's
/// commons conduct has passed beyond graduated sanctions until it has.
///
/// Second-class and co-signed, because a finding this heavy is not one community's to
/// make about another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonsEscalation {
    /// The registered commons whose abuse is at issue.
    pub commons_cid: Cid,
    pub subject_community: DidKey,
    /// The specific pattern found. Required non-empty and never read for content, as
    /// every stated ground in this tier is.
    pub basis: String,
    /// The sanction records this rests on. Required non-empty and each must be a
    /// sanction on *this* (commons, subject) pair: §5's condition 3 asks a designation
    /// to cite specific acts, and the commons-side finding owes the same.
    pub cited_sanctions: Vec<Cid>,
    /// The communities making the finding (each co-signs).
    pub movers: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub escalated_at: Timestamp,
}

/// The graduated-sanction ladder for a commons (Ostrom principle 5). Distinct
/// from the §5 dispute ladder: this tracks one community's abuse of a shared
/// resource, not a conflict between two communities. `None` is the resting state
/// and the de-escalation floor (restoration path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SanctionLevel {
    /// No sanction in force. The resting state and the de-escalation floor.
    None,
    /// Heightened monitoring (first-offense-tolerant: the lightest response).
    Monitoring,
    /// A formal warning on the record.
    Warning,
    /// Restricted drawdown rights until restoration. Second-class.
    Restriction,
    /// Excluded from the commons (subject to restoration). Second-class.
    Exclusion,
}

impl SanctionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Monitoring => "Monitoring",
            Self::Warning => "Warning",
            Self::Restriction => "Restriction",
            Self::Exclusion => "Exclusion",
        }
    }
}

/// Ordinal rank for the legal-progression check (higher = more severe).
pub fn sanction_rank(level: SanctionLevel) -> u8 {
    use SanctionLevel::*;
    match level {
        None => 0,
        Monitoring => 1,
        Warning => 2,
        Restriction => 3,
        Exclusion => 4,
    }
}

/// The two levels that impose compact-wide cost and so require a
/// distinct-community quorum and a non-unilateral imposer set — the commons
/// analogue of [`kc_escalation::is_second_class_rung`] (commons_federation.md
/// §"Light-lag-honest decision classes"; §"Community-applied sanctions").
pub fn is_second_class_level(level: SanctionLevel) -> bool {
    matches!(level, SanctionLevel::Restriction | SanctionLevel::Exclusion)
}

/// One step of a community along (or back down) a commons' graduated-sanction
/// ladder. Append-only: the current level of a (commons, subject) pair is the
/// `to_level` of its latest sanction. Each record names the level it leaves and
/// the level it enters, so the archive can reject out-of-order or skipping moves;
/// de-escalation (a lower `to_level`) is the always-legal restoration move.
/// Co-signed by the `imposers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonsSanction {
    /// The registered commons this concerns (its `CommonsRegistered` CID).
    pub commons_cid: Cid,
    pub subject_community: DidKey,
    pub from_level: SanctionLevel,
    pub to_level: SanctionLevel,
    /// The specific abuse cited (overdraw, pollution, withholding-as-leverage).
    pub basis: String,
    /// Prior archive records this cites (usage records, conduct, prior sanctions).
    pub references: Vec<Cid>,
    /// The communities imposing (each co-signs). For a second-class level these
    /// are the distinct-community quorum; a first-class level (monitoring,
    /// warning, or any de-escalation) may be moved by a single appropriator.
    pub imposers: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub decided_at: Timestamp,
}
