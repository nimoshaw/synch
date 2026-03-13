use synch_crypto::contract::Contract;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeStatus {
    Initiated,
    Received,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeState {
    pub contract: Contract,
    pub status: HandshakeStatus,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeManager {
    /// Maps contract_id to HandshakeState
    pub handshakes: HashMap<String, HandshakeState>,
}

impl HandshakeManager {
    pub fn new() -> Self {
        Self {
            handshakes: HashMap::new(),
        }
    }

    pub fn update_state(&mut self, state: HandshakeState) {
        self.handshakes.insert(state.contract.contract_id.clone(), state);
    }

    pub fn get_state(&self, contract_id: &str) -> Option<&HandshakeState> {
        self.handshakes.get(contract_id)
    }

    pub fn list_pending(&self) -> Vec<&HandshakeState> {
        self.handshakes
            .values()
            .filter(|s| s.status == HandshakeStatus::Initiated || s.status == HandshakeStatus::Received)
            .collect()
    }

    pub fn remove(&mut self, contract_id: &str) {
        self.handshakes.remove(contract_id);
    }
}
