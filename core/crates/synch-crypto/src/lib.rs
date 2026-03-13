//! synch-crypto — core cryptographic primitives for the Synch protocol.
//!
//! Implements:
//!   - Ed25519 (identity signing/verification)
//!   - X25519 (ECDH key exchange)
//!   - AES-256-GCM (symmetric encryption)
//!   - Blake3 (hashing & fingerprinting)
//!   - NodeIdentity generation

pub mod error;
pub mod identity;
pub mod keys;
pub mod encrypt;
pub mod hash;

pub use error::CryptoError;
pub use identity::{NodeIdentity, NodeKey};
pub use keys::{Ed25519KeyPair, X25519KeyPair, SharedSecret};
pub use encrypt::{EncryptedPayload, encrypt_aes_gcm, decrypt_aes_gcm};
pub use hash::{blake3_hash, blake3_fingerprint};
