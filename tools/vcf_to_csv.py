#!/usr/bin/env python3
"""
Convert VCF to CSV format for NTG genomic operator
Handles large gzipped VCF files with streaming
"""

import gzip
import sys
import argparse
from pathlib import Path
import csv
import time

def parse_vcf_header(vcf_file):
    """Extract sample names from VCF header"""
    samples = []
    open_func = gzip.open if str(vcf_file).endswith('.gz') else open

    with open_func(vcf_file, 'rt') as f:
        for line in f:
            if line.startswith('#CHROM'):
                parts = line.strip().split('\t')
                samples = parts[9:]  # Sample names start at column 9
                break
    return samples

def vcf_to_csv(vcf_file, output_csv, max_variants=None, verbose=True):
    """
    Convert VCF to CSV format

    Output CSV format:
    snp_id,position,sample1,sample2,...
    rs1000001,1000000,0,1,2,...

    Genotypes: 0=ref/ref, 1=ref/alt, 2=alt/alt, 3=missing
    """

    if verbose:
        print(f"[*] Parsing VCF header...")

    samples = parse_vcf_header(vcf_file)
    num_samples = len(samples)

    if verbose:
        print(f"[OK] Found {num_samples} samples")
        print(f"\n[*] Converting VCF to CSV...")
        print(f"  Output: {output_csv}")

    variant_count = 0
    skipped = 0
    start_time = time.time()
    open_func = gzip.open if str(vcf_file).endswith('.gz') else open

    with open_func(vcf_file, 'rt') as vcf_in, open(output_csv, 'w', newline='') as csv_out:
        writer = csv.writer(csv_out)

        # Write header
        writer.writerow(['snp_id', 'position'] + samples)

        for line in vcf_in:
            if line.startswith('#'):
                continue

            parts = line.strip().split('\t')
            if len(parts) < 10:
                skipped += 1
                continue

            # Extract variant info
            chrom = parts[0]
            pos = int(parts[1])
            var_id = parts[2] if parts[2] != '.' else f"chr{chrom}_{pos}"

            # Extract genotypes (convert to 0/1/2/3)
            genotypes = []
            for i in range(9, min(9 + num_samples, len(parts))):
                gt_field = parts[i]

                # Extract GT (first field before :)
                if ':' in gt_field:
                    gt = gt_field.split(':')[0]
                else:
                    gt = gt_field

                # Convert to 0/1/2/3
                if gt == './.':
                    genotypes.append('3')  # Missing
                elif gt == '.':
                    genotypes.append('3')  # Missing
                else:
                    # Handle both | and / separators
                    alleles = gt.replace('|', '/').split('/')
                    try:
                        count = sum(1 for a in alleles if a == '1')
                        genotypes.append(str(count))
                    except:
                        genotypes.append('3')  # Missing

            # Pad with missing values if needed
            while len(genotypes) < num_samples:
                genotypes.append('3')

            writer.writerow([var_id, pos] + genotypes[:num_samples])

            variant_count += 1
            if variant_count % 10000 == 0 and verbose:
                elapsed = time.time() - start_time
                rate = variant_count / elapsed
                print(f"  [*] Processed {variant_count:,} variants ({rate:.0f} variants/sec)...")

            if max_variants and variant_count >= max_variants:
                break

        elapsed = time.time() - start_time

        if verbose:
            print(f"\n[OK] Conversion complete!")
            print(f"  Variants: {variant_count:,}")
            print(f"  Samples: {num_samples:,}")
            print(f"  Total genotypes: {variant_count * num_samples:,}")
            print(f"  Skipped: {skipped}")
            print(f"  Time: {elapsed:.1f}s")
            print(f"  Rate: {variant_count/elapsed:.0f} variants/sec")

if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description='Convert VCF to CSV for NTG genomic operator',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python vcf_to_csv.py data/raw/1000g/ALL.chr22.*.vcf.gz -o data/processed/chr22.csv
  python vcf_to_csv.py chr22.vcf --max-variants 50000 -o chr22_test.csv
        """
    )

    parser.add_argument('vcf_file', help='Input VCF file (.vcf or .vcf.gz)')
    parser.add_argument('-o', '--output', help='Output CSV file', required=True)
    parser.add_argument('--max-variants', type=int, help='Maximum variants to process (for testing)')

    args = parser.parse_args()

    vcf_path = Path(args.vcf_file)

    if not vcf_path.exists():
        print(f"[ERROR] File not found: {vcf_path}", file=sys.stderr)
        sys.exit(1)

    try:
        vcf_to_csv(vcf_path, args.output, args.max_variants)
        print(f"\n📍 Next step: Load CSV into NTG")
        print(f"   python tools/load_genomic_data.py {args.output}")
    except Exception as e:
        print(f"❌ Conversion failed: {e}", file=sys.stderr)
        sys.exit(1)
