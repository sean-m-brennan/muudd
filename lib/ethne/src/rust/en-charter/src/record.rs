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

//! The charter record shapes and the ratify-not-replace founding API.

use std::collections::BTreeSet;

use en_core::{DidKey, KcError, KcResult, Timestamp};
use serde::{Deserialize, Serialize};

/// What the polity holds above itself — its non-ultimacy anchor. **Genesis-
/// immutable**: there is deliberately no amendment record that changes a
/// substrate, because a polity that re-decides what it defers to has become a
/// different polity. (For a derived — e.g. machine-majority — polity, `anchor`
/// names the human/guardian ground the substrate rests on.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Substrate {
    pub name: String,
    pub statement: String,
    pub anchor: Option<DidKey>,
}

/// A self-binding rule the polity commits to. Founding commitments are
/// genesis-immutable; later commitments are added at a simple majority and
/// removed only at a higher bar — the asymmetry that resists capture by erosion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub key: String,
    pub description: String,
}

/// A **proposed** founding: the candidate boundary and substrate, as read from
/// the emergent structure (a `SubstrateProvider`) or hand-authored, and offered
/// for ratification. It carries **no authority on its own** — that is the
/// "ratify, do not replace" discipline made structural: emergence *proposes*, and
/// only [`FoundingProposal::ratify`] (members co-signing) turns a proposal into a
/// [`PolityCharter`]. A proposal is never appended to the archive; only the
/// ratified charter is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundingProposal {
    /// The polity's root DID — the persistent entity the charter founds.
    pub polity: DidKey,
    pub substrate: Substrate,
    pub founding_commitments: Vec<Commitment>,
    /// The candidate membership roll (the crystallized emergent boundary).
    pub proposed_members: Vec<DidKey>,
    pub proposed_at: Timestamp,
}

impl FoundingProposal {
    /// Ratify the proposal into a [`PolityCharter`]. `ratifiers` are the founding
    /// members who consent (they must co-sign the resulting record). This checks
    /// the *local* invariants — ratifiers are distinct, non-empty, and a subset
    /// of the proposed roll — so a charter cannot be minted from thin air. The
    /// *quorum* (a majority of the roll) and *singleton* (one founding per polity)
    /// rules range over the archive and are enforced by the authority pass.
    pub fn ratify(self, ratifiers: Vec<DidKey>, founded_at: Timestamp) -> KcResult<PolityCharter> {
        if ratifiers.is_empty() {
            return Err(KcError::Authority(
                "ratification requires at least one ratifier".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for r in &ratifiers {
            if !seen.insert(r.as_str()) {
                return Err(KcError::Authority(format!("duplicate ratifier: {r}")));
            }
        }
        let roll: BTreeSet<&str> = self.proposed_members.iter().map(DidKey::as_str).collect();
        for r in &ratifiers {
            if !roll.contains(r.as_str()) {
                return Err(KcError::Authority(format!(
                    "ratifier {r} is not in the proposed membership roll"
                )));
            }
        }
        Ok(PolityCharter {
            polity: self.polity,
            substrate: self.substrate,
            founding_commitments: self.founding_commitments,
            members: self.proposed_members,
            ratifiers,
            proposed_at: self.proposed_at,
            founded_at,
        })
    }
}

/// The ratified founding charter — a **genesis singleton** for one polity.
/// Produced only via [`FoundingProposal::ratify`]. `members` is the ratified
/// roll (the whole proposed boundary becomes members); `ratifiers` are the
/// members who actively consented and must have co-signed the record. A majority
/// of the roll must ratify (enforced by the authority pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolityCharter {
    pub polity: DidKey,
    pub substrate: Substrate,
    pub founding_commitments: Vec<Commitment>,
    pub members: Vec<DidKey>,
    pub ratifiers: Vec<DidKey>,
    pub proposed_at: Timestamp,
    pub founded_at: Timestamp,
}

/// Whether a commitment amendment adds (expands) or removes (contracts) a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmendmentKind {
    Add,
    Remove,
}

impl AmendmentKind {
    /// A contraction rolls back a self-binding commitment — the capture-relevant
    /// direction, held to the higher bar.
    pub fn is_contraction(&self) -> bool {
        matches!(self, AmendmentKind::Remove)
    }
}

/// An amendment to a polity's (non-founding) commitments. There is no amendment
/// for the substrate (genesis-immutable) and none for a founding commitment
/// (also genesis-immutable — rejected by the authority pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentAmendment {
    pub polity: DidKey,
    pub kind: AmendmentKind,
    pub commitment: Commitment,
    pub rationale: String,
    /// The members deciding the amendment; they must co-sign. The bar depends on
    /// `kind` (majority to Add, majority+1 to Remove).
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// The deliberate winding-up of a polity's charter — the terminal charter-
/// lifecycle event. kith-covenant has no "dissolve" primitive (a covenant party
/// simply goes quiet); dissolution is an **ethne-side** record, because it is the
/// polity's own founding that a fission (or a decided wind-down) closes.
///
/// A dissolution names the `successors` the roll ratified — zero for a plain
/// wind-down, two for a fission — so a reader can follow the polity's line
/// forward. Like an amendment, it is authorized only by a **majority of the
/// current roll** co-signing (the `deciders`); "ratify, do not replace" applies
/// here too — a polity is not dissolved out from under its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharterDissolution {
    pub polity: DidKey,
    /// The polities the roll ratified as this one's line forward (two for a
    /// fission, empty for a plain wind-down).
    pub successors: Vec<DidKey>,
    pub rationale: String,
    /// The members deciding the dissolution; they must co-sign, and a majority of
    /// the current roll is required.
    pub deciders: Vec<DidKey>,
    pub dissolved_at: Timestamp,
}

/// How a [`MembershipChange`] moves the roll. The asymmetry mirrors
/// [`AmendmentKind`]: growing the polity is easier than shrinking it, and — as
/// with the covenant it sits under — a member cannot be conscripted (a join needs
/// the joiner's consent) nor forced to stay (a voluntary exit is the member's own
/// signed act).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipKind {
    /// Admit a new member. Needs a majority of the current roll **and** the
    /// joiner's own co-signature (consent — no conscription).
    Admit,
    /// A member leaves of their own accord. Signed by that member alone; the
    /// roll cannot force them to stay.
    DepartVoluntary,
    /// The roll removes a member against their will. The capture-relevant
    /// contraction, held to the higher bar (a majority **+ 1**), with the target
    /// excluded from the deciders.
    Expel,
}

impl MembershipKind {
    /// A contraction shrinks the roll — the capture-relevant direction, held to
    /// the self-govern floor. (`Admit` grows it.)
    pub fn is_contraction(&self) -> bool {
        matches!(self, MembershipKind::DepartVoluntary | MembershipKind::Expel)
    }
}

/// A change to a polity's membership roll after founding. The founding roll is
/// fixed by the genesis charter; every later admission, voluntary departure, and
/// expulsion is one of these records, so the *current* roll is the founding roll
/// plus admits minus departures, in order. The authority bar depends on `kind`
/// (see [`MembershipKind`]); a contraction is additionally refused when it would
/// leave the roll unable to form the `majority + 1` its own contraction rules
/// require (the self-govern floor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipChange {
    pub polity: DidKey,
    pub kind: MembershipKind,
    /// The member being admitted, departing, or expelled.
    pub member: DidKey,
    pub rationale: String,
    /// The members deciding the change; they must co-sign. Empty for a
    /// `DepartVoluntary` (the departing `member` is the sole signer). For `Admit`
    /// the joiner co-signs *in addition* to the deciders; for `Expel` the target
    /// is excluded from the deciders.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// A conferred internal role. Offices are set by a deliberate, signed act of the
/// roll and held until relinquished — never a readout of earned reputation
/// (design §3). The four canonical roles are named; `Other` lets a polity confer
/// a role of its own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Office {
    /// Presides over the roll and its deliberations.
    Convener,
    /// Presides over the intra-community induction / re-affirmation rite.
    Inductor,
    /// Authorizes the polity's envoys for kith-covenant recognition.
    EnvoyAuthorizer,
    /// Custodian of the charter log.
    CharterKeeper,
    /// A polity-specific office named by the string.
    Other(String),
}

/// How an [`OfficeChange`] moves an office holding. The three-kind shape mirrors
/// [`MembershipKind`], for the same reasons: an office is a duty, so it cannot be
/// conscripted (a conferral needs the holder's consent) nor forced to continue (a
/// holder may resign); and the roll may remove an officer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfficeKind {
    /// Confer the office on a member. Needs a majority of the current roll **and**
    /// the holder's own co-signature (consent).
    Confer,
    /// The holder steps down of their own accord. Signed by the holder alone.
    Resign,
    /// The roll removes the holder from the office. Signed by a majority of the
    /// roll, with the holder excluded from the deciders.
    Revoke,
}

impl OfficeKind {
    /// Whether this ends a holding (`Resign` / `Revoke`) rather than creating one
    /// (`Confer`).
    pub fn is_relinquishment(&self) -> bool {
        matches!(self, OfficeKind::Resign | OfficeKind::Revoke)
    }
}

/// A change to who holds an internal office. The founding roll confers the first
/// offices (Q3: the roll is the standing conferring authority, so no office ever
/// confers another and there is no regress); thereafter every conferral,
/// resignation, and revocation is one of these records. An office holding is
/// **effective only while its holder is on the current roll** — it lapses if the
/// holder departs and revives if they return; the roll ends it for good by
/// `Revoke`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeChange {
    pub polity: DidKey,
    pub kind: OfficeKind,
    pub office: Office,
    /// The member the office is conferred on, resigned by, or revoked from.
    pub holder: DidKey,
    pub rationale: String,
    /// The members deciding the change; they must co-sign. Empty for a `Resign`
    /// (the `holder` is the sole signer). For `Confer` the holder co-signs *in
    /// addition* (consent); for `Revoke` the holder is excluded.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// Whether an affirmation is a member's first acquisition of the substrate or a
/// periodic renewal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AffirmationKind {
    /// A member acquires the substrate for the first time — the induction rite.
    /// Witnessed by a majority of the roll (the community receives them).
    Induction,
    /// A member renews their standing affirmation of the substrate. Self-signed;
    /// its freshness decays, so a community that stops re-affirming shows erosion.
    ReAffirmation,
}

/// A member affirming the polity's substrate — the "transmission" half of
/// longevity (design §5): induction brings a new member into the substrate, and
/// re-affirmation renews it. Admission (a [`MembershipChange`]) puts someone on
/// the roll; **induction is the separate step by which they acquire the
/// substrate**, so a member can be on the roll yet un-inducted. Founding members
/// are inducted implicitly by ratifying the charter (dated `founded_at`), so they
/// need no `Induction` record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affirmation {
    pub polity: DidKey,
    pub member: DidKey,
    pub kind: AffirmationKind,
    /// For an `Induction`, the roll members who witness (receive) the inductee;
    /// they co-sign alongside the inductee. Empty for a `ReAffirmation` (the
    /// member is the sole signer).
    pub witnesses: Vec<DidKey>,
    pub affirmed_at: Timestamp,
}
