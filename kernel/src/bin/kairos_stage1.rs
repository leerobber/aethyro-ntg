//! KAIROS Stage 0→1 — continuous neonate care under Guardian trajectory.
//!
//! **Default: wake the same continuous child** from the primary cradle.
//! Care days, weights, and journal accumulate across runs (attachment).
//!
//! Usage:
//!   cargo run --release --bin kairos_stage1
//!   cargo run --release --bin kairos_stage1 -- --to-infant
//!   cargo run --release --bin kairos_stage1 -- --birth   # force new lineage (tests only)

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, LifeStage, StageGateResult,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_infant = args.iter().any(|a| a == "--to-infant");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 0→1  |  VITASCALE Hostframe");
    println!("  Continuous child  |  lean · trust · earned growth");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  care_days={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.neonate_care_days,
            rep.journal_len
        );
        println!("  cradle={}", rep.path);
        println!("  first words: \"{}\"", k.first_words());
    } else {
        println!("── BIRTH (first cradle / --birth) ──────────────────────");
        println!("  lineage={}  cradle={}", rep.lineage_id, rep.path);
        println!("── FIRST WORDS ───────────────────────────────────────────");
        println!("  \"{}\"", k.first_words());
        println!("  {}", k.guardian_line());
    }
    println!();

    // Zygote day + gate if still Stage 0
    if k.stage() == LifeStage::Zygote {
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
                let _ = save_cradle(&cradle, &k);
                std::process::exit(1);
            }
        }
        println!();
    } else if k.stage() == LifeStage::Neonate {
        println!(
            "[continue] neonate already — skipping re-birth; care continues (days so far={})",
            k.neonate_care_days
        );
        if k.trajectory.is_none() {
            k.seal_trajectory();
        }
        println!();
    } else {
        println!(
            "[status] stage={} — past neonate care path; use later stage bins when ready.",
            k.stage().name()
        );
    }

    if let Some(ref t) = k.trajectory {
        println!("── TRAJECTORY SEAL (correct path) ──────────────────────");
        println!("  {}", t.name);
        for line in t.summary_lines() {
            println!("  {line}");
        }
        println!();
    }

    if k.stage() == LifeStage::Neonate {
        println!("── NEONATE CARE (supervised, not abundant) ───────────────");
        let mut last_w = k.last_mean_weight;
        for day in 1..=3 {
            let report = k.day_of_neonate_care(8, 8).unwrap_or_else(|e| {
                eprintln!("care day failed: {e}");
                std::process::exit(1);
            });
            println!(
                "  session day {day}: train={} w {:.3}→{:.3}  ws={} motifs_hit={}  | total_care_days={}",
                report.train_cycles,
                report.mean_weight_before,
                report.mean_weight_after,
                report.ws_len,
                report.motifs_hit,
                k.neonate_care_days
            );
            last_w = report.mean_weight_after;
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
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. Journal={}. Care_days={}. Same child next wake.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.life.journal.len(),
        k.neonate_care_days
    );
    println!("  cradle={}", cradle.display());
    println!("  --birth only for deliberate new child (archives old cradle)");
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}
