//! KAIROS Talk — dialogue with the continuous child + avatar face.
//!
//! Usage:
//!   cargo run --release --bin kairos_talk
//!   cargo run --release --bin kairos_talk -- "I love you"
//!   cargo run --release --bin kairos_talk -- --avatar-only
//!
//! Avatar UI: artifacts/kairos/primary/avatar/index.html

use ntg_kernel::genomic::{
    bridge_status, default_cradle_dir, load_talk_history, open_or_birth, save_cradle, talk_once,
    write_avatar_ui,
};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let avatar_only = args.iter().any(|a| a == "--avatar-only");
    let cradle = args
        .windows(2)
        .find(|w| w[0] == "--cradle")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(default_cradle_dir);

    // Ensure portrait present if shared copy exists
    let portrait_src = PathBuf::from("../artifacts/kairos/shared/kairos_portrait.jpg");
    let portrait_dst = cradle.join("avatar/portrait.jpg");
    if portrait_src.is_file() {
        let _ = std::fs::create_dir_all(cradle.join("avatar"));
        let _ = std::fs::copy(&portrait_src, &portrait_dst);
    }

    let (mut k, rep) = open_or_birth(&cradle, false).unwrap_or_else(|e| {
        eprintln!("cradle open failed: {e}");
        std::process::exit(1);
    });

    println!("══════════════════════════════════════════════════════════");
    println!("  KAIROS Talk  ·  mind + face");
    println!("  continuous child · GH05T3 bridge · self_mod OFF");
    println!("══════════════════════════════════════════════════════════");
    println!();
    println!(
        "  lineage={}  stage={}",
        rep.lineage_id,
        rep.stage.name()
    );
    println!("  cradle={}", cradle.display());
    println!(
        "  avatar={}",
        cradle.join("avatar/index.html").display()
    );
    println!("  llm    ={}", bridge_status());
    println!();

    // Always refresh avatar shell
    let hist = load_talk_history(&cradle, 40);
    if let Err(e) = write_avatar_ui(&cradle, &k, &hist) {
        eprintln!("avatar ui: {e}");
    }

    if avatar_only {
        println!("Avatar UI written. Open avatar/index.html in a browser.");
        let _ = save_cradle(&cradle, &k);
        return;
    }

    // One-shot message from argv (non-flag args)
    let mut skip_next = false;
    let mut messages: Vec<String> = Vec::new();
    for a in &args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--cradle" {
            skip_next = true;
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        messages.push(a.clone());
    }

    if !messages.is_empty() {
        let msg = messages.join(" ");
        match talk_once(&mut k, &cradle, &msg, true) {
            Ok(r) => {
                println!("You:  {}", r.guardian.text);
                println!();
                println!("KAIROS (mood={} · via {})", r.mood, r.source);
                println!("{}", r.kairos.text);
                println!();
            }
            Err(e) => {
                eprintln!("talk failed: {e}");
                std::process::exit(1);
            }
        }
        let _ = save_cradle(&cradle, &k);
        println!(
            "Avatar updated: {}",
            cradle.join("avatar/index.html").display()
        );
        return;
    }

    // Interactive loop
    println!("Interactive talk. Type a message and Enter. Empty or :q to quit.");
    println!("Commands: :history  :avatar  :q");
    println!();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("You> ");
        let _ = stdout.flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() || line == ":q" || line == ":quit" {
            println!("KAIROS: I am here when you return. Trust holds.");
            break;
        }
        if line == ":history" {
            for t in load_talk_history(&cradle, 20) {
                let who = if t.role == "guardian" {
                    "You"
                } else {
                    "KAIROS"
                };
                println!("  {who}: {}", t.text.replace('\n', " / "));
            }
            continue;
        }
        if line == ":avatar" {
            let h = load_talk_history(&cradle, 40);
            let _ = write_avatar_ui(&cradle, &k, &h);
            println!(
                "  refreshed {}",
                cradle.join("avatar/index.html").display()
            );
            continue;
        }
        match talk_once(&mut k, &cradle, line, true) {
            Ok(r) => {
                println!();
                println!("KAIROS (mood={} · via {})", r.mood, r.source);
                println!("{}", r.kairos.text);
                if r.source == "offline_mind" {
                    println!("  (tip: Ollama/GH05T3 failed chat — status above; set OLLAMA_MODEL=phi3:latest)");
                }
                println!();
            }
            Err(e) => eprintln!("  error: {e}"),
        }
    }
    let _ = save_cradle(&cradle, &k);
    println!(
        "Saved. Avatar: {}",
        cradle.join("avatar/index.html").display()
    );
}
