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

pub mod canonical;
pub mod cid;
pub mod community;
#[cfg(feature = "source-protection")]
pub mod custody;
pub mod did;
pub mod error;
pub mod keys;
#[cfg(feature = "source-protection")]
pub mod pedersen;
#[cfg(feature = "source-protection")]
pub mod ring;
pub mod signature;
pub mod time;

pub use cid::Cid;
pub use community::{CommunityIdentity, SuccessorLink};
#[cfg(feature = "source-protection")]
pub use custody::{
    didkey_encryption_resolver, RevealedShare, SealedShare, SourcePreimage, ThresholdCustody,
};
pub use did::DidKey;
pub use error::{KcError, KcResult};
pub use keys::{Alg, PrivKey, PubKey, Signature};
#[cfg(feature = "source-protection")]
pub use pedersen::{Blinding, BoundProof, Commitment, RANGE_BITS};
#[cfg(feature = "source-protection")]
pub use ring::{Bytes32, DlEqProof, KeyImage, RingSignature, RingSigned};
pub use signature::{SignatureEntry, Signed};
pub use time::Timestamp;
