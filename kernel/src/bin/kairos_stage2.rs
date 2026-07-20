//! KAIROS Stage 2 — Infant: language tissue under continuous cradle.
//!
//! **Default: wake the same continuous child.** Stage 2 opens language:
//! lean curriculum, light calib, text→signature→genomic activate.
//! Still forbids prune / real VCF / phageguard / selection / self-mod.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage2
//!   cargo run --release --bin kairos_stage2 -- --to-toddler
//!   cargo run --release --bin kairos_stage2 -- --birth   # force new lineage (tests only)

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, LifeStage, StageGateResult,
};
use std::path::PathBuf;

const INFANT_QUERIES: &[&str] = &[
    "My Name Robert Lee Guardian and Protector trust",
    "haplotype LD on chr22 chromosome brain KAIROS",
    "lean path rails before freedom measure don't assume",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_toddler = args.iter().any(|a| a == "--to-toddler");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 2 Infant  |  VITASCALE Hostframe");
    println!("  Continuous child  |  language tissue · lean curriculum");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  neonate_days={}  infant_days={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.neonate_care_days,
            k.infant_care_days,
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

    // Catch-up: zygote → neonate → infant if needed (same child path).
    if k.stage() == LifeStage::Zygote {
        println!("[catch-up] zygote → neonate…");
        k.day_of_heartbeats(16).expect("zygote day");
        match k.try_graduate_zygote() {
            StageGateResult::Passed { from, to } => {
                println!("[gate] {} → {}", from.name(), to.name());
            }
            other => {
                eprintln!("[gate] zygote failed: {other:?}");
                let _ = save_cradle(&cradle, &k);
                std::process::exit(1);
            }
        }
    }
    if k.stage() == LifeStage::Neonate {
        println!("[catch-up] neonate care → infant…");
        while k.neonate_care_days < 3 {
            k.day_of_neonate_care(8, 8).expect("neonate care");
        }
        match k.try_graduate_neonate() {
            StageGateResult::Passed { from, to } => {
                println!("[gate] {} → {}", from.name(), to.name());
            }
            other => {
                eprintln!("[gate] neonate failed: {other:?}");
                let _ = save_cradle(&cradle, &k);
                std::process::exit(1);
            }
        }
        println!();
    }

    if k.stage() > LifeStage::Infant {
        println!(
            "[status] stage={} — past infant; Stage 2 care already complete.",
            k.stage().name()
        );
        println!("[hint] Stage 3 toddler (phageguard) is the next build.");
        if let Err(e) = save_cradle(&cradle, &k) {
            eprintln!("cradle save failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    if k.stage() != LifeStage::Infant {
        eprintln!(
            "expected infant after catch-up, got {}",
            k.stage().name()
        );
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    if let Some(ref t) = k.trajectory {
        println!("── TRAJECTORY (held) ───────────────────────────────────");
        println!("  {}", t.name);
        println!();
    }

    println!("── INFANT CARE (language tissue, lean) ───────────────────");
    println!("  curriculum: guardian words · host identity · genome↔language · lean path");
    println!("  still forbid: prune · real VCF · phageguard · selection · self-mod");
    println!();

    for (i, q) in INFANT_QUERIES.iter().enumerate() {
        let report = k.day_of_infant_care(6, 4, q).unwrap_or_else(|e| {
            eprintln!("infant care failed: {e}");
            std::process::exit(1);
        });
        println!(
            "  day {}: docs={} lang_nodes={} ws={} motifs={} calib_bal={:.3}",
            i + 1,
            report.docs_ingested,
            report.lang_nodes,
            report.ws_len,
            report.motifs_hit,
            report.calib_bal
        );
        println!("       q=\"{}\"", report.query);
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }
    println!();

    let r = k.report();
    println!(
        "[status] {} | stage={} | infant_days={} | lang_nodes={} | docs={} | ws={} | bal={:.3} | guardian={}",
        r.name,
        r.stage.name(),
        k.infant_care_days,
        k.last_lang_nodes,
        k.last_docs_ingested,
        k.last_ws_len,
        k.last_lang_bal,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_infant_gate();
    println!("[infant gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_toddler {
        match k.try_graduate_infant() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  next: phageguard drills (still no VCF dump)",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-toddler when infant gate is green (earned, not waste)");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("cradle save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. Journal={}. Infant_days={}. Same child next wake.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.life.journal.len(),
        k.infant_care_days
    );
    println!("  cradle={}", cradle.display());
    println!("  language tissue awake. Trust holds. Rails hold.");
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}
