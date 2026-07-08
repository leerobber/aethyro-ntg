//! The real audit ledger (Phase 3 start): SHA256-chained, tamper-evident
//! records.
//!
//! See [ADR 0002](../../../docs/architecture/0002-safety-rails-for-self-modification.md)
//! rule 5 and docs/EXPERIMENTS.md for the full context: ChronosLedger
//! (GH05T3) is real and reusable for its state-slot model, but has no
//! tamper-evidence; LexGenSeal (GH05T3) is real per-record SHA256
//! signing but doesn't chain records together. `chain.rs`'s `ChainLog`
//! proved the chaining *structure* works, using `std`'s non-cryptographic
//! `DefaultHasher` (explicitly scoped there as change-detection, not
//! security). This module is the real, security-relevant one: it uses
//! actual SHA256 (via the `sha2` crate -- this repo's first external
//! dependency, added deliberately for exactly this reason) rather than
//! reusing `ChainLog` unchanged, because a non-cryptographic hash here
//! would undercut the project's actual "provably tamper-evident" claim.

use sha2::{Digest, Sha256};

use super::graph::NodeId;

/// What actually gets logged -- matching ADR 0002 rule 5 precisely:
/// accepted/rejected topology changes need a fitness score and resource
/// budget recorded, not just "something happened."
#[derive(Clone, Debug, PartialEq)]
pub enum LedgerEvent {
    MutationAccepted { description: String, fitness_score: f32, budget_used: u64 },
    MutationRejected { description: String, fitness_score: f32, budget_used: u64 },
    NodeExecuted { node: NodeId, label: String },
}

impl LedgerEvent {
    /// Canonical byte representation for signing. Must be deterministic
    /// -- the same event always produces the same bytes -- or the
    /// chain would break for reasons that have nothing to do with
    /// tampering.
    fn canonical_bytes(&self) -> Vec<u8> {
        match self {
            LedgerEvent::MutationAccepted { description, fitness_score, budget_used } => {
                format!("MutationAccepted|{description}|{fitness_score}|{budget_used}").into_bytes()
            }
            LedgerEvent::MutationRejected { description, fitness_score, budget_used } => {
                format!("MutationRejected|{description}|{fitness_score}|{budget_used}").into_bytes()
            }
            LedgerEvent::NodeExecuted { node, label } => {
                format!("NodeExecuted|{node}|{label}").into_bytes()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LedgerRecord {
    pub event: LedgerEvent,
    pub signature: [u8; 32],
}

#[derive(Clone, Debug, Default)]
pub struct Ledger {
    records: Vec<LedgerRecord>,
}

const GENESIS_SIGNATURE: [u8; 32] = [0u8; 32];

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    fn sign(prev: [u8; 32], event: &LedgerEvent) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(prev);
        hasher.update(event.canonical_bytes());
        hasher.finalize().into()
    }

    /// Append an event, chained to the previous record's signature (or
    /// a fixed all-zero genesis value for the first record). Returns
    /// the new record's signature.
    pub fn record(&mut self, event: LedgerEvent) -> [u8; 32] {
        let prev = self.records.last().map(|r| r.signature).unwrap_or(GENESIS_SIGNATURE);
        let signature = Self::sign(prev, &event);
        self.records.push(LedgerRecord { event, signature });
        signature
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[LedgerRecord] {
        &self.records
    }

    /// Recompute the chain from scratch over the stored events and
    /// confirm every stored signature matches. `Ok(())` if the whole
    /// ledger verifies; `Err(i)` with the index of the first record
    /// whose stored signature doesn't match -- the tampering point.
    /// Self-contained: unlike a naive design, this needs no externally
    /// supplied "what should have happened" transcript.
    pub fn verify(&self) -> Result<(), usize> {
        let mut prev = GENESIS_SIGNATURE;
        for (i, record) in self.records.iter().enumerate() {
            let expected = Self::sign(prev, &record.event);
            if expected != record.signature {
                return Err(i);
            }
            prev = record.signature;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> LedgerEvent {
        LedgerEvent::MutationAccepted {
            description: "added node X".to_string(),
            fitness_score: 0.87,
            budget_used: 42,
        }
    }

    #[test]
    fn empty_ledger_verifies() {
        assert!(Ledger::new().verify().is_ok());
    }

    #[test]
    fn recorded_events_verify() {
        let mut ledger = Ledger::new();
        ledger.record(sample_event());
        ledger.record(LedgerEvent::MutationRejected {
            description: "removed edge Y".to_string(),
            fitness_score: 0.12,
            budget_used: 8,
        });
        ledger.record(LedgerEvent::NodeExecuted { node: 3, label: "fenced-block".to_string() });
        assert!(ledger.verify().is_ok());
        assert_eq!(ledger.len(), 3);
    }

    #[test]
    fn tampering_with_a_recorded_event_breaks_verification() {
        let mut ledger = Ledger::new();
        ledger.record(sample_event());
        ledger.record(sample_event());

        let mut records = ledger.records().to_vec();
        if let LedgerEvent::MutationAccepted { fitness_score, .. } = &mut records[0].event {
            *fitness_score = 0.99; // tamper: claim a better fitness than actually recorded
        }
        let tampered = Ledger { records };
        assert_eq!(tampered.verify(), Err(0));
    }

    #[test]
    fn removing_a_record_breaks_the_chain() {
        let mut ledger = Ledger::new();
        ledger.record(sample_event());
        ledger.record(sample_event());
        ledger.record(sample_event());

        let mut records = ledger.records().to_vec();
        records.remove(1);
        let shortened = Ledger { records };
        assert_eq!(shortened.verify(), Err(1));
    }

    #[test]
    fn same_event_content_produces_different_signature_depending_on_history() {
        let mut ledger = Ledger::new();
        let sig1 = ledger.record(sample_event());
        let sig2 = ledger.record(sample_event()); // identical event, different position
        assert_ne!(sig1, sig2, "chained signature must depend on history, not just event content");
    }

    #[test]
    fn signature_is_actually_sha256_not_a_toy_hash() {
        // Sanity check against a hand-computed SHA256, so a future
        // change can't silently swap this back to a non-cryptographic
        // hash without a test noticing.
        let event = LedgerEvent::NodeExecuted { node: 0, label: "x".to_string() };
        let mut hasher = Sha256::new();
        hasher.update(GENESIS_SIGNATURE);
        hasher.update(event.canonical_bytes());
        let expected: [u8; 32] = hasher.finalize().into();

        let mut ledger = Ledger::new();
        let sig = ledger.record(event);
        assert_eq!(sig, expected);
    }
}
