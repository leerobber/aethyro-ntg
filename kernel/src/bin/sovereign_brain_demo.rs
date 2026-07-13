//! Rung 1 + Rung 2 demo: multi-chromosome SovereignBrain + multi-axis selection.
//!
//! Modes:
//!   synthetic (default) — no VCF; builds two synthetic chromosomes, consolidates,
//!                         activates, and runs multi-axis prune selection steps.
//!   vcf — ingest one or more real VCFs then run the same loop.
//!
//! Usage:
//!   cargo run --release --bin sovereign_brain_demo
//!   cargo run --release --bin sovereign_brain_demo -- vcf \
//!     ../data/raw/1000g/ALL.chr22....vcf.gz 22 2000 \
//!     [second.vcf.gz 1 2000 ...]

use ntg_kernel::genomic::{
    BitstreamGenotypes, ChromosomeId, HaplotypeBlock, LdPair, SnpRecord, SovereignBrain,
    init_chromosome_brain,
};
use ntg_kernel::ntg::mutation::{
    MultiAxisEvaluator, proxy_biology_from_structure, proxy_task_from_structure,
};

fn make_synthetic_chr(chr: u8, n_snps: usize, n_samples: usize) -> ntg_kernel::genomic::ChromosomeBrain {
    let mut snps = Vec::new();
    let mut records = Vec::new();
    for i in 0..n_snps {
        let mut g = BitstreamGenotypes::new(n_samples);
        for s in 0..n_samples {
            let gt = if i % 2 == 0 {
                if s % 2 == 0 { 0 } else { 2 }
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

fn run_rung2_loop(brain: &mut SovereignBrain, steps: usize) {
    // min_delta=0.001: accept small but real multi-axis gains; production
    // loops can raise this. bio_slack=0.05 allows minor biology noise.
    let ev = MultiAxisEvaluator::new(0.001, 0.05);
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for step in 0..steps {
        let out = ev.select_prune_step(
            brain,
            0.20,
            1.0, // safety full while ledger not yet attached
            proxy_task_from_structure,
            proxy_biology_from_structure,
        );
        println!(
            "  step {step}: baseline_u={:.4} cand_u={:.4} task={:.3}->{:.3} bio={:.3}->{:.3} cost={:.3}->{:.3} accepted={}",
            out.baseline.utility(),
            out.candidate.utility(),
            out.baseline.task_accuracy,
            out.candidate.task_accuracy,
            out.baseline.biological_consistency,
            out.candidate.biological_consistency,
            out.baseline.structural_cost,
            out.candidate.structural_cost,
            out.accepted
        );
        if out.accepted {
            if let Some(child) = out.child {
                *brain = child;
                accepted += 1;
            }
        } else {
            rejected += 1;
        }
    }
    println!("[rung2] accepted={accepted} rejected={rejected} final_gen={}", brain.generation);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut brain = SovereignBrain::new(64);

    if args.get(1).map(|s| s.as_str()) == Some("vcf") {
        // pairs: path chr max_variants
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
    }

    print_structure("after_ingest", &brain);

    // Rung 1: train, consolidate, activate
    brain.train_all(5);
    let rep = brain.consolidate(0.5, 0.0);
    println!(
        "[rung1] consolidate motifs_added={} ltm_total={} gen={}",
        rep.motifs_added, rep.ltm_total, rep.generation
    );
    let ws = brain.activate(&[0.85, 1.2, 1.0, 0.05, 0.0, 0.0, 1.0, 1.0], None);
    println!(
        "[rung1] activate working_set={} motifs_hit={}",
        ws.len(),
        ws.motif_ids.len()
    );
    print_structure("after_rung1", &brain);

    // Rung 2: multi-axis selection loop
    println!("[*] multi-axis prune selection (Rung 2)");
    run_rung2_loop(&mut brain, 8);
    print_structure("final", &brain);

    let ltm = brain.ltm_stats();
    println!(
        "[done] chromosomes={} ltm_motifs={} ltm_hits={} — Rung1+2 loop complete",
        brain.n_chromosomes(),
        ltm.n_motifs,
        ltm.total_hits
    );
}
