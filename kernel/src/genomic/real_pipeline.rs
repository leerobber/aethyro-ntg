/// Shared real-data pipeline: VCF -> LD -> blocks -> brain -> Phase C
/// synthesis, all built from one real chromosome's VCF file. Factored out
/// because Phase D and Phase E both need "the real reference genome and
/// Phase C's real synthetic genome for chromosome N" and were duplicating
/// this glue.
use crate::genomic::chromosome_brain::{init_chromosome_brain, ChromosomeBrain, ChromosomeId};
use crate::genomic::haplotype_blocks::BlockDetector;
use crate::genomic::ld_compute::{LdComputer, LdPair};
use crate::genomic::synthesis::{Genome, GenomeSampler};
use crate::genomic::validation::{ReferenceGenome, SyntheticGenome};
use crate::genomic::vcf_stream::VcfParser;

/// Stable per-locus key shared between a chromosome's ReferenceGenome and
/// SyntheticGenome maps. Real 1000G variants are frequently missing an
/// rsID (id == "."), so the SNP's index in parse order is used instead of
/// its VCF ID to avoid collisions.
pub fn snp_key(idx: u32) -> String {
    format!("snp{}", idx)
}

/// Everything needed to validate one real chromosome: the real reference
/// (built directly from parsed VCF genotypes), the brain built from the
/// same data, and a Phase C synthetic genome sampled from that brain
/// (with its own empirically-derived SyntheticGenome view for comparison).
pub struct RealChromosomeData {
    pub chr: ChromosomeId,
    pub n_real_samples: usize,
    pub reference: ReferenceGenome,
    pub synthetic: SyntheticGenome,
    pub sampled_genome: Genome,
    pub ld_pairs: Vec<LdPair>,
    pub snp_order: Vec<String>,
    pub brain: ChromosomeBrain,
    /// Whether the synthetic genome was sampled using real haplotype
    /// pools (`GenomeSampler::from_brain_with_haplotypes`) or independent
    /// per-locus draws (`GenomeSampler::from_brain`). See
    /// `build_real_chromosome`'s `use_haplotypes` parameter.
    pub used_haplotype_sampling: bool,
}

/// Parse a real chromosome VCF, compute real LD, build the real brain, and
/// sample one Phase C synthetic genome whose per-locus targets are that
/// brain's real allele frequencies. `max_variants` bounds the parse to a
/// leading slice of the chromosome (real 1000G chromosomes run into the
/// millions of variants).
///
/// `use_haplotypes`: if true, parses phased genotype data (real 1000G
/// VCFs are phased) and samples via `GenomeSampler::from_brain_with_haplotypes`,
/// which preserves real within-block LD by resampling real observed
/// haplotype fragments instead of drawing each locus independently. Costs
/// roughly 2x the parse time/memory of the `false` path (see
/// `VcfParser::parse_vcf_phased_limited`).
pub fn build_real_chromosome(
    vcf_path: &str,
    chr: u8,
    max_variants: Option<usize>,
    synthetic_n_samples: usize,
    seed: u64,
    use_haplotypes: bool,
) -> Result<RealChromosomeData, String> {
    let parser = VcfParser::new(false);
    let chromosome = if use_haplotypes {
        parser.parse_vcf_phased_limited(vcf_path, chr, max_variants)?
    } else {
        parser.parse_vcf_limited(vcf_path, chr, max_variants)?
    };

    let positions: Vec<u32> = chromosome.snps.iter().map(|s| s.position).collect();
    let ld_matrix = LdComputer::new(false, 0.5).compute_ld(&chromosome.genotypes, &positions)?;

    let mut blocks = BlockDetector::new(false).detect_blocks(&ld_matrix.pairs, chromosome.snps.len())?;
    BlockDetector::new(false).annotate_blocks(&mut blocks, &positions)?;

    let brain = init_chromosome_brain(
        ChromosomeId(chr),
        &chromosome.genotypes,
        &chromosome.snps,
        &ld_matrix.pairs,
        &blocks,
    )?;

    let mut reference = ReferenceGenome::new(format!("1000G-chr{}", chr), chromosome.sample_names.len());
    for (idx, snp) in chromosome.genotypes.iter().enumerate() {
        let (freq_ref, freq_alt, _freq_missing) = snp.allele_frequencies();
        reference.add_snp(snp_key(idx as u32), freq_alt as f32, freq_ref as f32);
    }
    for pair in &ld_matrix.pairs {
        reference.add_ld_pair(snp_key(pair.snp1_idx), snp_key(pair.snp2_idx), pair.r_squared);
    }
    reference.finalize();

    let n_real_samples = chromosome.sample_names.len();
    let sampler = if use_haplotypes {
        GenomeSampler::from_brain_with_haplotypes(
            &brain,
            &chromosome.hap_a,
            &chromosome.hap_b,
            n_real_samples,
            synthetic_n_samples,
            seed,
        )
    } else {
        GenomeSampler::from_brain(&brain, synthetic_n_samples, seed)
    };
    let sampled_genome = sampler.sample(0);

    let mut synthetic = SyntheticGenome::new(synthetic_n_samples);
    for idx in 0..sampled_genome.genotypes.len() {
        let freq_alt = sampled_genome.allele_freq(idx);
        synthetic.add_snp(snp_key(idx as u32), freq_alt, 1.0 - freq_alt);
    }
    for pair in &ld_matrix.pairs {
        let r2 = sampled_genome.ld_r2(pair.snp1_idx as usize, pair.snp2_idx as usize);
        synthetic.add_ld_pair(snp_key(pair.snp1_idx), snp_key(pair.snp2_idx), r2);
    }
    synthetic.finalize();

    let snp_order: Vec<String> = (0..chromosome.snps.len() as u32).map(snp_key).collect();

    Ok(RealChromosomeData {
        chr: ChromosomeId(chr),
        n_real_samples,
        reference,
        synthetic,
        sampled_genome,
        ld_pairs: ld_matrix.pairs,
        snp_order,
        brain,
        used_haplotype_sampling: use_haplotypes,
    })
}
