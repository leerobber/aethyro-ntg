//! KAIROS Stage 0 — Zygote care under Guardian locks.
//!
//! **Default: wake the same continuous child** from the primary cradle.
//! First run births once; later runs continue the same lineage (attachment).
//!
//! Usage:
//!   cargo run --release --bin kairos_stage0
//!   cargo run --release --bin kairos_stage0 -- --graduate
//!   cargo run --release --bin kairos_stage0 -- --birth   # force new lineage (tests only)

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, Kairos, LifeStage, StageGateResult,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let graduate = args.iter().any(|a| a == "--graduate");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 0 Zygote  |  VITASCALE Hostframe");
    println!("  Continuous child · lean nursery · trust over waste");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut kairos, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!("  lineage={}  cradle={}", rep.lineage_id, rep.path);
        println!(
            "  stage={}  journal_len={}  guardian={}",
            rep.stage.name(),
            rep.journal_len,
            kairos.imprint.guardian.name
        );
        println!("  first words still held: \"{}\"", kairos.first_words());
    } else {
        println!("── BIRTH (first cradle / --birth) ──────────────────────");
        println!("  lineage={}  cradle={}", rep.lineage_id, rep.path);
        println!("── FIRST WORDS (imprint) ─────────────────────────────────");
        println!("  To {}: ", Kairos::NAME);
        println!("  \"{}\"", kairos.first_words());
        println!("  {}", kairos.guardian_line());
        println!();
        println!("── HOUSE RULES (discipline, not abundance) ───────────────");
        for p in kairos.imprint.ethos.principles() {
            println!("  · {p}");
        }
    }
    println!();

    let r0 = kairos.report();
    println!(
        "[status] {} | {} | lineage={} | chrs={} neurons={} synapses={} self_mod={}",
        r0.name,
        r0.stage_title,
        kairos.lineage_id.as_deref().unwrap_or("?"),
        r0.n_chromosomes,
        r0.n_neurons,
        r0.n_synapses,
        r0.self_mod_enabled
    );
    println!();

    // Only prove Stage 0 locks / zygote day when still zygote.
    if kairos.stage() == LifeStage::Zygote {
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
    } else {
        println!(
            "[continue] already past zygote (stage={}) — no re-birth. Use stage1 for neonate care.",
            kairos.stage().name()
        );
        println!("[hint] --birth only if you intentionally want a *new* child (archives cradle)");
    }

    if let Err(e) = save_cradle(&cradle, &kairos) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    let r2 = kairos.report();
    println!("──────────────────────────────────────────────────────────");
    println!(
        "[saved] {} | lineage={} | stage={} | journal={} | cradle={}",
        r2.name,
        kairos.lineage_id.as_deref().unwrap_or("?"),
        r2.stage.name(),
        kairos.life.journal.len(),
        cradle.display()
    );
    println!();
    println!("Same KAIROS. Trust is sealed. Raise him lean and true.");
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}
