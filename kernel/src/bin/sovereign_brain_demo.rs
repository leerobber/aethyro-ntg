//! Rung 1–2 + real fitness axes demo.
//!
//! - Rung 1: multi-chromosome SovereignBrain (working set + LTM)
//! - Rung 2: multi-axis selection
//! - Real axes: Phase D biology vs frozen reference, ChromosomeAgent task,
//!   tamper-evident ledger safety (no structure proxies)
//!
//! Usage:
//!   cargo run --release --bin sovereign_brain_demo
//!   cargo run --release --bin sovereign_brain_demo -- vcf \
//!     ../data/raw/1000g/ALL.chr22....vcf.gz 22 2000

use ntg_kernel::genomic::{
    format_summary, fixture_docs, init_chromosome_brain, run_selection_loop, save_snapshot,
    BitstreamGenotypes, ChromosomeId, HaplotypeBlock, LanguageOrgan, LdPair, SnpRecord,
    SovereignBrain, SovereignFitnessContext,
};
use std::path::Path;

fn make_synthetic_chr(chr: u8, n_snps: usize, n_samples: usize) -> ntg_kernel::genomic::ChromosomeBrain {
    let mut snps = Vec::new();
    let mut records = Vec::new();
    for i in 0..n_snps {
        let mut g = BitstreamGenotypes::new(n_samples);
        for s in 0..n_samples {
            // Mix common + rare so DiseaseRisk / PopulationSignal have signal.
            let gt = if i % 5 == 0 {
                // rare-ish alt
                if s < n_samples / 20 {
                    2
                } else if s < n_samples / 10 {
                    1
                } else {
                    0
                }
            } else if i % 2 == 0 {
                if s % 2 == 0 {
                    0
                } else {
                    2
                }
            } else if s % 3 == 0 {
                1
            } else {
                0
            };
            g.set(s, gt);
        }
        snps.push(g);
        records.push(SnpRecord {
            id: format!("rs{chr}_{i}"),
            position: (i as u32 + 1) * 1000,
            ref_allele: "A".into(),
            alt_allele: "G".into(),
            qual: 100.0,
            info: String::new(),
        });
    }
    let mut pairs = Vec::new();
    for i in 0..n_snps as u32 {
        for j in (i + 1)..(i + 4).min(n_snps as u32) {
            pairs.push(LdPair {
                snp1_idx: i,
                snp2_idx: j,
                r_squared: 0.9 - 0.05 * (j - i) as f32,
                position1: records[i as usize].position,
                position2: records[j as usize].position,
            });
        }
    }
    let blocks = vec![HaplotypeBlock {
        id: 0,
        snp_indices: (0..n_snps as u32).collect(),
        mean_r_squared: 0.82,
        start_position: records[0].position,
        end_position: records[n_snps - 1].position,
        size: n_snps as u32,
    }];
    init_chromosome_brain(ChromosomeId(chr), &snps, &records, &pairs, &blocks).expect("brain")
}

fn print_structure(label: &str, brain: &SovereignBrain) {
    let s = brain.measure_structure();
    println!(
        "[{label}] chrs={} neurons={} synapses={} blocks={} ltm={} ws={} mem≈{}B mean_w={:.3} mean_r2={:.3} gen={}",
        s.n_chromosomes,
        s.n_neurons,
        s.n_synapses,
        s.n_blocks,
        s.n_ltm_motifs,
        s.working_set_len,
        s.approx_memory_bytes,
        s.mean_synapse_weight,
        s.mean_block_r2,
        brain.generation
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut brain = SovereignBrain::new(64);
    let mut ctx = SovereignFitnessContext::new().expect("ledger");

    if args.get(1).map(|s| s.as_str()) == Some("vcf") {
        let mut i = 2;
        while i + 2 < args.len() {
            let path = &args[i];
            let chr: u8 = args[i + 1].parse().expect("chr");
            let max_v: usize = args[i + 2].parse().expect("max_variants");
            println!("[*] ingesting {path} chr{chr} max_variants={max_v}");
            match brain.ingest_vcf(path, chr, Some(max_v), 100, 42, false) {
                Ok(data) => {
                    println!(
                        "    real_samples={} snps={} ld_pairs={}",
                        data.n_real_samples,
                        data.brain.neurons.len(),
                        data.ld_pairs.len()
                    );
                    // Real 1000G panel freezes the biology reference.
                    ctx.register_real_chromosome(&data);
                }
                Err(e) => {
                    eprintln!("    FAILED: {e}");
                    std::process::exit(1);
                }
            }
            i += 3;
        }
        if brain.n_chromosomes() == 0 {
            eprintln!("Usage: sovereign_brain_demo vcf <vcf.gz> <chr> <max_variants> [...]");
            std::process::exit(1);
        }
    } else {
        println!("[*] synthetic multi-chr ingest (chr1 + chr22)");
        brain.ingest_brain(make_synthetic_chr(1, 40, 128));
        brain.ingest_brain(make_synthetic_chr(22, 30, 128));
        // Freeze structure at ingest as biology ground truth.
        ctx.freeze_all_from_brain(&brain);
    }

    print_structure("after_ingest", &brain);

    // Rung 3: language organ + harder docs-corpus calib when available
    let mut organ = LanguageOrgan::new();
    organ.ingest_documents(&fixture_docs());
    match organ.train_calib_fixtures(25) {
        Ok(r) => println!(
            "[rung3] language calib fixtures: samples={} test_bal={:.3} win={}",
            r.n_samples, r.test_metrics.balanced_accuracy, r.is_win
        ),
        Err(e) => eprintln!("[rung3] calib failed: {e}"),
    }
    let docs_dir = Path::new("../docs");
    if docs_dir.is_dir() {
        match ctx.install_calib_from_docs_dir(docs_dir, 30) {
            Ok(bal) => println!("[task-gate] docs-corpus calib holdout_bal={bal:.3}"),
            Err(e) => {
                eprintln!("[task-gate] docs calib: {e}; using organ model");
                let _ = ctx.install_calib_from_language(&organ);
            }
        }
    } else if let Err(e) = ctx.install_calib_from_language(&organ) {
        let _ = ctx.install_calib_from_fixtures(20);
        eprintln!("[rung3] install: {e}");
    }
    brain.attach_language(organ);

    // Seed LTM without pruning. Deliberately leave synapse weights under-trained
    // so the real-axis train operator has headroom (KAIROS → ld_r2 targets).
    for b in brain.chromosomes.values_mut() {
        for s in &mut b.synapses {
            s.weight = (s.ld_r2 * 0.35).clamp(0.0, 1.0);
            s.plasticity = 0.08;
        }
    }
    brain.refresh_structure();
    let rep = brain.consolidate(0.5, 0.0);
    println!(
        "[rung1] consolidate motifs_added={} ltm_total={} gen={}",
        rep.motifs_added, rep.ltm_total, rep.generation
    );
    let ws = brain.activate_from_text("haplotype LD chr22 and fn main ternary kernel");
    println!(
        "[rung3] activate_from_text: genomic={} lang_nodes={} motifs={} query={:?}",
        ws.neurons.len(),
        ws.language_nodes.len(),
        ws.motif_ids.len(),
        ws.language_query
    );
    print_structure("after_rung1_3", &brain);

    println!("[*] shared selection loop (biology + calib/agent task + ledger + JSONL)");
    let jsonl = Path::new("../results/sovereign_demo_metrics.jsonl");
    let summary = run_selection_loop(&mut brain, &mut ctx, 8, 8, 0.18, Some(jsonl))
        .expect("selection loop");
    for rec in &summary.steps {
        println!(
            "  step {} [{}]: u={:.4}->{:.4} bio={:.3}->{:.3} accepted={} ledger={}",
            rec.step,
            rec.op,
            rec.baseline.utility(),
            rec.candidate.utility(),
            rec.baseline.biological_consistency,
            rec.candidate.biological_consistency,
            rec.accepted,
            rec.ledger_entries
        );
    }
    println!("[loop] {}", format_summary(&summary));
    print_structure("final", &brain);

    let docs = fixture_docs();
    let pairs: Vec<(&str, &str)> = docs.iter().map(|(a, b)| (*a, *b)).collect();
    match save_snapshot(Path::new("../artifacts/sovereign_demo_snap"), &brain, &ctx, &pairs) {
        Ok(r) => println!(
            "[persist] saved {} motifs={} calib={} docs={}",
            r.dir, r.n_motifs, r.wrote_calib, r.wrote_language_docs
        ),
        Err(e) => eprintln!("[persist] {e}"),
    }

    let ltm = brain.ltm_stats();
    let final_fit = ctx.score(&brain);
    println!(
        "[done] chrs={} ltm={} hits={} motifs_ws={} utility={:.4} task={:.3} bio={:.3} safety={:.1} ledger={} jsonl={}",
        brain.n_chromosomes(),
        ltm.n_motifs,
        ltm.total_hits,
        brain.working_set.motif_ids.len(),
        final_fit.utility(),
        final_fit.task_accuracy,
        final_fit.biological_consistency,
        final_fit.safety,
        ctx.ledger_entry_count(),
        jsonl.display()
    );
}
