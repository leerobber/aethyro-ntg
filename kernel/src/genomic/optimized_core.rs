/// Fixed-slot (array-indexed) strategy engine: an alternative to
/// `epigenetic_engine::EpigeneticEngine`'s `HashMap<String, StrategyFn>`
/// for the case where the set of strategy slots is small, fixed, and
/// known at compile time. Array indexing by enum avoids a hash + string
/// compare per lookup; the tradeoff is a fixed upper bound on distinct
/// strategies (`MAX_GENES`) instead of an open-ended name space, and
/// indices are compile-time-checked via `GeneIndex` rather than raw
/// `usize` (so an out-of-range index is a compile error, not a runtime
/// panic).
///
/// Also adds an *optional* population-genetics-inspired gate for whether
/// to accept a candidate strategy: Kimura's fixation-probability formula,
/// applied to a measured (not asserted) fitness difference between the
/// candidate and the strategy it would replace. This produces a real
/// probability in [0, 1], not a guarantee -- population genetics doesn't
/// offer guarantees, and neither does this.
use crate::genomic::epigenetic_engine::StrategyFn;
use std::sync::{Arc, RwLock};

/// Upper bound on distinct strategy slots. Picked to keep the payload
/// small (32 * 8 bytes = 256 bytes) and give headroom over the 5 named
/// slots below -- not derived from any biological constant, despite the
/// naming theme.
pub const MAX_GENES: usize = 32;

/// Compile-time-checked slot indices instead of raw `usize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum GeneIndex {
    TranscriptionVelocity = 0,
    RibosomalThroughput = 1,
    MethylationGradient = 2,
    HaplotypeComparison = 3,
    RecombinationMatching = 4,
}

/// One generation of the fixed-slot strategy table.
///
/// `repr(align(64))` starts the struct on a fresh cache line. This does
/// NOT by itself "eliminate false sharing": false sharing is a concern
/// when multiple threads *write* to nearby memory concurrently, and
/// nothing here does that -- `splice_gene` always builds an entirely new
/// table and swaps the whole `Arc`, it never mutates a shared table's
/// array elements in place. The alignment's real effect is just ensuring
/// whichever core reads a freshly-swapped-in table starts its reads on a
/// cache line boundary.
#[repr(align(64))]
pub struct GeneTable {
    pub generation: u64,
    pub strategies: [Option<StrategyFn>; MAX_GENES],
}

impl GeneTable {
    pub fn new() -> Self {
        GeneTable {
            generation: 0,
            strategies: [None; MAX_GENES],
        }
    }

    pub fn with(mut self, gene: GeneIndex, strategy: StrategyFn) -> Self {
        self.strategies[gene as usize] = Some(strategy);
        self
    }
}

impl Default for GeneTable {
    fn default() -> Self {
        Self::new()
    }
}

/// A fixed set of representative inputs used to score a candidate
/// strategy against the one it might replace. "Fitness" here is defined
/// as closeness (negative mean squared error) to a caller-chosen target
/// function -- there is no universal notion of "better" for an arbitrary
/// `fn(f32) -> f32` without stating what it should be closer to.
pub struct FitnessBenchmark {
    pub inputs: Vec<f32>,
    pub target: fn(f32) -> f32,
}

impl FitnessBenchmark {
    pub fn new(inputs: Vec<f32>, target: fn(f32) -> f32) -> Self {
        FitnessBenchmark { inputs, target }
    }

    /// Negative mean squared error against `target` over `inputs` --
    /// higher (closer to 0) is better, 0.0 is a perfect match. `None`
    /// (an empty slot) is scored as the identity function, matching
    /// `execute_strategy`'s fallback.
    pub fn score(&self, strategy: Option<StrategyFn>) -> f64 {
        if self.inputs.is_empty() {
            return 0.0;
        }
        let mut sum_sq_err = 0.0f64;
        for &x in &self.inputs {
            let actual = match strategy {
                Some(f) => f(x),
                None => x,
            };
            let expected = (self.target)(x);
            let err = (actual - expected) as f64;
            sum_sq_err += err * err;
        }
        -(sum_sq_err / self.inputs.len() as f64)
    }
}

/// Result of a selection-gated splice attempt: the numbers that drove
/// the accept/reject decision, so a caller can log or assert on them.
#[derive(Debug, Clone, Copy)]
pub struct SelectionOutcome {
    pub accepted: bool,
    pub generation: u64,
    pub selection_coefficient: f64,
    pub fixation_probability: f64,
    pub baseline_fitness: f64,
    pub candidate_fitness: f64,
}

/// Kimura's diffusion-approximation fixation probability for a new
/// mutation with selection coefficient `s` in a population of effective
/// size `ne` (haploid Wright-Fisher convention). Reference: Kimura, M.
/// (1962), "On the Probability of Fixation of Mutant Genes in a
/// Population", Genetics 47(6):713-719.
///
/// `s > 0` favors fixation, `s < 0` disfavors it, `s == 0` is the neutral
/// case (probability `1 / (2*ne)`, the chance any single new copy
/// eventually fixes by drift alone). Always returns a finite value in
/// [0, 1] -- non-finite `s`, and `s`/`ne` combinations that would
/// otherwise overflow `exp()`, saturate to 0.0 or 1.0 by the sign of `s`
/// rather than producing NaN.
pub fn kimura_fixation_probability(s: f64, ne: f64) -> f64 {
    if !s.is_finite() || ne <= 0.0 {
        return 0.0;
    }
    if s.abs() < 1e-9 {
        return (1.0 / (2.0 * ne)).min(1.0);
    }

    let neg_2s = (-2.0 * s).exp();
    let neg_2nes = (-2.0 * ne * s).exp();

    if !neg_2s.is_finite() || !neg_2nes.is_finite() {
        return if s > 0.0 { 1.0 } else { 0.0 };
    }

    let numerator = 1.0 - neg_2s;
    let denominator = 1.0 - neg_2nes;
    if denominator.abs() < 1e-300 {
        return if s > 0.0 { 1.0 } else { 0.0 };
    }

    (numerator / denominator).clamp(0.0, 1.0)
}

/// Four named, independently-weighted telemetry channels, combined into
/// one scalar via caller-chosen weights. Not "4-dimensional" in any
/// deeper sense than "four numbers" -- named here so a caller has to be
/// explicit about what each channel means rather than passing an opaque
/// f32. The names match the four observability axes from
/// docs/architecture/0007 (structural/temporal/evolutionary/biological).
#[derive(Debug, Clone, Copy, Default)]
pub struct Telemetry4D {
    pub structural: f32,   // e.g. SIMD path / storage density signal
    pub temporal: f32,     // e.g. ledger/replay health signal
    pub evolutionary: f32, // e.g. mutation fitness trend
    pub biological: f32,   // e.g. genomic validation similarity
}

impl Telemetry4D {
    pub fn combined(&self, weights: [f32; 4]) -> f32 {
        self.structural * weights[0]
            + self.temporal * weights[1]
            + self.evolutionary * weights[2]
            + self.biological * weights[3]
    }
}

/// Fixed-slot hot-swap engine. Same Arc-based safety approach as
/// `EpigeneticEngine` (see that module's doc for why): readers clone the
/// `Arc` under a lock and execute against their own handle, so ownership
/// -- not manual bookkeeping -- keeps a generation alive for any
/// in-flight caller even after a concurrent splice. Uses `RwLock`
/// instead of `Mutex` so concurrent readers don't serialize against each
/// other, only against a writer.
pub struct SovereignEpigeneticEngine {
    current: RwLock<Arc<GeneTable>>,
}

impl SovereignEpigeneticEngine {
    pub fn new(baseline: GeneTable) -> Self {
        SovereignEpigeneticEngine {
            current: RwLock::new(Arc::new(baseline)),
        }
    }

    pub fn generation(&self) -> u64 {
        self.current.read().unwrap().generation
    }

    fn current_table(&self) -> Arc<GeneTable> {
        Arc::clone(&self.current.read().unwrap())
    }

    /// Hint to the CPU that `gene`'s slot is about to be read. Only
    /// potentially useful if followed immediately by an
    /// `execute_strategy` call for the *same* gene against the *same*
    /// generation -- a `splice_gene` call in between invalidates it,
    /// since splicing always allocates an entirely new table. No-op (and
    /// no unsafe code) on non-x86_64 targets.
    ///
    /// This is a hint, not a guaranteed speedup: at the scale of one
    /// 256-byte array it may make no measurable difference, since the
    /// array is likely already cache-resident after the first access in
    /// any realistic call pattern. See `optimized_core_demo`'s benchmark
    /// for measured numbers rather than an assumed effect.
    #[inline(always)]
    pub fn prefetch_strategy(&self, gene: GeneIndex) {
        #[cfg(target_arch = "x86_64")]
        {
            let table = self.current_table();
            let ptr = &table.strategies[gene as usize] as *const Option<StrategyFn> as *const i8;
            // Safety: `ptr` points into `table`, an Arc we hold for the
            // duration of this call, so it is valid for the prefetch.
            // _mm_prefetch never dereferences/reads through the pointer
            // in a way that can fault or produce UB even if speculative.
            unsafe {
                core::arch::x86_64::_mm_prefetch(ptr, core::arch::x86_64::_MM_HINT_T0);
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = gene;
        }
    }

    #[inline(always)]
    pub fn execute_strategy(&self, gene: GeneIndex, input: f32) -> f32 {
        let table = self.current_table();
        match table.strategies[gene as usize] {
            Some(strategy) => strategy(input),
            None => input,
        }
    }

    /// Unconditional splice: always replaces the slot, advances the
    /// generation, returns the new generation number.
    pub fn splice_gene(&self, gene: GeneIndex, mutated: StrategyFn) -> u64 {
        let mut guard = self.current.write().unwrap();
        let mut new_strategies = guard.strategies;
        new_strategies[gene as usize] = Some(mutated);
        let new_table = GeneTable {
            generation: guard.generation + 1,
            strategies: new_strategies,
        };
        let new_generation = new_table.generation;
        *guard = Arc::new(new_table);
        new_generation
    }

    /// Selection-gated splice: only commits if the candidate's measured
    /// fitness advantage over the current strategy, run through Kimura's
    /// fixation-probability formula, clears `min_fixation_probability`.
    /// If rejected, the engine's state (and generation) is unchanged.
    pub fn splice_gene_with_selection(
        &self,
        gene: GeneIndex,
        mutated: StrategyFn,
        benchmark: &FitnessBenchmark,
        effective_population_size: f64,
        min_fixation_probability: f64,
    ) -> SelectionOutcome {
        let mut guard = self.current.write().unwrap();

        let baseline_fn = guard.strategies[gene as usize];
        let baseline_fitness = benchmark.score(baseline_fn);
        let candidate_fitness = benchmark.score(Some(mutated));

        let selection_coefficient = if baseline_fitness.abs() > 1e-9 {
            (candidate_fitness - baseline_fitness) / baseline_fitness.abs()
        } else {
            candidate_fitness - baseline_fitness
        };

        let fixation_probability =
            kimura_fixation_probability(selection_coefficient, effective_population_size);
        let accepted = fixation_probability >= min_fixation_probability;

        let new_generation = if accepted {
            let mut new_strategies = guard.strategies;
            new_strategies[gene as usize] = Some(mutated);
            let new_table = GeneTable {
                generation: guard.generation + 1,
                strategies: new_strategies,
            };
            let g = new_table.generation;
            *guard = Arc::new(new_table);
            g
        } else {
            guard.generation
        };

        SelectionOutcome {
            accepted,
            generation: new_generation,
            selection_coefficient,
            fixation_probability,
            baseline_fitness,
            candidate_fitness,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(x: f32) -> f32 {
        x * 2.0
    }

    fn better_candidate(x: f32) -> f32 {
        x * 2.001 // very close to a target of x*2, slightly different
    }

    fn worse_candidate(x: f32) -> f32 {
        x * 20.0 // wildly off from a target of x*2
    }

    fn target_double(x: f32) -> f32 {
        x * 2.0
    }

    fn engine_with_baseline() -> SovereignEpigeneticEngine {
        let table = GeneTable::new().with(GeneIndex::TranscriptionVelocity, baseline);
        SovereignEpigeneticEngine::new(table)
    }

    #[test]
    fn test_execute_and_prefetch_baseline() {
        let engine = engine_with_baseline();
        engine.prefetch_strategy(GeneIndex::TranscriptionVelocity);
        assert_eq!(engine.execute_strategy(GeneIndex::TranscriptionVelocity, 5.0), 10.0);
        assert_eq!(engine.generation(), 0);
    }

    #[test]
    fn test_empty_slot_falls_through_identity() {
        let engine = engine_with_baseline();
        assert_eq!(engine.execute_strategy(GeneIndex::RibosomalThroughput, 7.0), 7.0);
    }

    #[test]
    fn test_unconditional_splice_advances_generation() {
        let engine = engine_with_baseline();
        let g = engine.splice_gene(GeneIndex::TranscriptionVelocity, better_candidate);
        assert_eq!(g, 1);
        assert_eq!(engine.generation(), 1);
    }

    #[test]
    fn test_kimura_neutral_case_matches_one_over_two_ne() {
        let p = kimura_fixation_probability(0.0, 1000.0);
        assert!((p - 1.0 / 2000.0).abs() < 1e-9);
    }

    #[test]
    fn test_kimura_beneficial_favored_over_deleterious() {
        let p_beneficial = kimura_fixation_probability(0.1, 500.0);
        let p_deleterious = kimura_fixation_probability(-0.1, 500.0);
        assert!(p_beneficial > p_deleterious);
    }

    #[test]
    fn test_kimura_strongly_beneficial_large_population_approaches_one() {
        let p = kimura_fixation_probability(5.0, 1000.0);
        assert!(p > 0.999);
    }

    #[test]
    fn test_kimura_strongly_deleterious_approaches_zero() {
        let p = kimura_fixation_probability(-5.0, 1000.0);
        assert!(p < 0.001);
    }

    #[test]
    fn test_kimura_extreme_and_nonfinite_inputs_never_panic_or_nan() {
        let cases = [
            (-1000.0, 1000.0),
            (1000.0, 1000.0),
            (f64::NAN, 100.0),
            (0.1, 0.0),
            (0.1, -5.0),
            (f64::INFINITY, 100.0),
            (f64::NEG_INFINITY, 100.0),
        ];
        for (s, ne) in cases {
            let p = kimura_fixation_probability(s, ne);
            assert!(p.is_finite(), "non-finite result for s={s}, ne={ne}");
            assert!((0.0..=1.0).contains(&p), "out-of-range result {p} for s={s}, ne={ne}");
        }
    }

    #[test]
    fn test_fitness_benchmark_perfect_match_scores_zero() {
        let bench = FitnessBenchmark::new(vec![1.0, 2.0, 3.0, 10.0], target_double);
        let score = bench.score(Some(baseline));
        assert!(score.abs() < 1e-9);
    }

    #[test]
    fn test_fitness_benchmark_worse_candidate_scores_lower() {
        let bench = FitnessBenchmark::new(vec![1.0, 2.0, 3.0, 10.0], target_double);
        let good = bench.score(Some(better_candidate));
        let bad = bench.score(Some(worse_candidate));
        assert!(good > bad);
    }

    #[test]
    fn test_beneficial_mutation_gets_accepted() {
        let engine = SovereignEpigeneticEngine::new(
            GeneTable::new().with(GeneIndex::TranscriptionVelocity, worse_candidate),
        );
        let bench = FitnessBenchmark::new(vec![1.0, 2.0, 3.0, 10.0], target_double);

        let outcome = engine.splice_gene_with_selection(
            GeneIndex::TranscriptionVelocity,
            better_candidate,
            &bench,
            1000.0, // large effective population size
            0.5,
        );

        assert!(outcome.accepted, "expected acceptance: {outcome:?}");
        assert_eq!(outcome.generation, 1);
        assert_eq!(engine.generation(), 1);
        assert!(outcome.selection_coefficient > 0.0);
    }

    #[test]
    fn test_deleterious_mutation_gets_rejected_and_state_unchanged() {
        let engine = SovereignEpigeneticEngine::new(
            GeneTable::new().with(GeneIndex::TranscriptionVelocity, better_candidate),
        );
        let bench = FitnessBenchmark::new(vec![1.0, 2.0, 3.0, 10.0], target_double);

        let outcome = engine.splice_gene_with_selection(
            GeneIndex::TranscriptionVelocity,
            worse_candidate,
            &bench,
            1000.0,
            0.5,
        );

        assert!(!outcome.accepted, "expected rejection: {outcome:?}");
        assert_eq!(outcome.generation, 0);
        assert_eq!(engine.generation(), 0);
        assert!(outcome.selection_coefficient < 0.0);

        // Rejected splice must leave the active strategy exactly as it was.
        let out = engine.execute_strategy(GeneIndex::TranscriptionVelocity, 5.0);
        assert_eq!(out, better_candidate(5.0));
    }

    #[test]
    fn test_telemetry_4d_combined_is_weighted_sum() {
        let t = Telemetry4D {
            structural: 1.0,
            temporal: 2.0,
            evolutionary: 3.0,
            biological: 4.0,
        };
        let combined = t.combined([0.1, 0.2, 0.3, 0.4]);
        // 1*0.1 + 2*0.2 + 3*0.3 + 4*0.4 = 0.1+0.4+0.9+1.6 = 3.0
        assert!((combined - 3.0).abs() < 1e-6);
    }
}
