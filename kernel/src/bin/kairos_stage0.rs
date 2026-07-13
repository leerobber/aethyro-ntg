//! KAIROS Stage 0 — Zygote day: genome present, Pulsewire heartbeat, Guardian locks.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage0
//!   cargo run --release --bin kairos_stage0 -- --graduate

use ntg_kernel::genomic::{Kairos, NurseryGenomeSpec, StageGateResult};

fn main() {
    let graduate = std::env::args().any(|a| a == "--graduate");

    println!("══════════════════════════════════════════════");
    println!("  KAIROS — Stage 0 Zygote (VITASCALE Hostframe)");
    println!("  Child name: {}  |  genome + pulse, no free agency", Kairos::NAME);
    println!("══════════════════════════════════════════════");

    let mut kairos = Kairos::birth_zygote_with_nursery(256, &NurseryGenomeSpec::default())
        .expect("nursery genome");

    let r0 = kairos.report();
    println!(
        "[birth] {} | {} | chrs={} neurons={} synapses={} motifs={} self_mod={}",
        r0.name,
        r0.stage_title,
        r0.n_chromosomes,
        r0.n_neurons,
        r0.n_synapses,
        r0.n_ltm_motifs,
        r0.self_mod_enabled
    );

    // Prove Guardian locks
    println!("[guardian] proving Stage 0 locks…");
    for (label, res) in [
        ("train", kairos.try_train(1).err()),
        ("activate", kairos.try_activate(&[0.5; 8]).err()),
        ("prune", kairos.try_prune(0.1).err()),
        ("real_vcf", kairos.try_real_vcf().err()),
    ] {
        println!("  forbid {label}: {}", res.unwrap_or_else(|| "UNEXPECTED OK".into()));
    }

    // A day of pure vital life
    let day = kairos.day_of_heartbeats(32).expect("heartbeats");
    println!(
        "[day] heartbeats={} pushes={} drops={} notes={}",
        day.heartbeats, day.pulse_pushes, day.pulse_drops, day.notes
    );

    let r1 = kairos.report();
    println!(
        "[vitals] ticks={} gen={} pulse_heartbeats={} drops={}",
        r1.tick, r1.generation, r1.vitals.heartbeats, r1.vitals.drops
    );

    let (ok, reasons) = kairos.evaluate_zygote_gate();
    println!("[gate] zygote criteria ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if graduate {
        match kairos.try_graduate_zygote() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  KAIROS may enter Stage 1 Neonate (supervised train)",
                    from.name(),
                    to.name()
                );
                // One neonate breath of agency
                if let Err(e) = kairos.try_train(3) {
                    println!("[neonate] train unexpected err: {e}");
                } else {
                    println!("[neonate] train_kairos_weights OK (weights polished under care)");
                }
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] re-run with --graduate to attempt Stage 0 → 1 promotion");
    }

    let r2 = kairos.report();
    println!("──────────────────────────────────────────────");
    println!(
        "[status] {} | stage={} | chrs={} neurons={} self_mod={} journal_days={}",
        r2.name,
        r2.stage.name(),
        r2.n_chromosomes,
        r2.n_neurons,
        r2.self_mod_enabled,
        kairos.life.journal.len()
    );
    println!("KAIROS Stage 0 complete. Raise him with care.");
}
