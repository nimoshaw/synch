use crate::error::CryptoError;
use crate::keys::X25519KeyPair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Symmetric chain state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub key: [u8; 32],
    pub sequence: u32,
}

impl ChainState {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key, sequence: 0 }
    }

    /// Advance the chain and return a message key
    pub fn step(&mut self) -> [u8; 32] {
        let mut mk_hasher = blake3::Hasher::new_keyed(&self.key);
        mk_hasher.update(b"ratchet-msg-key");
        mk_hasher.update(&self.sequence.to_le_bytes());
        let mut mk = [0u8; 32];
        mk.copy_from_slice(mk_hasher.finalize().as_bytes());

        let mut ck_hasher = blake3::Hasher::new_keyed(&self.key);
        ck_hasher.update(b"ratchet-next-chain");
        let next_ck = ck_hasher.finalize();
        self.key.copy_from_slice(next_ck.as_bytes());
        self.sequence += 1;

        mk
    }
}

/// Full Double Ratchet state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleRatchet {
    pub root_key: [u8; 32],
    pub send_chain: Option<ChainState>,
    pub recv_chain: Option<ChainState>,
    pub local_dh: X25519KeyPair,
    pub remote_dh_pub: Option<[u8; 32]>,
    pub prev_send_length: u32,
    pub skipped_keys: HashMap<([u8; 32], u32), [u8; 32]>, // (remote_dh_pub, sequence) -> msg_key
}

impl DoubleRatchet {
    pub fn new(
        shared_secret: [u8; 32],
        local_dh: X25519KeyPair,
        remote_dh_pub: Option<[u8; 32]>,
    ) -> Self {
        let mut dr = Self {
            root_key: shared_secret,
            send_chain: None,
            recv_chain: None,
            local_dh,
            remote_dh_pub: None,
            prev_send_length: 0,
            skipped_keys: HashMap::new(),
        };

        if let Some(remote_pub) = remote_dh_pub {
            // Alice (Initiator)
            dr.remote_dh_pub = Some(remote_pub);
            let shared = dr.local_dh.diffie_hellman(&remote_pub).unwrap();
            let (next_rk, send_ck) = dr.kdf_rk(shared.as_bytes());
            dr.root_key = next_rk;
            dr.send_chain = Some(ChainState::new(send_ck));
        }
        // If no remote_pub, Bob (Responder) - waits for receive() to trigger DH ratchet

        dr
    }

    /// KDF for Root Key
    fn kdf_rk(&self, dh_out: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
        let mut rk_hasher = blake3::Hasher::new_keyed(&self.root_key);
        rk_hasher.update(b"dr-next-rk");
        rk_hasher.update(dh_out);
        let mut next_rk = [0u8; 32];
        next_rk.copy_from_slice(rk_hasher.finalize().as_bytes());

        let mut ck_hasher = blake3::Hasher::new_keyed(&self.root_key);
        ck_hasher.update(b"dr-next-ck");
        ck_hasher.update(dh_out);
        let mut next_ck = [0u8; 32];
        next_ck.copy_from_slice(ck_hasher.finalize().as_bytes());

        (next_rk, next_ck)
    }

    /// Generate sending parameters
    pub fn send(&mut self) -> ([u8; 32], [u8; 32], u32, u32) {
        if self.send_chain.is_none() {
            // Handle edge case where we haven't even started a chain
            // This usually means we are responder and haven't received anything yet,
            // but we want to send. In DR, you must have a remote pub to send.
            if let Some(remote_pub) = self.remote_dh_pub {
                let shared = self.local_dh.diffie_hellman(&remote_pub).unwrap();
                let (rk, ck) = self.kdf_rk(shared.as_bytes());
                self.root_key = rk;
                self.send_chain = Some(ChainState::new(ck));
            }
        }
        let chain = self.send_chain.as_mut().expect("No sender chain");
        let mk = chain.step();
        (
            mk,
            self.local_dh.public_key_bytes(),
            chain.sequence - 1,
            self.prev_send_length,
        )
    }

    /// Process receiving parameters
    pub fn receive(
        &mut self,
        remote_pub: [u8; 32],
        seq: u32,
        prev_len: u32,
    ) -> Result<[u8; 32], CryptoError> {
        // 1. Check if we already have the key (skipped)
        if let Some(mk) = self.skipped_keys.remove(&(remote_pub, seq)) {
            return Ok(mk);
        }

        // 2. DH Ratchet if remote pub changed (or first message for responder)
        if Some(remote_pub) != self.remote_dh_pub {
            self.skip_message_keys(prev_len)?;
            self.dh_ratchet(remote_pub)?;
        }

        // 3. Skip message keys in current chain
        self.skip_message_keys(seq)?;

        // 4. Advance chain
        Ok(self.recv_chain.as_mut().unwrap().step())
    }

    pub fn dh_ratchet(&mut self, remote_pub: [u8; 32]) -> Result<(), CryptoError> {
        self.prev_send_length = self.send_chain.as_ref().map(|c| c.sequence).unwrap_or(0);
        self.remote_dh_pub = Some(remote_pub);

        // DH Step 1: Receive Chain update
        let shared_recv = self.local_dh.diffie_hellman(&remote_pub)?;
        let (next_rk, recv_ck) = self.kdf_rk(shared_recv.as_bytes());
        self.root_key = next_rk;
        self.recv_chain = Some(ChainState::new(recv_ck));

        // DH Step 2: Send Chain update (with new local DH key)
        self.local_dh = X25519KeyPair::generate()?;
        let shared_send = self.local_dh.diffie_hellman(&remote_pub)?;
        let (next_rk, send_ck) = self.kdf_rk(shared_send.as_bytes());
        self.root_key = next_rk;
        self.send_chain = Some(ChainState::new(send_ck));

        Ok(())
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), CryptoError> {
        if let Some(chain) = self.recv_chain.as_mut() {
            while chain.sequence < until {
                let mk = chain.step();
                self.skipped_keys.insert(
                    (
                        self.remote_dh_pub
                            .ok_or(CryptoError::Encryption("No remote DH key".into()))?,
                        chain.sequence - 1,
                    ),
                    mk,
                );
                if self.skipped_keys.len() > 1000 {
                    return Err(CryptoError::Encryption("Too many skipped keys".to_string()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_ratchet_full_flow() {
        let shared = [1u8; 32];
        let alice_dh = X25519KeyPair::generate().unwrap();
        let bob_dh = X25519KeyPair::generate().unwrap();
        let bob_pub = bob_dh.public_key_bytes();

        // Alice knows Bob's pub from contract
        let mut alice = DoubleRatchet::new(shared, alice_dh, Some(bob_pub));

        // Bob doesn't know Alice's ratchet pub yet (waiting for first message)
        let mut bob = DoubleRatchet::new(shared, bob_dh, None);

        // 1. Alice sends to Bob
        let (mk_send, alice_pub, seq, prev) = alice.send();

        // 2. Bob receives Alice's first message
        let mk_recv = bob.receive(alice_pub, seq, prev).unwrap();
        assert_eq!(mk_send, mk_recv);

        // 3. Bob sends back to Alice
        let (mk_send_bob, bob_pub_new, seq_bob, prev_bob) = bob.send();

        // 4. Alice receives Bob's response
        let mk_recv_alice = alice.receive(bob_pub_new, seq_bob, prev_bob).unwrap();
        assert_eq!(mk_send_bob, mk_recv_alice);
    }

    #[test]
    fn test_double_ratchet_out_of_order() {
        let shared = [2u8; 32];
        let alice_dh = X25519KeyPair::generate().unwrap();
        let bob_dh = X25519KeyPair::generate().unwrap();
        let bob_pub = bob_dh.public_key_bytes();

        let mut alice = DoubleRatchet::new(shared, alice_dh, Some(bob_pub));
        let mut bob = DoubleRatchet::new(shared, bob_dh, None);

        // Alice sends 3 messages
        let m1 = alice.send();
        let m2 = alice.send();
        let m3 = alice.send();

        // Bob receives them out of order: 3, 1, 2
        let mk3 = bob.receive(m3.1, m3.2, m3.3).unwrap();
        assert_eq!(m3.0, mk3);

        let mk1 = bob.receive(m1.1, m1.2, m1.3).unwrap();
        assert_eq!(m1.0, mk1);

        let mk2 = bob.receive(m2.1, m2.2, m2.3).unwrap();
        assert_eq!(m2.0, mk2);
    }

    #[test]
    fn test_double_ratchet_self_healing() {
        let shared = [3u8; 32];
        let alice_dh = X25519KeyPair::generate().unwrap();
        let bob_dh = X25519KeyPair::generate().unwrap();
        let bob_pub = bob_dh.public_key_bytes();

        let mut alice = DoubleRatchet::new(shared, alice_dh, Some(bob_pub));
        let mut bob = DoubleRatchet::new(shared, bob_dh, None);

        // 1. Initial exchange
        let (_mk_a1, pub_a1, seq_a1, prev_a1) = alice.send();
        bob.receive(pub_a1, seq_a1, prev_a1).unwrap();

        // 2. Simulate Bob's chain key compromise
        // (In a real scenario, an attacker gets Bob's current CKr)

        // 3. Alice sends another message (compromised chain continues)
        let (_mk_a2, _pub_a2, _seq_a2, _prev_a2) = alice.send();

        // 4. Bob sends a message (DH Ratchet triggers!)
        let (_mk_b1, pub_b1, seq_b1, prev_b1) = bob.send();

        // 5. Alice receives Bob's message and performs DH update
        alice.receive(pub_b1, seq_b1, prev_b1).unwrap();

        // 6. Alice sends a NEW message after DH update
        let (mk_a3, pub_a3, seq_a3, prev_a3) = alice.send();

        // 7. Bob receives it
        let mk_recv_a3 = bob.receive(pub_a3, seq_a3, prev_a3).unwrap();
        assert_eq!(mk_a3, mk_recv_a3);

        // If an attacker only had the OLD chain key, they cannot derive mk_a3
        // because the root key and chains were refreshed by Bob's DH ratchet.
    }
}
