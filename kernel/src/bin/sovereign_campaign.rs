//! Multi-chromosome real-VCF campaign with frozen 1000G panels.
//!
//! Runs Rung 1–3 end-to-end on real data:
//!   1. Ingest one or more chromosomes from VCF.gz
//!   2. Freeze real Phase D reference panels for biology axis
//!   3. Attach language organ + Phase 4 calib (fixtures / docs)
//!   4. Multi-axis train/prune selection under ledger
//!
//! Usage (from kernel/):
//!   cargo run --release --bin sovereign_campaign -- \
//!     --vcf-dir ../data/raw/1000g \
//!     --chrs 22,1 \
//!     --max-variants 1200 \
//!     --steps 6
//!
//! Defaults use available 1000G phase3 files if present.

use ntg_kernel::genomic::{
    fixture_docs, LanguageOrgan, SovereignBrain, SovereignFitnessContext,
};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn default_vcf_name(chr: u8) -> String {
    format!(
        "ALL.chr{chr}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz"
    )
}

fn parse_chrs(s: &str) -> Vec<u8> {
    s.split(',')
        .filter_map(|p| p.trim().parse::<u8>().ok())
        .filter(|c| *c >= 1 && *c <= 22)
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut vcf_dir = PathBuf::from("../data/raw/1000g");
    let mut chrs = vec![22u8, 1u8];
    let mut max_variants: usize = 1200;
    let mut steps: usize = 6;
    let mut synthetic_n: usize = 80;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--vcf-dir" => {
                i += 1;
                vcf_dir = PathBuf::from(&args[i]);
            }
            "--chrs" => {
                i += 1;
                chrs = parse_chrs(&args[i]);
            }
            "--max-variants" => {
                i += 1;
                max_variants = args[i].parse().expect("max-variants");
            }
            "--steps" => {
                i += 1;
                steps = args[i].parse().expect("steps");
            }
            "--synthetic-n" => {
                i += 1;
                synthetic_n = args[i].parse().expect("synthetic-n");
            }
            "--help" | "-h" => {
                eprintln!(
                    "usage: sovereign_campaign [--vcf-dir DIR] [--chrs 22,1,2] [--max-variants N] [--steps K]"
                );
                return;
            }
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    if chrs.is_empty() {
        eprintln!("no chromosomes specified");
        std::process::exit(2);
    }

    println!("=== Sovereign multi-chr campaign ===");
    println!(
        "vcf_dir={} chrs={:?} max_variants={max_variants} steps={steps}",
        vcf_dir.display(),
        chrs
    );

    let mut brain = SovereignBrain::new(96);
    let mut ctx = SovereignFitnessContext::new().expect("ledger");
    let t0 = Instant::now();

    for &chr in &chrs {
        let path = vcf_dir.join(default_vcf_name(chr));
        if !path.is_file() {
            eprintln!("[skip] missing {}", path.display());
            continue;
        }
        println!("[*] ingest chr{chr} from {}", path.display());
        let t = Instant::now();
        match brain.ingest_vcf(
            path.to_str().unwrap(),
            chr,
            Some(max_variants),
            synthetic_n,
            42 + chr as u64,
            false,
        ) {
            Ok(data) => {
                ctx.register_real_chromosome(&data);
                println!(
                    "    snps={} ld_pairs={} samples={} ({:.1}s)",
                    data.brain.neurons.len(),
                    data.ld_pairs.len(),
                    data.n_real_samples,
                    t.elapsed().as_secs_f64()
                );
            }
            Err(e) => {
                eprintln!("    FAILED chr{chr}: {e}");
                std::process::exit(1);
            }
        }
    }

    if brain.n_chromosomes() == 0 {
        eprintln!("no chromosomes ingested — check --vcf-dir and files");
        std::process::exit(1);
    }

    // Rung 3 language organ + calib task axis
    let mut organ = LanguageOrgan::new();
    organ.ingest_documents(&fixture_docs());
    // Optional: also pull a couple of real repo docs if present.
    for rel in [
        "../docs/ROADMAP.md",
        "../docs/STATUS.md",
        "../docs/DESIGN.md",
    ] {
        let p = Path::new(rel);
        if let Ok(text) = std::fs::read_to_string(p) {
            organ.ingest_document(rel, &text);
        }
    }
    match organ.train_calib_fixtures(30) {
        Ok(rep) => println!(
            "[*] language calib fixtures: n={} test_bal={:.3} win={}",
            rep.n_samples, rep.test_metrics.balanced_accuracy, rep.is_win
        ),
        Err(e) => eprintln!("[warn] calib fixtures: {e}"),
    }
    if let Err(e) = ctx.install_calib_from_language(&organ) {
        eprintln!("[warn] install calib: {e}");
        let _ = ctx.install_calib_from_fixtures(25);
    }
    brain.attach_language(organ);

    // Under-train weights so train operator has headroom.
    for b in brain.chromosomes.values_mut() {
        for s in &mut b.synapses {
            s.weight = (s.ld_r2 * 0.4).clamp(0.0, 1.0);
            s.plasticity = 0.08;
        }
    }
    brain.refresh_structure();
    let _ = brain.consolidate(0.5, 0.0);

    let ws = brain.activate_from_text("haplotype LD recombination on chr22 and ternary kernel");
    println!(
        "[*] activate_from_text: genomic_ws={} lang_nodes={} motifs={}",
        ws.neurons.len(),
        ws.language_nodes.len(),
        ws.motif_ids.len()
    );

    let s0 = brain.measure_structure();
    let f0 = ctx.score(&brain);
    println!(
        "[axes0] chrs={} neurons={} synapses={} u={:.4} task={:.3} (calib={:.3} genomic={:.3}) bio={:.3} cost={:.3} safety={:.1} cov={:.3}",
        s0.n_chromosomes,
        s0.n_neurons,
        s0.n_synapses,
        f0.utility(),
        f0.task_accuracy,
        ctx.last_calib_task,
        ctx.last_genomic_task,
        f0.biological_consistency,
        f0.structural_cost,
        f0.safety,
        ctx.last_ld_coverage
    );

    let mut train_acc = 0usize;
    let mut prune_acc = 0usize;
    let mut prune_rej = 0usize;
    for step in 0..steps {
        let (label, out) = if step % 2 == 0 {
            ("train", ctx.select_train_step(&brain, 6).expect("train"))
        } else {
            ("prune", ctx.select_prune_step(&brain, 0.12).expect("prune"))
        };
        println!(
            "  step {step} [{label}]: u={:.4}->{:.4} task={:.3}->{:.3} bio={:.3}->{:.3} cost={:.3}->{:.3} accepted={} ledger={}",
            out.baseline.utility(),
            out.candidate.utility(),
            out.baseline.task_accuracy,
            out.candidate.task_accuracy,
            out.baseline.biological_consistency,
            out.candidate.biological_consistency,
            out.baseline.structural_cost,
            out.candidate.structural_cost,
            out.accepted,
            ctx.ledger_entry_count()
        );
        if out.accepted {
            if let Some(child) = out.child {
                brain = child;
            }
            if label == "train" {
                train_acc += 1;
            } else {
                prune_acc += 1;
            }
        } else if label == "prune" {
            prune_rej += 1;
        }
    }

    let s1 = brain.measure_structure();
    let f1 = ctx.score(&brain);
    println!("=== campaign summary ===");
    println!(
        "elapsed={:.1}s chrs={} ledger={} verify=OK",
        t0.elapsed().as_secs_f64(),
        brain.n_chromosomes(),
        ctx.ledger_entry_count()
    );
    println!(
        "train_accepted={train_acc} prune_accepted={prune_acc} prune_rejected={prune_rej}"
    );
    println!(
        "utility {:.4} → {:.4} | task {:.3} → {:.3} | bio {:.3} → {:.3} | cost {:.3} → {:.3}",
        f0.utility(),
        f1.utility(),
        f0.task_accuracy,
        f1.task_accuracy,
        f0.biological_consistency,
        f1.biological_consistency,
        f0.structural_cost,
        f1.structural_cost
    );
    println!(
        "synapses {} → {} | mean_w {:.3} → {:.3} | mem {} → {}",
        s0.n_synapses,
        s1.n_synapses,
        s0.mean_synapse_weight,
        s1.mean_synapse_weight,
        s0.approx_memory_bytes,
        s1.approx_memory_bytes
    );
    println!(
        "calib_task={:.3} genomic_task={:.3} ld_cov={:.3}",
        ctx.last_calib_task, ctx.last_genomic_task, ctx.last_ld_coverage
    );
}
