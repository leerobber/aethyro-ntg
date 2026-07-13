//! Integration regression for the sovereign stack harden pass.
//!
//! Asserts the measured train-accept / prune-reject pattern under real
//! multi-axis fitness (no VCF required).

use ntg_kernel::genomic::{
    format_summary, init_chromosome_brain, run_selection_loop, BitstreamGenotypes, ChromosomeId,
    HaplotypeBlock, LdPair, SnpRecord, SovereignBrain, SovereignFitnessContext,
};

fn synthetic_brain() -> SovereignBrain {
    fn make_chr(chr: u8, n_snps: usize, n_samples: usize) -> ntg_kernel::genomic::ChromosomeBrain {
        let mut snps = Vec::new();
        let mut records = Vec::new();
        for i in 0..n_snps {
            let mut g = BitstreamGenotypes::new(n_samples);
            for s in 0..n_samples {
                let gt = if i % 2 == 0 {
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
            for j in (i + 1)..(i + 3).min(n_snps as u32) {
                pairs.push(LdPair {
                    snp1_idx: i,
                    snp2_idx: j,
                    r_squared: 0.85 - 0.05 * (j - i) as f32,
                    position1: records[i as usize].position,
                    position2: records[j as usize].position,
                });
            }
        }
        let blocks = vec![HaplotypeBlock {
            id: 0,
            snp_indices: (0..n_snps as u32).collect(),
            mean_r_squared: 0.8,
            start_position: records[0].position,
            end_position: records[n_snps - 1].position,
            size: n_snps as u32,
        }];
        init_chromosome_brain(ChromosomeId(chr), &snps, &records, &pairs, &blocks).unwrap()
    }

    let mut brain = SovereignBrain::new(32);
    brain.ingest_brain(make_chr(1, 12, 64));
    brain.ingest_brain(make_chr(22, 10, 64));
    for b in brain.chromosomes.values_mut() {
        for s in &mut b.synapses {
            s.weight = (s.ld_r2 * 0.3).clamp(0.0, 1.0);
            s.plasticity = 0.1;
        }
    }
    brain.refresh_structure();
    brain
}

#[test]
fn train_tends_to_accept_prune_tends_to_reject() {
    let mut brain = synthetic_brain();
    let mut ctx = SovereignFitnessContext::new().unwrap();
    ctx.freeze_all_from_brain(&brain);
    let _ = ctx.install_calib_from_fixtures(15);

    let summary = run_selection_loop(&mut brain, &mut ctx, 6, 8, 0.18, None).unwrap();
    assert_eq!(summary.steps.len(), 6);
    // At least one train accept under under-trained weights.
    assert!(
        summary.train_accepted >= 1,
        "expected train accepts, got {}",
        format_summary(&summary)
    );
    // At least one prune reject under real biology coverage gate.
    assert!(
        summary.prune_rejected >= 1,
        "expected prune rejects, got {}",
        format_summary(&summary)
    );
    assert!(ctx.ledger_entry_count() >= 6);
    assert!((ctx.score_safety() - 1.0).abs() < 1e-6);
}

#[test]
fn ltm_motifs_hit_on_activate() {
    let mut brain = synthetic_brain();
    brain.consolidate(0.5, 0.0);
    brain.activate(&[0.0; 8], None);
    assert!(
        !brain.working_set.motif_ids.is_empty(),
        "harden: LTM must contribute motifs"
    );
}
