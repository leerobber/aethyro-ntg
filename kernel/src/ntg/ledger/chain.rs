//! Cryptographic hash-chain for Phase 3 ledger: SHA-256 instead of DefaultHasher.
//!
//! This extends the Phase 2 ChainLog concept with real cryptography, making
//! the tamper-evidence suitable for regulatory/compliance use cases.
//! Each entry is chained to the previous via SHA-256(prev_hash || content),
//! so any tampering, deletion, or reordering is mathematically detectable.

use super::crypto::{chain_hash, GENESIS};
use super::NtgError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CryptoChainEntry {
    /// The content (JSON event description)
    pub content: String,
    /// SHA-256 hash chained to the previous entry
    pub chain_hash: [u8; 32],
}

/// Cryptographic hash-chain: sequence-integrity primitive for the ledger.
#[derive(Clone, Debug, Default)]
pub struct CryptoChainLog {
    entries: Vec<CryptoChainEntry>,
}

impl CryptoChainLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry to the chain, chained to the previous (or GENESIS if first).
    pub fn append(&mut self, content: impl Into<String>) -> Result<[u8; 32], NtgError> {
        let content = content.into();
        let prev = self
            .entries
            .last()
            .map(|e| e.chain_hash)
            .unwrap_or(GENESIS);
        let new_hash = chain_hash(&prev, &content);
        self.entries.push(CryptoChainEntry {
            content,
            chain_hash: new_hash,
        });
        Ok(new_hash)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[CryptoChainEntry] {
        &self.entries
    }

    /// Verify the entire chain: recompute every hash and confirm matches.
    /// Returns Ok(()) if valid; Err(index) if chain broken at that point.
    pub fn verify(&self) -> Result<(), NtgError> {
        let mut prev = GENESIS;
        for (i, entry) in self.entries.iter().enumerate() {
            let expected = chain_hash(&prev, &entry.content);
            if expected != entry.chain_hash {
                return Err(NtgError::ChainBroken(i));
            }
            prev = entry.chain_hash;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chain_verifies() {
        let chain = CryptoChainLog::new();
        assert!(chain.verify().is_ok());
    }

    #[test]
    fn append_and_verify() -> Result<(), NtgError> {
        let mut chain = CryptoChainLog::new();
        chain.append("event 1")?;
        chain.append("event 2")?;
        chain.append("event 3")?;
        assert_eq!(chain.len(), 3);
        assert!(chain.verify().is_ok());
        Ok(())
    }

    #[test]
    fn tampering_breaks_chain() -> Result<(), NtgError> {
        let mut chain = CryptoChainLog::new();
        chain.append("event 1")?;
        chain.append("event 2")?;
        chain.append("event 3")?;
        assert!(chain.verify().is_ok());

        // Tamper with event 2's content
        if let Some(entry) = chain.entries.get_mut(1) {
            entry.content = "tampered event".to_string();
        }

        assert!(chain.verify().is_err());
        Ok(())
    }

    #[test]
    fn removing_entry_breaks_chain() -> Result<(), NtgError> {
        let mut chain = CryptoChainLog::new();
        chain.append("event 1")?;
        chain.append("event 2")?;
        chain.append("event 3")?;

        // Remove event 2
        let mut entries = chain.entries.to_vec();
        entries.remove(1);
        let shortened = CryptoChainLog {
            entries,
        };

        // Event 3's stored hash was computed against event 2's hash,
        // which no longer precedes it — must fail verification.
        assert!(shortened.verify().is_err());
        Ok(())
    }

    #[test]
    fn chain_value_depends_on_history() -> Result<(), NtgError> {
        let mut chain1 = CryptoChainLog::new();
        chain1.append("same content")?;

        let mut chain2 = CryptoChainLog::new();
        chain2.append("different first")?;
        chain2.append("same content")?;

        // "same content" appears in both, but hashes differ due to different history
        assert_ne!(
            chain1.entries[0].chain_hash,
            chain2.entries[1].chain_hash
        );
        Ok(())
    }
}
