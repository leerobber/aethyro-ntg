//! Doctorate-style schooling runner for Aethyro NTG.
//!
//! ```bash
//! cargo run --release --bin ntg_school -- --docs ../docs --out ../docs/schooling/runs --runs 5
//! cargo run --release --bin ntg_school -- --docs ../docs --phase 1 --runs 3
//! ```
//!
//! Pass bar: **75%**. Below threshold ⇒ full study+exam redo (max attempts).

use ntg_kernel::ntg::schooling::{run_schooling_program, SchoolConfig, PASS_THRESHOLD};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut docs = PathBuf::from("../docs");
    let mut out = PathBuf::from("../docs/schooling/runs");
    let mut runs: u32 = 5;
    let mut max_attempts: u32 = 5;
    let mut only_phase: Option<u32> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--docs" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    docs = PathBuf::from(v);
                }
            }
            "--out" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    out = PathBuf::from(v);
                }
            }
            "--runs" => {
                i += 1;
                runs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5)
                    .max(1);
            }
            "--max-attempts" => {
                i += 1;
                max_attempts = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5)
                    .max(1);
            }
            "--phase" => {
                i += 1;
                only_phase = args.get(i).and_then(|s| s.parse().ok());
            }
            "-h" | "--help" => {
                eprintln!(
                    "ntg_school — doctorate schooling (pass bar {:.0}%)\n\
                     \n\
                     --docs DIR          real docs corpus (default ../docs)\n\
                     --out DIR           notebook/json output (default ../docs/schooling/runs)\n\
                     --runs N            independent campaign runs (default 5)\n\
                     --max-attempts N    full redos per phase on FAIL (default 5)\n\
                     --phase N           only phase 0..5 (default all)\n",
                    PASS_THRESHOLD * 100.0
                );
                return;
            }
            _ => {}
        }
        i += 1;
    }

    println!("# ntg_school doctorate program");
    println!(
        "docs={} out={} runs={} max_attempts={} phase={:?} pass_bar={:.0}%",
        docs.display(),
        out.display(),
        runs,
        max_attempts,
        only_phase,
        PASS_THRESHOLD * 100.0
    );

    let cfg = SchoolConfig {
        docs_dir: docs,
        out_dir: out,
        max_attempts,
        campaign_runs: runs,
        only_phase,
    };

    match run_schooling_program(&cfg) {
        Ok(summary) => {
            println!("{summary}");
            println!("# done — read MASTER_NOTEBOOK.md in out dir");
        }
        Err(e) => {
            eprintln!("schooling error: {e}");
            process::exit(1);
        }
    }
}
