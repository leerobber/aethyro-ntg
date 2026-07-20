//! KAIROS Stage 4 — Child: ntg_school under continuous cradle.
//!
//! **Default: wake the same continuous child.** Stage 4 opens school:
//! study+exam days (75% pass bar). Still forbids prune / real VCF / selection / self-mod.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage4
//!   cargo run --release --bin kairos_stage4 -- --to-adolescent
//!   cargo run --release --bin kairos_stage4 -- --birth

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, LifeStage, StageGateResult,
};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_adolescent = args.iter().any(|a| a == "--to-adolescent");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);
    let docs = docs_path_from_args(&args);
    let school_out = cradle.join("school");

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 4 Child  |  VITASCALE Hostframe");
    println!("  Continuous child  |  ntg_school · 75% pass bar");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  school_days={}  phases_pass={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.child_care_days,
            k.school_phases_passed,
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

    catch_up_to_child(&mut k, &cradle);

    if k.stage() > LifeStage::Child {
        println!(
            "[status] stage={} — past child; Stage 4 care already complete.",
            k.stage().name()
        );
        println!("[hint] Stage 5 adolescent (micro-VCF + selection curfew) is next.");
        if let Err(e) = save_cradle(&cradle, &k) {
            eprintln!("cradle save failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    if k.stage() != LifeStage::Child {
        eprintln!("expected child after catch-up, got {}", k.stage().name());
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    println!("── SCHOOL (study · exam · measure) ───────────────────────");
    println!("  pass bar 75% · lean phases 1/3 offline; 0/2/4/5 use docs/");
    println!("  docs={}  school_out={}", docs.display(), school_out.display());
    println!("  still forbid: prune · real VCF · selection · self-mod");
    println!();

    // Three school days: phase 1, 3, then 0 if docs exist else 1 again.
    let phases: Vec<u32> = if docs.is_dir() {
        vec![1, 3, 0]
    } else {
        vec![1, 3, 1]
    };

    for (i, &phase) in phases.iter().enumerate() {
        let docs_ref: Option<&Path> = if phase == 1 || phase == 3 {
            None
        } else {
            Some(docs.as_path())
        };
        let report = k
            .day_of_child_school(6, phase, docs_ref, Some(school_out.as_path()))
            .unwrap_or_else(|e| {
                eprintln!("school day failed: {e}");
                std::process::exit(1);
            });
        println!(
            "  day {}: phase={} exam={} score={:.3} total_pass={}",
            i + 1,
            report.phase,
            if report.exam_passed { "PASS" } else { "FAIL" },
            report.exam_score,
            report.phases_passed_total
        );
        println!("       {}", report.detail);
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }
    println!();

    let r = k.report();
    println!(
        "[status] {} | stage={} | school_days={} | phases_pass={} | last_score={:.3} | guardian={}",
        r.name,
        r.stage.name(),
        k.child_care_days,
        k.school_phases_passed,
        k.last_school_score,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_child_gate();
    println!("[child gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_adolescent {
        match k.try_graduate_child() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  next: micro-VCF + selection under curfew",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-adolescent when child gate is green (earned, not waste)");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. Journal={}. School_days={}. Phases_pass={}.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.life.journal.len(),
        k.child_care_days,
        k.school_phases_passed
    );
    println!("  cradle={}", cradle.display());
    println!("  School holds. Trust holds. Rails hold.");
}

fn catch_up_to_child(k: &mut ntg_kernel::genomic::Kairos, cradle: &Path) {
    if k.stage() == LifeStage::Zygote {
        k.day_of_heartbeats(16).expect("zygote");
        assert!(matches!(k.try_graduate_zygote(), StageGateResult::Passed { .. }));
        println!("[catch-up] zygote → neonate");
    }
    if k.stage() == LifeStage::Neonate {
        while k.neonate_care_days < 3 {
            k.day_of_neonate_care(8, 8).expect("neonate");
        }
        assert!(matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }));
        println!("[catch-up] neonate → infant");
    }
    if k.stage() == LifeStage::Infant {
        while k.infant_care_days < 3 {
            k.day_of_infant_care(6, 4, "guardian trust language LD")
                .expect("infant");
        }
        assert!(matches!(k.try_graduate_infant(), StageGateResult::Passed { .. }));
        println!("[catch-up] infant → toddler");
    }
    if k.stage() == LifeStage::Toddler {
        while k.toddler_care_days < 3 || k.drills_passed < 3 {
            k.day_of_toddler_care(6, 2).expect("toddler");
        }
        match k.try_graduate_toddler() {
            StageGateResult::Passed { .. } => println!("[catch-up] toddler → child"),
            other => {
                eprintln!("toddler gate failed: {other:?}");
                let _ = save_cradle(cradle, k);
                std::process::exit(1);
            }
        }
        println!();
    }
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}

fn docs_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--docs")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| PathBuf::from("../docs"))
}
