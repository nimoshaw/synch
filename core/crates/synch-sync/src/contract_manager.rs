use synch_crypto::contract::{Contract, ContractStatus};
use synch_crypto::keys::{Ed25519KeyPair, X25519KeyPair};
use synch_crypto::ratchet::DoubleRatchet;
use crate::error::SyncError;
use crate::handshake::{HandshakeManager, HandshakeState, HandshakeStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HandshakePolicy {
    /// Public keys of trusted nodes (hex or raw)
    pub trusted_nodes: Vec<Vec<u8>>,
    /// Capabilities that are automatically accepted for trusted nodes
    pub auto_accept_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractManager {
    pub handshake_manager: HandshakeManager,
    pub policy: HandshakePolicy,
    /// Active Double Ratchet sessions indexed by contract_id
    pub active_ratchets: HashMap<String, DoubleRatchet>,
}

impl ContractManager {
    pub fn new() -> Self {
        Self {
            handshake_manager: HandshakeManager::new(),
            policy: HandshakePolicy::default(),
            active_ratchets: HashMap::new(),
        }
    }

    pub fn set_policy(&mut self, policy: HandshakePolicy) {
        self.policy = policy;
    }

    /// Persistence: Save manager state to a JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SyncError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SyncError::Crypto(format!("Failed to serialize ContractManager: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| SyncError::Crypto(format!("Failed to write ContractManager to file: {}", e)))?;
        Ok(())
    }

    /// Persistence: Load manager state from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SyncError> {
        let json = fs::read_to_string(path)
            .map_err(|e| SyncError::Crypto(format!("Failed to read ContractManager file: {}", e)))?;
        let manager: Self = serde_json::from_str(&json)
            .map_err(|e| SyncError::Crypto(format!("Failed to deserialize ContractManager: {}", e)))?;
        Ok(manager)
    }

    /// Check if an incoming contract should be auto-accepted based on policy
    pub fn should_auto_accept(&self, contract: &Contract) -> bool {
        // 1. Is the requester in our trusted list?
        if !self.policy.trusted_nodes.contains(&contract.requester_id) {
            return false;
        }

        // 2. Are all requested capabilities in our auto-accept list?
        for cap in &contract.capabilities {
            if !self.policy.auto_accept_capabilities.contains(cap) {
                return false;
            }
        }

        true
    }

    /// Step 1: Alice initiates a handshake with Bob (BIND_REQ)
    pub fn initiate_handshake(
        &mut self,
        my_keys: &Ed25519KeyPair,
        target_pk: &[u8],
        capabilities: Vec<String>,
        duration_days: u64,
    ) -> Result<Contract, SyncError> {
        let expires_at = now_secs() + (duration_days * 86400);
        let mut contract = Contract::new(
            &my_keys.public_key_bytes(),
            target_pk,
            capabilities,
            expires_at,
        );
        
        contract.sign_requester(my_keys).map_err(|e| SyncError::Crypto(e.to_string()))?;
        
        let state = HandshakeState {
            contract: contract.clone(),
            status: HandshakeStatus::Initiated,
            last_updated: now_secs(),
        };
        self.handshake_manager.update_state(state);
        
        Ok(contract)
    }

    /// Step 2: Bob receives BIND_REQ and responds with BIND_ACK.
    /// If `manual_accept` is None, it uses the policy engine.
    /// In full DH ratchet mode, this also initializes the DoubleRatchet session for Bob.
    pub fn respond_to_handshake(
        &mut self,
        my_keys: &Ed25519KeyPair,
        req_json: &str,
        manual_accept: Option<bool>,
        my_exchange_key: Option<X25519KeyPair>,
        remote_exchange_pub: Option<[u8; 32]>,
    ) -> Result<Contract, SyncError> {
        let mut contract = Contract::from_json(req_json).map_err(|e| SyncError::Crypto(e.to_string()))?;
        
        let accept = match manual_accept {
            Some(val) => val,
            None => self.should_auto_accept(&contract),
        };

        if !accept {
            let state = HandshakeState {
                contract: contract.clone(),
                status: HandshakeStatus::Failed("Rejected (Manual or Policy)".to_string()),
                last_updated: now_secs(),
            };
            self.handshake_manager.update_state(state);
            return Ok(contract); 
        }

        contract.sign_target(my_keys).map_err(|e| SyncError::Crypto(e.to_string()))?;
        
        // Initialize Double Ratchet if keys are provided
        if let (Some(my_x), Some(remote_x)) = (my_exchange_key, remote_exchange_pub) {
            let shared_secret = contract.derive_contract_key(&my_x, &remote_x)
                .map_err(|e| SyncError::Crypto(e.to_string()))?;
            
            // Bob is responder, remote_dh_pub is initially None in simplified DR handshake
            // but since we derive the shared secret from a fixed contract key,
            // we can start with the known remote_x.
            let ratchet = DoubleRatchet::new(shared_secret, my_x, Some(remote_x));
            self.active_ratchets.insert(contract.contract_id.clone(), ratchet);
        }

        let state = HandshakeState {
            contract: contract.clone(),
            status: HandshakeStatus::Received,
            last_updated: now_secs(),
        };
        self.handshake_manager.update_state(state);
        
        Ok(contract)
    }

    /// Step 3: Alice receives Bob's BIND_ACK and finalizes.
    /// In full DH ratchet mode, this also initializes the DoubleRatchet session.
    pub fn finalize_handshake(
        &mut self,
        ack_json: &str,
        my_exchange_key: Option<X25519KeyPair>,
        remote_exchange_pub: Option<[u8; 32]>,
    ) -> Result<Contract, SyncError> {
        let contract = Contract::from_json(ack_json).map_err(|e| SyncError::Crypto(e.to_string()))?;
        
        if !contract.verify() {
            return Err(SyncError::Crypto("Invalid signatures in contract".to_string()));
        }

        if contract.status != ContractStatus::Active {
             return Err(SyncError::Crypto("Contract is not in Active state".to_string()));
        }

        // Initialize Double Ratchet if keys are provided
        if let (Some(my_x), Some(remote_x)) = (my_exchange_key, remote_exchange_pub) {
            let shared_secret = contract.derive_contract_key(&my_x, &remote_x)
                .map_err(|e| SyncError::Crypto(e.to_string()))?;
            
            let ratchet = DoubleRatchet::new(shared_secret, my_x, Some(remote_x));
            self.active_ratchets.insert(contract.contract_id.clone(), ratchet);
        }

        let state = HandshakeState {
            contract: contract.clone(),
            status: HandshakeStatus::Completed,
            last_updated: now_secs(),
        };
        self.handshake_manager.update_state(state);
        
        Ok(contract)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use synch_crypto::keys::Ed25519KeyPair;

    #[test]
    fn test_full_handshake_flow() {
        let mut alice_mgr = ContractManager::new();
        let mut bob_mgr = ContractManager::new();
        
        let alice_keys = Ed25519KeyPair::generate().unwrap();
        let bob_keys = Ed25519KeyPair::generate().unwrap();
        
        let alice_x = X25519KeyPair::generate().unwrap();
        let bob_x = X25519KeyPair::generate().unwrap();
        
        // 1. Alice initiates
        let req = alice_mgr.initiate_handshake(
            &alice_keys,
            &bob_keys.public_key_bytes(),
            vec!["chat".to_string()],
            30
        ).unwrap();
        
        let req_json = req.to_json().unwrap();
        
        // 2. Bob responds (manual accept, with exchange keys)
        let ack = bob_mgr.respond_to_handshake(
            &bob_keys, 
            &req_json, 
            Some(true), 
            Some(bob_x.clone()), 
            Some(alice_x.public_key_bytes())
        ).unwrap();
        let ack_json = ack.to_json().unwrap();
        
        // 3. Alice finalizes
        let final_contract = alice_mgr.finalize_handshake(
            &ack_json, 
            Some(alice_x), 
            Some(bob_x.public_key_bytes())
        ).unwrap();
        
        assert!(final_contract.verify());
        assert_eq!(final_contract.status, ContractStatus::Active);
        
        // Check Alice's state
        let alice_state = alice_mgr.handshake_manager.get_state(&final_contract.contract_id).unwrap();
        assert_eq!(alice_state.status, HandshakeStatus::Completed);
        
        // Check Bob's state
        let bob_state = bob_mgr.handshake_manager.get_state(&final_contract.contract_id).unwrap();
        assert_eq!(bob_state.status, HandshakeStatus::Received);

        // Check active ratchets
        assert!(alice_mgr.active_ratchets.contains_key(&final_contract.contract_id));
        assert!(bob_mgr.active_ratchets.contains_key(&final_contract.contract_id));
    }

    #[test]
    fn test_policy_auto_accept() {
        let mut alice_mgr = ContractManager::new();
        let mut bob_mgr = ContractManager::new();
        
        let alice_keys = Ed25519KeyPair::generate().unwrap();
        let bob_keys = Ed25519KeyPair::generate().unwrap();
        
        // Setup Bob's policy to trust Alice
        bob_mgr.set_policy(HandshakePolicy {
            trusted_nodes: vec![alice_keys.public_key_bytes().to_vec()],
            auto_accept_capabilities: vec!["chat".to_string()],
        });

        // 1. Alice initiates
        let req = alice_mgr.initiate_handshake(
            &alice_keys,
            &bob_keys.public_key_bytes(),
            vec!["chat".to_string()],
            30
        ).unwrap();
        
        let req_json = req.to_json().unwrap();

        // 2. Bob responds using policy (manual_accept = None)
        let ack = bob_mgr.respond_to_handshake(&bob_keys, &req_json, None, None, None).unwrap();
        assert_eq!(ack.status, ContractStatus::Active); // Auto-accepted!
    }

    #[test]
    fn test_persistence() {
        let mut mgr = ContractManager::new();
        let keys = Ed25519KeyPair::generate().unwrap();
        mgr.initiate_handshake(&keys, &[0u8; 32], vec!["test".to_string()], 7).unwrap();
        
        let path = "test_contract_mgr.json";
        mgr.save_to_file(path).unwrap();
        
        let loaded = ContractManager::load_from_file(path).unwrap();
        assert_eq!(loaded.handshake_manager.handshakes.len(), 1);
        
        fs::remove_file(path).unwrap();
    }
}
