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

mod anti_spiral;
mod examiner;
mod runner;
mod sponsorship;
mod witness_record;

#[cfg(test)]
mod tests;

pub use anti_spiral::{
    AntiSpiralFeature, AntiSpiralFeatureClaim, EvidenceKind, EvidenceRef, FeatureAcceptance,
    REQUIRED_FEATURES,
};
pub use examiner::{ExaminerEntry, ExaminerRegistry};
pub use runner::{InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext};
pub use sponsorship::SponsorshipRecord;
pub use witness_record::{
    ExaminationExchange, InterCommunityWitnessRecord, NonAdmissionRecord, RejectionDetail,
    SequencedRite,
};
