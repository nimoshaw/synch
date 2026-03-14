use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Vault not found: {0}")]
    VaultNotFound(String),

    #[error("Entry not found: {path}")]
    EntryNotFound { path: String },

    #[error(
        "Conflict detected at path: {path} — local seq {local_seq} vs remote seq {remote_seq}"
    )]
    Conflict {
        path: String,
        local_seq: u64,
        remote_seq: u64,
    },

    #[error("Invalid delta: {0}")]
    InvalidDelta(String),

    #[error("Version mismatch: base version {expected} but vault is at {actual}")]
    VersionMismatch { expected: u64, actual: u64 },

    #[error("Crypto error: {0}")]
    Crypto(String),
}
