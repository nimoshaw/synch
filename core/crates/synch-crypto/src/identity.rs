use crate::error::CryptoError;
use crate::hash::blake3_fingerprint;
use crate::keys::{Ed25519KeyPair, X25519KeyPair};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Node type matching proto definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    Unspecified,
    Agent,
    Human,
    Bridge,
    Plugin,
    Mobile,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Unspecified => write!(f, "unspecified"),
            NodeType::Agent => write!(f, "agent"),
            NodeType::Human => write!(f, "human"),
            NodeType::Bridge => write!(f, "bridge"),
            NodeType::Plugin => write!(f, "plugin"),
            NodeType::Mobile => write!(f, "mobile"),
        }
    }
}

/// Node key bundle: Ed25519 (identity) + X25519 (encryption)
pub struct NodeKey {
    pub identity: Ed25519KeyPair,
    pub exchange: X25519KeyPair,
}

impl NodeKey {
    /// Generate a new node key bundle
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self {
            identity: Ed25519KeyPair::generate()?,
            exchange: X25519KeyPair::generate()?,
        })
    }

    /// Get the Ed25519 public key (32 bytes)
    pub fn identity_public_key(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    /// Get the X25519 public key (32 bytes)
    pub fn exchange_public_key(&self) -> [u8; 32] {
        self.exchange.public_key_bytes()
    }

    /// Get fingerprint of identity key
    pub fn fingerprint(&self) -> String {
        self.identity.fingerprint()
    }
}

/// Node identity descriptor (serializable subset of proto NodeIdentity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Format: "agent://{fingerprint}" / "plugin://{fingerprint}" etc.
    pub node_id: String,
    /// Ed25519 public key (32 bytes as hex)
    pub public_key_hex: String,
    /// Node type
    pub node_type: NodeType,
    /// Host platform (e.g., "vcp-agent", "android")
    pub platform: String,
    /// Capabilities declared by this node
    pub capabilities: Vec<String>,
    /// Unix epoch millis at registration
    pub registered_at: u64,
    /// Human-readable display name
    pub display_name: String,
    /// Parent node ID (Optional)
    pub parent_node_id: Option<String>,
    /// Extension metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl NodeIdentity {
    /// Create a new NodeIdentity from a key and metadata.
    pub fn new(
        key: &NodeKey,
        node_type: NodeType,
        platform: impl Into<String>,
        display_name: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Self {
        let pk = key.identity_public_key();
        let fingerprint = blake3_fingerprint(&pk);
        let prefix = match &node_type {
            NodeType::Agent => "agent",
            NodeType::Human => "user",
            NodeType::Bridge => "bridge",
            NodeType::Plugin => "plugin",
            NodeType::Mobile => "mobile",
            NodeType::Unspecified => "node",
        };
        let node_id = format!("{}://{}", prefix, fingerprint);
        let registered_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            node_id,
            public_key_hex: hex::encode(pk),
            node_type,
            platform: platform.into(),
            display_name: display_name.into(),
            capabilities,
            registered_at,
            parent_node_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set parent node ID
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_node_id = Some(parent_id.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the raw public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        hex::decode(&self.public_key_hex).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let key = NodeKey::generate().expect("key gen failed");
        let identity = NodeIdentity::new(
            &key,
            NodeType::Agent,
            "test-platform",
            "Test Agent",
            vec!["chat".to_string(), "vault-sync".to_string()],
        );
        assert!(identity.node_id.starts_with("agent://"));
        assert_eq!(identity.public_key_bytes().len(), 32);
        println!("NodeIdentity: {:?}", identity.node_id);
    }
}
