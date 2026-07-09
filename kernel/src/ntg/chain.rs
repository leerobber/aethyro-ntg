//! A genuine hash-chain primitive: each entry's chain value depends on
//! its own content *and* the previous entry's chain value, so altering,
//! removing, or reordering any historical entry breaks the chain from
//! that point forward on verification.
//!
//! **Why this exists, precisely (2026-07-08 finding, see
//! docs/EXPERIMENTS.md):** ADR 0001/0002 originally claimed this
//! project would reuse GH05T3's "ChronosLedger" as a ready-made
//! "tamper-evident, hash-chained audit ledger." Reading the actual
//! source (`backend/oss/core/chronos_ledger.py`) found that claim
//! false: ChronosLedger is a real-time **mutable** mmap agent-state
//! store (32-byte slots, overwritten in place via `write_agent`/
//! `update_fitness`/etc.) with no hashing and no tamper-evidence of any
//! kind -- genuinely useful for fast slot state and lineage tracing
//! (`parent_offset` chains), not for an audit trail. A second
//! candidate, `backend/oss/core/seal.py` ("LexGenSeal"), is real and
//! genuinely tamper-evident *per record* (SHA256 over each record's own
//! content, append-only by file-naming convention) -- but does not
//! chain records together, so deleting one seal file is undetectable
//! from the rest. **No genuine hash chain existed anywhere in the
//! checked codebase.** This module is that missing piece, built once
//! the gap was found rather than assumed away.
//!
//! Phase 3's real ledger should combine: ChronosLedger's state-slot
//! model (state), something like LexGenSeal's per-record signing
//! (content integrity), and this module's chaining (sequence
//! integrity) -- reusing the two real, proven pieces and adding only
//! the piece that was actually missing.
//!
//! **Not cryptographic yet**, same honest caveat as `Graph::fingerprint`:
//! this uses `std`'s `DefaultHasher` (SipHash, not collision-resistant
//! against an adversary) to prove the *chaining structure* works.
//! Swapping in a real cryptographic hash (SHA-256, matching LexGenSeal's
//! own choice, or BLAKE3) is a dependency decision for whoever
//! implements Phase 3's real ledger -- this module's chaining logic
//! does not need to change to support that swap, only the hash function
//! it calls.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const GENESIS: u64 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainEntry {
    pub content: String,
    pub chain_value: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ChainLog {
    entries: Vec<ChainEntry>,
}

fn chain_step(prev: u64, content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prev.hash(&mut hasher);
    content.hash(&mut hasher);
    hasher.finish()
}

impl ChainLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry, chained to the previous one (or a fixed
    /// genesis value if this is the first entry). Returns the new
    /// entry's chain value.
    pub fn append(&mut self, content: impl Into<String>) -> u64 {
        let content = content.into();
        let prev = self.entries.last().map(|e| e.chain_value).unwrap_or(GENESIS);
        let chain_value = chain_step(prev, &content);
        self.entries.push(ChainEntry { content, chain_value });
        chain_value
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[ChainEntry] {
        &self.entries
    }

    /// Recompute the chain from scratch over the stored content and
    /// confirm every stored `chain_value` matches. `Ok(())` if the
    /// whole chain verifies; `Err(i)` with the index of the first
    /// entry whose stored value doesn't match what recomputing the
    /// chain up to that point produces -- the tampering point.
    pub fn verify(&self) -> Result<(), usize> {
        let mut prev = GENESIS;
        for (i, entry) in self.entries.iter().enumerate() {
            let expected = chain_step(prev, &entry.content);
            if expected != entry.chain_value {
                return Err(i);
            }
            prev = entry.chain_value;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_verifies() {
        assert!(ChainLog::new().verify().is_ok());
    }

    #[test]
    fn appended_entries_verify() {
        let mut log = ChainLog::new();
        log.append("first event");
        log.append("second event");
        log.append("third event");
        assert!(log.verify().is_ok());
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn altering_an_entry_breaks_the_chain_from_that_point() {
        let mut log = ChainLog::new();
        log.append("first event");
        log.append("second event");
        log.append("third event");
        assert!(log.verify().is_ok());

        // Simulate an attacker editing history without recomputing
        // chain_value (i.e. tampering with stored content directly).
        let mut entries = log.entries().to_vec();
        entries[1].content = "tampered event".to_string();
        let tampered = ChainLog { entries };
        assert_eq!(tampered.verify(), Err(1));
    }

    #[test]
    fn removing_an_entry_breaks_the_chain() {
        let mut log = ChainLog::new();
        log.append("first event");
        log.append("second event");
        log.append("third event");

        let mut entries = log.entries().to_vec();
        entries.remove(1); // delete the middle entry
        let shortened = ChainLog { entries };
        // The remaining "third event" entry's stored chain_value was
        // computed against "second event"'s chain_value, which no
        // longer precedes it in the shortened log -- must fail.
        assert_eq!(shortened.verify(), Err(1));
    }

    #[test]
    fn chain_value_depends_on_full_history_not_just_own_content() {
        let mut log_a = ChainLog::new();
        log_a.append("same content");

        let mut log_b = ChainLog::new();
        log_b.append("different first entry");
        log_b.append("same content");

        // "same content" appears in both, but its chain_value differs
        // because the preceding history differs -- proving the chain
        // captures sequence, not just per-entry content.
        assert_ne!(log_a.entries()[0].chain_value, log_b.entries()[1].chain_value);
    }
}
