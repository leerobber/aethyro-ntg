//! Phase 4 calibration runner (ADR 0006) + Phase 5 prep hooks.
//!
//! ```bash
//! cargo run --release --bin phase4_calib
//! cargo run --release --bin phase4_calib -- --docs ../docs
//! cargo run --release --bin phase4_calib -- --docs ../docs --self-mod --json
//! cargo run --release --bin phase4_calib -- --docs ../docs --write-model /tmp/ntg.calib
//! cargo run --release --bin phase4_calib -- --eval-model /tmp/ntg.calib --docs ../docs
//! cargo run --release --bin phase4_calib -- --eval-model /tmp/ntg.calib --predict 'fn main(){}'
//! cargo run --release --bin phase4_calib -- --docs ../docs --write-model /tmp/a.calib \
//!   --write-sparse /tmp/a.sparse --write-report /tmp/a.json
//! cargo run --release --bin phase4_calib -- --docs ../docs \
//!   --eval-model /tmp/a.calib --compare-model /tmp/b.calib
//! ```

use ntg_kernel::ntg::calib::{
    batch_predict_parallel, calibrate, compare_models, evaluate_model, fixture_documents,
    ledger_weight_snapshot, metrics_to_json, optional_self_mod_probe, report_to_json,
    samples_from_documents, write_report_json, CalibModel, Sample,
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

fn collect_samples(docs_path: &Option<String>, probe_graph: &mut Graph) -> Vec<Sample> {
    if let Some(ref dir) = docs_path {
        match load_docs_dir(Path::new(dir)) {
            Ok(docs) => {
                let refs: Vec<(&str, &str)> = docs
                    .iter()
                    .map(|(n, t)| (n.as_str(), t.as_str()))
                    .collect();
                println!("# loaded {} markdown files from {}", docs.len(), dir);
                for &(n, t) in &refs {
                    docparse::parse_into(probe_graph, n, t);
                }
                samples_from_documents(&refs).expect("parse docs")
            }
            Err(e) => {
                eprintln!("warn: {e}; falling back to fixtures");
                for (n, t) in fixture_documents() {
                    docparse::parse_into(probe_graph, n, t);
                }
                samples_from_documents(&fixture_documents()).expect("fixtures")
            }
        }
    } else {
        println!("# using built-in fixtures (pass --docs path for real docs)");
        for (n, t) in fixture_documents() {
            docparse::parse_into(probe_graph, n, t);
        }
        samples_from_documents(&fixture_documents()).expect("fixtures")
    }
}

fn print_help() {
    eprintln!(
        "phase4_calib [options]\n\
         \n\
         Data:\n\
         \t--docs DIR              load markdown under DIR (else fixtures)\n\
         \t--epochs N              training epochs (default 40)\n\
         \n\
         Output:\n\
         \t--json                  print metrics JSON line\n\
         \t--write-model PATH      save NTG_CALIB_V1 weights\n\
         \t--write-sparse PATH     save NTG_SPARSE_V1 COO dump\n\
         \t--write-report PATH     save one-line JSON report\n\
         \n\
         Inference / A/B:\n\
         \t--eval-model PATH       evaluate frozen model (no train)\n\
         \t--compare-model PATH    with --eval-model: A/B metrics delta\n\
         \t--predict LABEL         score one label (uses eval model or trains)\n\
         \t--sparse-score          use TOBL sparse path for --predict\n\
         \n\
         Safety:\n\
         \t--self-mod              optional topology probe (OFF default)\n\
         \t-h, --help              this help"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut docs_path: Option<String> = None;
    let mut epochs: usize = 40;
    let mut self_mod = false;
    let mut json = false;
    let mut write_model: Option<String> = None;
    let mut write_sparse: Option<String> = None;
    let mut write_report: Option<String> = None;
    let mut eval_model: Option<String> = None;
    let mut compare_model: Option<String> = None;
    let mut predict_label: Option<String> = None;
    let mut sparse_score = false;
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
            "--self-mod" => self_mod = true,
            "--json" => json = true,
            "--sparse-score" => sparse_score = true,
            "--write-model" => {
                i += 1;
                write_model = args.get(i).cloned();
            }
            "--write-sparse" => {
                i += 1;
                write_sparse = args.get(i).cloned();
            }
            "--write-report" => {
                i += 1;
                write_report = args.get(i).cloned();
            }
            "--eval-model" => {
                i += 1;
                eval_model = args.get(i).cloned();
            }
            "--compare-model" => {
                i += 1;
                compare_model = args.get(i).cloned();
            }
            "--predict" => {
                i += 1;
                predict_label = args.get(i).cloned();
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let mut probe_graph = Graph::new();
    let samples = collect_samples(&docs_path, &mut probe_graph);

    // Inference-only path: load model and score / compare
    if let Some(ref path) = eval_model {
        let model = CalibModel::load_path(Path::new(path)).expect("load model");
        if let Some(ref label) = predict_label {
            let sc = if sparse_score {
                model.score_label_sparse(label)
            } else {
                model.score_label(label)
            };
            let exec = sc >= model.threshold;
            println!(
                "predict label={label:?} score={sc} execution={exec} thr={} sparse={sparse_score}",
                model.threshold
            );
            return;
        }
        if let Some(ref path_b) = compare_model {
            let model_b = CalibModel::load_path(Path::new(path_b)).expect("load compare model");
            let cmp = compare_models(&model, &model_b, &samples);
            println!("# compare A={} B={}", path, path_b);
            println!(
                "A: n={} {}",
                samples.len(),
                format_metrics(&cmp.a_metrics)
            );
            println!(
                "B: n={} {}",
                samples.len(),
                format_metrics(&cmp.b_metrics)
            );
            println!(
                "delta_b_minus_a bal={:+.4} f1={:+.4}",
                cmp.bal_delta_b_minus_a, cmp.f1_delta_b_minus_a
            );
            if json {
                println!(
                    "json={{\"a\":{},\"b\":{},\"bal_delta\":{:.6},\"f1_delta\":{:.6}}}",
                    metrics_to_json(&cmp.a_metrics),
                    metrics_to_json(&cmp.b_metrics),
                    cmp.bal_delta_b_minus_a,
                    cmp.f1_delta_b_minus_a
                );
            }
            return;
        }
        let m = evaluate_model(&model, &samples);
        println!("# eval-model {}", path);
        println!("n={} {}", samples.len(), format_metrics(&m));
        if json {
            println!("json={}", metrics_to_json(&m));
        }
        // Phase 5 smoke: prove GraphNode warm-start works from frozen model
        let node = model.to_graph_node(0);
        println!(
            "warm_start GraphNode id=0 dens={:.4} nonzero={}",
            node.weights.density(),
            model.nonzero_count()
        );
        // Parallel batch path identity vs serial graph-node predict
        let labels: Vec<&str> = samples
            .iter()
            .take(64)
            .map(|s| s.label_preview.as_str())
            .collect();
        if !labels.is_empty() {
            let par = batch_predict_parallel(&model, &labels);
            let ser: Vec<bool> = labels
                .iter()
                .map(|l| model.predict_via_graph_node(l))
                .collect();
            let match_n = par.iter().zip(ser.iter()).filter(|(a, b)| a == b).count();
            println!(
                "batch_parallel identity={}/{} (graph-node path)",
                match_n,
                labels.len()
            );
        }
        return;
    }

    if let Some(ref label) = predict_label {
        // Train first then predict single label
        let report = calibrate(&samples, epochs, 1).expect("calibrate");
        let model = CalibModel::from_report(&report);
        let sc = if sparse_score {
            model.score_label_sparse(label)
        } else {
            model.score_label(label)
        };
        println!(
            "predict label={:?} score={} execution={} thr={} sparse={}",
            label,
            sc,
            sc >= model.threshold,
            model.threshold,
            sparse_score
        );
        persist_artifacts(&model, &report, &write_model, &write_sparse, &write_report);
        return;
    }

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
    if report.is_win {
        println!("result: WIN — balanced metrics beat majority baseline");
    } else {
        println!("result: NON-WIN — balanced metrics did not clear win bar");
    }
    if json {
        println!("json={}", report_to_json(&report));
    }

    let model = CalibModel::from_report(&report);
    persist_artifacts(&model, &report, &write_model, &write_sparse, &write_report);

    // Warm-start path for later GraphNode / TOBL work
    let sparse = model.to_sparse_weights();
    println!(
        "sparse_weights dens={:.4} blocks={} nonzero={}",
        sparse.density(),
        sparse.blocks.len(),
        model.nonzero_count()
    );
    let _rt = model.to_runtime_layer().expect("runtime warm-start");
    println!("runtime_layer nodes=1 (calib warm-start ok)");
    // Phase 5 production path: graph-node score equals dense score
    let probe_label = "fn main() { println!(\"hi\"); }";
    let d = model.score_label(probe_label);
    let g = model.score_via_graph_node(probe_label);
    println!(
        "path_identity dense={} graph_node={} match={}",
        d,
        g,
        d == g
    );

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

fn format_metrics(m: &ntg_kernel::ntg::calib::ClassMetrics) -> String {
    format!(
        "acc={:.3} bal={:.3} f1={:.3} rec={:.3} prec={:.3} tp={} tn={} fp={} fn={}",
        m.accuracy,
        m.balanced_accuracy,
        m.f1_exec,
        m.recall_exec,
        m.precision_exec,
        m.tp,
        m.tn,
        m.fp,
        m.fn_
    )
}

fn persist_artifacts(
    model: &CalibModel,
    report: &ntg_kernel::ntg::calib::CalibReport,
    write_model: &Option<String>,
    write_sparse: &Option<String>,
    write_report: &Option<String>,
) {
    if let Some(ref path) = write_model {
        model.save_path(Path::new(path)).expect("write model");
        println!("wrote_model={path}");
    }
    if let Some(ref path) = write_sparse {
        model.save_sparse_path(Path::new(path)).expect("write sparse");
        println!("wrote_sparse={path}");
    }
    if let Some(ref path) = write_report {
        write_report_json(Path::new(path), report).expect("write report");
        println!("wrote_report={path}");
    }
}
