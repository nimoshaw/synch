use crate::error::SyncError;
use serde::{Deserialize, Serialize};

/// Operation type for a delta entry (mirrors proto EntryOperation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryOperation {
    Create,
    Modify,
    Delete,
    Rename { old_path: String },
}

/// A single atomic change in the Vault delta log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEntry {
    /// Relative path from Vault root (e.g., "notes/daily/2026-03-13.md")
    pub path: String,
    /// What kind of change
    pub operation: EntryOperation,
    /// Blake3 hash of content (as hex), None for Delete
    pub content_hash: Option<String>,
    /// Inline content bytes (for small files)
    pub delta_bytes: Option<Vec<u8>>,
    /// File size in bytes
    pub size: u64,
    /// Modification time (Unix epoch millis)
    pub modified_at: u64,
    /// Which node originated this change
    pub origin_node_id: String,
    /// Sequence number from the origin node's version vector
    pub origin_sequence: u64,
}

impl DeltaEntry {
    pub fn new_create(
        path: impl Into<String>,
        content: Vec<u8>,
        origin_node_id: impl Into<String>,
        origin_sequence: u64,
        modified_at: u64,
    ) -> Self {
        let content_hash = hex::encode(synch_crypto::blake3_hash(&content));
        let size = content.len() as u64;
        Self {
            path: path.into(),
            operation: EntryOperation::Create,
            content_hash: Some(content_hash),
            delta_bytes: Some(content),
            size,
            modified_at,
            origin_node_id: origin_node_id.into(),
            origin_sequence,
        }
    }

    pub fn new_modify(
        path: impl Into<String>,
        content: Vec<u8>,
        origin_node_id: impl Into<String>,
        origin_sequence: u64,
        modified_at: u64,
    ) -> Self {
        let content_hash = hex::encode(synch_crypto::blake3_hash(&content));
        let size = content.len() as u64;
        Self {
            path: path.into(),
            operation: EntryOperation::Modify,
            content_hash: Some(content_hash),
            delta_bytes: Some(content),
            size,
            modified_at,
            origin_node_id: origin_node_id.into(),
            origin_sequence,
        }
    }

    pub fn new_delete(
        path: impl Into<String>,
        origin_node_id: impl Into<String>,
        origin_sequence: u64,
        modified_at: u64,
    ) -> Self {
        Self {
            path: path.into(),
            operation: EntryOperation::Delete,
            content_hash: None,
            delta_bytes: None,
            size: 0,
            modified_at,
            origin_node_id: origin_node_id.into(),
            origin_sequence,
        }
    }

    pub fn new_rename(
        old_path: impl Into<String>,
        new_path: impl Into<String>,
        origin_node_id: impl Into<String>,
        origin_sequence: u64,
        modified_at: u64,
    ) -> Self {
        let old = old_path.into();
        Self {
            path: new_path.into(),
            operation: EntryOperation::Rename { old_path: old },
            content_hash: None,
            delta_bytes: None,
            size: 0,
            modified_at,
            origin_node_id: origin_node_id.into(),
            origin_sequence,
        }
    }
}

/// A batch of delta entries for a single sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaBatch {
    pub vault_id: String,
    pub base_version: u64,
    pub target_version: u64,
    pub changes: Vec<DeltaEntry>,
}

impl DeltaBatch {
    pub fn new(vault_id: impl Into<String>, base_version: u64, target_version: u64) -> Self {
        Self {
            vault_id: vault_id.into(),
            base_version,
            target_version,
            changes: Vec::new(),
        }
    }

    pub fn with_changes(mut self, changes: Vec<DeltaEntry>) -> Self {
        self.changes = changes;
        self
    }

    pub fn add_change(&mut self, entry: DeltaEntry) {
        self.changes.push(entry);
    }

    pub fn validate(&self) -> Result<(), SyncError> {
        if self.target_version <= self.base_version && !self.changes.is_empty() {
            return Err(SyncError::InvalidDelta(format!(
                "target_version {} must be > base_version {}",
                self.target_version, self.base_version
            )));
        }
        Ok(())
    }
}
