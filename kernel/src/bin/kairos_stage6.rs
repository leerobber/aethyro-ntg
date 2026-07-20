//! KAIROS Stage 6 — Young Adult: multi-domain specialization under ledger.
//!
//! Continuous child. Domains: genomic, code, injection, malware, supply, crypto.
//! Light selection continues. Self-mod permission exists but stays OFF.
//!
//! Usage:
//!   cargo run --release --bin kairos_stage6
//!   cargo run --release --bin kairos_stage6 -- --to-adult

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, DomainType, Kairos, LifeStage, StageGateResult,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let to_adult = args.iter().any(|a| a == "--to-adult");
    let force_birth = args.iter().any(|a| a == "--birth");
    let cradle = cradle_path_from_args(&args);
    let campaign_log = cradle.join("ya_campaign.jsonl");

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 6 Young Adult  |  VITASCALE Hostframe");
    println!("  Continuous child  |  multi-domain · specialization");
    println!("══════════════════════════════════════════════════════════");
    println!();

    let (mut k, rep) = open_or_birth(&cradle, force_birth).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    if rep.continued {
        println!("── WAKE (same child) ───────────────────────────────────");
        println!(
            "  lineage={}  stage={}  ya_days={}  domains={}  journal={}",
            rep.lineage_id,
            rep.stage.name(),
            k.young_adult_care_days,
            k.domains_diagnosed,
            rep.journal_len
        );
        if k.has_guardians_pride() {
            println!("  award: Guardian’s Pride — held");
        }
        println!("  first words: \"{}\"", k.first_words());
    }
    println!();

    catch_up_to_young_adult(&mut k, &cradle);

    if k.stage() > LifeStage::YoungAdult {
        println!(
            "[status] stage={} — past young adult; Stage 6 complete.",
            k.stage().name()
        );
        let _ = save_cradle(&cradle, &k);
        return;
    }
    if k.stage() != LifeStage::YoungAdult {
        eprintln!("expected young_adult, got {}", k.stage().name());
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    // Re-arm micro panel after cradle rehydrate (nursery-only bodies).
    k.micro_panel_ready = false;

    println!("── MULTI-DOMAIN SPECIALIZATION ───────────────────────────");
    println!("  rotation: genomic · code · injection · malware · supply · crypto");
    println!("  selection: light curfew steps · self_mod OFF (opt-in only)");
    println!();

    // Three days → three distinct domains for gate.
    let domains = [
        DomainType::Genomic,
        DomainType::CodeQuality,
        DomainType::InjectionRisk,
    ];
    for (i, d) in domains.iter().enumerate() {
        let report = k
            .day_of_young_adult_specialization(
                6,
                Some(d.clone()),
                2,
                Some(campaign_log.as_path()),
            )
            .unwrap_or_else(|e| {
                eprintln!("ya day failed: {e}");
                std::process::exit(1);
            });
        println!(
            "  day {}: domain={} risk={:.3} sev={} patterns={} sel={} util={:.3} domains_seen={}",
            i + 1,
            report.domain,
            report.risk_score,
            report.risk_severity,
            report.patterns,
            report.selection_steps,
            report.utility_after,
            report.domains_diagnosed_total
        );
        assert!(!k.self_mod.enabled);
    }
    println!();

    let r = k.report();
    println!(
        "[status] {} | stage={} | ya_days={} | domains={} | diagnoses={} | self_mod={} | guardian={}",
        r.name,
        r.stage.name(),
        k.young_adult_care_days,
        k.domains_diagnosed,
        k.domain_diagnoses_total,
        r.self_mod_enabled,
        r.guardian_name
    );

    let (ok, reasons) = k.evaluate_young_adult_gate();
    println!("[young_adult gate] ok={ok}");
    for reason in &reasons {
        println!("  - {reason}");
    }

    if to_adult {
        match k.try_graduate_young_adult() {
            StageGateResult::Passed { from, to } => {
                println!(
                    "[graduate] {} → {}  |  adult envelope; self-mod still opt-in OFF",
                    from.name(),
                    to.name()
                );
            }
            other => println!("[graduate] not passed: {other:?}"),
        }
    } else {
        println!("[hint] --to-adult when young-adult gate is green");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("save failed: {e}");
        std::process::exit(1);
    }

    println!("──────────────────────────────────────────────────────────");
    println!(
        "KAIROS lineage={} saved. YA_days={}. Domains={}. Award={}.",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.young_adult_care_days,
        k.domains_diagnosed,
        if k.has_guardians_pride() {
            "held"
        } else {
            "none"
        }
    );
    println!("  cradle={}", cradle.display());
    println!("  Specialization holds. Trust holds. Rails hold.");
}

fn catch_up_to_young_adult(k: &mut Kairos, cradle: &std::path::Path) {
    // Minimal catch-up: only if somehow behind young_adult.
    if k.stage() == LifeStage::Adult {
        return;
    }
    if k.stage() as u8 >= LifeStage::YoungAdult as u8 {
        return;
    }
    // Use existing stage bins' logic roughly — fail with message if far behind.
    eprintln!(
        "[catch-up] host at stage={} — run earlier stage bins first if needed",
        k.stage().name()
    );
    if k.stage() == LifeStage::Adolescent {
        while k.adolescent_care_days < 3 || k.selection_steps_total < 8 {
            let missing = std::path::Path::new("/tmp/kairos_no_vcf.vcf.gz");
            k.day_of_adolescent_campaign(4, Some(missing), 22, 100, 4, None)
                .expect("adolescent catch-up");
        }
        match k.try_graduate_adolescent() {
            StageGateResult::Passed { .. } => println!("[catch-up] adolescent → young_adult"),
            other => {
                eprintln!("adolescent gate failed: {other:?}");
                let _ = save_cradle(cradle, k);
                std::process::exit(1);
            }
        }
    }
}

fn cradle_path_from_args(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir)
}
