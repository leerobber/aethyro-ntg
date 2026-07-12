#!/usr/bin/env python3
"""
Compute LD (Linkage Disequilibrium) Matrix from 1000 Genomes CSV
Shows how genetic variants correlate across the population
"""

import csv
import sys
import time
from collections import defaultdict

def compute_ld_matrix(csv_path):
    print("=" * 64)
    print("COMPUTING: Linkage Disequilibrium (LD) Matrix")
    print("=" * 64)
    print("")
    print("Input: " + csv_path)
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
        header = next(reader)
        num_samples = len(header) - 2
        print("[OK] Found " + str(num_samples) + " samples")

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

            if len(genotypes) % 10000 == 0:
                print("  [OK] Loaded " + str(len(genotypes)) + " variants...")

    num_variants = len(genotypes)
    load_time = time.time() - start

    print("[OK] Loaded " + str(num_variants) + " variants in " + str(round(load_time, 1)) + "s")
    print("  Memory: ~" + str(round((num_variants * num_samples) / 1_000_000, 1)) + " MB")
    print("")

    # Compute statistics
    print("[*] Computing statistics...")
    start = time.time()

    means = [0.0] * num_variants
    variances = [0.0] * num_variants

    for v, geno in enumerate(genotypes):
        total = 0.0
        sum2 = 0.0
        valid = 0.0

        for g in geno:
            if g != 3:
                total += g
                sum2 += g * g
                valid += 1.0

        if valid > 0:
            means[v] = total / valid
            variances[v] = (sum2 / valid) - (means[v] * means[v])

    stats_time = time.time() - start
    print("[OK] Statistics computed in " + str(round(stats_time, 1)) + "s")
    print("")

    # Compute LD matrix
    print("[*] Computing LD matrix (" + str(num_variants) + " SNPs)...")
    print("  This will compute " + str(num_variants * (num_variants - 1) // 2) + " pairwise correlations")
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

                if r2 > 0.5:
                    high_ld_pairs.append((i, j, r2))

                ld_count += 1

                if ld_count % 100000 == 0:
                    elapsed = time.time() - start
                    rate = ld_count / elapsed
                    print("  [OK] " + str(ld_count) + " pairs computed (" + str(int(rate)) + " pairs/sec)...")

    ld_time = time.time() - start
    total_time = time.time() - start_overall

    print("")
    print("=" * 64)
    print("[OK] LD MATRIX COMPUTATION COMPLETE!")
    print("=" * 64)
    print("")

    print("[STATS] LD Statistics:")
    print("  Total SNP pairs: " + str(num_variants * (num_variants - 1) // 2))
    print("  Pairs computed: " + str(ld_count))
    print("  High LD pairs (r^2 > 0.5): " + str(len(high_ld_pairs)))
    print("")

    print("[TIME] Performance:")
    print("  Load time: " + str(round(load_time, 1)) + "s")
    print("  Statistics: " + str(round(stats_time, 1)) + "s")
    print("  LD computation: " + str(round(ld_time, 1)) + "s")
    print("  Total time: " + str(round(total_time, 1)) + "s")
    print("  Rate: " + str(int(ld_count / ld_time)) + " pairs/sec")
    print("")

    # Print top LD pairs
    if high_ld_pairs:
        print("[RESULTS] Top 20 Linked Variant Pairs (highest r^2):")
        print("")

        high_ld_pairs.sort(key=lambda x: x[2], reverse=True)

        print("{:<20} {:<20} {:<12} {:<20}".format("SNP1", "SNP2", "r^2", "Distance (bp)"))
        print("-" * 80)

        for i, (idx1, idx2, r2) in enumerate(high_ld_pairs[:20]):
            dist = abs(int(positions[idx2]) - int(positions[idx1]))
            print("{:<20} {:<20} {:<12.4f} {:<20}".format(
                variant_ids[idx1][:20],
                variant_ids[idx2][:20],
                r2,
                str(dist)
            ))

        print("")

    # LD decay analysis
    print("[ANALYSIS] LD Decay:")
    print("")

    ld_by_distance = defaultdict(list)

    for idx1, idx2, r2 in high_ld_pairs:
        dist = (abs(int(positions[idx2]) - int(positions[idx1])) // 10000) * 10000
        ld_by_distance[dist].append(r2)

    distances = sorted(ld_by_distance.keys())

    print("{:<20} {:<15} {:<20}".format("Distance (bp)", "Pairs", "Avg r^2"))
    print("-" * 55)

    for dist in distances[:20]:
        values = ld_by_distance[dist]
        avg_r2 = sum(values) / len(values)
        print("{:<20} {:<15} {:<20.4f}".format(dist, len(values), avg_r2))

    print("")
    print("=" * 64)
    print("Analysis complete! LD patterns extracted from real 1000G data.")
    print("=" * 64)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: compute_ld_simple.py <input.csv>")
        sys.exit(1)

    csv_path = sys.argv[1]
    compute_ld_matrix(csv_path)
