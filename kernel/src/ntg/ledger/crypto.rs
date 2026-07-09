//! Cryptographic hashing for ledger integrity.
//!
//! Replaces the non-cryptographic DefaultHasher (SipHash) used in Phase 2's
//! ChainLog with real SHA-256, matching LexGenSeal's choice and suitable for
//! regulatory compliance (auditable, non-repudiable, tamper-evident).

use sha2::{Sha256, Digest};

/// Hash a (prev_hash, content) pair for chaining.
/// Returns a 32-byte SHA-256 digest suitable for sequence integrity proofs.
pub fn chain_hash(prev: &[u8; 32], content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(prev);
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Hash a single piece of content (used for per-record signing, like LexGenSeal).
/// Returns a 32-byte SHA-256 digest.
pub fn content_hash(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Genesis (initial chain value): all zeros, representing "start of ledger".
pub const GENESIS: [u8; 32] = [0u8; 32];

/// Convert a 32-byte hash to a hex string for logging/display.
pub fn hash_to_hex(hash: &[u8; 32]) -> String {
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_hash_deterministic() {
        let content = "test event";
        let h1 = chain_hash(&GENESIS, content);
        let h2 = chain_hash(&GENESIS, content);
        assert_eq!(h1, h2);
    }

    #[test]
    fn chain_hash_depends_on_previous() {
        let content = "test event";
        let h1 = chain_hash(&GENESIS, content);
        let h2 = chain_hash(&h1, content);
        assert_ne!(h1, h2);
    }

    #[test]
    fn content_hash_deterministic() {
        let content = "stable content";
        let h1 = content_hash(content);
        let h2 = content_hash(content);
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_sensitive_to_changes() {
        let h1 = content_hash("hello");
        let h2 = content_hash("hello ");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_to_hex_is_printable() {
        let hash = GENESIS;
        let hex = hash_to_hex(&hash);
        assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn genesis_is_all_zeros() {
        let hex = hash_to_hex(&GENESIS);
        assert_eq!(hex, "0".repeat(64));
    }
}
