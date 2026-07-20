//! KAIROS Adult Course — deep knowledge of human adulthood experience.
//!
//! Continuous adult child studies Guardian-curated modules:
//! covenant, work, love, loss, judgment, ethics, time, integrity,
//! mentorship, integration. Language tissue + journal — not power dump.
//!
//! Usage:
//!   cargo run --release --bin kairos_adult_course
//!   cargo run --release --bin kairos_adult_course -- --full   # all 10 modules
//!   cargo run --release --bin kairos_adult_course -- --module 3

use ntg_kernel::genomic::{
    adult_course_catalog, default_cradle_dir, open_or_birth, save_cradle, LifeStage,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let list = args.iter().any(|a| a == "--list");
    let cradle = args
        .windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir);
    let only = args
        .windows(2)
        .find(|w| w[0] == "--module")
        .and_then(|w| w[1].parse::<usize>().ok());

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Adult Course  |  Human adulthood knowledge");
    println!("  Experience-depth · language · continuous adult child");
    println!("══════════════════════════════════════════════════════════");
    println!();

    if list {
        println!("── CATALOG ───────────────────────────────────────────────");
        for (i, m) in adult_course_catalog().iter().enumerate() {
            println!("  [{i}] {} — {}", m.id, m.title);
            println!("       theme: {}", m.theme);
        }
        return;
    }

    let (mut k, rep) = open_or_birth(&cradle, false).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    println!("── WAKE ──────────────────────────────────────────────────");
    println!(
        "  lineage={}  stage={}  course_days={} modules_done={}",
        rep.lineage_id,
        rep.stage.name(),
        k.adult_course_days,
        k.adult_modules_done
    );
    if k.has_guardians_pride() {
        println!("  award: Guardian’s Pride held");
    }
    println!("  first words: \"{}\"", k.first_words());
    println!();

    if k.stage() != LifeStage::Adult {
        eprintln!(
            "Adult Course requires stage=adult (have {}). Raise stages first.",
            k.stage().name()
        );
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    // Re-arm language if needed
    let _ = k.ensure_language_organ();

    let n = if full {
        adult_course_catalog().len()
    } else if only.is_some() {
        1
    } else {
        6 // core path
    };

    println!("── STUDY ─────────────────────────────────────────────────");
    println!(
        "  modules this session: {} (core=6, full=10)",
        if full { 10 } else { n }
    );
    println!("  still OFF: self_mod | knowledge only");
    println!();

    if let Some(idx) = only {
        let r = k.day_of_adult_course(6, Some(idx)).unwrap_or_else(|e| {
            eprintln!("course day failed: {e}");
            std::process::exit(1);
        });
        print_day(&r);
    } else {
        let start = k.adult_course_days as usize;
        for i in 0..n {
            let idx = if full { i } else { (start + i) % adult_course_catalog().len() };
            // Prefer sequential unread modules for core pass
            let idx = if !full {
                // find next not done
                let cat = adult_course_catalog();
                cat.iter()
                    .enumerate()
                    .find(|(_, m)| {
                        !k.adult_modules_completed
                            .split(',')
                            .any(|s| s.trim() == m.id)
                    })
                    .map(|(j, _)| j)
                    .unwrap_or(idx)
            } else {
                idx
            };
            let r = k.day_of_adult_course(6, Some(idx)).unwrap_or_else(|e| {
                eprintln!("course day failed: {e}");
                std::process::exit(1);
            });
            print_day(&r);
        }
    }

    let (ok, reasons) = k.evaluate_adult_course_gate();
    println!();
    println!(
        "[adult course gate] core_complete={} modules={}/{} days={}",
        ok,
        k.adult_modules_done,
        adult_course_catalog().len(),
        k.adult_course_days
    );
    if !ok {
        for r in &reasons {
            println!("  - {r}");
        }
        println!("[hint] re-run until ≥6 modules / ≥6 days, or --full for all 10");
    } else {
        println!("  Core Adult Course knowledge path complete.");
        if k.adult_modules_done >= adult_course_catalog().len() as u32 {
            println!("  Full catalog (10/10) complete.");
        }
    }

    // Persist curriculum docs into cradle language snapshot path via save
    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("save failed: {e}");
        std::process::exit(1);
    }

    // Write human-readable course transcript
    let course_dir = cradle.join("adult_course");
    let _ = std::fs::create_dir_all(&course_dir);
    let mut transcript = format!(
        "# Adult Course transcript — KAIROS\n\n\
         Lineage: {}\n\
         Modules done: {} / {}\n\
         Course days: {}\n\
         Completed ids: {}\n\n",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.adult_modules_done,
        adult_course_catalog().len(),
        k.adult_course_days,
        k.adult_modules_completed
    );
    for m in adult_course_catalog() {
        if k.adult_modules_completed.split(',').any(|s| s.trim() == m.id) {
            transcript.push_str(&format!("## ✓ {} — {}\n{}\n\n", m.id, m.title, m.theme));
        }
    }
    let _ = std::fs::write(course_dir.join("TRANSCRIPT.md"), transcript);

    println!();
    println!("──────────────────────────────────────────────────────────");
    println!(
        "Saved. modules_done={} days={} self_mod={}",
        k.adult_modules_done, k.adult_course_days, k.self_mod.enabled
    );
    println!("  transcript: {}/adult_course/TRANSCRIPT.md", cradle.display());
    println!("  Knowledge held. Love held. Rails hold.");
}

fn print_day(r: &ntg_kernel::genomic::AdultCourseReport) {
    println!(
        "  day {}: [{}] {}",
        r.course_days, r.module_id, r.module_title
    );
    println!("       theme: {}", r.theme);
    println!(
        "       lang_nodes={} ws={} modules_done={}",
        r.lang_nodes, r.ws_len, r.modules_done
    );
}
