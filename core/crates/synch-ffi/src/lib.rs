//! synch-ffi — UniFFI-based FFI layer exposing Synch core to mobile/desktop.
//!
//! Exposes safe wrappers for:
//!   - Key generation (Ed25519 / X25519)
//!   - AES-256-GCM encryption/decryption
//!   - NodeIdentity creation
//!   - Vault operations (create vault, apply delta, list entries)

#[allow(unused_imports)]
use synch_crypto::{
    identity::{NodeIdentity, NodeKey, NodeType},
    keys::{Ed25519KeyPair, X25519KeyPair},
    encrypt::{encrypt_aes_gcm, decrypt_aes_gcm, EncryptedPayload},
    hash::{blake3_hash, blake3_fingerprint},
};
#[allow(unused_imports)]
use synch_sync::{
    vault::{Vault, VaultEntry},
    delta::{DeltaBatch, DeltaEntry, EntryOperation},
    version_vector::VersionVector,
};
use hex;

// ─── FFI-safe result type ───────────────────────────────────────────────────

/// A C-ABI-compatible key bundle result
#[repr(C)]
pub struct FfiResult {
    pub success: bool,
    pub error_msg: *mut std::os::raw::c_char,
}

// ─── Public C exports ────────────────────────────────────────────────────────

/// Generate a new Ed25519 key pair.
/// Returns a JSON object: { "public_key": "hex", "secret_key": "hex", "fingerprint": "hex" }
/// Caller must free the returned string with `synch_free_string`.
#[no_mangle]
pub extern "C" fn synch_generate_ed25519_keypair() -> *mut std::os::raw::c_char {
    let result = Ed25519KeyPair::generate().map(|kp| {
        format!(
            r#"{{"public_key":"{}","secret_key":"{}","fingerprint":"{}"}}"#,
            hex::encode(kp.public_key_bytes()),
            hex::encode(kp.secret_key_bytes()),
            kp.fingerprint()
        )
    });

    match result {
        Ok(json) => string_to_cstr(json),
        Err(e) => string_to_cstr(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Generate a new X25519 key pair.
/// Returns JSON: { "public_key": "hex", "secret_key": "hex" }
#[no_mangle]
pub extern "C" fn synch_generate_x25519_keypair() -> *mut std::os::raw::c_char {
    let result = X25519KeyPair::generate().map(|kp| {
        format!(
            r#"{{"public_key":"{}"}}"#,
            hex::encode(kp.public_key_bytes()),
        )
    });

    match result {
        Ok(json) => string_to_cstr(json),
        Err(e) => string_to_cstr(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Generate a full NodeIdentity JSON given a node type string (e.g., "agent", "mobile").
/// Returns JSON matching proto NodeIdentity structure.
#[no_mangle]
pub extern "C" fn synch_generate_node_identity(
    node_type_str: *const std::os::raw::c_char,
    platform_str: *const std::os::raw::c_char,
    display_name_str: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let node_type_s = unsafe { cstr_to_string(node_type_str) }.unwrap_or_default();
    let platform_s = unsafe { cstr_to_string(platform_str) }.unwrap_or_default();
    let display_name_s = unsafe { cstr_to_string(display_name_str) }.unwrap_or_default();

    let node_type = match node_type_s.as_str() {
        "agent" => NodeType::Agent,
        "human" => NodeType::Human,
        "bridge" => NodeType::Bridge,
        "plugin" => NodeType::Plugin,
        "mobile" => NodeType::Mobile,
        _ => NodeType::Agent,
    };

    let result = NodeKey::generate().map(|key| {
        let identity = NodeIdentity::new(
            &key,
            node_type,
            platform_s,
            display_name_s,
            vec!["vault-sync".to_string()],
        );
        serde_json::to_string(&identity).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    });

    match result {
        Ok(json) => string_to_cstr(json),
        Err(e) => string_to_cstr(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Sign data with an Ed25519 secret key (hex-encoded seed).
/// Returns JSON: { "signature": "hex" }
#[no_mangle]
pub extern "C" fn synch_ed25519_sign(
    secret_key_hex: *const std::os::raw::c_char,
    message_hex: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let result = (|| -> Result<String, String> {
        let sk_hex = unsafe { cstr_to_string(secret_key_hex) }.ok_or("null secret key")?;
        let msg_hex = unsafe { cstr_to_string(message_hex) }.ok_or("null message")?;
        let sk_bytes = hex::decode(&sk_hex).map_err(|e: hex::FromHexError| e.to_string())?;
        let msg_bytes = hex::decode(&msg_hex).map_err(|e: hex::FromHexError| e.to_string())?;
        let kp = Ed25519KeyPair::from_bytes(&sk_bytes).map_err(|e| e.to_string())?;
        let sig = kp.sign(&msg_bytes).map_err(|e| e.to_string())?;
        Ok(format!(r#"{{"signature":"{}"}}"#, hex::encode(sig)))
    })();

    match result {
        Ok(json) => string_to_cstr(json),
        Err(e) => string_to_cstr(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Free a string previously returned by Synch FFI functions.
#[no_mangle]
pub extern "C" fn synch_free_string(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}

/// Compute Blake3 hash of data (hex-encoded input → hex-encoded output)
#[no_mangle]
pub extern "C" fn synch_blake3_hash(
    data_hex: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let result = (|| -> Result<String, String> {
        let hex_in = unsafe { cstr_to_string(data_hex) }.ok_or("null data")?;
        let bytes = hex::decode(&hex_in).map_err(|e: hex::FromHexError| e.to_string())?;
        Ok(hex::encode(blake3_hash(&bytes)))
    })();
    match result {
        Ok(h) => string_to_cstr(h),
        Err(e) => string_to_cstr(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn string_to_cstr(s: String) -> *mut std::os::raw::c_char {
    match std::ffi::CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_owned())
        .ok()
}

// ─── UniFFI ──────────────────────────────────────────────────────────────────

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SynchError {
    #[error("Crypto error: {0}")]
    CryptoError(String),
}

#[derive(uniffi::Record)]
pub struct Ed25519KeyResult {
    pub public_key: String,
    pub secret_key: String,
    pub fingerprint: String,
}

#[uniffi::export]
pub fn generate_ed25519_keypair_uniffi() -> Result<Ed25519KeyResult, SynchError> {
    let kp = Ed25519KeyPair::generate().map_err(|e| SynchError::CryptoError(e.to_string()))?;
    Ok(Ed25519KeyResult {
        public_key: hex::encode(kp.public_key_bytes()),
        secret_key: hex::encode(kp.secret_key_bytes()),
        fingerprint: kp.fingerprint(),
    })
}

// ─── Vault UniFFI Wrapper ────────────────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct SynchVault {
    inner: std::sync::Arc<std::sync::Mutex<Vault>>,
}

#[uniffi::export]
impl SynchVault {
    #[uniffi::constructor]
    pub fn new(vault_id: String) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(Vault::new(vault_id))),
        }
    }

    pub fn get_version(&self) -> u64 {
        self.inner.lock().unwrap().version
    }

    pub fn get_vault_id(&self) -> String {
        self.inner.lock().unwrap().vault_id.clone()
    }

    /// Apply a mock update to verify version increments and state persistence
    pub fn apply_mock_update(&self, path: String, content: String) {
        let mut vault = self.inner.lock().unwrap();
        let node_id = "mobile-test-node".to_string();
        let seq = vault.version + 1;
        
        // Construct a simple delta batch
        let delta = DeltaBatch {
            vault_id: vault.vault_id.clone(),
            base_version: vault.version,
            target_version: seq,
            changes: vec![DeltaEntry::new_create(
                path,
                content.into_bytes(),
                node_id,
                seq,
                now_millis(),
            )],
        };

        let _ = vault.apply_batch(delta);
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ─── Handshake UniFFI Wrapper ────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct HandshakeStateRecord {
    pub contract_id: String,
    pub status: String,
    pub requester_id: String,
    pub target_id: String,
    pub capabilities: Vec<String>,
}

#[derive(uniffi::Object)]
pub struct HandshakeController {
    pub inner: std::sync::Arc<std::sync::Mutex<synch_sync::contract_manager::ContractManager>>,
}

#[uniffi::export]
impl HandshakeController {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(synch_sync::contract_manager::ContractManager::new())),
        }
    }

    pub fn initiate_handshake(
        &self,
        secret_key_hex: String,
        target_pk_hex: String,
        capabilities: Vec<String>,
        duration_days: u64,
    ) -> Result<String, SynchError> {
        let sk_bytes = hex::decode(&secret_key_hex).map_err(|e: hex::FromHexError| SynchError::CryptoError(e.to_string()))?;
        let target_pk = hex::decode(&target_pk_hex).map_err(|e: hex::FromHexError| SynchError::CryptoError(e.to_string()))?;
        let my_keys = Ed25519KeyPair::from_bytes(&sk_bytes).map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))?;
        
        let mut manager = self.inner.lock().unwrap();
        let contract = manager.initiate_handshake(&my_keys, &target_pk, capabilities, duration_days)
            .map_err(|e: synch_sync::error::SyncError| SynchError::CryptoError(e.to_string()))?;
            
        contract.to_json().map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))
    }

    pub fn respond_to_handshake(
        &self,
        secret_key_hex: String,
        req_json: String,
        accept: bool,
        my_exchange_sk_hex: Option<String>,
        remote_exchange_pk_hex: Option<String>,
    ) -> Result<String, SynchError> {
        let sk_bytes = hex::decode(&secret_key_hex).map_err(|e: hex::FromHexError| SynchError::CryptoError(e.to_string()))?;
        let my_keys = Ed25519KeyPair::from_bytes(&sk_bytes).map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))?;
        
        let my_exchange = if let Some(sk) = my_exchange_sk_hex {
            let bytes = hex::decode(&sk).map_err(|e| SynchError::CryptoError(e.to_string()))?;
            Some(X25519KeyPair::from_bytes(&bytes).map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))?)
        } else {
            None
        };

        let remote_exchange = if let Some(pk) = remote_exchange_pk_hex {
            let bytes = hex::decode(&pk).map_err(|e| SynchError::CryptoError(e.to_string()))?;
            if bytes.len() != 32 {
                return Err(SynchError::CryptoError("Invalid remote exchange key length".into()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        } else {
            None
        };

        let mut manager = self.inner.lock().unwrap();
        let contract = manager.respond_to_handshake(&my_keys, &req_json, Some(accept), my_exchange, remote_exchange)
            .map_err(|e: synch_sync::error::SyncError| SynchError::CryptoError(e.to_string()))?;
            
        contract.to_json().map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))
    }

    pub fn finalize_handshake(
        &self,
        ack_json: String,
        my_exchange_sk_hex: Option<String>,
        remote_exchange_pk_hex: Option<String>,
    ) -> Result<String, SynchError> {
        let my_exchange = if let Some(sk) = my_exchange_sk_hex {
            let bytes = hex::decode(&sk).map_err(|e| SynchError::CryptoError(e.to_string()))?;
            Some(X25519KeyPair::from_bytes(&bytes).map_err(|e| SynchError::CryptoError(e.to_string()))?)
        } else {
            None
        };

        let remote_exchange = if let Some(pk) = remote_exchange_pk_hex {
            let bytes = hex::decode(&pk).map_err(|e| SynchError::CryptoError(e.to_string()))?;
            if bytes.len() != 32 {
                return Err(SynchError::CryptoError("Invalid remote exchange key length".into()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        } else {
            None
        };

        let mut manager = self.inner.lock().unwrap();
        let contract = manager.finalize_handshake(&ack_json, my_exchange, remote_exchange)
            .map_err(|e: synch_sync::error::SyncError| SynchError::CryptoError(e.to_string()))?;
            
        contract.to_json().map_err(|e: synch_crypto::error::CryptoError| SynchError::CryptoError(e.to_string()))
    }

    pub fn get_pending_handshakes(&self) -> Vec<HandshakeStateRecord> {
        let manager = self.inner.lock().unwrap();
        manager.handshake_manager.list_pending().into_iter().map(|s| {
            HandshakeStateRecord {
                contract_id: s.contract.contract_id.clone(),
                status: format!("{:?}", s.status),
                requester_id: hex::encode(&s.contract.requester_id),
                target_id: hex::encode(&s.contract.target_id),
                capabilities: s.contract.capabilities.clone(),
            }
        }).collect()
    }

    pub fn set_policy(&self, trusted_nodes_hex: Vec<String>, auto_accept_caps: Vec<String>) -> Result<(), SynchError> {
        let mut manager = self.inner.lock().unwrap();
        let mut trusted = Vec::new();
        for hex_pk in trusted_nodes_hex {
            trusted.push(hex::decode(hex_pk).map_err(|e| SynchError::CryptoError(e.to_string()))?);
        }
        manager.set_policy(synch_sync::contract_manager::HandshakePolicy {
            trusted_nodes: trusted,
            auto_accept_capabilities: auto_accept_caps,
        });
        Ok(())
    }

    pub fn export_state(&self) -> Result<String, SynchError> {
        let manager = self.inner.lock().unwrap();
        serde_json::to_string(&*manager).map_err(|e| SynchError::CryptoError(e.to_string()))
    }

    pub fn import_state(&self, json: String) -> Result<(), SynchError> {
        let mut manager = self.inner.lock().unwrap();
        *manager = serde_json::from_str(&json).map_err(|e| SynchError::CryptoError(e.to_string()))?;
        Ok(())
    }

    pub fn save_to_file(&self, path: String) -> Result<(), SynchError> {
        let manager = self.inner.lock().unwrap();
        manager.save_to_file(path).map_err(|e| SynchError::CryptoError(e.to_string()))
    }

    pub fn load_from_file(&self, path: String) -> Result<(), SynchError> {
        let mut manager = self.inner.lock().unwrap();
        *manager = synch_sync::contract_manager::ContractManager::load_from_file(path)
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
        Ok(())
    }

    pub fn seal_vault_batch(
        &self,
        vault: &SynchVault,
        contract_id: String,
        base_version: u64,
        target_version: u64,
    ) -> Result<String, SynchError> {
        let mut manager = self.inner.lock().unwrap();
        let ratchet = manager.active_ratchets.get_mut(&contract_id)
            .ok_or_else(|| SynchError::CryptoError(format!("No active ratchet for contract {}", contract_id)))?;
        
        let vault_inner = vault.inner.lock().unwrap();
        let changes = vault_inner.deltas_since(base_version).into_iter().cloned().collect();
        let batch = DeltaBatch::new(vault_inner.vault_id.clone(), base_version, target_version).with_changes(changes);
        
        let secured = synch_sync::secure::seal_batch(ratchet, contract_id, &batch)
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
            
        serde_json::to_string(&secured).map_err(|e| SynchError::CryptoError(e.to_string()))
    }

    pub fn apply_secured_vault_batch(
        &self,
        vault: &SynchVault,
        secured_json: String,
    ) -> Result<u64, SynchError> {
        let secured: synch_sync::secure::SecuredBatch = serde_json::from_str(&secured_json)
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
            
        let mut manager = self.inner.lock().unwrap();
        let ratchet = manager.active_ratchets.get_mut(&secured.contract_id)
            .ok_or_else(|| SynchError::CryptoError(format!("No active ratchet for contract {}", secured.contract_id)))?;
            
        let vault_id = vault.inner.lock().unwrap().vault_id.clone();
        let batch = synch_sync::secure::open_batch(ratchet, &secured, &vault_id)
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
            
        let mut vault_inner = vault.inner.lock().unwrap();
        let count = vault_inner.apply_batch(batch)
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
            
        Ok(count as u64)
    }
}

impl Drop for SynchVault {
    fn drop(&mut self) {
        // This log will appear in logcat for Android debugging
        println!("[SynchVault] Rust object is being dropped. Resources released.");
    }
}

// ─── Relay UniFFI Wrapper ───────────────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct RelayController {
    inner: std::sync::Arc<synch_sync::net::RelayManager>,
    rt: std::sync::Arc<tokio::runtime::Runtime>,
}

#[uniffi::export]
impl RelayController {
    #[uniffi::constructor]
    pub fn new() -> Result<Self, SynchError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| SynchError::CryptoError(e.to_string()))?;
            
        Ok(Self {
            inner: std::sync::Arc::new(synch_sync::net::RelayManager::new()),
            rt: std::sync::Arc::new(rt),
        })
    }

    pub fn add_relay(&self, url: String, is_primary: bool) {
        let role = if is_primary {
            synch_sync::net::RelayRole::Primary
        } else {
            synch_sync::net::RelayRole::Secondary
        };
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            let _ = inner.add_relay(&url, role).await;
        });
    }

    pub fn remove_relay(&self, url: String) {
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            inner.remove_relay(&url).await;
        });
    }

    pub fn broadcast(&self, payload_hex: String) -> Result<(), SynchError> {
        let data = hex::decode(&payload_hex).map_err(|e| SynchError::CryptoError(e.to_string()))?;
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            inner.broadcast(data).await;
        });
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_generate_ed25519() {
        let ptr = synch_generate_ed25519_keypair();
        assert!(!ptr.is_null());
        let json = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
        synch_free_string(ptr);
        println!("Ed25519 keypair: {}", json);
        assert!(json.contains("public_key"));
        assert!(json.contains("fingerprint"));
    }

    #[test]
    fn test_ffi_generate_node_identity() {
        let nt = std::ffi::CString::new("agent").unwrap();
        let pl = std::ffi::CString::new("test-platform").unwrap();
        let dn = std::ffi::CString::new("Test Node").unwrap();
        let ptr = synch_generate_node_identity(nt.as_ptr(), pl.as_ptr(), dn.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
        synch_free_string(ptr);
        println!("NodeIdentity: {}", json);
        assert!(json.contains("node_id"));
        assert!(json.contains("agent://"));
    }
}
