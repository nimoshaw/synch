use crate::error::CryptoError;
use crate::hash::blake3_hash;
use crate::keys::{Ed25519KeyPair, X25519KeyPair};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContractStatus {
    Unspecified,
    Pending,
    Active,
    Suspended,
    Expiring,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_id: String,
    pub nonce: u64,
    pub requester_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub capabilities: Vec<String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub renewal_policy: String,
    pub status: ContractStatus,
    pub requester_signature: Option<Vec<u8>>,
    pub target_signature: Option<Vec<u8>>,
    /// Preferred relay URLs for the requester (matches proto field 11)
    #[serde(default)]
    pub requester_relays: Vec<String>,
    /// Preferred relay URLs for the target (matches proto field 12)
    #[serde(default)]
    pub target_relays: Vec<String>,
}

impl Contract {
    pub fn new(
        requester_pk: &[u8],
        target_pk: &[u8],
        capabilities: Vec<String>,
        expires_at: u64,
    ) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut data = Vec::new();
        data.extend_from_slice(requester_pk);
        data.extend_from_slice(target_pk);
        let nonce = rand::random::<u64>();
        data.extend_from_slice(&nonce.to_le_bytes());
        data.extend_from_slice(&created_at.to_le_bytes());
        let contract_id = hex::encode(&blake3_hash(&data)[0..8]);

        Self {
            contract_id,
            nonce,
            requester_id: requester_pk.to_vec(),
            target_id: target_pk.to_vec(),
            capabilities,
            created_at,
            expires_at,
            renewal_policy: "prompt".to_string(),
            status: ContractStatus::Pending,
            requester_signature: None,
            target_signature: None,
            requester_relays: Vec::new(),
            target_relays: Vec::new(),
        }
    }

    pub fn sign_requester(&mut self, key: &Ed25519KeyPair) -> Result<(), CryptoError> {
        let data = self.signing_data();
        let sig = key.sign(&data)?;
        self.requester_signature = Some(sig);
        Ok(())
    }

    pub fn sign_target(&mut self, key: &Ed25519KeyPair) -> Result<(), CryptoError> {
        let data = self.signing_data();
        let sig = key.sign(&data)?;
        self.target_signature = Some(sig);
        self.status = ContractStatus::Active;
        Ok(())
    }

    pub fn verify(&self) -> bool {
        let data = self.signing_data();
        let req_valid = if let Some(sig) = &self.requester_signature {
            crate::keys::verify_ed25519(&self.requester_id, &data, sig).is_ok()
        } else {
            false
        };

        let target_valid = if let Some(sig) = &self.target_signature {
            crate::keys::verify_ed25519(&self.target_id, &data, sig).is_ok()
        } else {
            true // Target signature might be missing in Pending state
        };

        req_valid && target_valid
    }

    fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(b'C'); // Domain separator for Contract
        data.extend_from_slice(self.contract_id.as_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.requester_id);
        data.extend_from_slice(&self.target_id);
        for cap in &self.capabilities {
            data.extend_from_slice(cap.as_bytes());
        }
        data.extend_from_slice(&self.created_at.to_le_bytes());
        data.extend_from_slice(&self.expires_at.to_le_bytes());
        data
    }

    /// Derive the root contract key using Diffie-Hellman
    pub fn derive_contract_key(
        &self,
        my_exchange_key: &X25519KeyPair,
        other_exchange_pk: &[u8],
    ) -> Result<[u8; 32], CryptoError> {
        let shared = my_exchange_key.diffie_hellman(other_exchange_pk)?;
        
        // KDF: Blake3 keyed hash
        let mut hasher = blake3::Hasher::new_keyed(shared.as_bytes());
        hasher.update(b"SYNCH_CONTRACT_V1");
        hasher.update(self.contract_id.as_bytes());
        
        let mut key = [0u8; 32];
        key.copy_from_slice(hasher.finalize().as_bytes());
        Ok(key)
    }

    pub fn to_json(&self) -> Result<String, CryptoError> {
        serde_json::to_string(self).map_err(|e| CryptoError::Signing(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, CryptoError> {
        serde_json::from_str::<Self>(json).map_err(|e| CryptoError::Signing(e.to_string()))
    }
}

pub struct ContractStore {
    contracts: HashMap<String, Contract>,
}

impl ContractStore {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    pub fn add(&mut self, contract: Contract) {
        self.contracts.insert(contract.contract_id.clone(), contract);
    }

    pub fn get(&self, id: &str) -> Option<&Contract> {
        self.contracts.get(id)
    }

    pub fn list_for_target(&self, target_pk: &[u8]) -> Vec<&Contract> {
        self.contracts
            .values()
            .filter(|c| c.target_id == target_pk)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Ed25519KeyPair;

    #[test]
    fn test_contract_lifecycle() {
        let alice_kp = Ed25519KeyPair::generate().unwrap();
        let bob_kp = Ed25519KeyPair::generate().unwrap();
        let alice_pk = alice_kp.public_key_bytes();
        let bob_pk = bob_kp.public_key_bytes();

        let mut contract = Contract::new(&alice_pk, &bob_pk, vec!["chat".to_string()], 0);
        
        // Sign
        contract.sign_requester(&alice_kp).unwrap();
        contract.sign_target(&bob_kp).unwrap();
        
        // Verify
        assert!(contract.verify());
        assert_eq!(contract.status, ContractStatus::Active);
    }

    #[test]
    fn test_contract_key_derivation() {
        let alice_exchange = X25519KeyPair::generate().unwrap();
        let bob_exchange = X25519KeyPair::generate().unwrap();
        let alice_pk = [0u8; 32]; // dummy
        let bob_pk = [1u8; 32]; // dummy

        let contract = Contract::new(&alice_pk, &bob_pk, vec![], 0);
        
        let key_alice = contract.derive_contract_key(&alice_exchange, &bob_exchange.public_key_bytes()).unwrap();
        let key_bob = contract.derive_contract_key(&bob_exchange, &alice_exchange.public_key_bytes()).unwrap();
        
        assert_eq!(key_alice, key_bob);
    }
}
