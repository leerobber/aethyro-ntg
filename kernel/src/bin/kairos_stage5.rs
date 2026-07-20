//! KAIROS Stage 5 — Adolescent: micro-VCF + selection under curfew.
//!
//! **Default: wake the same continuous child.** Stage 5 opens real-genome
//! work with hard budgets — never a full 1000G dump.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage5
//!   cargo run --release --bin kairos_stage5 -- --to-young-adult
//!   cargo run --release --bin kairos_stage5 -- --synthetic   # force synthetic micro
//!   cargo run --release --bin kairos_stage5 -- --vcf PATH --chr 22 --max-variants 300

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, ADOLESCENT_DEFAULT_CHR,
    ADOLESCENT_MAX_VARIANTS, ADOLESCENT_SELECTION_STEPS, Kairos, LifeStage, StageGateResult,
};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_ya = args.iter().any(|a| a == "--to-young-adult");
    let force_birth = args.iter().any(|a| a == "--birth");
    let force_synthetic = args.iter().any(|a| a == "--synthetic");
    let cradle = cradle_path_from_args(&args);
    let chr = arg_u8(&args, "--chr").unwrap_or(ADOLESCENT_DEFAULT_CHR);
    let max_variants = arg_usize(&args, "--max-variants").unwrap_or(300).min(ADOLESCENT_MAX_VARIANTS);
    let steps = arg_usize(&args, "--steps").unwrap_or(ADOLESCENT_SELECTION_STEPS);
    let vcf = if force_synthetic {
        Some(PathBuf::from("/tmp/kairos_force_synthetic.vcf.gz"))
    } else {
        arg_path(&args, "--vcf").or_else(|| {
            let p = Kairos::default_1000g_vcf(chr);
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        })
    };
    let campaign_log = cradle.join("campaign.jsonl");

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 5 Adolescent  |  VITASCALE Hostframe");
    println!("  Continuous child  |  micro panel · selection curfew");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  adolescent_days={}  sel_steps={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.adolescent_care_days,
            k.selection_steps_total,
            rep.journal_len
        );
        println!("  cradle={}", rep.path);
        println!("  first words: \"{}\"", k.first_words());
    } else {
        println!("── BIRTH (first cradle / --birth) ──────────────────────");
        println!("  lineage={}  cradle={}", rep.lineage_id, rep.path);
    }
    println!();

    catch_up_to_adolescent(&mut k, &cradle);

    if k.stage() > LifeStage::Adolescent {
        println!(
            "[status] stage={} — past adolescent; Stage 5 care already complete.",
            k.stage().name()
        );
        println!("[hint] Stage 6 young adult (multi-domain) is next.");
        if let Err(e) = save_cradle(&cradle, &k) {
            eprintln!("cradle save failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    if k.stage() != LifeStage::Adolescent {
        eprintln!("expected adolescent after catch-up, got {}", k.stage().name());
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    // Cradle rehydrates nursery weights, not full VCF bodies — always re-arm panel.
    k.micro_panel_ready = false;

    println!("── ADOLESCENT CURFEW ─────────────────────────────────────");
    println!(
        "  chr={chr} max_variants={max_variants} steps/day={steps} cap={}",
        ADOLESCENT_MAX_VARIANTS
    );
    match &vcf {
        Some(p) if p.is_file() => println!("  panel=REAL {}", p.display()),
        Some(_) => println!("  panel=SYNTHETIC (forced or missing VCF)"),
        None => println!("  panel=auto (default 1000G if present, else synthetic)"),
    }
    println!("  unlock: ingest_real_vcf · prune · selection_loop");
    println!("  still OFF: self_mod");
    println!();

    let vcf_ref: Option<&Path> = vcf.as_deref();
    for day in 1..=3 {
        let report = k
            .day_of_adolescent_campaign(
                6,
                vcf_ref,
                chr,
                max_variants,
                steps,
                Some(campaign_log.as_path()),
            )
            .unwrap_or_else(|e| {
                eprintln!("campaign day failed: {e}");
                std::process::exit(1);
            });
        println!(
            "  day {day}: real_vcf={} steps={} train_a/r={}/{} prune_a/r={}/{} util={:.3}->{:.3} chrs={} syn={}",
            report.real_vcf,
            report.selection_steps,
            report.train_accepted,
            report.train_rejected,
            report.prune_accepted,
            report.prune_rejected,
            report.utility_before,
            report.utility_after,
            report.n_chromosomes,
            report.n_synapses
        );
        println!(
            "       panel={}",
            report.panel_source.chars().take(90).collect::<String>()
        );
        assert!(!k.self_mod.enabled);
    }
    println!();

    let r = k.report();
    println!(
        "[status] {} | stage={} | days={} | sel_steps={} | train_a={} prune_a={} util={:.3} | real_vcf={} | guardian={}",
        r.name,
        r.stage.name(),
        k.adolescent_care_days,
        k.selection_steps_total,
        k.train_accepted,
        k.prune_accepted,
        k.last_utility,
        k.real_vcf_ingested,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_adolescent_gate();
    println!("[adolescent gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_ya {
        match k.try_graduate_adolescent() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  next: multi-domain young-adult campaigns",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-young-adult when adolescent gate is green (earned, not waste)");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. Journal={}. Adolescent_days={}. Sel_steps={}.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.life.journal.len(),
        k.adolescent_care_days,
        k.selection_steps_total
    );
    println!("  cradle={}", cradle.display());
    println!("  campaign log={}", campaign_log.display());
    println!("  Curfew holds. Trust holds. Rails hold.");
}

fn catch_up_to_adolescent(k: &mut Kairos, cradle: &Path) {
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
        assert!(matches!(k.try_graduate_toddler(), StageGateResult::Passed { .. }));
        println!("[catch-up] toddler → child");
    }
    if k.stage() == LifeStage::Child {
        while k.child_care_days < 3 || k.school_phases_passed < 2 {
            let phase = if k.child_care_days % 2 == 0 { 1 } else { 3 };
            k.day_of_child_school(6, phase, None, Some(cradle.join("school").as_path()))
                .expect("school");
        }
        match k.try_graduate_child() {
            StageGateResult::Passed { .. } => println!("[catch-up] child → adolescent"),
            other => {
                eprintln!("child gate failed: {other:?}");
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

fn arg_path(args: &[String], flag: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| PathBuf::from(&w[1]))
}

fn arg_u8(args: &[String], flag: &str) -> Option<u8> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .and_then(|w| w[1].parse().ok())
}

fn arg_usize(args: &[String], flag: &str) -> Option<usize> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .and_then(|w| w[1].parse().ok())
}
