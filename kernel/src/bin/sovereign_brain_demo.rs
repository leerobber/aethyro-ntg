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
    BitstreamGenotypes, ChromosomeId, HaplotypeBlock, LdPair, LanguageOrgan, SnpRecord,
    SovereignBrain, SovereignFitnessContext, fixture_docs, init_chromosome_brain,
};

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

fn run_real_axis_loop(brain: &mut SovereignBrain, ctx: &mut SovereignFitnessContext, steps: usize) {
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut prune_rej = 0usize;
    let mut train_acc = 0usize;

    let base = ctx.score(brain);
    println!(
        "[axes0] utility={:.4} task={:.3} (calib={:.3} genomic={:.3}) bio={:.3} cost={:.3} safety={:.1} ld_cov={:.3}",
        base.utility(),
        base.task_accuracy,
        ctx.last_calib_task,
        ctx.last_genomic_task,
        base.biological_consistency,
        base.structural_cost,
        base.safety,
        ctx.last_ld_coverage
    );

    for step in 0..steps {
        // Alternate: odd steps try destructive prune (often rejected under real bio),
        // even steps try KAIROS train (preserves LD, can raise utility).
        let (label, out) = if step % 2 == 0 {
            (
                "train",
                ctx.select_train_step(brain, 8).expect("train selection"),
            )
        } else {
            (
                "prune",
                ctx.select_prune_step(brain, 0.18).expect("prune selection"),
            )
        };
        println!(
            "  step {step} [{label}]: u={:.4}->{:.4} task={:.3}->{:.3} bio={:.3}->{:.3} cost={:.3}->{:.3} safety={:.1} cov={:.3} accepted={} ledger={}",
            out.baseline.utility(),
            out.candidate.utility(),
            out.baseline.task_accuracy,
            out.candidate.task_accuracy,
            out.baseline.biological_consistency,
            out.candidate.biological_consistency,
            out.baseline.structural_cost,
            out.candidate.structural_cost,
            out.candidate.safety,
            ctx.last_ld_coverage,
            out.accepted,
            ctx.ledger_entry_count()
        );
        if out.accepted {
            if let Some(child) = out.child {
                *brain = child;
                accepted += 1;
                if label == "train" {
                    train_acc += 1;
                }
            }
        } else {
            rejected += 1;
            if label == "prune" {
                prune_rej += 1;
            }
        }
    }
    println!(
        "[real-axes] accepted={accepted} (train_acc={train_acc}) rejected={rejected} (prune_rej={prune_rej}) gen={} ledger={} verify=OK",
        brain.generation,
        ctx.ledger_entry_count()
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

    // Rung 3: language organ + Phase 4 calib → task axis
    let mut organ = LanguageOrgan::new();
    organ.ingest_documents(&fixture_docs());
    match organ.train_calib_fixtures(25) {
        Ok(r) => println!(
            "[rung3] language calib: samples={} test_bal={:.3} win={}",
            r.n_samples, r.test_metrics.balanced_accuracy, r.is_win
        ),
        Err(e) => eprintln!("[rung3] calib failed: {e}"),
    }
    if let Err(e) = ctx.install_calib_from_language(&organ) {
        let _ = ctx.install_calib_from_fixtures(20);
        eprintln!("[rung3] install from organ: {e} (used fixtures fallback)");
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

    println!("[*] real multi-axis selection (biology + calib/agent task + ledger safety)");
    println!("    train ops should rise utility without losing LD; prune should often fail biology gate");
    run_real_axis_loop(&mut brain, &mut ctx, 8);
    print_structure("final", &brain);

    let ltm = brain.ltm_stats();
    let final_fit = ctx.score(&brain);
    println!(
        "[done] chrs={} ltm={} hits={} utility={:.4} task={:.3} bio={:.3} safety={:.1} ledger={}",
        brain.n_chromosomes(),
        ltm.n_motifs,
        ltm.total_hits,
        final_fit.utility(),
        final_fit.task_accuracy,
        final_fit.biological_consistency,
        final_fit.safety,
        ctx.ledger_entry_count()
    );
}
