//! synch-crypto — core cryptographic primitives for the Synch protocol.
//!
//! Implements:
//!   - Ed25519 (identity signing/verification)
//!   - X25519 (ECDH key exchange)
//!   - AES-256-GCM (symmetric encryption)
//!   - Blake3 (hashing & fingerprinting)
//!   - NodeIdentity generation

pub mod contract;
pub mod encrypt;
pub mod error;
pub mod hash;
pub mod identity;
pub mod keys;
pub mod ratchet;

pub use encrypt::{
    decrypt_aes_gcm, decrypt_ratchet, encrypt_aes_gcm, encrypt_ratchet, EncryptedPayload,
};
pub use error::CryptoError;
pub use hash::{blake3_fingerprint, blake3_hash};
pub use identity::{NodeIdentity, NodeKey};
pub use keys::{Ed25519KeyPair, SharedSecret, X25519KeyPair};
pub use ratchet::{ChainState, DoubleRatchet};
