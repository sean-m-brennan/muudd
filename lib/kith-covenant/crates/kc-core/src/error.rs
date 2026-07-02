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

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KcError {
    #[error("canonical encoding failed: {0}")]
    CanonicalEncode(String),

    #[error("canonical decoding failed: {0}")]
    CanonicalDecode(String),

    #[error("signature verification failed")]
    SignatureInvalid,

    #[error("unsupported algorithm: {0}")]
    UnsupportedAlg(String),

    #[error("key error: {0}")]
    Key(String),

    #[error("cid mismatch: expected {expected}, got {actual}")]
    CidMismatch { expected: String, actual: String },

    #[error("invalid did:key: {0}")]
    InvalidDid(String),

    #[error("archive error: {0}")]
    Archive(String),
}

pub type KcResult<T> = Result<T, KcError>;
