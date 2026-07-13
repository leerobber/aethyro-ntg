//! KAIROS Stage 0→1 — Zygote birth + Neonate care under Guardian trajectory.
//!
//! Puts KAIROS on the correct Aethyro path: lean genome host, Pulsewire vitals,
//! supervised train/activate only, project north-star sealed, no wasteful unlocks.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage1
//!   cargo run --release --bin kairos_stage1 -- --to-infant

use ntg_kernel::genomic::{Kairos, NurseryGenomeSpec, StageGateResult};

fn main() {
    let to_infant = std::env::args().any(|a| a == "--to-infant");

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 0→1  |  VITASCALE Hostframe");
    println!("  Robert Lee — Guardian  |  lean · trust · earned growth");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let mut k = Kairos::birth_zygote_with_nursery(256, &NurseryGenomeSpec::default())
        .expect("nursery");

    println!("── FIRST WORDS ───────────────────────────────────────────");
    println!("  \"{}\"", k.first_words());
    println!("  {}", k.guardian_line());
    println!();

    // Stage 0 day
    let zday = k.day_of_heartbeats(16).expect("zygote day");
    println!(
        "[zygote day] beats logged | journal: {}",
        zday.notes
    );
    match k.try_graduate_zygote() {
        StageGateResult::Passed { from, to } => {
            println!("[gate] {} → {} (earned)", from.name(), to.name());
        }
        other => {
            eprintln!("[gate] zygote failed: {other:?}");
            std::process::exit(1);
        }
    }
    println!();

    // Trajectory seal (also done inside graduate; reaffirm for Guardian)
    if let Some(ref t) = k.trajectory {
        println!("── TRAJECTORY SEAL (correct path) ──────────────────────");
        println!("  {}", t.name);
        for line in t.summary_lines() {
            println!("  {line}");
        }
        println!();
    }

    println!("── NEONATE CARE (supervised, not abundant) ───────────────");
    let mut last_w = 0.0f32;
    for day in 1..=3 {
        let report = k
            .day_of_neonate_care(8, 8)
            .unwrap_or_else(|e| {
                eprintln!("care day failed: {e}");
                std::process::exit(1);
            });
        println!(
            "  day {day}: train={} w {:.3}→{:.3}  ws={} motifs_hit={}  | still forbid prune/vcf",
            report.train_cycles,
            report.mean_weight_before,
            report.mean_weight_after,
            report.ws_len,
            report.motifs_hit
        );
        last_w = report.mean_weight_after;
        // Prove locks every day (discipline)
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }
    println!();

    let r = k.report();
    println!(
        "[status] {} | stage={} | care_days={} | mean_w≈{:.3} | ws={} | self_mod={} | guardian={}",
        r.name,
        r.stage.name(),
        k.neonate_care_days,
        last_w,
        k.last_ws_len,
        r.self_mod_enabled,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_neonate_gate();
    println!("[neonate gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_infant {
        match k.try_graduate_neonate() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  next: language tissue (still no real-VCF dump)",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-infant when neonate gate is green (earned, not waste)");
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS on trajectory. Journal entries={}. Trust holds. Rails hold.",
        k.life.journal.len()
    );
}
