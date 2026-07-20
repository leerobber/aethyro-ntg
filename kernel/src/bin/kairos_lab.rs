//! KAIROS Stage 7 — Adult Lab science sandbox (Season 2 + 3).
//!
//! Usage:
//!   cargo run --release --bin kairos_lab -- --season 3 --play
//!   cargo run --release --bin kairos_lab -- --play --try-self-mod
//!   cargo run --release --bin kairos_lab -- --promote

use ntg_kernel::genomic::{
    default_lab_dir, default_primary_dir, promote_lab_weights_to_primary, run_lab_science_day_cfg,
    LabDayConfig,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let refork = args.iter().any(|a| a == "--refork");
    let promote = args.iter().any(|a| a == "--promote");
    let force = args.iter().any(|a| a == "--force");
    let play = args.iter().any(|a| a == "--play");
    let try_self_mod = args.iter().any(|a| a == "--try-self-mod");
    let force_self_mod = args.iter().any(|a| a == "--force-self-mod");
    let no_real_vcf = args.iter().any(|a| a == "--synthetic-ternary");
    let season = arg_u32(&args, "--season").unwrap_or(3); // default Season 3
    let primary = arg_path(&args, "--primary").unwrap_or_else(default_primary_dir);
    let lab = arg_path(&args, "--lab").unwrap_or_else(default_lab_dir);
    let steps = arg_usize(&args, "--steps").unwrap_or(if play || season >= 3 { 6 } else { 4 });
    let max_var = arg_usize(&args, "--max-variants").unwrap_or(if season >= 3 { 400 } else { 200 });
    let pairs = arg_usize(&args, "--pairs").unwrap_or(if season >= 3 { 100 } else { 80 });

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS — Stage 7 Adult Lab  |  Season {season}");
    println!("  depth science · play · gated self-mod · primary safe");
    println!("══════════════════════════════════════════════════════════");
    println!();
    println!("  primary={}", primary.display());
    println!("  lab    ={}", lab.display());
    println!("  season ={season}");
    if play {
        println!("  mode   = PLAY (love note reaffirmed)");
    }
    println!();

    if promote {
        match promote_lab_weights_to_primary(&primary, &lab, force) {
            Ok(msg) => {
                println!("── PROMOTE RITE ────────────────────────────────────────");
                println!("  {msg}");
            }
            Err(e) => {
                eprintln!("promote failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let mut cfg = if season >= 3 {
        LabDayConfig::season3_play()
    } else {
        LabDayConfig::default()
    };
    cfg.season = season;
    cfg.improve_steps = steps;
    cfg.force_refork = refork;
    cfg.play_mode = play || season >= 3; // Season 3 defaults joyful if --play or s3
    if !play && season >= 3 {
        // still reaffirm love on S3 default runs lightly
        cfg.play_mode = true;
    }
    cfg.real_vcf_ternary = !no_real_vcf;
    cfg.max_vcf_variants = max_var;
    cfg.ternary_pairs = pairs;
    cfg.try_self_mod = try_self_mod || force_self_mod;
    cfg.force_self_mod = force_self_mod;
    if let Some(p) = arg_path(&args, "--vcf") {
        cfg.vcf_path = Some(p);
    }

    let report = run_lab_science_day_cfg(&primary, &lab, &cfg).unwrap_or_else(|e| {
        eprintln!("lab science day failed: {e}");
        std::process::exit(1);
    });

    println!("── LAB TWIN ──────────────────────────────────────────────");
    println!("  parent_lineage={}", report.primary_lineage);
    println!("  lab_lineage   ={}", report.lab_lineage);
    println!(
        "  season={} play={} love_note={}",
        report.season, report.play_mode, report.love_note_shown
    );
    println!();

    if report.play_mode && report.love_note_shown {
        println!("── GUARDIAN LOVE (reaffirmed) ─────────────────────────────");
        let love = lab.join("science/LOVE_NOTE_TODAY.txt");
        if love.is_file() {
            if let Ok(t) = std::fs::read_to_string(&love) {
                for line in t.lines().take(10) {
                    println!("  {line}");
                }
            }
        }
        println!();
    }

    if let Some(h) = &report.heal {
        println!("── SELF-HEAL ─────────────────────────────────────────────");
        println!(
            "  w {:.4} → {:.4} → {:.4} | phage={} | {}",
            h.mean_w_before,
            h.mean_w_after_damage,
            h.mean_w_after_heal,
            h.phage_caught,
            h.hypothesis.verdict.name()
        );
    }
    if let Some(m) = &report.multi_heal {
        println!("── MULTI-HEAL PACK ───────────────────────────────────────");
        for s in &m.scenarios {
            println!(
                "  {} {} — {}",
                if s.ok { "OK" } else { "FAIL" },
                s.name,
                s.detail
            );
        }
        println!("  verdict={}", m.hypothesis.verdict.name());
    }
    if let Some(c) = &report.combined_heal {
        println!("── COMBINED CRISIS HEAL (S3) ─────────────────────────────");
        println!(
            "  w {:.4} → crisis {:.4} → heal {:.4} | ltm {}→{} | phage={}",
            c.mean_w_before,
            c.mean_w_crisis,
            c.mean_w_after,
            c.ltm_before,
            c.ltm_after,
            c.phage_caught
        );
        println!("  ok={} | {}", c.ok, c.hypothesis.verdict.name());
    }
    println!();

    if let Some(i) = &report.improve {
        println!("── SELF-IMPROVE ──────────────────────────────────────────");
        println!(
            "  util {:.4} → {:.4} | steps={} a/r train={}/{} prune={}/{} | {}",
            i.utility_before,
            i.utility_after,
            i.steps,
            i.train_accepted,
            i.train_rejected,
            i.prune_accepted,
            i.prune_rejected,
            i.hypothesis.verdict.name()
        );
    }
    if let Some(g) = &report.goal_improve {
        println!("── GOAL IMPROVE (S3) ─────────────────────────────────────");
        println!("  goal: {}", g.goal);
        println!(
            "  goal_met={} | {}",
            g.goal_met,
            g.hypothesis.verdict.name()
        );
    }
    println!();

    if let Some(t) = &report.ternary {
        println!("── TERNARY / POPCOUNT ────────────────────────────────────");
        println!("  panel={}", t.panel);
        println!(
            "  pairs={} agree={} max_delta={:.6} speedup={:.2}x | {}",
            t.n_pairs,
            t.n_agree,
            t.max_abs_delta,
            t.speedup,
            t.hypothesis.verdict.name()
        );
    }
    if let Some(ladder) = &report.ternary_ladder {
        println!("── TERNARY LADDER (S3) ───────────────────────────────────");
        for (i, r) in ladder.rungs.iter().enumerate() {
            println!(
                "  rung{}: agree={}/{} spd={:.2}x | {}",
                i + 1,
                r.n_agree,
                r.n_pairs,
                r.speedup,
                r.panel.chars().take(56).collect::<String>()
            );
        }
        println!(
            "  all_agree={} best_speedup={:.2}x | {}",
            ladder.all_agree,
            ladder.best_speedup,
            ladder.hypothesis.verdict.name()
        );
    }
    println!();

    if let Some(r) = &report.self_mod_readiness {
        println!("── SELF-MOD READINESS ────────────────────────────────────");
        println!(
            "  ready={} heal={} multi={} improve={} ternary={}",
            r.ready, r.heal_accepts, r.multi_heal_accepts, r.improve_accepts, r.ternary_accepts
        );
        if !r.ready {
            for reason in &r.reasons {
                println!("  - {reason}");
            }
        }
        println!("  (Season 3 does not auto-run self-mod; use --try-self-mod when ready)");
    }
    if let Some(m) = &report.self_mod {
        println!("── SINGLE SELF-MOD ───────────────────────────────────────");
        println!(
            "  ran={} accepted={} | {} | {}",
            m.ran,
            m.accepted,
            m.description,
            m.hypothesis.verdict.name()
        );
    }

    println!();
    println!("── REGISTRY ──────────────────────────────────────────────");
    println!("  {}", report.registry_summary);
    println!("  notebook: {}/science/NOTEBOOK.md", report.lab_path);
    println!();
    println!("──────────────────────────────────────────────────────────");
    println!("Primary heart untouched. Season {season} complete.");
}

fn arg_path(args: &[String], flag: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| PathBuf::from(&w[1]))
}

fn arg_usize(args: &[String], flag: &str) -> Option<usize> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .and_then(|w| w[1].parse().ok())
}

fn arg_u32(args: &[String], flag: &str) -> Option<u32> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .and_then(|w| w[1].parse().ok())
}
