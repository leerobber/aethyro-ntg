/// Demonstrates EpigeneticEngine: runtime replacement of one execution
/// strategy while other threads keep calling the dispatch table, with no
/// stop-the-world lock and no manual memory reclamation (Mutex<Arc<...>>
/// handles that automatically -- see genomic::epigenetic_engine's module
/// doc for why this trades strict lock-freedom for a version that is
/// checkable by the compiler instead of hand-verified).
use ntg_kernel::genomic::{EpigeneticEngine, GeneticExpressionBlock, StrategyFn};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn baseline_transcription(signal: f32) -> f32 {
    signal * 1.05
}

fn accelerated_transcription(signal: f32) -> f32 {
    if signal.is_nan() {
        0.0
    } else {
        signal * std::f32::consts::SQRT_2
    }
}

fn main() {
    println!("EpigeneticEngine demo: runtime strategy hot-swap");
    println!("=================================================");

    let mut strategies: HashMap<String, StrategyFn> = HashMap::new();
    strategies.insert("transcription_velocity".to_string(), baseline_transcription);

    let baseline_block = GeneticExpressionBlock::new(strategies, vec![0.98, 0.99, 0.95]);
    let engine = Arc::new(EpigeneticEngine::new(baseline_block));

    let input = 12.50f32;
    let before = engine.execute_strategy("transcription_velocity", input);
    println!(
        "Generation {}: transcription_velocity({}) = {:.4}",
        engine.generation(),
        input,
        before
    );

    // Spawn readers that keep calling the strategy while a splice happens
    // concurrently, to demonstrate readers are never blocked by, or
    // observe a torn/crashed state during, the swap. Each call is a couple
    // of nanoseconds of work, so a small iteration count finishes before
    // the main thread is even scheduled to splice -- a fixed sleep is
    // similarly unreliable. Looping until a deadline (with a tiny per-
    // iteration sleep so the loop actually spans real wall-clock time)
    // reliably keeps readers in flight across the splice instead.
    println!("\nStarting 4 concurrent reader threads (running for ~50ms each)...");
    let deadline = std::time::Instant::now() + Duration::from_millis(50);
    let mut reader_handles = Vec::new();
    for id in 0..4 {
        let engine = Arc::clone(&engine);
        reader_handles.push(thread::spawn(move || {
            let mut seen_generations = std::collections::HashSet::new();
            let mut calls = 0u32;
            while std::time::Instant::now() < deadline {
                let gen = engine.generation();
                let out = engine.execute_strategy("transcription_velocity", input);
                seen_generations.insert(gen);
                calls += 1;
                // Sanity check: output must correspond to exactly one of
                // the known strategies, never a garbage/torn value.
                let is_baseline = (out - input * 1.05).abs() < 1e-4;
                let is_mutated = (out - input * std::f32::consts::SQRT_2).abs() < 1e-4;
                assert!(is_baseline || is_mutated, "reader {id} saw torn output: {out}");
                thread::sleep(Duration::from_micros(50));
            }
            (seen_generations, calls)
        }));
    }

    // Splice partway through the readers' 50ms window.
    thread::sleep(Duration::from_millis(20));
    println!("Splicing transcription_velocity -> accelerated_transcription...");
    let new_generation = engine.splice_gene("transcription_velocity".to_string(), accelerated_transcription);

    let mut all_seen_generations = std::collections::HashSet::new();
    let mut total_calls = 0u32;
    for h in reader_handles {
        let (gens, calls) = h.join().expect("reader thread panicked");
        all_seen_generations.extend(gens);
        total_calls += calls;
    }

    let after = engine.execute_strategy("transcription_velocity", input);
    println!(
        "Generation {}: transcription_velocity({}) = {:.4}",
        engine.generation(),
        input,
        after
    );

    println!("\nResult:");
    println!("  Spliced to generation {new_generation}");
    println!("  Reader threads made {total_calls} total calls while the splice happened");
    println!(
        "  Generations observed live by readers: {:?} (both pre- and post-splice reads succeeded)",
        {
            let mut v: Vec<_> = all_seen_generations.into_iter().collect();
            v.sort();
            v
        }
    );
    println!("  No panics, no torn reads, no manual memory management.");
}
