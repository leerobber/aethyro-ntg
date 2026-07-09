//! Phase 4 calibration runner (ADR 0006) — class-balanced + optional self-mod.
//!
//! ```bash
//! cargo run --release --bin phase4_calib
//! cargo run --release --bin phase4_calib -- --docs ../docs
//! cargo run --release --bin phase4_calib -- --docs ../docs --self-mod
//! ```

use ntg_kernel::ntg::calib::{
    calibrate, fixture_documents, ledger_weight_snapshot, optional_self_mod_probe,
    samples_from_documents, Sample,
};
use ntg_kernel::ntg::docparse;
use ntg_kernel::ntg::graph::Graph;
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
    let mut epochs: usize = 40;
    let mut self_mod = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--docs" => {
                i += 1;
                docs_path = args.get(i).cloned();
            }
            "--epochs" => {
                i += 1;
                epochs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(40);
            }
            "--self-mod" => {
                self_mod = true;
            }
            "-h" | "--help" => {
                eprintln!("phase4_calib [--docs DIR] [--epochs N] [--self-mod]");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let mut probe_graph = Graph::new();
    let samples: Vec<Sample> = if let Some(ref dir) = docs_path {
        match load_docs_dir(Path::new(dir)) {
            Ok(docs) => {
                let refs: Vec<(&str, &str)> = docs
                    .iter()
                    .map(|(n, t)| (n.as_str(), t.as_str()))
                    .collect();
                println!("# loaded {} markdown files from {}", docs.len(), dir);
                for &(n, t) in &refs {
                    docparse::parse_into(&mut probe_graph, n, t);
                }
                samples_from_documents(&refs).expect("parse docs")
            }
            Err(e) => {
                eprintln!("warn: {e}; falling back to fixtures");
                for (n, t) in fixture_documents() {
                    docparse::parse_into(&mut probe_graph, n, t);
                }
                samples_from_documents(&fixture_documents()).expect("fixtures")
            }
        }
    } else {
        println!("# using built-in fixtures (pass --docs path for real docs)");
        for (n, t) in fixture_documents() {
            docparse::parse_into(&mut probe_graph, n, t);
        }
        samples_from_documents(&fixture_documents()).expect("fixtures")
    };

    let report = calibrate(&samples, epochs, 1).expect("calibrate");
    println!("# phase4_calib (ADR 0006) — class-balanced + hold-out");
    println!("{}", report.summary_line());
    println!(
        "confusion_test: tp={} tn={} fp={} fn={}",
        report.test_metrics.tp,
        report.test_metrics.tn,
        report.test_metrics.fp,
        report.test_metrics.fn_
    );
    println!(
        "win_definition: (bal>base+0.05 && rec>=0.25) OR (f1>=0.25 && bal>=0.55) OR (rec>=0.5 && prec>=0.15)"
    );
    if report.is_win {
        println!("result: WIN — balanced metrics beat majority baseline");
    } else {
        println!("result: NON-WIN — balanced metrics did not clear win bar");
    }

    let mut ledger = TamperEvidentLedger::new(None).expect("ledger");
    let id = ledger_weight_snapshot(&mut ledger, &report, 1).expect("snapshot");
    println!("ledger_snapshot_id={id}");

    let probe = optional_self_mod_probe(&probe_graph, self_mod, &mut ledger, 2).expect("self-mod");
    println!(
        "self_mod: enabled={} proposed={} accepted={} detail={}",
        probe.enabled, probe.proposed, probe.accepted, probe.detail
    );
    if let Some(mid) = probe.ledger_mutation_id {
        println!("self_mod_ledger_id={mid}");
    }

    ledger.verify_full_ledger().expect("ledger verify");
    println!("ledger verified=ok");
}
