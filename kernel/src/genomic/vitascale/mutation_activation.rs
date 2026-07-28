//! Self-modification engine activation — unlocks mutations at Adulthood.
//! ADR 0002 §3, 0010 §5: Bounded mutation authority gated by lifecycle stage.

use super::adulthood_gate::AdulthoodGraduation;
use crate::ntg::mutation::SelfModConfig;
use crate::ntg::chain::ChainLog;

/// Bounded mutation authorization — enforced at runtime.
#[derive(Clone, Debug)]
pub struct MutationAuthorization {
    /// Is self-modification allowed?
    pub enabled: bool,
    /// Mutations per window (default 1 per 100 ticks).
    pub max_mutations_per_window: usize,
    pub window_size_ticks: u64,
    /// Maximum pending (unapplied) mutations.
    pub max_pending_mutations: usize,
    /// Generation that unlocked self-mod.
    pub unlocked_at_generation: u32,
}

impl Default for MutationAuthorization {
    fn default() -> Self {
        Self {
            enabled: false,
            max_mutations_per_window: 1,
            window_size_ticks: 100,
            max_pending_mutations: 50,
            unlocked_at_generation: 0,
        }
    }
}

/// Mutation lifecycle tracker — counts mutations in current window.
#[derive(Clone, Debug)]
pub struct MutationTracker {
    pub tick_at_last_mutation: u64,
    pub mutations_in_window: usize,
    pub pending_mutation_count: usize,
    pub total_mutations_accepted: usize,
}

impl Default for MutationTracker {
    fn default() -> Self {
        Self {
            tick_at_last_mutation: 0,
            mutations_in_window: 0,
            pending_mutation_count: 0,
            total_mutations_accepted: 0,
        }
    }
}

/// Activate self-modification authority upon Adulthood graduation.
pub fn activate_on_graduation(
    graduation: &AdulthoodGraduation,
) -> (MutationAuthorization, SelfModConfig) {
    let auth = MutationAuthorization {
        enabled: true,
        max_mutations_per_window: 1,
        window_size_ticks: 100,
        max_pending_mutations: 50,
        unlocked_at_generation: graduation.generation,
    };

    let config = SelfModConfig {
        enabled: true,
        cycle_budget_us: 1_000_000, // 1 ms per mutation cycle
        max_mutations_per_cycle: 5,
        fitness_improvement_threshold: 1.01, // 1% improvement required
        auto_rollback_on_regression: true,
    };

    (auth, config)
}

/// Check if a mutation is allowed under current constraints.
pub fn is_mutation_allowed(
    auth: &MutationAuthorization,
    tracker: &MutationTracker,
    current_tick: u64,
    max_ticks_since_last: u64,
) -> bool {
    // 1. Must be enabled
    if !auth.enabled {
        return false;
    }

    // 2. Cannot exceed pending mutation limit
    if tracker.pending_mutation_count >= auth.max_pending_mutations {
        return false;
    }

    // 3. Rate-limit: at most max_mutations_per_window in window_size_ticks
    let ticks_since_last = current_tick.saturating_sub(tracker.tick_at_last_mutation);
    if ticks_since_last >= auth.window_size_ticks {
        // Window expired; can mutate
        true
    } else {
        // In active window; reject if limit reached
        tracker.mutations_in_window < auth.max_mutations_per_window
    }
}

/// Record a mutation acceptance in the ledger.
pub fn log_mutation_to_ledger(
    ledger: &mut ChainLog,
    generation: u32,
    mutation_kind: &str,
    fitness_before: (u64, u64),
    fitness_after: (u64, u64),
) -> u64 {
    let entry = format!(
        "MUTATION gen={} kind={} latency_us_before={} latency_us_after={} memory_bytes_before={} memory_bytes_after={}",
        generation,
        mutation_kind,
        fitness_before.0,
        fitness_after.0,
        fitness_before.1,
        fitness_after.1
    );
    ledger.append(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_auth_default_disabled() {
        let auth = MutationAuthorization::default();
        assert!(!auth.enabled);
    }

    #[test]
    fn activation_on_graduation_enables_mutations() {
        use super::super::adulthood_gate::AdulthoodEvidence;
        let evidence = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.85,
            heartbeats: 5000,
            mean_utility: 0.80,
        };
        let grad = AdulthoodGraduation::new(42, evidence);
        let (auth, config) = activate_on_graduation(&grad);

        assert!(auth.enabled);
        assert!(config.enabled);
        assert_eq!(auth.unlocked_at_generation, 42);
    }

    #[test]
    fn mutation_allowed_when_enabled_and_under_limits() {
        let auth = MutationAuthorization {
            enabled: true,
            max_mutations_per_window: 2,
            window_size_ticks: 100,
            max_pending_mutations: 50,
            unlocked_at_generation: 1,
        };
        let tracker = MutationTracker {
            tick_at_last_mutation: 0,
            mutations_in_window: 0,
            pending_mutation_count: 10,
            total_mutations_accepted: 0,
        };
        assert!(is_mutation_allowed(&auth, &tracker, 50, 100));
    }

    #[test]
    fn mutation_rejected_when_disabled() {
        let auth = MutationAuthorization {
            enabled: false,
            ..Default::default()
        };
        let tracker = MutationTracker::default();
        assert!(!is_mutation_allowed(&auth, &tracker, 0, 100));
    }

    #[test]
    fn mutation_rejected_when_pending_limit_reached() {
        let auth = MutationAuthorization {
            enabled: true,
            max_pending_mutations: 5,
            ..Default::default()
        };
        let tracker = MutationTracker {
            pending_mutation_count: 5,
            ..Default::default()
        };
        assert!(!is_mutation_allowed(&auth, &tracker, 0, 100));
    }

    #[test]
    fn mutation_rejected_when_rate_limit_exceeded() {
        let auth = MutationAuthorization {
            enabled: true,
            max_mutations_per_window: 1,
            window_size_ticks: 100,
            max_pending_mutations: 50,
            unlocked_at_generation: 1,
        };
        let tracker = MutationTracker {
            tick_at_last_mutation: 10,
            mutations_in_window: 1,
            pending_mutation_count: 1,
            total_mutations_accepted: 0,
        };
        // Current tick 50 is within window of 100; mutation already happened; should reject
        assert!(!is_mutation_allowed(&auth, &tracker, 50, 100));
    }

    #[test]
    fn mutation_allowed_after_window_expires() {
        let auth = MutationAuthorization {
            enabled: true,
            max_mutations_per_window: 1,
            window_size_ticks: 100,
            max_pending_mutations: 50,
            unlocked_at_generation: 1,
        };
        let tracker = MutationTracker {
            tick_at_last_mutation: 0,
            mutations_in_window: 1,
            pending_mutation_count: 0,
            total_mutations_accepted: 1,
        };
        // Current tick 150, window started at 0, so window_size_ticks (100) ticks have passed
        assert!(is_mutation_allowed(&auth, &tracker, 150, 100));
    }

    #[test]
    fn ledger_records_mutation() {
        let mut ledger = ChainLog::new();
        let chain_val = log_mutation_to_ledger(
            &mut ledger,
            5,
            "AddNode",
            (1000, 2048),
            (950, 2000),
        );

        assert_eq!(ledger.len(), 1);
        assert!(chain_val != 0);
        let entry = &ledger.entries()[0];
        assert!(entry.content.contains("gen=5"));
        assert!(entry.content.contains("AddNode"));
        assert!(entry.content.contains("latency_us_before=1000"));
    }

    #[test]
    fn ledger_chains_mutations() {
        let mut ledger = ChainLog::new();
        let chain1 = log_mutation_to_ledger(&mut ledger, 1, "AddNode", (1000, 2048), (950, 2000));
        let chain2 = log_mutation_to_ledger(&mut ledger, 2, "RemoveEdge", (950, 2000), (900, 1950));

        assert_eq!(ledger.len(), 2);
        assert_ne!(chain1, chain2); // Chain values should differ
        assert!(ledger.verify().is_ok()); // Chain should verify
    }
}
