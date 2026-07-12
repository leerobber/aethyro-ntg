/// Demonstrates SovereignEpigeneticEngine: fixed-slot (array-indexed)
/// strategy hot-swap, an unconditional splice, a population-genetics-
/// gated splice (Kimura fixation probability against a measured fitness
/// difference), and a real benchmark of the prefetch hint's effect --
/// measured, not asserted.
use ntg_kernel::genomic::{
    FitnessBenchmark, GeneIndex, GeneTable, SovereignEpigeneticEngine, Telemetry4D,
};
use std::time::Instant;

fn baseline_transcription(signal: f32) -> f32 {
    signal * 1.05
}

fn accelerated_transcription(signal: f32) -> f32 {
    if signal.is_nan() {
        0.0
    } else {
        signal * 1.4142
    }
}

// A deliberately bad "mutation" for the selection-gate demo: further from
// the target the fitness benchmark is scored against than the baseline is.
fn regressed_transcription(signal: f32) -> f32 {
    signal * 5.0
}

fn target_transcription(signal: f32) -> f32 {
    signal * 1.4142 // what "good" looks like, for benchmark scoring
}

fn baseline_recombination(signal: f32) -> f32 {
    signal + 0.05
}

fn optimized_recombination(signal: f32) -> f32 {
    signal + 0.9917
}

fn main() {
    println!("SovereignEpigeneticEngine demo: fixed-slot hot-swap + selection gate");
    println!("=======================================================================");

    let table = GeneTable::new()
        .with(GeneIndex::TranscriptionVelocity, baseline_transcription)
        .with(GeneIndex::RecombinationMatching, baseline_recombination);
    let engine = SovereignEpigeneticEngine::new(table);

    let input = 12.50f32;
    println!(
        "\nGeneration {}: transcription_velocity({input}) = {:.4}, recombination_matching({input}) = {:.4}",
        engine.generation(),
        engine.execute_strategy(GeneIndex::TranscriptionVelocity, input),
        engine.execute_strategy(GeneIndex::RecombinationMatching, input),
    );

    // ---- Unconditional splice on a different gene, to keep
    // TranscriptionVelocity's baseline available for the selection-gate
    // demo below. ----
    println!("\n[Unconditional splice] recombination_matching -> optimized_recombination");
    let g = engine.splice_gene(GeneIndex::RecombinationMatching, optimized_recombination);
    println!(
        "Generation {g}: recombination_matching({input}) = {:.4}",
        engine.execute_strategy(GeneIndex::RecombinationMatching, input)
    );

    // ---- Selection-gated splice: a genuinely better candidate ----
    // Active strategy is still baseline_transcription (x*1.05); the
    // candidate matches the benchmark target exactly (x*1.4142), so this
    // should show a clear positive selection coefficient and be accepted.
    println!("\n[Selection-gated splice] Candidate close to target (expect ACCEPT):");
    let bench = FitnessBenchmark::new(vec![1.0, 5.0, 10.0, 50.0, 100.0], target_transcription);
    let outcome = engine.splice_gene_with_selection(
        GeneIndex::TranscriptionVelocity,
        accelerated_transcription,
        &bench,
        1000.0, // effective population size
        0.5,    // minimum fixation probability to accept
    );
    println!(
        "  baseline_fitness={:.6} candidate_fitness={:.6} s={:.6} p_fix={:.6} accepted={} generation={}",
        outcome.baseline_fitness,
        outcome.candidate_fitness,
        outcome.selection_coefficient,
        outcome.fixation_probability,
        outcome.accepted,
        outcome.generation
    );
    assert!(outcome.accepted, "demo invariant violated: expected this splice to be accepted");

    // ---- Selection-gated splice: now compare a worse candidate against
    // the just-accepted (better) baseline ----
    println!("\n[Selection-gated splice] Candidate further from target than current (expect REJECT):");
    let outcome = engine.splice_gene_with_selection(
        GeneIndex::TranscriptionVelocity,
        regressed_transcription,
        &bench,
        1000.0,
        0.5,
    );
    println!(
        "  baseline_fitness={:.6} candidate_fitness={:.6} s={:.6} p_fix={:.6} accepted={} generation={}",
        outcome.baseline_fitness,
        outcome.candidate_fitness,
        outcome.selection_coefficient,
        outcome.fixation_probability,
        outcome.accepted,
        outcome.generation
    );
    assert!(!outcome.accepted, "demo invariant violated: expected this splice to be rejected");
    println!(
        "  Post-rejection check: transcription_velocity({input}) = {:.4} (unchanged -- still accelerated_transcription)",
        engine.execute_strategy(GeneIndex::TranscriptionVelocity, input)
    );

    // ---- Telemetry4D: four named channels, explicitly combined ----
    let telemetry = Telemetry4D {
        structural: 0.9,
        temporal: 1.0,
        evolutionary: outcome.fixation_probability as f32,
        biological: 0.94, // e.g. a Phase E genome-wide similarity reading
    };
    println!(
        "\n[Telemetry4D] structural={:.2} temporal={:.2} evolutionary={:.4} biological={:.2} -> combined={:.4}",
        telemetry.structural,
        telemetry.temporal,
        telemetry.evolutionary,
        telemetry.biological,
        telemetry.combined([0.25, 0.25, 0.25, 0.25])
    );

    // ---- Real prefetch benchmark, not an assumed effect ----
    println!("\n[Prefetch benchmark] measuring, not asserting, any effect:");
    const ITERS: u32 = 2_000_000;

    let start = Instant::now();
    let mut acc = 0.0f32;
    for _ in 0..ITERS {
        engine.prefetch_strategy(GeneIndex::TranscriptionVelocity);
        acc += engine.execute_strategy(GeneIndex::TranscriptionVelocity, input);
    }
    let with_prefetch = start.elapsed();
    std::hint::black_box(acc);

    let start = Instant::now();
    let mut acc = 0.0f32;
    for _ in 0..ITERS {
        acc += engine.execute_strategy(GeneIndex::TranscriptionVelocity, input);
    }
    let without_prefetch = start.elapsed();
    std::hint::black_box(acc);

    let ns_with = with_prefetch.as_secs_f64() * 1e9 / ITERS as f64;
    let ns_without = without_prefetch.as_secs_f64() * 1e9 / ITERS as f64;
    let delta = ns_with - ns_without;
    println!("  {ITERS} calls with    prefetch: {with_prefetch:?} ({ns_with:.2} ns/call)");
    println!("  {ITERS} calls without prefetch: {without_prefetch:?} ({ns_without:.2} ns/call)");
    let verdict = if delta.abs() < 1.0 {
        "no measurable difference (within noise)"
    } else if delta < 0.0 {
        "prefetch measured faster"
    } else {
        "prefetch measured SLOWER -- consistent with this module's doc comment: a 256-byte \
         array is already cache-resident after the first access, so the prefetch call is \
         pure overhead here, not a win"
    };
    println!("  Difference: {delta:.2} ns/call ({verdict})");

    println!("\nDone. No panics, no unsafe outside the single x86_64 prefetch intrinsic call.");
}
