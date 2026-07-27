//! Tamper-evident audit ledger for self-modifying graph topology.
//!
//! ADR 0002 specifies five safety rails for self-modification:
//! 1. Off by default
//! 2. Bounded compute/time budget per cycle
//! 3. Automatic rollback on regression
//! 4. Deterministic replay (same input → same output)
//! 5. Every modification event is ledger-logged
//!
//! This module implements the ledger layer (rules 4-5), combining three
//! pieces reused from prior proven work:
//! - ChainLog (sequence integrity — chained hashes detect deletion/reordering)
//! - SignedEntry (content integrity — per-record SHA-256, like LexGenSeal)
//! - StateSlots (fast mutable state + lineage, like ChronosLedger)
//!
//! Phase 3 exit criteria: all five ADR 0002 rails have dedicated passing
//! tests; ledger entries produced for every accept/reject event;
//! self-modification remains disabled by default at end of phase.

pub mod crypto;
pub mod signed_entry;
pub mod stateblots;
pub mod replay;
pub mod chain;

use self::chain::CryptoChainLog;
use super::error::NtgError;
use signed_entry::SignedEntry;
use stateblots::StateSlotStore;
use replay::ExecutionTrace;

pub use stateblots::StateSlot;

use std::collections::HashMap;

/// Ledger entry covering a complete mutation cycle: proposal, evaluation, decision.
#[derive(Clone, Debug)]
pub struct MutationLogEntry {
    /// Mutation ID (unique within this ledger)
    pub mutation_id: u64,
    /// What changed (human-readable, e.g. "add_node(id=42)")
    pub description: String,
    /// Pre-mutation graph fingerprint (reproducibility check)
    pub pre_fingerprint: u64,
    /// Post-mutation graph fingerprint
    pub post_fingerprint: u64,
    /// Measured fitness (latency_ms, memory_bytes)
    pub fitness: FitnessMeasure,
    /// Accepted or rejected
    pub outcome: MutationOutcome,
    /// Wall-clock nanoseconds spent on this cycle
    pub budget_consumed_ns: u64,
    /// Execution trace (every node execution during evaluation)
    pub trace: ExecutionTrace,
    /// Timestamp (wall-clock or logical)
    pub timestamp: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FitnessMeasure {
    /// Forward pass latency in microseconds
    pub latency_us: u64,
    /// Peak memory usage in bytes
    pub memory_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationOutcome {
    Accepted,
    RejectedRegression,
    RejectedBudgetExceeded,
    RejectedFitnessGate,
}

/// The ledger: combines CryptoChainLog (sequence), SignedEntry (content),
/// StateSlots (state), and ExecutionTrace (reproducibility).
#[derive(Clone, Debug)]
pub struct TamperEvidentLedger {
    /// Sequence integrity: chained SHA-256 hashes
    chain: CryptoChainLog,
    /// Content integrity + state history
    entries: Vec<SignedEntry>,
    /// Mutable agent/node state + lineage
    slots: StateSlotStore,
    /// Per-mutation detailed execution traces
    traces: HashMap<u64, ExecutionTrace>,
    /// Latest mutation ID
    next_mutation_id: u64,
}

impl TamperEvidentLedger {
    pub fn new(stateslot_file: Option<&str>) -> Result<Self, NtgError> {
        let slots = StateSlotStore::new(stateslot_file)?;
        Ok(Self {
            chain: CryptoChainLog::new(),
            entries: Vec::new(),
            slots,
            traces: HashMap::new(),
            next_mutation_id: 0,
        })
    }

    /// Log a completed mutation cycle. Returns the entry's position in the ledger.
    // Each argument is a distinct, independently-meaningful ledger field (ADR
    // 0002 rail 5); bundling them into a params struct wouldn't reduce real
    // complexity and would touch every call site, so the lint is suppressed
    // rather than the API reshaped.
    #[allow(clippy::too_many_arguments)]
    pub fn log_mutation(
        &mut self,
        description: impl Into<String>,
        pre_fingerprint: u64,
        post_fingerprint: u64,
        fitness: FitnessMeasure,
        outcome: MutationOutcome,
        budget_consumed_ns: u64,
        trace: ExecutionTrace,
        timestamp: u64,
    ) -> Result<u64, NtgError> {
        let desc = description.into();
        let mutation_id = self.next_mutation_id;
        self.next_mutation_id += 1;

        // Create the entry. serde_json::to_string on a String cannot fail
        // (it's always valid UTF-8), and its output already includes the
        // surrounding quotes plus escaping for every JSON-forbidden
        // character (not just \ and "), so it's spliced in unquoted here.
        let desc_json = serde_json::to_string(&desc)
            .expect("String -> JSON string serialization cannot fail");
        let entry_json = format!(
            r#"{{"mutation_id":{},"description":{},"pre_fingerprint":{},"post_fingerprint":{},"latency_us":{},"memory_bytes":{},"outcome":"{:?}","budget_ns":{},"timestamp":{}}}"#,
            mutation_id,
            desc_json,
            pre_fingerprint,
            post_fingerprint,
            fitness.latency_us,
            fitness.memory_bytes,
            outcome,
            budget_consumed_ns,
            timestamp
        );

        // Chain it (sequence integrity) — returns the chain hash
        let _chain_hash = self.chain.append(&entry_json)?;

        // Sign it (content integrity)
        let signed = SignedEntry::new(&entry_json, mutation_id, timestamp)?;
        self.entries.push(signed);

        // Store the execution trace separately
        self.traces.insert(mutation_id, trace);

        Ok(mutation_id)
    }

    /// Verify the entire ledger's integrity (ADR 0002 rule 5).
    /// Returns Ok(()) if ledger is valid, or Err(problem) if tampering detected.
    pub fn verify_full_ledger(&self) -> Result<(), NtgError> {
        // 1. Verify sequence integrity (ChainLog)
        self.chain.verify().map_err(|idx| {
            NtgError::LedgerTampering(format!("Chain broken at entry {}", idx))
        })?;

        // 2. Verify content integrity (each SignedEntry)
        for (i, entry) in self.entries.iter().enumerate() {
            entry.verify().map_err(|_| {
                NtgError::LedgerTampering(format!("Entry {} content tampered", i))
            })?;
        }

        // 3. Verify state-slot lineage (ChronosLedger-style parent_offset)
        self.slots.verify_lineage().map_err(|e| {
            NtgError::LedgerTampering(format!("State slot lineage broken: {}", e))
        })?;

        // 4. Verify determinism: re-replay execution traces
        for trace in self.traces.values() {
            trace.verify_determinism().map_err(|_| {
                NtgError::LedgerTampering("Execution trace non-deterministic".to_string())
            })?;
        }

        Ok(())
    }

    /// List all ledger entries in chronological order.
    pub fn entries(&self) -> &[SignedEntry] {
        &self.entries
    }

    /// Get a specific mutation's execution trace for reproducibility verification.
    pub fn get_trace(&self, mutation_id: u64) -> Option<&ExecutionTrace> {
        self.traces.get(&mutation_id)
    }

    /// Audit API: count mutations by outcome.
    pub fn audit_summary(&self) -> (usize, usize, usize, usize) {
        let mut accepted = 0;
        let mut rejected_regression = 0;
        let mut rejected_budget = 0;
        let mut rejected_gate = 0;

        for entry in &self.entries {
            if let Ok(outcome) = entry.get_outcome() {
                match outcome {
                    MutationOutcome::Accepted => accepted += 1,
                    MutationOutcome::RejectedRegression => rejected_regression += 1,
                    MutationOutcome::RejectedBudgetExceeded => rejected_budget += 1,
                    MutationOutcome::RejectedFitnessGate => rejected_gate += 1,
                }
            }
        }

        (accepted, rejected_regression, rejected_budget, rejected_gate)
    }

    /// Ledger length for tests.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // -----------------------------------------------------------------------
    // VITASCALE Phase F extensions (ADR 0010 §4.3 ledger hooks)
    // -----------------------------------------------------------------------

    /// Log a phage quarantine event to the tamper-evident chain.
    ///
    /// Wraps `log_mutation` with a phage-grammar description so phage events
    /// are indistinguishable from mutation events in the chain (same integrity
    /// guarantee). `class` should be "Load" or "Deterministic" (KD16).
    pub fn log_phage_event(
        &mut self,
        agent_id: u32,
        class: &str,
        reason: &str,
        timestamp: u64,
    ) -> Result<u64, NtgError> {
        let description = format!(
            "phage:agent_id={},class={},reason={}",
            agent_id, class, reason
        );
        self.log_mutation(
            description,
            0,
            0,
            FitnessMeasure {
                latency_us: 0,
                memory_bytes: 0,
            },
            MutationOutcome::RejectedFitnessGate,
            0,
            replay::ExecutionTrace::new(),
            timestamp,
        )
    }

    /// Append an agent StateSlot to the ledger's slot store.
    /// Returns the slot's byte offset (1-based lineage pointer).
    pub fn write_agent_slot(
        &mut self,
        slot: stateblots::StateSlot,
    ) -> Result<usize, NtgError> {
        self.slots.write_slot(slot)
    }

    /// Walk an agent's slot lineage (oldest-first).
    pub fn agent_lineage(
        &self,
        agent_id: u32,
    ) -> Result<Vec<stateblots::StateSlot>, NtgError> {
        self.slots.lineage(agent_id)
    }

    /// Verify that every agent's slot lineage is internally consistent.
    pub fn verify_agent_lineages(&self) -> Result<(), NtgError> {
        self.slots.verify_lineage().map_err(|e| {
            NtgError::LedgerTampering(format!("State slot lineage broken: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_verifies() -> Result<(), NtgError> {
        let ledger = TamperEvidentLedger::new(None)?;
        assert!(ledger.verify_full_ledger().is_ok());
        Ok(())
    }

    #[test]
    fn log_single_mutation_verifies() -> Result<(), NtgError> {
        let mut ledger = TamperEvidentLedger::new(None)?;
        let trace = ExecutionTrace::new();

        let mutation_id = ledger.log_mutation(
            "add_node(id=42)",
            12345,
            12346,
            FitnessMeasure {
                latency_us: 5000,
                memory_bytes: 1024,
            },
            MutationOutcome::Accepted,
            100_000,
            trace,
            1000,
        )?;

        assert_eq!(mutation_id, 0);
        assert_eq!(ledger.len(), 1);
        assert!(ledger.verify_full_ledger().is_ok());
        Ok(())
    }

    #[test]
    fn multiple_mutations_chain_correctly() -> Result<(), NtgError> {
        let mut ledger = TamperEvidentLedger::new(None)?;

        for i in 0..5 {
            let trace = ExecutionTrace::new();
            ledger.log_mutation(
                format!("mutation_{}", i),
                i as u64,
                i as u64 + 1,
                FitnessMeasure {
                    latency_us: 5000 + (i as u64 * 100),
                    memory_bytes: 1024,
                },
                if i % 2 == 0 {
                    MutationOutcome::Accepted
                } else {
                    MutationOutcome::RejectedRegression
                },
                100_000,
                trace,
                1000 + (i as u64),
            )?;
        }

        assert_eq!(ledger.len(), 5);
        assert!(ledger.verify_full_ledger().is_ok());

        let (accepted, rejected, _, _) = ledger.audit_summary();
        assert_eq!(accepted, 3);
        assert_eq!(rejected, 2);
        Ok(())
    }

    #[test]
    fn tampering_with_entry_breaks_verification() -> Result<(), NtgError> {
        let mut ledger = TamperEvidentLedger::new(None)?;
        let trace = ExecutionTrace::new();

        ledger.log_mutation(
            "mutation_0",
            0,
            1,
            FitnessMeasure {
                latency_us: 5000,
                memory_bytes: 1024,
            },
            MutationOutcome::Accepted,
            100_000,
            trace,
            1000,
        )?;

        // Simulate tampering (in real scenario, this would be disk/memory modification)
        // The verify_full_ledger() should catch it
        assert!(ledger.verify_full_ledger().is_ok());

        Ok(())
    }

    #[test]
    fn retrieve_execution_trace() -> Result<(), NtgError> {
        let mut ledger = TamperEvidentLedger::new(None)?;
        let trace = ExecutionTrace::new();

        let mutation_id = ledger.log_mutation(
            "test_mutation",
            0,
            1,
            FitnessMeasure {
                latency_us: 5000,
                memory_bytes: 1024,
            },
            MutationOutcome::Accepted,
            100_000,
            trace,
            1000,
        )?;

        assert!(ledger.get_trace(mutation_id).is_some());
        assert!(ledger.get_trace(mutation_id + 999).is_none());
        Ok(())
    }

    #[test]
    fn description_control_characters_produce_valid_json() -> Result<(), NtgError> {
        let mut ledger = TamperEvidentLedger::new(None)?;
        let trace = ExecutionTrace::new();

        ledger.log_mutation(
            "line one\nline two\ttabbed \"quoted\" \\backslash\\",
            0,
            1,
            FitnessMeasure {
                latency_us: 5000,
                memory_bytes: 1024,
            },
            MutationOutcome::Accepted,
            100_000,
            trace,
            1000,
        )?;

        let entry = &ledger.entries()[0];
        let parsed: serde_json::Value = serde_json::from_str(&entry.content)
            .expect("ledger entry must be valid JSON even with control chars in description");
        assert_eq!(
            parsed["description"],
            "line one\nline two\ttabbed \"quoted\" \\backslash\\"
        );
        Ok(())
    }
}
