/// Hot-swappable strategy dispatch table for runtime code replacement.
///
/// Named with the same genomics metaphor as the rest of this module
/// (chromosome_brain, agents, ...): a "gene" here is a named execution
/// strategy (`StrategyFn`), and "splicing" replaces one at runtime without
/// stopping callers that are mid-execution against the previous version.
///
/// Mechanism: the active `GeneticExpressionBlock` is held behind
/// `Mutex<Arc<...>>`. A reader takes the lock only long enough to clone
/// the `Arc` (a refcount bump, not a data copy), then drops the lock and
/// executes against its own owned handle. A writer (`splice_gene`) takes
/// the same lock, builds a new block by cloning the current strategy map
/// and inserting the replacement, then stores the new `Arc`. Because the
/// reader is holding its own `Arc` clone, the old block cannot be freed
/// out from under it even if a splice completes concurrently -- Rust's
/// ownership rules guarantee this, not manual bookkeeping.
///
/// This is NOT literally lock-free (both reads and writes take a mutex),
/// but the critical section is just an `Arc` clone/store, never the
/// strategy call itself -- so contention is limited to that brief window,
/// not to however long a strategy function takes to run. A truly
/// lock-free version (raw atomic pointer + manual epoch-based reclamation,
/// the pattern crates like `arc-swap`/`crossbeam-epoch` implement) would
/// need unsafe code with a correct grace-period scheme; that is easy to
/// get subtly wrong (an earlier draft of this exact engine had a
/// use-after-free from skipping the grace period), and there is no
/// measured need for it here, so this module deliberately favors the
/// simple, checkable-by-the-compiler version instead.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A named runtime-swappable execution strategy.
pub type StrategyFn = fn(f32) -> f32;

/// One generation's worth of active strategies.
#[derive(Clone)]
pub struct GeneticExpressionBlock {
    pub generation: u64,
    pub strategies: HashMap<String, StrategyFn>,
    pub methylation_weights: Vec<f32>,
}

impl GeneticExpressionBlock {
    pub fn new(strategies: HashMap<String, StrategyFn>, methylation_weights: Vec<f32>) -> Self {
        GeneticExpressionBlock {
            generation: 0,
            strategies,
            methylation_weights,
        }
    }
}

/// Runtime engine holding the currently active strategy block and
/// providing safe hot-swap replacement.
pub struct EpigeneticEngine {
    active_block: Mutex<Arc<GeneticExpressionBlock>>,
}

impl EpigeneticEngine {
    pub fn new(baseline: GeneticExpressionBlock) -> Self {
        EpigeneticEngine {
            active_block: Mutex::new(Arc::new(baseline)),
        }
    }

    /// Current generation number.
    pub fn generation(&self) -> u64 {
        self.active_block.lock().unwrap().generation
    }

    /// An owned handle to the currently active block. Cheap (Arc clone);
    /// once obtained, this handle stays valid regardless of any later
    /// `splice_gene` call.
    pub fn current(&self) -> Arc<GeneticExpressionBlock> {
        self.active_block.lock().unwrap().clone()
    }

    /// Execute a named strategy against the currently active generation.
    /// Falls through to the identity function if the key isn't found.
    pub fn execute_strategy(&self, target_key: &str, input: f32) -> f32 {
        let block = self.current();
        match block.strategies.get(target_key) {
            Some(strategy) => strategy(input),
            None => input,
        }
    }

    /// Replace one named strategy, advancing the generation. Callers
    /// already executing against the previous generation (i.e. holding a
    /// handle from an earlier `current()`/`execute_strategy()` call) are
    /// unaffected; new calls bind to the spliced generation. Returns the
    /// new generation number.
    pub fn splice_gene(&self, target_key: String, mutated_func: StrategyFn) -> u64 {
        let mut guard = self.active_block.lock().unwrap();

        let mut new_strategies = guard.strategies.clone();
        new_strategies.insert(target_key, mutated_func);

        let new_block = GeneticExpressionBlock {
            generation: guard.generation + 1,
            strategies: new_strategies,
            methylation_weights: guard.methylation_weights.clone(),
        };
        let new_generation = new_block.generation;

        *guard = Arc::new(new_block);
        new_generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    fn baseline(x: f32) -> f32 {
        x * 2.0
    }

    fn mutated(x: f32) -> f32 {
        x * 3.0
    }

    fn engine_with_baseline() -> EpigeneticEngine {
        let mut strategies: HashMap<String, StrategyFn> = HashMap::new();
        strategies.insert("velocity".to_string(), baseline as StrategyFn);
        EpigeneticEngine::new(GeneticExpressionBlock::new(strategies, vec![1.0]))
    }

    #[test]
    fn test_execute_baseline_strategy() {
        let engine = engine_with_baseline();
        assert_eq!(engine.execute_strategy("velocity", 5.0), 10.0);
        assert_eq!(engine.generation(), 0);
    }

    #[test]
    fn test_unknown_strategy_falls_through_identity() {
        let engine = engine_with_baseline();
        assert_eq!(engine.execute_strategy("nonexistent", 7.0), 7.0);
    }

    #[test]
    fn test_splice_gene_advances_generation_and_output() {
        let engine = engine_with_baseline();
        let new_gen = engine.splice_gene("velocity".to_string(), mutated);

        assert_eq!(new_gen, 1);
        assert_eq!(engine.generation(), 1);
        assert_eq!(engine.execute_strategy("velocity", 5.0), 15.0);
    }

    #[test]
    fn test_splice_adds_new_key_without_disturbing_existing_ones() {
        let engine = engine_with_baseline();
        engine.splice_gene("acceleration".to_string(), mutated);

        // Original strategy is untouched.
        assert_eq!(engine.execute_strategy("velocity", 5.0), 10.0);
        // New strategy is live.
        assert_eq!(engine.execute_strategy("acceleration", 5.0), 15.0);
        assert_eq!(engine.generation(), 1);
    }

    #[test]
    fn test_handle_from_current_survives_a_later_splice() {
        let engine = engine_with_baseline();
        let old_handle = engine.current();

        engine.splice_gene("velocity".to_string(), mutated);

        // The handle obtained before the splice still reflects generation 0.
        assert_eq!(old_handle.generation, 0);
        assert_eq!(old_handle.strategies.get("velocity").unwrap()(5.0), 10.0);
        // The engine itself has moved on.
        assert_eq!(engine.generation(), 1);
        assert_eq!(engine.execute_strategy("velocity", 5.0), 15.0);
    }

    /// Regression test for the exact bug class this module exists to
    /// avoid: concurrent readers calling execute_strategy while a writer
    /// repeatedly splices must never panic, deadlock, or observe a torn
    /// state, regardless of thread interleaving. An AtomicPtr-based
    /// version that frees the old block immediately after a successful
    /// compare_exchange (no grace period) is a use-after-free under
    /// exactly this workload.
    #[test]
    fn test_concurrent_reads_and_splices_never_crash_or_tear() {
        let engine = Arc::new(engine_with_baseline());
        let total_reads = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        for _ in 0..8 {
            let engine = Arc::clone(&engine);
            let total_reads = Arc::clone(&total_reads);
            handles.push(thread::spawn(move || {
                for _ in 0..2000 {
                    // Every possible generation multiplies by either 2 or
                    // 3 (baseline or mutated); either is a valid, whole
                    // (non-garbage) result for an integer input.
                    let out = engine.execute_strategy("velocity", 10.0);
                    assert!(out == 20.0 || out == 30.0, "torn/garbage read: {}", out);
                    total_reads.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        for _ in 0..4 {
            let engine = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    engine.splice_gene("velocity".to_string(), mutated);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(total_reads.load(Ordering::Relaxed), 8 * 2000);
        assert!(engine.generation() >= 1);
    }
}
