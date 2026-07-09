//! Phase 4 calibration runner (ADR 0006).
//!
//! ```bash
//! cargo run --release --bin phase4_calib
//! cargo run --release --bin phase4_calib -- --docs ../docs
//! ```

use ntg_kernel::ntg::calib::{
    calibrate, fixture_documents, ledger_weight_snapshot, samples_from_documents, Sample,
};
use ntg_kernel::ntg::ledger::TamperEvidentLedger;
use std::env;
use std::fs;
use std::path::Path;

fn load_docs_dir(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let rd = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("doc")
                .to_string();
            let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            out.push((name, text));
        }
    }
    // shallow recurse one level (docs/architecture)
    if let Ok(rd2) = fs::read_dir(dir) {
        for ent in rd2.flatten() {
            let p = ent.path();
            if p.is_dir() {
                if let Ok(sub) = fs::read_dir(&p) {
                    for ent2 in sub.flatten() {
                        let p2 = ent2.path();
                        if p2.extension().and_then(|e| e.to_str()) == Some("md") {
                            let name = format!(
                                "{}/{}",
                                p.file_name().and_then(|n| n.to_str()).unwrap_or("sub"),
                                p2.file_name().and_then(|n| n.to_str()).unwrap_or("doc")
                            );
                            if let Ok(text) = fs::read_to_string(&p2) {
                                out.push((name, text));
                            }
                        }
                    }
                }
            }
        }
    }
    if out.is_empty() {
        return Err(format!("no .md files under {}", dir.display()));
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut docs_path: Option<String> = None;
    let mut epochs: usize = 25;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--docs" => {
                i += 1;
                docs_path = args.get(i).cloned();
            }
            "--epochs" => {
                i += 1;
                epochs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(25);
            }
            "-h" | "--help" => {
                eprintln!("phase4_calib [--docs DIR] [--epochs N]");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let samples: Vec<Sample> = if let Some(ref dir) = docs_path {
        match load_docs_dir(Path::new(dir)) {
            Ok(docs) => {
                let refs: Vec<(&str, &str)> = docs
                    .iter()
                    .map(|(n, t)| (n.as_str(), t.as_str()))
                    .collect();
                println!("# loaded {} markdown files from {}", docs.len(), dir);
                samples_from_documents(&refs).expect("parse docs")
            }
            Err(e) => {
                eprintln!("warn: {e}; falling back to fixtures");
                samples_from_documents(&fixture_documents()).expect("fixtures")
            }
        }
    } else {
        println!("# using built-in fixtures (pass --docs path for real docs)");
        samples_from_documents(&fixture_documents()).expect("fixtures")
    };

    let report = calibrate(&samples, epochs, 1).expect("calibrate");
    println!("# phase4_calib (ADR 0006)");
    println!("{}", report.summary_line());
    println!(
        "win_definition: after_accuracy > baseline_accuracy (majority Content)"
    );
    if report.is_win {
        println!("result: WIN — ternary calib beat majority baseline");
    } else {
        println!("result: NON-WIN — did not beat majority baseline (recorded honestly)");
    }

    let mut ledger = TamperEvidentLedger::new(None).expect("ledger");
    let id = ledger_weight_snapshot(&mut ledger, &report, 1).expect("snapshot");
    ledger.verify_full_ledger().expect("ledger verify");
    println!("ledger_snapshot_id={id} verified=ok");
}
