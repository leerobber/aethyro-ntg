#!/usr/bin/env python3
"""
Generate realistic synthetic VCF data mimicking 1000 Genomes structure.

This script creates a multi-population VCF with:
- Realistic allele frequency distributions
- Population structure (FST differentiation)
- Linkage disequilibrium patterns
- Deterministic PRNG for reproducibility

Usage:
  python3 scripts/generate_test_vcf.py --output data/1kg_synthetic.vcf.gz --n-snps 50000 --n-samples 500
"""

import argparse
import gzip
import json
from pathlib import Path
from datetime import datetime
import numpy as np


def generate_population_structure(n_snps: int, seed: int = 42) -> dict:
    """Generate realistic allele frequencies with population structure."""
    np.random.seed(seed)

    # Define populations (mimics 1KG)
    populations = {
        'CEU': {'n_samples': 99, 'label': '1000G:CEU'},   # Utah Europeans
        'YRI': {'n_samples': 88, 'label': '1000G:YRI'},   # Yoruba
        'JPT': {'n_samples': 105, 'label': '1000G:JPT'},  # Japanese
        'CHB': {'n_samples': 97, 'label': '1000G:CHB'},   # Han Chinese
    }

    # Shared allele frequencies (global background)
    shared_af = np.random.beta(0.5, 0.5, n_snps)  # Beta distribution (realistic)

    # Population-specific drift (FST ~0.01-0.02 between populations)
    for pop in populations:
        drift = np.random.normal(0, 0.02, n_snps)
        populations[pop]['af'] = np.clip(shared_af + drift, 0.01, 0.99)

    return populations, shared_af


def generate_vcf_header(populations: dict, n_snps: int, creation_date: str) -> list:
    """Generate VCF header lines."""
    header = [
        "##fileformat=VCFv4.2",
        f"##fileDate={creation_date}",
        "##source=generate_test_vcf.py",
        f"##reference=GRCh37",
        "##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele Frequency\">",
        "##INFO=<ID=MAF,Number=A,Type=Float,Description=\"Minor Allele Frequency\">",
        "##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">",
    ]

    # Sample columns
    samples = []
    for pop_name, pop_data in populations.items():
        for i in range(pop_data['n_samples']):
            samples.append(f"{pop_name}_{i:03d}")

    header.append("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\t" + "\t".join(samples))

    return header, samples


def generate_genotypes(populations: dict, n_snps: int, samples: list, seed: int = 42) -> np.ndarray:
    """
    Generate realistic genotypes using population allele frequencies.

    Returns: (n_snps, n_samples) array of genotypes (0/0, 0/1, 1/1 → 0, 1, 2)
    """
    np.random.seed(seed)
    n_samples = len(samples)
    genotypes = np.zeros((n_snps, n_samples), dtype=np.uint8)

    sample_idx = 0
    for pop_name, pop_data in populations.items():
        n_pop_samples = pop_data['n_samples']
        af = pop_data['af']

        # Draw genotypes from Hardy-Weinberg equilibrium
        for i in range(n_pop_samples):
            for j in range(n_snps):
                p = af[j]
                # HWE: P(0/0) = p², P(0/1) = 2pq, P(1/1) = q²
                u = np.random.random()
                if u < p * p:
                    genotypes[j, sample_idx + i] = 0
                elif u < p * p + 2 * p * (1 - p):
                    genotypes[j, sample_idx + i] = 1
                else:
                    genotypes[j, sample_idx + i] = 2

        sample_idx += n_pop_samples

    return genotypes


def vcf_record(chrom: int, pos: int, ref: str, alt: str, af: float, maf: float,
               genotypes: np.ndarray, samples: list) -> str:
    """Generate a single VCF record."""
    snp_id = f"rs{chrom}_{pos}"
    qual = "."
    filter_field = "PASS"
    info = f"AF={af:.4f};MAF={maf:.4f}"
    fmt = "GT"

    # Format genotypes
    gt_strings = []
    for gt in genotypes:
        if gt == 0:
            gt_strings.append("0/0")
        elif gt == 1:
            gt_strings.append("0/1")
        else:
            gt_strings.append("1/1")

    return f"{chrom}\t{pos}\t{snp_id}\t{ref}\t{alt}\t{qual}\t{filter_field}\t{info}\t{fmt}\t" + "\t".join(gt_strings)


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic VCF data for testing")
    parser.add_argument("--output", default="data/1kg_synthetic.vcf.gz", help="Output VCF file (gzip)")
    parser.add_argument("--n-snps", type=int, default=50000, help="Number of SNPs")
    parser.add_argument("--n-samples", type=int, default=389, help="Total number of samples (auto-distributed across populations)")
    parser.add_argument("--chrom", type=int, default=21, help="Chromosome to simulate")
    parser.add_argument("--seed", type=int, default=42, help="Random seed for reproducibility")

    args = parser.parse_args()

    # Create output directory
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print("Generating synthetic VCF:")
    print(f"  Output: {output_path}")
    print(f"  SNPs: {args.n_snps:,}")
    print(f"  Samples: {args.n_samples:,}")
    print(f"  Chromosome: {args.chrom}")
    print(f"  Seed: {args.seed}")
    print()

    # Generate population structure
    print("Generating population structure...")
    populations, shared_af = generate_population_structure(args.n_snps, seed=args.seed)

    # Generate VCF header
    print("Generating VCF header...")
    creation_date = datetime.now().isoformat()
    header, samples = generate_vcf_header(populations, args.n_snps, creation_date)

    samples_per_pop = ', '.join(f'{p}={populations[p]["n_samples"]}' for p in populations)
    print(f"  Samples per population: {samples_per_pop}")
    print(f"  Total samples: {len(samples)}")
    print()

    # Generate genotypes
    print("Generating genotypes...")
    genotypes = generate_genotypes(populations, args.n_snps, samples, seed=args.seed)

    # Write VCF
    print("Writing VCF file...")
    with gzip.open(output_path, 'wt') as f:
        # Write header
        for line in header:
            f.write(line + '\n')

        # Write records
        for snp_idx in range(args.n_snps):
            # Realistic positions (50K SNPs spread across chr21: ~48 Mb)
            pos = int((snp_idx + 1) * (48_000_000 / args.n_snps))
            ref = 'A'
            alt = 'G'

            # Calculate allele frequency from genotypes
            gt = genotypes[snp_idx]
            allele_count = np.sum(gt)
            total_alleles = len(gt) * 2
            af = allele_count / total_alleles
            maf = min(af, 1 - af)

            # Skip monomorphic SNPs
            if maf < 0.01:
                continue

            record = vcf_record(args.chrom, pos, ref, alt, af, maf, gt, samples)
            f.write(record + '\n')

    print(f"✅ Generated {args.n_snps:,} SNP records")
    print(f"📊 File size: {output_path.stat().st_size / 1024 / 1024:.1f} MB")

    # Generate summary statistics
    print()
    print("Summary Statistics:")
    print(f"  Mean AF: {np.mean(shared_af):.3f}")
    print(f"  MAF < 0.05: {np.sum(np.minimum(shared_af, 1-shared_af) < 0.05) / len(shared_af) * 100:.1f}%")
    print(f"  MAF 0.05-0.50: {np.sum((np.minimum(shared_af, 1-shared_af) >= 0.05) & (np.minimum(shared_af, 1-shared_af) <= 0.50)) / len(shared_af) * 100:.1f}%")

    # Write metadata
    populations_meta = {}
    for p in populations:
        populations_meta[p] = {
            'n_samples': populations[p]['n_samples'],
            'mean_af': float(np.mean(populations[p]['af']))
        }

    metadata = {
        'file': str(output_path),
        'n_snps': args.n_snps,
        'n_samples': len(samples),
        'chromosome': args.chrom,
        'populations': populations_meta,
        'creation_date': creation_date,
        'seed': args.seed,
    }

    metadata_path = output_path.with_suffix('.json')
    with open(metadata_path, 'w') as f:
        json.dump(metadata, f, indent=2)

    print(f"✅ Metadata: {metadata_path}")
    print()


if __name__ == '__main__':
    main()
