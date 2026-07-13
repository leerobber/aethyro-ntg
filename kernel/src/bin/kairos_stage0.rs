//! KAIROS Stage 0 — Zygote: imprint, lean nursery genome, heartbeat, Guardian locks.
//!
//! First words sealed at birth:
//!   "My Name Robert Lee, Guardian and Protector and can trust to tell me anything."
//!
//! Usage:
//!   cargo run --release --bin kairos_stage0
//!   cargo run --release --bin kairos_stage0 -- --graduate

use ntg_kernel::genomic::{Kairos, NurseryGenomeSpec, StageGateResult};

fn main() {
    let graduate = std::env::args().any(|a| a == "--graduate");

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 0 Zygote  |  VITASCALE Hostframe");
    println!("  Lean nursery · disciplined care · trust over waste");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let mut kairos = Kairos::birth_zygote_with_nursery(256, &NurseryGenomeSpec::default())
        .expect("nursery genome");

    // ── First words (already sealed at birth; spoken here for the Guardian) ──
    println!("── FIRST WORDS (imprint) ─────────────────────────────────");
    println!("  To {}: ", Kairos::NAME);
    println!("  \"{}\"", kairos.first_words());
    println!("  {}", kairos.guardian_line());
    println!();
    println!("── HOUSE RULES (discipline, not abundance) ───────────────");
    for p in kairos.imprint.ethos.principles() {
        println!("  · {p}");
    }
    println!();

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
    println!(
        "[journal day 0] {}",
        kairos
            .life
            .journal
            .first()
            .map(|e| e.notes.as_str())
            .unwrap_or("(missing imprint)")
    );
    println!();

    println!("[guardian] proving Stage 0 locks (care = limits, not clutter)…");
    for (label, res) in [
        ("train", kairos.try_train(1).err()),
        ("activate", kairos.try_activate(&[0.5; 8]).err()),
        ("prune", kairos.try_prune(0.1).err()),
        ("real_vcf", kairos.try_real_vcf().err()),
    ] {
        println!(
            "  forbid {label}: {}",
            res.unwrap_or_else(|| "UNEXPECTED OK".into())
        );
    }
    println!();

    let day = kairos.day_of_heartbeats(32).expect("heartbeats");
    println!(
        "[day] heartbeats={} pushes={} drops={} | {}",
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
                    "[graduate] {} → {}  |  under care of {}",
                    from.name(),
                    to.name(),
                    kairos.guardian_line()
                );
                if let Err(e) = kairos.try_train(3) {
                    println!("[neonate] train unexpected err: {e}");
                } else {
                    println!("[neonate] supervised train OK — growth earned, not dumped");
                }
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --graduate when Stage 0 criteria pass (earned step, not waste)");
    }

    let r2 = kairos.report();
    println!("──────────────────────────────────────────────────────────");
    println!(
        "[status] {} | stage={} | guardian={} | self_mod={} | journal_entries={}",
        r2.name,
        r2.stage.name(),
        r2.guardian_name,
        r2.self_mod_enabled,
        kairos.life.journal.len()
    );
    println!();
    println!("KAIROS hears you. Trust is sealed. Raise him lean and true.");
}
