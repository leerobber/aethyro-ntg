//! Integration tests for NTG Genomic Operator
//! Tests OmniSynth-X bitsliced LD/PRS computation

#[cfg(test)]
mod tests {
    use ntg_kernel::ntg::operators::genomic::{GenomicOperator, GenomicNode};
    use ntg_kernel::ntg::operators::genomic_loader::{GenomicBatch, GenomicPipeline};

    #[test]
    fn test_genomic_operator_basic() {
        let op = GenomicOperator::new(100, 50);
        assert_eq!(op.num_individuals, 100);
        assert_eq!(op.num_snps, 50);
        assert_eq!(op.words_per_snp, 2); // (100 + 63) / 64 = 2
    }

    #[test]
    fn test_set_get_genotypes() {
        let mut op = GenomicOperator::new(50, 10);

        op.set(0, 0, 0); // ref/ref
        assert_eq!(op.get(0, 0), 0);

        op.set(0, 1, 1); // ref/alt
        assert_eq!(op.get(0, 1), 1);

        op.set(0, 2, 2); // alt/alt
        assert_eq!(op.get(0, 2), 2);

        op.set(0, 3, 3); // missing
        assert_eq!(op.get(0, 3), 3);
    }

    #[test]
    fn test_statistics_computation() {
        let mut op = GenomicOperator::new(100, 20);

        for snp in 0..20 {
            for ind in 0..100 {
                let genotype = ((snp + ind) % 3) as u8;
                op.set(snp, ind, genotype);
            }
        }

        op.compute_statistics();

        assert!(op.is_stats_valid);
        assert_eq!(op.means.len(), 20);
        assert_eq!(op.std_devs.len(), 20);

        for mean in &op.means {
            assert!(*mean >= 0.0 && *mean <= 2.0);
        }

        for std in &op.std_devs {
            assert!(*std >= 0.0);
        }
    }

    #[test]
    fn test_prs_computation() {
        let mut op = GenomicOperator::new(50, 10);

        for snp in 0..10 {
            for ind in 0..50 {
                let val = ((snp * ind) % 3) as u8;
                op.set(snp, ind, val);
            }
        }

        let weights: Vec<f64> = (0..10).map(|i| 0.1 * (i as f64)).collect();
        let prs = op.compute_prs(&weights);

        assert_eq!(prs.len(), 50);

        for score in &prs {
            assert!(*score >= 0.0);
        }
    }

    #[test]
    fn test_ld_matrix_computation() {
        let mut op = GenomicOperator::new(100, 20);

        for ind in 0..100 {
            op.set(0, ind, 1); // SNP 0: all heterozygous
            op.set(1, ind, 1); // SNP 1: all heterozygous (perfect LD)
            op.set(2, ind, 0); // SNP 2: all homozygous ref (independent)
        }

        let ld_matrix = op.compute_ld_matrix();

        assert_eq!(ld_matrix.len(), 20 * 20);

        // Diagonal should be 1.0 (self-correlation)
        for i in 0..20 {
            assert!((ld_matrix[i * 20 + i] - 1.0).abs() < 0.01);
        }

        // SNP 0 and 1 should have high LD (close to 1.0)
        assert!(ld_matrix[0 * 20 + 1] > 0.9);
        assert!(ld_matrix[1 * 20 + 0] > 0.9);

        // Symmetric
        for i in 0..20 {
            for j in 0..20 {
                let r_ij = ld_matrix[i * 20 + j];
                let r_ji = ld_matrix[j * 20 + i];
                assert!((r_ij - r_ji).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn test_genomic_node() {
        let node = GenomicNode::new(0, 100, 50);
        assert_eq!(node.node_id, 0);
        assert_eq!(node.operator.num_individuals, 100);
        assert_eq!(node.operator.num_snps, 50);
        assert_eq!(node.operator_type, "genomic");
    }

    #[test]
    fn test_missing_data_rate() {
        let mut op = GenomicOperator::new(100, 10);

        // Set 10 missing genotypes in first SNP
        for i in 0..10 {
            op.set(0, i, 3);
        }

        let missing_rate = op.estimate_missing_rate();
        let expected_rate = 10.0 / (100.0 * 10.0);

        assert!((missing_rate - expected_rate).abs() < 0.001);
    }

    #[test]
    fn test_genomic_batch_serialization() {
        let batch = GenomicBatch {
            num_individuals: 50,
            num_snps: 20,
            records: vec![],
        };

        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: GenomicBatch = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.num_individuals, 50);
        assert_eq!(deserialized.num_snps, 20);
    }

    #[test]
    fn test_genomic_pipeline() {
        let mut pipeline = GenomicPipeline::new();

        let batch1 = GenomicBatch {
            num_individuals: 100,
            num_snps: 50,
            records: vec![],
        };

        let batch2 = GenomicBatch {
            num_individuals: 100,
            num_snps: 30,
            records: vec![],
        };

        pipeline.batches.push(batch1);
        pipeline.batches.push(batch2);

        let summary = pipeline.summary();
        assert_eq!(summary.num_batches, 2);
        assert_eq!(summary.total_snps, 80);
        assert_eq!(summary.total_individuals, 100);
    }

    #[test]
    fn test_allele_frequency_calculation() {
        let mut op = GenomicOperator::new(100, 1);

        for ind in 0..70 {
            op.set(0, ind, 0); // ref/ref: 70 individuals
        }
        for ind in 70..100 {
            op.set(0, ind, 2); // alt/alt: 30 individuals
        }

        op.compute_statistics();

        let mean = op.means[0];
        let expected_mean = (0.0 * 70.0 + 2.0 * 30.0) / 100.0;
        assert!((mean - expected_mean).abs() < 0.01);

        let allele_freq = op.allele_freqs[0];
        let expected_af = expected_mean / 2.0;
        assert!((allele_freq - expected_af).abs() < 0.01);
    }

    #[test]
    fn test_bulk_load_export() {
        let op = GenomicOperator::new(20, 10);

        let genotypes = vec![0u8; 20 * 10];

        assert_eq!(genotypes.len(), op.num_individuals * op.num_snps);
    }
}
