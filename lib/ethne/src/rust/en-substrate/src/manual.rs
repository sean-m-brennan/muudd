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

//! A manual-founding provider: a human convener asserts the roll and substrate
//! directly, with no emergent computation. It is the simplest possible
//! [`SubstrateProvider`], and it is what demonstrates that a polity can be
//! founded — and thence recognized by kith-covenant — with **no autonomous_trust
//! anywhere in the dependency graph**.

use en_charter::{BoundarySource, Substrate};
use en_core::{DidKey, KcResult};

use crate::provider::{FoundingCandidate, MemberCandidate, SubstrateProvider};

/// A provider whose candidate is supplied outright by a convener.
#[derive(Debug, Clone)]
pub struct ManualFounding {
    candidate: FoundingCandidate,
}

impl ManualFounding {
    /// A convener asserts the membership roll and substrate. Asserted members are
    /// given full reputation (there is no emergent track record to read) and no
    /// guardian; use [`ManualFounding::from_candidate`] for finer control.
    ///
    /// Full reputation is `Some(1.0)`, an assertion, and deliberately not `None`: a
    /// convener vouching for a roll *is* the reading here, where `None` would claim
    /// the opposite — that nothing is known about these members.
    pub fn new(members: Vec<DidKey>, substrate: Substrate) -> Self {
        let members = members
            .into_iter()
            .map(|did| MemberCandidate {
                did,
                reputation: Some(1.0),
                derived: false,
                guardian: None,
                attested_at: None,
            })
            .collect();
        Self {
            candidate: FoundingCandidate {
                members,
                substrate: Some(substrate),
                // Nobody but the convener attests this roll (design **D21**). The
                // charter will say so for as long as it exists.
                source: BoundarySource::ConvenerAsserted,
            },
        }
    }

    /// Wrap an explicitly-built candidate (e.g. to assert per-member guardians).
    pub fn from_candidate(candidate: FoundingCandidate) -> Self {
        Self { candidate }
    }
}

impl SubstrateProvider for ManualFounding {
    fn founding_candidate(&self) -> KcResult<FoundingCandidate> {
        Ok(self.candidate.clone())
    }

    fn member_reputation(&self, member: &DidKey) -> Option<f64> {
        self.candidate.members.iter().find(|m| &m.did == member).and_then(|m| m.reputation)
    }
}
