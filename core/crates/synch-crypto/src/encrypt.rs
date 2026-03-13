use crate::error::CryptoError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// AES-256-GCM encrypted payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Ciphertext bytes
    pub ciphertext: Vec<u8>,
    /// 12-byte nonce
    pub nonce: Vec<u8>,
    /// Optional sender public key (for ECDH-derived keys)
    pub sender_public_key: Option<Vec<u8>>,
}

/// Encrypt data using AES-256-GCM with a 32-byte key.
/// The nonce is generated randomly.
pub fn encrypt_aes_gcm(
    key: &[u8],
    plaintext: &[u8],
    aad: Option<&[u8]>,
) -> Result<EncryptedPayload, CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 32,
            got: key.len(),
        });
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = if let Some(aad_bytes) = aad {
        use aes_gcm::aead::Payload;
        cipher
            .encrypt(&nonce, Payload { msg: plaintext, aad: aad_bytes })
            .map_err(|e| CryptoError::Encryption(e.to_string()))?
    } else {
        cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| CryptoError::Encryption(e.to_string()))?
    };

    Ok(EncryptedPayload {
        ciphertext,
        nonce: nonce.to_vec(),
        sender_public_key: None,
    })
}

/// Decrypt AES-256-GCM encrypted payload.
pub fn decrypt_aes_gcm(
    key: &[u8],
    payload: &EncryptedPayload,
    aad: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 32,
            got: key.len(),
        });
    }
    if payload.nonce.len() != 12 {
        return Err(CryptoError::InvalidNonceLength {
            expected: 12,
            got: payload.nonce.len(),
        });
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    let nonce = Nonce::from_slice(&payload.nonce);

    let plaintext = if let Some(aad_bytes) = aad {
        use aes_gcm::aead::Payload;
        cipher
            .decrypt(nonce, Payload { msg: &payload.ciphertext, aad: aad_bytes })
            .map_err(|_| CryptoError::Decryption("Authentication tag mismatch".into()))?
    } else {
        cipher
            .decrypt(nonce, payload.ciphertext.as_ref())
            .map_err(|_| CryptoError::Decryption("Authentication tag mismatch".into()))?
    };

    Ok(plaintext)
}

/// Derive a symmetric key from an X25519 shared secret using HKDF-Blake3
pub fn derive_symmetric_key(shared_secret: &[u8; 32], info: &[u8]) -> [u8; 32] {
    // Use Blake3 keyed hash as a simple KDF
    let mut key = [0u8; 32];
    let mut hasher = blake3::Hasher::new_keyed(shared_secret);
    hasher.update(info);
    let output = hasher.finalize();
    key.copy_from_slice(output.as_bytes());
    key
}
