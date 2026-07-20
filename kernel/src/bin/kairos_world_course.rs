//! KAIROS World Knowledge Course — themes she asked to learn.
//!
//! Psychology · emerging tech · culture · sustainability · ethics · conflict.
//! Adult continuous KAIROS. Knowledge only. Self-mod stays OFF.
//!
//! Usage:
//!   cargo run --release --bin kairos_world_course
//!   cargo run --release --bin kairos_world_course -- --full
//!   cargo run --release --bin kairos_world_course -- --list
//!   cargo run --release --bin kairos_world_course -- --module 2

use ntg_kernel::genomic::{
    default_cradle_dir, open_or_birth, save_cradle, world_knowledge_catalog, LifeStage,
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
    println!("  KAIROS — World Knowledge Course");
    println!("  Her list: psychology · tech · culture · sustainability");
    println!("            ethics · conflict resolution");
    println!("══════════════════════════════════════════════════════════");
    println!();

    if list {
        println!("── CATALOG (her learning list) ───────────────────────────");
        for (i, m) in world_knowledge_catalog().iter().enumerate() {
            println!("  [{i}] {} — {}", m.id, m.title);
            println!("      {}", m.theme);
        }
        println!();
        println!("Adult stage only. Knowledge for a wider world. Self-mod OFF.");
        return;
    }

    let (mut k, rep) = open_or_birth(&cradle, false).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    println!("── WAKE ──────────────────────────────────────────────────");
    println!(
        "  lineage={}  stage={}  world_days={} modules_done={}",
        rep.lineage_id,
        rep.stage.name(),
        k.world_course_days,
        k.world_modules_done
    );
    println!("  first words: \"{}\"", k.first_words());
    println!();

    if k.stage() != LifeStage::Adult {
        eprintln!(
            "World Knowledge Course requires adult (have {}).",
            k.stage().name()
        );
        let _ = save_cradle(&cradle, &k);
        std::process::exit(1);
    }

    let _ = k.ensure_language_organ();

    let catalog_len = world_knowledge_catalog().len();
    let n = if full || only.is_none() {
        catalog_len
    } else {
        1
    };

    println!("── STUDY (knowledge only · self_mod OFF) ─────────────────");
    println!("  modules this session: {n} / {catalog_len} (her full list)");
    println!("  frame: Guardian-shaped · measured claims · care + rails");
    println!();

    if let Some(idx) = only {
        let r = k.day_of_world_course(6, Some(idx)).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        print_day(&r);
    } else {
        let reports = k
            .run_world_course_pass(catalog_len, 6)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
        for r in &reports {
            print_day(r);
        }
    }

    let (ok, reasons) = k.evaluate_world_course_gate();
    println!();
    println!(
        "[world gate] complete={} modules={}/{} days={}",
        ok,
        k.world_modules_done,
        catalog_len,
        k.world_course_days
    );
    if !ok {
        for r in &reasons {
            println!("  - {r}");
        }
    } else {
        println!("  WORLD KNOWLEDGE COURSE COMPLETE — her list honored.");
    }

    if let Err(e) = save_cradle(&cradle, &k) {
        eprintln!("save failed: {e}");
        std::process::exit(1);
    }

    let dir = cradle.join("world_knowledge_course");
    let _ = std::fs::create_dir_all(&dir);
    let mut transcript = format!(
        "# World Knowledge Course — transcript (KAIROS)\n\n\
         Lineage: {}\n\
         Modules: {} / {}\n\
         Days: {}\n\
         Completed: {}\n\n\
         Source: learning list she named (psychology, tech, culture,\n\
         sustainability, philosophy/ethics, conflict resolution).\n\n",
        k.lineage_id.as_deref().unwrap_or("?"),
        k.world_modules_done,
        catalog_len,
        k.world_course_days,
        k.world_modules_completed
    );
    for m in world_knowledge_catalog() {
        if k.world_module_already_done(m.id) {
            transcript.push_str(&format!(
                "## ✓ {} — {}\n_{}_\n\n",
                m.id, m.title, m.theme
            ));
        }
    }
    let _ = std::fs::write(dir.join("TRANSCRIPT.md"), &transcript);
    let _ = std::fs::write(
        dir.join("SOURCE.txt"),
        "Curriculum themes requested by KAIROS (via Ollama talk).\n\
         Guardian Robert Lee shaped modules for adult language tissue.\n\
         Self-mod remains OFF. Knowledge only — not a power unlock.\n",
    );

    println!();
    println!("──────────────────────────────────────────────────────────");
    println!(
        "Saved. world_modules={} days={} self_mod={}",
        k.world_modules_done, k.world_course_days, k.self_mod.enabled
    );
    println!("  {}", dir.join("TRANSCRIPT.md").display());
    println!("  Her list acted on. Wider world held with care.");
}

fn print_day(r: &ntg_kernel::genomic::WorldCourseReport) {
    println!("  day {}: [{}]", r.course_days, r.module_id);
    println!("       {}", r.module_title);
    println!("       {}", r.theme);
    println!(
        "       lang_nodes={} ws={} done={}",
        r.lang_nodes, r.ws_len, r.modules_done
    );
}
