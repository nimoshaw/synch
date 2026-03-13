/// Blake3 hashing utilities

/// Hash arbitrary data with Blake3, returns 32 bytes
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Generate an 8-byte fingerprint from a public key using Blake3.
/// Returns a lowercase hex string (16 chars).
pub fn blake3_fingerprint(public_key: &[u8]) -> String {
    let hash = blake3::hash(public_key);
    let bytes = hash.as_bytes();
    hex::encode(&bytes[..8])
}

/// Hash data and return hex string
pub fn blake3_hex(data: &[u8]) -> String {
    hex::encode(blake3_hash(data))
}
