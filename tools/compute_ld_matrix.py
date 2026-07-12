#!/usr/bin/env python3
"""
Compute LD (Linkage Disequilibrium) Matrix from 1000 Genomes CSV
Shows how genetic variants correlate across the population
"""

import csv
import sys
import time
from collections import defaultdict

def compute_ld_matrix(csv_path, summary_only=False, sample_n=None):
    print("=" * 64)
    print("COMPUTING: Linkage Disequilibrium (LD) Matrix")
    print("=" * 64)
    print("")
    print(f"Input: {csv_path}")
    print("")

    start_overall = time.time()

    # Load CSV
    print("[*] Loading genotypes...")
    start = time.time()

    genotypes = []
    variant_ids = []
    positions = []
    num_samples = 0

    with open(csv_path, 'r') as f:
        reader = csv.reader(f)

        # Parse header
        header = next(reader)
        num_samples = len(header) - 2
        print(f"✓ Found {num_samples} samples")

        # Load genotypes
        for row_num, row in enumerate(reader):
            if len(row) < 3:
                continue

            variant_ids.append(row[0])
            positions.append(int(row[1]))

            geno = []
            for i in range(2, min(2 + num_samples, len(row))):
                try:
                    val = int(row[i])
                except:
                    val = 3
                geno.append(val)

            genotypes.append(geno)

            if sample_n and len(genotypes) >= sample_n:
                break

            if len(genotypes) % 10000 == 0:
                print(f"  ✓ Loaded {len(genotypes)} variants...")

    num_variants = len(genotypes)
    load_time = time.time() - start

    print(f"✓ Loaded {num_variants} variants in {load_time:.1f}s")
    print(f"  Memory: ~{(num_variants * num_samples) / 1_000_000:.1f} MB")
    print("")

    if summary_only:
        print_summary_only(genotypes, variant_ids, positions, num_samples)
        return

    # Compute statistics
    print("📊 Computing statistics...")
    start = time.time()

    means = [0.0] * num_variants
    variances = [0.0] * num_variants

    for v, geno in enumerate(genotypes):
        total = 0.0
        sum2 = 0.0
        valid = 0.0

        for g in geno:
            if g != 3:  # Skip missing
                total += g
                sum2 += g * g
                valid += 1.0

        if valid > 0:
            means[v] = total / valid
            variances[v] = (sum2 / valid) - (means[v] * means[v])

    stats_time = time.time() - start
    print(f"✓ Statistics computed in {stats_time:.1f}s")
    print("")

    # Compute LD matrix
    print(f"🔗 Computing LD matrix ({num_variants} SNPs)...")
    print(f"  This will compute {num_variants * (num_variants - 1) // 2} pairwise correlations")
    print("")

    start = time.time()
    ld_count = 0
    high_ld_pairs = []

    for i in range(num_variants):
        for j in range(i + 1, num_variants):
            dot = 0.0
            valid = 0.0

            for k in range(num_samples):
                gi = genotypes[i][k]
                gj = genotypes[j][k]

                if gi != 3 and gj != 3:
                    dot += gi * gj
                    valid += 1.0

            if valid > 0 and variances[i] > 1e-9 and variances[j] > 1e-9:
                r = ((dot / valid) - (means[i] * means[j])) / (variances[i] ** 0.5 * variances[j] ** 0.5)
                r_clamped = max(-1.0, min(1.0, r))
                r2 = r_clamped * r_clamped

                # Track high LD pairs (r² > 0.5)
                if r2 > 0.5:
                    high_ld_pairs.append((i, j, r2))

                ld_count += 1

                if ld_count % 100000 == 0:
                    elapsed = time.time() - start
                    rate = ld_count / elapsed
                    print(f"  ✓ {ld_count} pairs computed ({rate:.0f} pairs/sec)...")

    ld_time = time.time() - start
    total_time = time.time() - start_overall

    print("")
    print("=" * 64)
    print("[OK] LD MATRIX COMPUTATION COMPLETE!")
    print("=" * 64)
    print("")

    print("📈 LD Statistics:")
    print(f"  Total SNP pairs: {num_variants * (num_variants - 1) // 2}")
    print(f"  Pairs computed: {ld_count}")
    print(f"  High LD pairs (r² > 0.5): {len(high_ld_pairs)}")
    print("")

    print("⏱️  Performance:")
    print(f"  Load time: {load_time:.1f}s")
    print(f"  Statistics: {stats_time:.1f}s")
    print(f"  LD computation: {ld_time:.1f}s")
    print(f"  Total time: {total_time:.1f}s")
    print(f"  Rate: {ld_count / ld_time:.0f} pairs/sec")
    print("")

    # Print top LD pairs
    if high_ld_pairs:
        print("🔗 Top 20 Linked Variant Pairs (highest r²):")
        print("")

        high_ld_pairs.sort(key=lambda x: x[2], reverse=True)

        print(f"{'SNP1':<20} {'SNP2':<20} {'r²':<12} {'Distance (bp)':<20}")
        print("-" * 80)

        for i, (idx1, idx2, r2) in enumerate(high_ld_pairs[:20]):
            dist = abs(int(positions[idx2]) - int(positions[idx1]))
            print(
                f"{variant_ids[idx1][:20]:<20} {variant_ids[idx2][:20]:<20} {r2:<12.4f} {dist:<20}"
            )

        print("")

    # LD decay analysis
    print("📉 LD Decay Analysis:")
    print("")

    ld_by_distance = defaultdict(list)

    for idx1, idx2, r2 in high_ld_pairs:
        dist = (abs(int(positions[idx2]) - int(positions[idx1])) // 10000) * 10000
        ld_by_distance[dist].append(r2)

    distances = sorted(ld_by_distance.keys())

    print(f"{'Distance (bp)':<20} {'Pairs':<15} {'Avg r²':<20}")
    print("-" * 55)

    for dist in distances[:20]:
        values = ld_by_distance[dist]
        avg_r2 = sum(values) / len(values)
        print(f"{dist:<20} {len(values):<15} {avg_r2:<20.4f}")

    print("")
    print("════════════════════════════════════════════════════════════════")
    print("Analysis complete! LD patterns extracted from real 1000G data.")
    print("════════════════════════════════════════════════════════════════")


def print_summary_only(genotypes, variant_ids, positions, num_samples):
    print("📊 Data Summary (--summary mode):")
    print("")
    print(f"  Variants: {len(genotypes)}")
    print(f"  Samples: {num_samples}")
    print(f"  Total genotypes: {len(genotypes) * num_samples}")
    print("")

    # Compute allele frequencies
    allele_freq = []

    for geno in genotypes:
        total = 0.0
        valid = 0.0

        for g in geno:
            if g != 3:
                total += g
                valid += 1.0

        if valid > 0:
            af = total / (valid * 2.0)  # Divide by 2 since each person has 2 alleles
        else:
            af = 0.0

        allele_freq.append(af)

    maf = [min(af, 1.0 - af) for af in allele_freq]

    rare = sum(1 for m in maf if m < 0.001)
    very_rare = sum(1 for m in maf if m < 0.01)
    common = sum(1 for m in maf if m > 0.05)
    intermediate = sum(1 for m in maf if 0.01 <= m <= 0.05)

    print(f"  Common (MAF > 5%): {common}")
    print(f"  Intermediate (1-5%): {intermediate}")
    print(f"  Rare (0.1-1%): {very_rare - rare}")
    print(f"  Very rare (< 0.1%): {rare}")
    print("")

    print("  First 10 variants:")
    for i in range(min(10, len(variant_ids))):
        print(
            f"    {variant_ids[i][:15]} pos={positions[i]} AF={allele_freq[i]:.4f}"
        )


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: compute_ld_matrix.py <input.csv> [--summary] [--sample N]")
        sys.exit(1)

    csv_path = sys.argv[1]
    summary_only = '--summary' in sys.argv
    sample_n = None

    if '--sample' in sys.argv:
        idx = sys.argv.index('--sample')
        if idx + 1 < len(sys.argv):
            try:
                sample_n = int(sys.argv[idx + 1])
            except:
                pass

    compute_ld_matrix(csv_path, summary_only=summary_only, sample_n=sample_n)
