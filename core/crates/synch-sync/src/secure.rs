use crate::delta::DeltaBatch;
use crate::error::SyncError;
use serde::{Deserialize, Serialize};
use synch_crypto::encrypt::{decrypt_ratchet, encrypt_ratchet, EncryptedPayload};
use synch_crypto::ratchet::DoubleRatchet;

/// High-level E2EE wrapper for a sync batch.
/// This corresponds to the content of a SecuredMessage in the protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuredBatch {
    pub contract_id: String,
    pub payload: EncryptedPayload,
}

pub fn seal_batch(
    ratchet: &mut DoubleRatchet,
    contract_id: String,
    batch: &DeltaBatch,
) -> Result<SecuredBatch, SyncError> {
    let bytes = serde_json::to_vec(batch)
        .map_err(|e| SyncError::Crypto(format!("Serialization failed: {}", e)))?;

    // Use vault_id as AAD to bind the encryption to this specific vault
    let payload = encrypt_ratchet(ratchet, &bytes, Some(batch.vault_id.as_bytes()))
        .map_err(|e| SyncError::Crypto(e.to_string()))?;

    Ok(SecuredBatch {
        contract_id,
        payload,
    })
}

pub fn open_batch(
    ratchet: &mut DoubleRatchet,
    secured: &SecuredBatch,
    vault_id: &str,
) -> Result<DeltaBatch, SyncError> {
    // Verify AAD (vault_id) during decryption
    let bytes = decrypt_ratchet(ratchet, &secured.payload, Some(vault_id.as_bytes()))
        .map_err(|e| SyncError::Crypto(e.to_string()))?;

    let batch: DeltaBatch = serde_json::from_slice(&bytes)
        .map_err(|e| SyncError::Crypto(format!("Deserialization failed: {}", e)))?;

    if batch.vault_id != vault_id {
        return Err(SyncError::InvalidDelta(
            "Vault ID mismatch in secured batch".into(),
        ));
    }

    Ok(batch)
}
