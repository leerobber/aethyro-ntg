//! KAIROS Stage 3 — Toddler: Phageguard drills under continuous cradle.
//!
//! **Default: wake the same continuous child.** Stage 3 opens immune tissue:
//! detect + quarantine (Load vs Deterministic), supervised drill pack.
//! Still forbids prune / real VCF / selection / self-mod.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage3
//!   cargo run --release --bin kairos_stage3 -- --to-child
//!   cargo run --release --bin kairos_stage3 -- --birth

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, LifeStage, StageGateResult,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_child = args.iter().any(|a| a == "--to-child");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 3 Toddler  |  VITASCALE Hostframe");
    println!("  Continuous child  |  Phageguard drills · lean immune");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  toddler_days={}  drills={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.toddler_care_days,
            k.drills_passed,
            rep.journal_len
        );
        println!("  cradle={}", rep.path);
        println!("  first words: \"{}\"", k.first_words());
    } else {
        println!("── BIRTH (first cradle / --birth) ──────────────────────");
        println!("  lineage={}  cradle={}", rep.lineage_id, rep.path);
        println!("  \"{}\"", k.first_words());
    }
    println!();

    // Catch-up path to toddler if needed.
    if k.stage() == LifeStage::Zygote {
        k.day_of_heartbeats(16).expect("zygote");
        if !matches!(k.try_graduate_zygote(), StageGateResult::Passed { .. }) {
            eprintln!("zygote gate failed");
            let _ = save_cradle(&cradle, &k);
            std::process::exit(1);
        }
        println!("[catch-up] zygote → neonate");
    }
    if k.stage() == LifeStage::Neonate {
        while k.neonate_care_days < 3 {
            k.day_of_neonate_care(8, 8).expect("neonate");
        }
        if !matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }) {
            eprintln!("neonate gate failed");
            let _ = save_cradle(&cradle, &k);
            std::process::exit(1);
        }
        println!("[catch-up] neonate → infant");
    }
    if k.stage() == LifeStage::Infant {
        let qs = [
            "Robert Lee Guardian trust",
            "haplotype LD chr22",
            "lean rails measure",
        ];
        for q in qs {
            if k.infant_care_days >= 3 {
                break;
            }
            k.day_of_infant_care(6, 4, q).expect("infant");
        }
        while k.infant_care_days < 3 {
            k.day_of_infant_care(6, 4, "guardian trust language").expect("infant");
        }
        if !matches!(k.try_graduate_infant(), StageGateResult::Passed { .. }) {
            eprintln!("infant gate failed");
            let _ = save_cradle(&cradle, &k);
            std::process::exit(1);
        }
        println!("[catch-up] infant → toddler");
        println!();
    }

    if k.stage() > LifeStage::Toddler {
        println!(
            "[status] stage={} — past toddler; Stage 3 care already complete.",
            k.stage().name()
        );
        println!("[hint] Stage 4 child (ntg_school) is the next build.");
        if let Err(e) = save_cradle(&cradle, &k) {
            eprintln!("cradle save failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    if k.stage() != LifeStage::Toddler {
        eprintln!("expected toddler after catch-up, got {}", k.stage().name());
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    println!("── PHAGEGUARD (detect · quarantine · drill) ──────────────");
    println!("  Load class: pulse storms → proposal freeze only");
    println!("  Deterministic: hostile / self-mod / weight → selection_veto");
    println!("  still forbid: prune · real VCF · selection loop · self-mod");
    println!();

    for day in 1..=3 {
        let report = k.day_of_toddler_care(6, 2).unwrap_or_else(|e| {
            eprintln!("toddler care failed: {e}");
            std::process::exit(1);
        });
        println!(
            "  day {day}: drill={} threats={} q_total={} drills_pass={} lang_n={} ws={}",
            if report.drill_passed { "PASS" } else { "FAIL" },
            report.threats_seen,
            report.quarantines_total,
            report.drills_passed,
            report.lang_nodes,
            report.ws_len
        );
        println!("       {}", report.drill_detail);
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }
    println!();

    if let Some(ref pg) = k.phageguard {
        println!("  {}", pg.summary_line());
    }

    let r = k.report();
    println!(
        "[status] {} | stage={} | toddler_days={} | drills={} | self_mod={} | guardian={}",
        r.name,
        r.stage.name(),
        k.toddler_care_days,
        k.drills_passed,
        r.self_mod_enabled,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_toddler_gate();
    println!("[toddler gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_child {
        match k.try_graduate_toddler() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  next: ntg_school (still no real-VCF dump)",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-child when toddler gate is green (earned, not waste)");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. Journal={}. Toddler_days={}. Drills={}.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.life.journal.len(),
        k.toddler_care_days,
        k.drills_passed
    );
    println!("  cradle={}", cradle.display());
    println!("  Phageguard awake. Trust holds. Rails hold.");
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}
