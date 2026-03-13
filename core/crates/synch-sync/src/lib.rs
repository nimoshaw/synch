//! synch-sync — in-memory Vault sync with Delta log and version vectors.
//!
//! Implements:
//!   - Version vector (logical clocks for causality tracking)
//!   - Delta log entries (CRDT-inspired operations)
//!   - In-memory Vault state with apply/replay
//!   - Conflict detection

pub mod error;
pub mod version_vector;
pub mod delta;
pub mod vault;
pub mod handshake;
pub mod contract_manager;
pub mod secure;
pub mod net;

pub use error::SyncError;
pub use version_vector::VersionVector;
pub use delta::{DeltaEntry, EntryOperation, DeltaBatch};
pub use vault::{Vault, VaultEntry};
