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
    /// Optional sender public key (identity or original exchange key)
    pub sender_public_key: Option<Vec<u8>>,
    /// Current DH ratchet public key
    pub ratchet_key: Option<Vec<u8>>,
    /// Current chain sequence number
    pub ratchet_seq: u32,
    /// Previous chain length
    pub prev_chain_length: u32,
}

/// SealedBox provides high-level authenticated encryption with associated data (AAD).
/// It ensures that the ciphertext is bound to a specific context (e.g. Vault ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBox {
    pub payload: EncryptedPayload,
    pub ad: Vec<u8>,
}

impl SealedBox {
    pub fn seal(
        key: &[u8; 32],
        plaintext: &[u8],
        ad: &[u8],
    ) -> Result<Self, CryptoError> {
        let payload = encrypt_aes_gcm(key, plaintext, Some(ad))?;
        Ok(Self {
            payload,
            ad: ad.to_vec(),
        })
    }

    pub fn open(
        &self,
        key: &[u8; 32],
    ) -> Result<Vec<u8>, CryptoError> {
        decrypt_aes_gcm(key, &self.payload, Some(&self.ad))
    }
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
        ratchet_key: None,
        ratchet_seq: 0,
        prev_chain_length: 0,
    })
}

/// High-level encryption using Double Ratchet
pub fn encrypt_ratchet(
    ratchet: &mut crate::ratchet::DoubleRatchet,
    plaintext: &[u8],
    aad: Option<&[u8]>,
) -> Result<EncryptedPayload, CryptoError> {
    let (mk, ratchet_key, seq, prev) = ratchet.send();
    let mut payload = encrypt_aes_gcm(&mk, plaintext, aad)?;
    payload.ratchet_key = Some(ratchet_key.to_vec());
    payload.ratchet_seq = seq;
    payload.prev_chain_length = prev;
    Ok(payload)
}

/// High-level decryption using Double Ratchet
pub fn decrypt_ratchet(
    ratchet: &mut crate::ratchet::DoubleRatchet,
    payload: &EncryptedPayload,
    aad: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    let ratchet_key = payload.ratchet_key.as_ref().ok_or(CryptoError::Encryption("Missing ratchet key".into()))?;
    if ratchet_key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength { expected: 32, got: ratchet_key.len() });
    }
    let mut rk = [0u8; 32];
    rk.copy_from_slice(ratchet_key);

    let mk = ratchet.receive(rk, payload.ratchet_seq, payload.prev_chain_length)?;
    decrypt_aes_gcm(&mk, payload, aad)
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
/// The context string prevents key reuse across different protocols or layers.
pub fn derive_symmetric_key(shared_secret: &[u8; 32], context: &str) -> [u8; 32] {
    // Use Blake3 keyed hash as a simple KDF
    let mut key = [0u8; 32];
    let mut hasher = blake3::Hasher::new_keyed(shared_secret);
    hasher.update(b"SYNCH_V1_KDF");
    hasher.update(context.as_bytes());
    let output = hasher.finalize();
    key.copy_from_slice(output.as_bytes());
    key
}
