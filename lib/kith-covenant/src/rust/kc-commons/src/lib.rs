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

//! Commons governance records (protocol component §7), v1 slice: the commons
//! registry, self-reported usage accounting, and the graduated-sanction ledger.
//!
//! Applies Ostrom's design principles (commons_federation.md) to the five space
//! commons: clear boundaries (principle 1) via [`CommonsRegistration`]'s
//! appropriator set; proportional provision (principle 2) via its
//! `provision_rule` and the [`UsageRecord`] accounting that feeds free-rider
//! detection; and graduated sanctions (principle 5) via the append-only
//! [`CommonsSanction`] ladder, which is a commons-abuse analogue of the §5
//! dispute ladder (a community drawing down or polluting a shared resource, not
//! two communities in conflict).
//!
//! This crate carries only the *record* shapes plus the pure sanction-ladder
//! ordering ([`sanction_rank`], [`is_second_class_level`]). The structural rules
//! the research requires — a commons registration is boundary-defined and
//! second-class; a usage record is self-reported by an appropriator; a sanction
//! follows the current level and does not skip, and its heavier levels
//! (restriction, exclusion) carry a distinct-community quorum — are enforced at
//! the archive's authority layer (`kc_archive::authority`), because they range
//! over the whole archive (the referenced registration, the ledger history).
//!
//! Light-lag decision classes (commons_federation.md §"Light-lag-honest decision
//! classes") are realized here as the existing multi-community quorum pass:
//! second-class commons records validate only with a quorum spanning distinct
//! communities.
//!
//! **Constitutional reclassification** was on this list and should not have stayed
//! there. Moving a matter into or out of the compact-incapable list is
//! `kc_constitution::CompactIncapableAmendment` with `AmendmentKind::{Add, Remove}`,
//! built in the §9 slice with a genesis-immutability guard and an asymmetric bar on
//! contraction. It was deferred here, built there, and the deferral note outlived the
//! deferral by several slices, from which the tracker then copied it.
//!
//! Deferred still: the full CRDT/BFT consensus mechanics and the signature-latency
//! window, which need real distributed hosts.

mod record;

#[cfg(test)]
mod tests;

pub use record::{
    is_life_critical, is_second_class_level, sanction_rank, AppropriatorLevel, CommonsEscalation,
    CommonsKind, CommonsRegistration, CommonsSanction, SanctionLevel, SealedUsage, UsageFigures,
    UsageOpened, UsageRecord,
};
