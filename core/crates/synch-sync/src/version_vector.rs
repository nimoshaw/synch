use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Version vector maps node_id → highest sequence number seen from that node.
/// This provides a causal ordering for detecting conflicts in distributed systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionVector {
    clocks: HashMap<String, u64>,
}

impl VersionVector {
    /// Create an empty version vector
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the clock for the given node and return the new sequence number
    pub fn increment(&mut self, node_id: &str) -> u64 {
        let entry = self.clocks.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Get the current sequence number for a node (0 if not seen)
    pub fn get(&self, node_id: &str) -> u64 {
        *self.clocks.get(node_id).unwrap_or(&0)
    }

    /// Merge another vector into this one, taking the max for each node
    pub fn merge(&mut self, other: &VersionVector) {
        for (node_id, &seq) in &other.clocks {
            let entry = self.clocks.entry(node_id.clone()).or_insert(0);
            *entry = (*entry).max(seq);
        }
    }

    /// Check if this vector dominates (>=) the other — i.e., we've seen everything `other` has seen
    pub fn dominates(&self, other: &VersionVector) -> bool {
        for (node_id, &other_seq) in &other.clocks {
            if self.get(node_id) < other_seq {
                return false;
            }
        }
        true
    }

    /// Returns true if the two vectors are concurrent (neither dominates the other)
    pub fn is_concurrent_with(&self, other: &VersionVector) -> bool {
        !self.dominates(other) && !other.dominates(self)
    }

    /// Update a specific node's sequence, only if it's newer
    pub fn update(&mut self, node_id: &str, seq: u64) {
        let entry = self.clocks.entry(node_id.to_string()).or_insert(0);
        if seq > *entry {
            *entry = seq;
        }
    }

    /// Get the maximum "global version" (the sum of all clocks, for simplicity)
    pub fn global_version(&self) -> u64 {
        self.clocks.values().sum()
    }

    /// Get the raw clock map
    pub fn clocks(&self) -> &HashMap<String, u64> {
        &self.clocks
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.increment("node-A");
        vv1.increment("node-A"); // A: 2

        let mut vv2 = VersionVector::new();
        vv2.increment("node-A"); // A: 1
        vv2.increment("node-B"); // B: 1

        // vv1 has A:2, vv2 has A:1, B:1 — concurrent
        assert!(vv1.is_concurrent_with(&vv2));

        vv1.merge(&vv2); // vv1 now has A:2, B:1
        assert_eq!(vv1.get("node-A"), 2);
        assert_eq!(vv1.get("node-B"), 1);
        assert!(vv1.dominates(&vv2));
    }
}
