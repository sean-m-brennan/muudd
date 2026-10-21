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

pub mod authority;
pub mod dump;
pub mod entry;
pub mod store;
pub mod verify;

pub use authority::{
    authority_violations, authorize, authorize_with_caps, check_authority, insider_keys,
    insider_keys_upto, mediator_encryption_keys, mediator_encryption_resolver,
    probationary_at, ExaminerCaps, MediatorCaps, ObserverCaps,
    DEFAULT_EXAMINER_MAX_TERM_SECONDS,
    DEFAULT_EXAMINER_PANEL_MIN, DEFAULT_EXAMINER_POOL_FLOOR,
    DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS, DEFAULT_MEDIATOR_MAX_TERM_SECONDS,
    DEFAULT_MEDIATOR_POOL_FLOOR, DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
    DEFAULT_OBSERVER_MAX_TERM_SECONDS,
    DEFAULT_OBSERVER_POOL_FLOOR, DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS, DEFAULT_PANEL_MIN,
    DEFAULT_QUORUM_THRESHOLD, DEFAULT_REENTRY_WITNESS_MIN,
    DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS, DEFAULT_STRUCTURED_SKEPTICISM_MIN_CORROBORATORS,
    DEFAULT_COMMONS_ESCALATION_CYCLES, DEFAULT_COMMONS_EXCLUSION_DWELL_SECONDS,
    DEFAULT_LIFE_CRITICAL_ESCALATION_CYCLES,
    DEFAULT_WAITING_PERIOD_SECONDS, MAX_DECISION_SEAL_SECONDS,
    MAX_INVOKED_SKEPTICISM_HOLD_SECONDS, MIN_DECISION_SEAL_SECONDS,
    MIN_INVOKED_SKEPTICISM_HOLD_SECONDS,
};
pub use entry::ArchiveEntry;
pub use store::{Archive, InMemoryArchive};
pub use verify::verify_all;

#[cfg(test)]
mod tests;
