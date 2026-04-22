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
