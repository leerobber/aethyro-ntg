//! Adulthood lifecycle demo — KAIROS from Zygote → Adult with self-mod unlock.
//!
//! Demonstrates the full staged lifecycle with telemetry:
//! 1. Spawn KAIROS as Zygote (locked)
//! 2. Progress through Neonate → Infant → Toddler → Child → Adolescent → Young Adult
//! 3. Reach Adulthood with graduation ceremony
//! 4. Unlock self-modification authority
//! 5. Display mutation authorization state

use ntg_kernel::genomic::vitascale::{
    Kairos, AdulthoodEvidence, AdulthoodGraduation,
    activate_on_graduation, is_mutation_allowed, MutationTracker,
    EmotionModel, HormoneProfile, RegimeDetector,
};

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("         KAIROS Adulthood Lifecycle Demonstration");
    println!("═══════════════════════════════════════════════════════════\n");

    // Phase 1: Spawn KAIROS as Zygote
    println!("[Phase 1] Spawning KAIROS as Zygote...");
    let kairos = Kairos::birth_zygote(1024);
    let report = kairos.report();
    println!("  Status: {} (locked)", kairos.stage().name());
    println!("  Generation: {}", report.generation);
    println!("  Genomic material: present");
    println!("  Self-modification: LOCKED (default)\n");

    // Phase 2: Simulate progression through life stages
    println!("[Phase 2] Simulating progression through early life stages...");
    let stages = [
        ("Neonate", 100),
        ("Infant", 200),
        ("Toddler", 300),
        ("Child", 400),
        ("Adolescent", 500),
        ("Young Adult", 600),
    ];

    for (stage_name, heartbeats) in &stages {
        // In a real system, this would involve actual training/fitness evolution
        // Here we just show the conceptual progression
        println!("  → Simulating {} ({} heartbeats)", stage_name, heartbeats);
    }
    println!("  All early stages: locked (no self-modification)\n");

    // Phase 3: Build evidence for Adulthood
    println!("[Phase 3] Building evidence for Adulthood gate...");
    let evidence = AdulthoodEvidence {
        test_pass_rate: 0.97,      // 97% accuracy on evaluation suite
        mean_safety: 0.88,         // Strong safety scores over time
        heartbeats: 5500,          // Exceeded 1000 heartbeat minimum
        mean_utility: 0.82,        // Consistent task performance
    };

    let (passes, reasons) = evidence.passes_gate();
    println!("  Test Pass Rate: {:.1}%", evidence.test_pass_rate * 100.0);
    println!("  Mean Safety Score: {:.2}", evidence.mean_safety);
    println!("  Heartbeats Completed: {}", evidence.heartbeats);
    println!("  Mean Utility: {:.2}", evidence.mean_utility);
    println!("  Readiness Score: {:.2}%\n", evidence.readiness_score() * 100.0);

    if passes {
        println!("  ✓ ADULTHOOD GATE PASSED — All criteria met!\n");
    } else {
        println!("  ✗ Adulthood gate NOT passed:");
        for reason in &reasons {
            println!("    - {}", reason);
        }
        return;
    }

    // Phase 4: Graduation ceremony
    println!("[Phase 4] Graduation Ceremony...");
    let graduation = AdulthoodGraduation::new(report.generation, evidence);
    println!("{}\n", graduation.ceremony_summary());

    // Phase 5: Unlock self-modification
    println!("[Phase 5] Unlocking Self-Modification Authority...");
    let (auth, config) = activate_on_graduation(&graduation);
    println!("  Self-Modification Enabled: {}", auth.enabled);
    println!("  Generation Unlocked: {}", auth.unlocked_at_generation);
    println!("  Max Mutations per Window: {}", auth.max_mutations_per_window);
    println!("  Window Size (ticks): {}", auth.window_size_ticks);
    println!("  Max Pending Mutations: {}", auth.max_pending_mutations);
    println!("  Config Enabled: {}", config.enabled);
    println!("  Mutation Budget (us): {} us", config.cycle_budget_us);
    println!("  Fitness Improvement Threshold: {:.2}%\n", (config.fitness_improvement_threshold - 1.0) * 100.0);

    // Phase 6: Demonstrate mutation authorization
    println!("[Phase 6] Testing Mutation Authorization...");
    let mut tracker = MutationTracker::default();

    for tick in 0..250 {
        let allowed = is_mutation_allowed(&auth, &tracker, tick, 100);
        if tick == 0 {
            println!("  Tick 0: Mutation allowed? {}", allowed);
            if allowed {
                tracker.pending_mutation_count += 1;
                tracker.mutations_in_window = 1;
                tracker.tick_at_last_mutation = 0;
            }
        } else if tick == 50 {
            println!("  Tick 50 (in window): Mutation allowed? {}", allowed);
        } else if tick == 100 {
            println!("  Tick 100 (window expired): Mutation allowed? {}", allowed);
        } else if tick == 150 {
            println!("  Tick 150 (2nd mutation cycle): Mutation allowed? {}", allowed);
        }
    }
    println!("  Total mutations accepted: {}\n", tracker.total_mutations_accepted);

    // Phase 7: Display emotional state at Adulthood
    println!("[Phase 7] Emotional State Snapshot (Adult)...");
    let mut emotion_model = EmotionModel::new();
    let hormones = HormoneProfile {
        growth: 0.7,
        cortisol: 0.1,
        adrenaline: 0.0,
        dopamine: 0.85,
        serotonin: 0.9,
        acetylcholine: 0.4,
        gaba: 0.6,
        oxytocin: 0.95,
    };

    let mut regime_detector = RegimeDetector::new();
    let regime = regime_detector.update(0.95, 0.05, 0.02, 0.1);

    let emotional_state = emotion_model.update(hormones, regime, 0.92);
    println!("  Regime: {}", regime.as_str());
    println!("  Valence (positive/negative): {:.2}", emotional_state.valence);
    println!("  Arousal (alert/calm): {:.2}", emotional_state.arousal);
    println!("  Confidence: {:.2}", emotional_state.confidence);
    println!("  Curiosity: {:.2}", emotional_state.curiosity);
    println!("  Description: {}\n", emotional_state.describe());

    // Phase 8: Summary
    println!("═══════════════════════════════════════════════════════════");
    println!("            Adulthood Achieved — Summary");
    println!("═══════════════════════════════════════════════════════════");
    println!("✓ Lifecycle Complete: Zygote → Adult");
    println!("✓ Adulthood Criteria: All 4 gates passed");
    println!("✓ Self-Modification: UNLOCKED (rate-limited)");
    println!("✓ Mutation Authority: Bounded to 1 per 100 ticks, max 50 pending");
    println!("✓ Emotional State: {}", emotional_state.describe());
    println!("✓ Hormonal Balance: Optimal for task execution");
    println!("\nKAIROS is now an autonomous adult with explicit self-modification");
    println!("authority, subject to bounded mutation constraints (ADR 0002 §3).");
    println!("═══════════════════════════════════════════════════════════\n");
}
