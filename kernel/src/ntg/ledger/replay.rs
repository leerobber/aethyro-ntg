//! Deterministic execution tracing and replay validation.
//!
//! ADR 0002 rule 4: "Deterministic replay. Same topology + same input must
//! produce the same output." This module logs every step of graph execution
//! (forward pass), enabling reproducibility verification and debugging.
//!
//! Each ExecutionTrace contains ReplayEvents, which are immutable once
//! recorded. Determinism is verified by re-replaying and comparing outputs.

use super::NtgError;

/// A single operation during graph execution: one node's forward pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayEvent {
    /// Which node executed (node ID)
    pub node_id: u32,
    /// Input signal (leaf signal or aggregated from children)
    pub input_signal: u64,
    /// Output after this node's computation
    pub output_signal: u64,
    /// Wall-clock or logical timestamp
    pub timestamp: u64,
}

/// Execution trace: ordered sequence of every node execution in a forward pass.
#[derive(Clone, Debug)]
pub struct ExecutionTrace {
    /// Events in execution order (deterministic via topological sort)
    pub events: Vec<ReplayEvent>,
    /// Graph fingerprint at execution time (for reproducibility checks)
    pub graph_fingerprint: u64,
    /// Output hash (aggregate of all final node outputs)
    pub output_hash: u64,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            graph_fingerprint: 0,
            output_hash: 0,
        }
    }

    pub fn with_fingerprint(graph_fingerprint: u64) -> Self {
        Self {
            events: Vec::new(),
            graph_fingerprint,
            output_hash: 0,
        }
    }

    /// Record a node's execution step.
    pub fn record_event(
        &mut self,
        node_id: u32,
        input_signal: u64,
        output_signal: u64,
        timestamp: u64,
    ) {
        self.events.push(ReplayEvent {
            node_id,
            input_signal,
            output_signal,
            timestamp,
        });
    }

    /// Set the final output hash (XOR of all terminal node outputs).
    pub fn set_output_hash(&mut self, hash: u64) {
        self.output_hash = hash;
    }

    /// Verify determinism: confirm this trace is self-consistent.
    /// Real determinism check (comparing two traces) happens at the
    /// ledger level; this just checks internal consistency.
    pub fn verify_determinism(&self) -> Result<(), NtgError> {
        // 1. Events must be in ascending node_id order (proof of topological sort)
        for i in 1..self.events.len() {
            if self.events[i].node_id < self.events[i - 1].node_id {
                return Err(NtgError::InvalidInput(
                    "Execution trace not in order: non-deterministic".to_string(),
                ));
            }
        }

        // 2. No timestamp should be earlier than the previous event
        for i in 1..self.events.len() {
            if self.events[i].timestamp < self.events[i - 1].timestamp {
                return Err(NtgError::InvalidInput(
                    "Timestamp ordering violation: clock regression".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Compare two traces for equality (same topology + same input should
    /// produce exactly matching traces). Returns true if identical.
    pub fn compare(&self, other: &ExecutionTrace) -> bool {
        if self.events.len() != other.events.len() {
            return false;
        }
        if self.graph_fingerprint != other.graph_fingerprint {
            return false;
        }
        if self.output_hash != other.output_hash {
            return false;
        }

        // Event-by-event comparison (most expensive, but most thorough)
        for (e1, e2) in self.events.iter().zip(other.events.iter()) {
            if e1.node_id != e2.node_id
                || e1.input_signal != e2.input_signal
                || e1.output_signal != e2.output_signal
            {
                return false;
            }
        }

        true
    }

    /// Summary: how many events, which nodes touched, range of signals.
    pub fn summary(&self) -> (usize, u32, u64) {
        let event_count = self.events.len();
        let max_node_id = self
            .events
            .iter()
            .map(|e| e.node_id)
            .max()
            .unwrap_or(0);
        let max_signal = self
            .events
            .iter()
            .map(|e| e.output_signal.max(e.input_signal))
            .max()
            .unwrap_or(0);
        (event_count, max_node_id, max_signal)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trace_verifies() {
        let trace = ExecutionTrace::new();
        assert!(trace.verify_determinism().is_ok());
    }

    #[test]
    fn record_events_in_order() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(1, 100, 200, 1000);
        trace.record_event(2, 200, 300, 1001);
        trace.record_event(3, 300, 400, 1002);

        assert_eq!(trace.len(), 3);
        assert!(trace.verify_determinism().is_ok());
    }

    #[test]
    fn out_of_order_nodes_fails_determinism() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(1, 100, 200, 1000);
        trace.record_event(3, 300, 400, 1001); // node 3 before node 2
        trace.record_event(2, 200, 300, 1002);

        assert!(trace.verify_determinism().is_err());
    }

    #[test]
    fn timestamp_regression_fails() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(1, 100, 200, 2000);
        trace.record_event(2, 200, 300, 1000); // timestamp before previous

        assert!(trace.verify_determinism().is_err());
    }

    #[test]
    fn identical_traces_compare_equal() {
        let mut t1 = ExecutionTrace::with_fingerprint(12345);
        t1.record_event(1, 100, 200, 1000);
        t1.record_event(2, 200, 300, 1001);
        t1.set_output_hash(999);

        let mut t2 = ExecutionTrace::with_fingerprint(12345);
        t2.record_event(1, 100, 200, 1000);
        t2.record_event(2, 200, 300, 1001);
        t2.set_output_hash(999);

        assert!(t1.compare(&t2));
    }

    #[test]
    fn different_events_compare_unequal() {
        let mut t1 = ExecutionTrace::with_fingerprint(12345);
        t1.record_event(1, 100, 200, 1000);
        t1.set_output_hash(999);

        let mut t2 = ExecutionTrace::with_fingerprint(12345);
        t2.record_event(1, 100, 999, 1000); // Different output_signal
        t2.set_output_hash(999);

        assert!(!t1.compare(&t2));
    }

    #[test]
    fn different_fingerprints_compare_unequal() {
        let mut t1 = ExecutionTrace::with_fingerprint(11111);
        t1.record_event(1, 100, 200, 1000);
        t1.set_output_hash(999);

        let mut t2 = ExecutionTrace::with_fingerprint(22222);
        t2.record_event(1, 100, 200, 1000);
        t2.set_output_hash(999);

        assert!(!t1.compare(&t2));
    }

    #[test]
    fn summary_works() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(1, 100, 5000, 1000);
        trace.record_event(3, 200, 6000, 1001);
        trace.record_event(5, 300, 7000, 1002);

        let (count, max_id, max_signal) = trace.summary();
        assert_eq!(count, 3);
        assert_eq!(max_id, 5);
        assert_eq!(max_signal, 7000);
    }
}
