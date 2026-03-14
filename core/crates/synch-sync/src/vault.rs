use crate::delta::{DeltaBatch, DeltaEntry, EntryOperation};
use crate::error::SyncError;
use crate::version_vector::VersionVector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An entry stored within a Vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// Relative path from vault root
    pub path: String,
    /// Content or None if deleted
    pub content: Option<Vec<u8>>,
    /// Blake3 hash of content (hex), or None if deleted
    pub content_hash: Option<String>,
    /// Size in bytes
    pub size: u64,
    /// Last modified (Unix epoch millis)
    pub modified_at: u64,
    /// Which node last modified this entry
    pub last_modified_by: String,
    /// Sequence number of the last modification
    pub last_modified_seq: u64,
}

impl VaultEntry {
    pub fn is_deleted(&self) -> bool {
        self.content.is_none() && self.content_hash.is_none()
    }
}

/// A conflict record for reporting to callers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub path: String,
    pub local_node_id: String,
    pub local_seq: u64,
    pub local_hash: Option<String>,
    pub remote_node_id: String,
    pub remote_seq: u64,
    pub remote_hash: Option<String>,
}

/// In-memory Vault state with delta log replay and conflict detection
pub struct Vault {
    /// Unique Vault identifier
    pub vault_id: String,
    /// Current version (monotonically increasing)
    pub version: u64,
    /// Per-node causal clocks
    pub version_vector: VersionVector,
    /// Entry map: path → VaultEntry
    entries: HashMap<String, VaultEntry>,
    /// Append-only delta log
    delta_log: Vec<DeltaEntry>,
    /// Conflict records
    pub conflicts: Vec<ConflictRecord>,
}

impl Vault {
    /// Create a new empty Vault
    pub fn new(vault_id: impl Into<String>) -> Self {
        Self {
            vault_id: vault_id.into(),
            version: 0,
            version_vector: VersionVector::new(),
            entries: HashMap::new(),
            delta_log: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Get a snapshot of all live (non-deleted) entries
    pub fn live_entries(&self) -> Vec<&VaultEntry> {
        self.entries.values().filter(|e| !e.is_deleted()).collect()
    }

    /// Get an entry by path
    pub fn get_entry(&self, path: &str) -> Option<&VaultEntry> {
        self.entries.get(path)
    }

    /// Get the full delta log (for sync purposes)
    pub fn delta_log(&self) -> &[DeltaEntry] {
        &self.delta_log
    }

    /// Get deltas since a specific global version
    pub fn deltas_since(&self, base_version: u64) -> Vec<&DeltaEntry> {
        // Simple approach: the delta_log is ordered by insertion
        // In a real system we'd track per-entry version numbers
        self.delta_log.iter().skip(base_version as usize).collect()
    }

    /// Apply a single DeltaEntry to the vault.
    /// Performs conflict detection using version vectors.
    pub fn apply_delta(&mut self, entry: DeltaEntry) -> Result<(), SyncError> {
        // Check for duplicate or older deltas from the same node
        if self.version_vector.get(&entry.origin_node_id) >= entry.origin_sequence {
            // Already seen this or a newer update from this node — ignore
            return Ok(());
        }

        // Check for concurrent modifications (conflict detection)
        if let Some(existing) = self.entries.get(&entry.path) {
            let existing_seq = existing.last_modified_seq;
            let existing_node = &existing.last_modified_by;

            // If the existing entry was made by a DIFFERENT node, check for conflict
            if existing_node != &entry.origin_node_id {
                // Last-Write-Wins using modified_at timestamp (LWW strategy)
                let existing_modified = existing.modified_at;

                if entry.modified_at < existing_modified {
                    // Incoming delta is older — record conflict and skip apply (local wins)
                    self.conflicts.push(ConflictRecord {
                        path: entry.path.clone(),
                        local_node_id: existing_node.clone(),
                        local_seq: existing_seq,
                        local_hash: existing.content_hash.clone(),
                        remote_node_id: entry.origin_node_id.clone(),
                        remote_seq: entry.origin_sequence,
                        remote_hash: entry.content_hash.clone(),
                    });

                    self.version_vector
                        .update(&entry.origin_node_id, entry.origin_sequence);
                    self.delta_log.push(entry);
                    self.version += 1;
                    return Ok(());
                } else {
                    // Incoming is newer or same timestamp — record conflict but apply it
                    self.conflicts.push(ConflictRecord {
                        path: entry.path.clone(),
                        local_node_id: existing_node.clone(),
                        local_seq: existing_seq,
                        local_hash: existing.content_hash.clone(),
                        remote_node_id: entry.origin_node_id.clone(),
                        remote_seq: entry.origin_sequence,
                        remote_hash: entry.content_hash.clone(),
                    });
                }
            }
        }

        // Update version vector
        self.version_vector
            .update(&entry.origin_node_id, entry.origin_sequence);

        // Apply the operation
        match &entry.operation {
            EntryOperation::Create | EntryOperation::Modify => {
                let vault_entry = VaultEntry {
                    path: entry.path.clone(),
                    content: entry.delta_bytes.clone(),
                    content_hash: entry.content_hash.clone(),
                    size: entry.size,
                    modified_at: entry.modified_at,
                    last_modified_by: entry.origin_node_id.clone(),
                    last_modified_seq: entry.origin_sequence,
                };
                self.entries.insert(entry.path.clone(), vault_entry);
            }
            EntryOperation::Delete => {
                // Tombstone: keep the entry but mark as deleted
                let vault_entry = VaultEntry {
                    path: entry.path.clone(),
                    content: None,
                    content_hash: None,
                    size: 0,
                    modified_at: entry.modified_at,
                    last_modified_by: entry.origin_node_id.clone(),
                    last_modified_seq: entry.origin_sequence,
                };
                self.entries.insert(entry.path.clone(), vault_entry);
            }
            EntryOperation::Rename { old_path } => {
                if let Some(old_entry) = self.entries.remove(old_path) {
                    let vault_entry = VaultEntry {
                        path: entry.path.clone(),
                        content: old_entry.content,
                        content_hash: old_entry.content_hash,
                        size: old_entry.size,
                        modified_at: entry.modified_at,
                        last_modified_by: entry.origin_node_id.clone(),
                        last_modified_seq: entry.origin_sequence,
                    };
                    self.entries.insert(entry.path.clone(), vault_entry);
                }
            }
        }

        // Append to delta log
        self.delta_log.push(entry);
        self.version += 1;

        Ok(())
    }

    /// Apply a full DeltaBatch, validating base version first
    pub fn apply_batch(&mut self, batch: DeltaBatch) -> Result<usize, SyncError> {
        batch.validate()?;

        if batch.base_version > self.version {
            return Err(SyncError::VersionMismatch {
                expected: batch.base_version,
                actual: self.version,
            });
        }

        let count = batch.changes.len();
        for entry in batch.changes {
            self.apply_delta(entry)?;
        }
        Ok(count)
    }

    /// Prune the delta log by keeping only the latest delta for each unique path.
    /// This is a simple compaction strategy to prevent unbounded log growth.
    /// WARNING: Pruning can make it impossible for very out-of-sync nodes to
    /// catch up using only the delta log; they would need a full state transfer.
    pub fn compact_log(&mut self) {
        let mut latest_deltas: HashMap<String, usize> = HashMap::new();

        // Find the index of the latest delta for each path
        for (i, entry) in self.delta_log.iter().enumerate() {
            latest_deltas.insert(entry.path.clone(), i);
        }

        // Collect the indices to keep, maintaining order
        let mut keep_indices: Vec<usize> = latest_deltas.values().cloned().collect();
        keep_indices.sort_unstable();

        let mut new_log = Vec::with_capacity(keep_indices.len());
        for idx in keep_indices {
            new_log.push(self.delta_log[idx].clone());
        }

        self.delta_log = new_log;
        // Note: we don't reset 'self.version' as it represents the causal history count,
        // but the log itself is now smaller.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::DeltaEntry;

    fn now_millis() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    #[test]
    fn test_vault_create_and_modify() {
        let mut vault = Vault::new("test-vault-001");
        let node_a = "node-A";

        // Create a file
        let entry1 = DeltaEntry::new_create(
            "notes/hello.md",
            b"# Hello World".to_vec(),
            node_a,
            1,
            now_millis(),
        );
        vault.apply_delta(entry1).unwrap();
        assert_eq!(vault.version, 1);
        assert!(vault.get_entry("notes/hello.md").is_some());

        // Modify it
        let entry2 = DeltaEntry::new_modify(
            "notes/hello.md",
            b"# Hello Updated".to_vec(),
            node_a,
            2,
            now_millis() + 100,
        );
        vault.apply_delta(entry2).unwrap();
        assert_eq!(vault.version, 2);

        // Delete it
        let entry3 = DeltaEntry::new_delete("notes/hello.md", node_a, 3, now_millis() + 200);
        vault.apply_delta(entry3).unwrap();
        assert_eq!(vault.version, 3);
        assert!(vault.get_entry("notes/hello.md").unwrap().is_deleted());

        // Live entries should be empty
        assert!(vault.live_entries().is_empty());

        println!("Delta log length: {}", vault.delta_log().len());
        println!("Vault version: {}", vault.version);
    }

    #[test]
    fn test_vault_sync_batch() {
        let mut vault = Vault::new("test-vault-002");

        let changes = vec![
            DeltaEntry::new_create("a.txt", b"aaa".to_vec(), "node-B", 1, 1000),
            DeltaEntry::new_create("b.txt", b"bbb".to_vec(), "node-B", 2, 1001),
        ];

        let batch = DeltaBatch::new("test-vault-002", 0, 2).with_changes(changes);
        let applied = vault.apply_batch(batch).unwrap();
        assert_eq!(applied, 2);
        assert_eq!(vault.live_entries().len(), 2);
    }

    #[test]
    fn test_vault_log_compaction() {
        let mut vault = Vault::new("test-vault-compact");
        let node_a = "node-A";

        // Create, then modify multiple times
        vault
            .apply_delta(DeltaEntry::new_create(
                "a.txt",
                b"v1".to_vec(),
                node_a,
                1,
                1000,
            ))
            .unwrap();
        vault
            .apply_delta(DeltaEntry::new_modify(
                "a.txt",
                b"v2".to_vec(),
                node_a,
                2,
                1100,
            ))
            .unwrap();
        vault
            .apply_delta(DeltaEntry::new_modify(
                "a.txt",
                b"v3".to_vec(),
                node_a,
                3,
                1200,
            ))
            .unwrap();

        vault
            .apply_delta(DeltaEntry::new_create(
                "b.txt",
                b"b1".to_vec(),
                node_a,
                4,
                1300,
            ))
            .unwrap();

        assert_eq!(vault.delta_log.len(), 4);
        assert_eq!(vault.version, 4);

        vault.compact_log();

        // Should now have only 2 deltas (latest for a.txt and b.txt)
        assert_eq!(vault.delta_log.len(), 2);
        assert_eq!(vault.delta_log[0].path, "a.txt");
        assert_eq!(vault.delta_log[0].origin_sequence, 3);
        assert_eq!(vault.delta_log[1].path, "b.txt");

        // Version (causal clock) should remain 4
        assert_eq!(vault.version, 4);
        assert_eq!(
            vault.get_entry("a.txt").unwrap().content,
            Some(b"v3".to_vec())
        );
    }
}
