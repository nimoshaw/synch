use crate::error::CryptoError;
use crate::hash::blake3_fingerprint;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

/// Ed25519 key pair for digital signatures
#[derive(ZeroizeOnDrop)]
pub struct Ed25519KeyPair {
    signing_key: SigningKey,
}

impl Ed25519KeyPair {
    /// Generate a new random Ed25519 key pair
    pub fn generate() -> Result<Self, CryptoError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Self { signing_key })
    }

    /// Restore from raw 32-byte seed
    pub fn from_bytes(seed: &[u8]) -> Result<Self, CryptoError> {
        if seed.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: seed.len(),
            });
        }
        let bytes: [u8; 32] = seed.try_into().unwrap();
        Ok(Self {
            signing_key: SigningKey::from_bytes(&bytes),
        })
    }

    /// Return the 32-byte public key
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Return the 32-byte private seed
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message. Returns 64-byte signature.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let signature: Signature = self.signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    /// Verify a signature against this key pair's public key
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        let verifying_key = self.signing_key.verifying_key();
        verify_ed25519(&verifying_key.to_bytes(), message, signature)
    }

    /// Return hex fingerprint (Blake3 of public key, first 8 bytes)
    pub fn fingerprint(&self) -> String {
        blake3_fingerprint(&self.public_key_bytes())
    }
}

/// Verify an Ed25519 signature given a public key
pub fn verify_ed25519(
    public_key_bytes: &[u8],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<(), CryptoError> {
    if public_key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 32,
            got: public_key_bytes.len(),
        });
    }
    if signature_bytes.len() != 64 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 64,
            got: signature_bytes.len(),
        });
    }
    let pk_arr: [u8; 32] = public_key_bytes.try_into().unwrap();
    let sig_arr: [u8; 64] = signature_bytes.try_into().unwrap();

    let verifying_key = VerifyingKey::from_bytes(&pk_arr)
        .map_err(|e| CryptoError::Signing(e.to_string()))?;
    let signature = Signature::from_bytes(&sig_arr);
    verifying_key
        .verify(message, &signature)
        .map_err(|_| CryptoError::VerificationFailed)
}

// ─── X25519 ────────────────────────────────────────────────────────────────

/// Shared secret from X25519 ECDH
#[derive(ZeroizeOnDrop)]
pub struct SharedSecret {
    bytes: [u8; 32],
}

impl SharedSecret {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

/// X25519 key pair for ECDH key exchange
#[derive(ZeroizeOnDrop)]
pub struct X25519KeyPair {
    secret: x25519_dalek::StaticSecret,
}

impl X25519KeyPair {
    /// Generate a new random X25519 key pair
    pub fn generate() -> Result<Self, CryptoError> {
        let secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        Ok(Self { secret })
    }

    /// Restore from raw 32-byte seed
    pub fn from_bytes(seed: &[u8]) -> Result<Self, CryptoError> {
        if seed.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: seed.len(),
            });
        }
        let bytes: [u8; 32] = seed.try_into().unwrap();
        Ok(Self {
            secret: x25519_dalek::StaticSecret::from(bytes),
        })
    }

    /// Return the 32-byte public key
    pub fn public_key_bytes(&self) -> [u8; 32] {
        let pk = x25519_dalek::PublicKey::from(&self.secret);
        pk.to_bytes()
    }

    /// Perform ECDH with a remote public key
    pub fn diffie_hellman(&self, remote_public_key: &[u8]) -> Result<SharedSecret, CryptoError> {
        if remote_public_key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: remote_public_key.len(),
            });
        }
        let remote_arr: [u8; 32] = remote_public_key.try_into().unwrap();
        let remote_pk = x25519_dalek::PublicKey::from(remote_arr);
        let shared = self.secret.diffie_hellman(&remote_pk);
        Ok(SharedSecret {
            bytes: shared.to_bytes(),
        })
    }
}
