//! synch-sync — in-memory Vault sync with Delta log and version vectors.
//!
//! Implements:
//!   - Version vector (logical clocks for causality tracking)
//!   - Delta log entries (CRDT-inspired operations)
//!   - In-memory Vault state with apply/replay
//!   - Conflict detection

pub mod contract_manager;
pub mod delta;
pub mod error;
pub mod handshake;
pub mod net;
pub mod secure;
pub mod vault;
pub mod version_vector;

pub use delta::{DeltaBatch, DeltaEntry, EntryOperation};
pub use error::SyncError;
pub use vault::{Vault, VaultEntry};
pub use version_vector::VersionVector;
