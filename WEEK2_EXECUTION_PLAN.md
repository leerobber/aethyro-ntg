# Week 2 Execution Plan: Chr1-3 Complete Analysis + Parallel Download

**Start Date**: 2026-07-12 10:50 UTC  
**Status**: Week 1 Complete, Week 2 Initiated  
**Target Completion**: 2026-07-19 (7 days)  

---

## Current Status (as of Now)

### Completed in Week 1:
✅ GenomicBrain architecture (3 Rust binaries compiled)
✅ End-to-end pipeline validated (load → train → synthesis)
✅ Partial chr1 data: 111,860 SNPs × 2,504 samples (535 MB)
✅ Proof-of-concept metrics:
  - LD computation: 40,479 high-LD pairs (7.3s)
  - KAIROS training: Converged in 2 cycles (19.2s total)
  - Haplotype blocks: 272 identified (BFS-based clustering)

### In Progress:
⏳ Chr2 VCF download (1.08 GB, started)
⏳ Chr3 VCF download (0.87 GB, started)

### Data Status:
```
Chr1: 111K variants (partial)   → Will expand to 4.3M (full)
Chr2: Downloading...              → Target: 4.2M variants, 1.08 GB
Chr3: Downloading...              → Target: 3.4M variants, 0.87 GB
```

---

## Week 2 Detailed Timeline

### Day 1 (Today): Parallel Download Initiation ⏱️

**Time Estimate: 60-90 minutes (network bandwidth limited)**

#### Task 1.1: Queue Parallel Downloads
- **Status**: ✅ In progress (curl background processes)
- **Target Files**:
  - `data/raw/1000g/ALL.chr2.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz` (1.08 GB)
  - `data/raw/1000g/ALL.chr3.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz` (0.87 GB)
- **Expected Completion**: 60-90 min (concurrent FTP downloads)
- **Bandwidth**: ~20 MB/s typical for 1000G FTP
  - Chr2: 1080 MB ÷ 20 MB/s = 54 min
  - Chr3: 870 MB ÷ 20 MB/s = 44 min
  - Parallel overhead: 15-20 min (connection setup, variability)
  - **Total ETA**: 70-75 min

#### Task 1.2: Expand Chr1 to Full Conversion
- **Status**: Ready to start (111K → 4.3M variants)
- **Current**: 535 MB CSV (111,860 variants up to position 3.7M)
- **Full chr1 VCF**: 1.1 GB (4.3M variants)
- **Approach**: 
  - Delete current partial chr1.csv
  - Re-run vcf_to_csv.py on full VCF (Unicode fixes verified)
  - Expected output: ~2.1 GB CSV
- **Time**: 30-60 min (VCF parsing + genotype encoding)
- **Parallelization**: Can start immediately (while chr2/chr3 download)
- **Priority**: HIGH (needed for LD computation)

#### Task 1.3: Validation Checkpoint (End of Day 1)
```
Expected by 14:00 UTC:
  ✅ Chr2 VCF downloaded (1.08 GB, checksum verified)
  ✅ Chr3 VCF downloaded (0.87 GB, checksum verified)
  ✅ Chr1 CSV expanded (2.1 GB, 4.3M variants + header)
  
Validation:
  - chr1.csv: head -2, tail -2, wc -l
  - chr2.vcf.gz: gunzip test (no corruption)
  - chr3.vcf.gz: gunzip test (no corruption)
  - All files in data/raw/1000g/ and data/processed/
```

---

### Day 2-3: Conversion & LD Computation (72 hours)

**Time Estimate: 6-8 hours total (4 processes in queue)**

#### Task 2.1: Convert Chr2 to CSV
- **Input**: `ALL.chr2.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz` (1.08 GB)
- **Process**: `python tools/vcf_to_csv.py chr2.vcf.gz -o chr2.csv`
- **Expected Output**: ~2.0 GB CSV, 4.2M SNPs, 2,504 samples
- **Time**: 30-45 min
- **Parallelization**: Can run simultaneously with chr1 LD computation
- **Start**: Right after chr1 VCF downloaded

#### Task 2.2: Compute LD for Chr1 (Full)
- **Input**: `data/processed/1000g_chr1.csv` (2.1 GB, 4.3M SNPs)
- **Process**: `./compute_ld_fast.exe chr1.csv`
- **Output**: 
  - Top 20 linked pairs (stdout)
  - LD decay analysis
  - Summary statistics
- **Expected Results**:
  - Pairs checked: 9.2B (4.3M × 4.3M / 2)
  - High-LD pairs (r² > 0.5): 1.2M - 1.5M
  - Mean r² (high-LD): 0.74
  - LD decay: r² = 0.5 at ~45-50 kb distance
- **Time**: 15-20 min (201k pairs/sec rate)
- **Memory**: 500 MB - 1 GB (streaming mode)
- **Parallelization**: Can run while chr2 converts

#### Task 2.3: Convert Chr3 to CSV
- **Input**: `ALL.chr3.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz` (0.87 GB)
- **Process**: `python tools/vcf_to_csv.py chr3.vcf.gz -o chr3.csv`
- **Expected Output**: ~1.65 GB CSV, 3.4M SNPs
- **Time**: 25-35 min
- **Parallelization**: Queue after chr2 starts

#### Task 2.4: Compute LD for Chr2
- **Input**: `data/processed/1000g_chr2.csv` (2.0 GB, 4.2M SNPs)
- **Process**: `./compute_ld_fast.exe chr2.csv`
- **Expected Results**:
  - High-LD pairs (r² > 0.5): 1.1M - 1.4M
  - Mean r²: 0.73
  - LD decay: similar to chr1 (human chromosomes have similar recombination rates)
- **Time**: 14-18 min
- **Start**: After chr2 CSV ready

#### Task 2.5: Compute LD for Chr3
- **Input**: `data/processed/1000g_chr3.csv` (1.65 GB, 3.4M SNPs)
- **Process**: `./compute_ld_fast.exe chr3.csv`
- **Expected Results**:
  - High-LD pairs (r² > 0.5): 900K - 1.1M
  - Mean r²: 0.72
- **Time**: 12-15 min
- **Start**: After chr3 CSV ready

#### Task 2.6: End-of-Day Checkpoint
```
Expected by end of Day 3 (Friday 20:00 UTC):
  ✅ Chr1 LD matrix computed (1.3M high-LD pairs)
  ✅ Chr2 LD matrix computed (1.2M high-LD pairs)
  ✅ Chr3 LD matrix computed (1.0M high-LD pairs)
  ✅ All metrics logged in summary files
  
Total high-LD pairs (chr1-3): 3.5M pairs
Total genotypes analyzed: 12M SNPs × 2,504 samples
```

---

### Day 4-5: GenomicBrain Training (48 hours)

**Time Estimate: 6-8 hours total (3 independent trains)**

#### Task 3.1: Load & Train GenomicBrain for Chr1
- **Input**: 
  - CSV: `1000g_chr1.csv` (2.1 GB, 4.3M SNPs)
  - LD data: 1.3M high-LD pairs (computed in Day 2)
- **Process**:
  ```
  # Step 1: Load CSV into brain structure
  ./load_brain_from_csv.exe chr1.csv
  Expected: 4.3M neurons, ~1.3M synapses, ~8K-10K haplotype blocks
  Time: 8-12 min
  
  # Step 2: Train KAIROS cycles
  ./train_genomic_brain.exe chr1.csv 5 --population ALL
  Expected: 5 cycles × ~13 sec/cycle = 65-90 sec per cycle
  Total: 6-8 min (will converge by cycle 2-3, early stopping)
  
  # Step 3: Checkpoint & metrics
  Time: 2 min
  ```
- **Total Time**: 18-25 min per chromosome
- **Output**:
  - Brain checkpoint: `data/checkpoints/brain_chr1.bin`
  - Metrics log: Loss, LD_mean, connectivity, convergence cycle
  - Haplotype blocks: 8K-10K annotated blocks
- **Parallelization**: Run chr1, chr2, chr3 sequentially (CPU bound)
  - But! Can interleave with other tasks
  - While chr1 trains, chr2 can compute LD if not yet started

#### Task 3.2: Load & Train GenomicBrain for Chr2
- **Estimated Results**:
  - Neurons: 4.2M
  - Synapses: ~1.2M
  - Haplotype blocks: ~7.8K
  - KAIROS cycles: 5 (converge by 2-3)
  - Checkpoint: `data/checkpoints/brain_chr2.bin`
- **Time**: 18-25 min
- **Start**: After chr1 training completes

#### Task 3.3: Load & Train GenomicBrain for Chr3
- **Estimated Results**:
  - Neurons: 3.4M
  - Synapses: ~1.0M
  - Haplotype blocks: ~6.2K
  - KAIROS cycles: 5
  - Checkpoint: `data/checkpoints/brain_chr3.bin`
- **Time**: 16-20 min
- **Start**: After chr2 training completes

#### Task 3.4: End-of-Day Checkpoint
```
Expected by end of Day 5 (Saturday 20:00 UTC):
  ✅ Chr1 GenomicBrain trained & checkpointed
  ✅ Chr2 GenomicBrain trained & checkpointed
  ✅ Chr3 GenomicBrain trained & checkpointed
  
  Brain Statistics Summary:
  └─ Total neurons (SNPs): 12M
  └─ Total synapses (LD edges): 3.5M
  └─ Total haplotype blocks: ~22K
  └─ Convergence: All chr reached stable loss by cycle 2-3
```

---

### Day 6-7: Synthesis & Validation (48 hours)

**Time Estimate: 4-6 hours total**

#### Task 4.1: Synthetic Genome Generation (100 individuals per chromosome)

**Process**: For each chromosome, sample from learned distribution
```
for each of 100 individuals:
  for each of chr1, chr2, chr3:
    1. Sample ancestry (CEU=10, YRI=10, EAS=10, SAS=10, AMR=10, ..., mixed=50)
    2. Initialize haplotypes per block
    3. Propagate through synaptic connections (LD-based coherence)
    4. Write VCF-format synthetic genotype
    
Output: 300 synthetic individuals (100 × 3 chromosomes)
```

**Time per chromosome**:
- Load checkpoint: 2 min
- Sample 100 individuals: 15-20 min (parallel: 8 at a time)
- Validation: 5 min
- **Per chromosome**: 25-30 min
- **All 3 chromosomes**: 75-90 min (sequential) or 25-30 min (parallel)

#### Task 4.2: Quality Control - Allele Frequency Check
```
For each chromosome:
  Compute AF(synthetic) vs AF(empirical)
  
Expected result:
  - Common SNPs (AF > 5%): match within 0.5%
  - Low-frequency (0.1% < AF < 5%): match within 1-2%
  - Rare (AF < 0.1%): expected 80% of empirical count
  
Success criteria: max AF deviation < 1.5% for common variants
```

#### Task 4.3: LD Preservation Check
```
Compute r² on synthetic genomes
Compare r² distribution to empirical
  
Expected:
  Correlation(r²_synthetic, r²_empirical) > 0.85
  Max deviation in LD decay curve < 0.05 (in r² units)
```

#### Task 4.4: Haplotype Block Coherence
```
For each synthetic individual:
  For each haplotype block:
    Compute mean r² within block (should be high)
    Check block boundaries (should have LD drop)
    
Expected:
  Mean r² within block > 0.7
  Mean r² across boundary < 0.2
```

#### Task 4.5: Complex Trait PRS Calculation
```
For each synthetic individual:
  Compute Polygenic Risk Scores:
  - T2D (Type 2 Diabetes)
  - CAD (Coronary Artery Disease)
  - Height
  
Compare PRS distributions:
  Synthetic vs empirical
  
Expected:
  Correlation of PRS scores > 0.95
  Distribution shapes similar (KS test p > 0.05)
```

#### Task 4.6: End-of-Week Checkpoint
```
Expected by end of Day 7 (Sunday 20:00 UTC):
  ✅ 100 synthetic individuals per chromosome (300 total)
  ✅ All QC checks passed (AF, LD, blocks, PRS)
  ✅ Final report with fitness metrics
  
Fitness Comparison Report:
  Metric                | Empirical | Synthetic v1 | Δ
  ─────────────────────┼───────────┼──────────────┼──────
  Nucleotide diversity  | 0.001234  | 0.001385     | +12.2%
  Rare alleles (AF<1%)  | 1,840,000 | 1,650,000    | -10.3%
  Heterozygosity (He)   | 0.328     | 0.341        | +4.0%
  T2D PRS μ             | 0.000     | -0.18        | -18 SD
  CAD PRS μ             | 0.000     | -0.12        | -12 SD
  Disease risk (T2D)    | 8.2%      | 6.5%         | -20.7%
```

---

## Parallel Execution Strategy

### Task Queue (Optimized for Wall-Clock Time)

```
Timeline (Wall-Clock View):

START (Now: ~10:50 UTC)
│
├─ [PARALLEL] Download chr2 + chr3
│  ├─ Chr2 FTP: 1.08 GB @ 20 MB/s = 54 min → 11:45 UTC
│  └─ Chr3 FTP: 0.87 GB @ 20 MB/s = 44 min → 11:35 UTC
│
├─ [PARALLEL] Convert chr1 (full) + Chr2 download
│  ├─ Chr1 CSV convert: 45 min (starts now)
│  └─ Complete: 11:35 UTC
│
├─ [PARALLEL] LD compute chr1 + chr2/3 conversions
│  ├─ Chr1 LD: 18 min (starts at 11:35 UTC)
│  ├─ Chr2 CSV: 40 min (starts at 11:45 UTC)
│  ├─ Chr3 CSV: 30 min (starts at 12:25 UTC)
│  └─ All complete: 12:55 UTC
│
├─ [PARALLEL] LD compute chr2+3 + chr1 training
│  ├─ Chr2 LD: 16 min (starts 12:25 UTC)
│  ├─ Chr3 LD: 14 min (starts 12:55 UTC)
│  ├─ Chr1 train: 22 min (starts 11:53 UTC)
│  └─ All complete: 13:45 UTC
│
├─ [QUEUE] Train chr2 + chr3
│  ├─ Chr2 train: 22 min (starts 13:45 UTC)
│  ├─ Chr3 train: 20 min (starts 14:07 UTC)
│  └─ All complete: 14:27 UTC
│
└─ [SYNTHESIS + QC] Generate 300 synthetic genomes
   ├─ Parallel synthesis: 30 min
   ├─ Validation checks: 60 min
   └─ Final report: 15 min
   
   TOTAL WALL-CLOCK: ~3-4 hours
   SEQUENTIAL TIME: ~18-20 hours
   SPEEDUP: 4.5-6×
```

### Resource Allocation

```
CPU Cores (8 available):
  ├─ Conversion (single-threaded, sequential): 1 core
  ├─ LD computation (parallel): 4 cores (process N chr in parallel)
  ├─ KAIROS training (single-threaded): 1 core
  └─ Synthesis (parallel): 2-4 cores

I/O Bandwidth (500 MB/s SSD):
  ├─ CSV reads (streaming): 50-100 MB/s per process
  ├─ LD matrix write: 20 MB/s
  └─ Checkpoint writes: 10 MB/s

Memory (32 GB available):
  ├─ Chr1 CSV in memory: 2.1 GB
  ├─ LD computation (streaming): 500 MB
  ├─ GenomicBrain checkpoint: 200 MB
  ├─ Synthesis (1000 genomes): 4 GB
  └─ Headroom: ~20 GB (safe)
```

---

## Expected Final Results (End of Week 2)

### Data Summary
```
Downloaded & Processed:
  ✅ Chr1: 4.3M SNPs × 2,504 samples = 2.1 GB CSV
  ✅ Chr2: 4.2M SNPs × 2,504 samples = 2.0 GB CSV
  ✅ Chr3: 3.4M SNPs × 2,504 samples = 1.65 GB CSV
  ────────────────────────────────────────────────
  Total: 11.8M SNPs, 5.75 GB data
```

### LD Network
```
High-LD Pairs (r² > 0.5):
  ├─ Chr1: 1.3M pairs (0.03% of all possible pairs)
  ├─ Chr2: 1.2M pairs (0.03% of all possible pairs)
  ├─ Chr3: 1.0M pairs (0.03% of all possible pairs)
  ─────────────────────────────────
  Total: 3.5M synaptic connections
  
Haplotype Blocks:
  ├─ Chr1: ~8.5K blocks (mean 507 SNPs/block)
  ├─ Chr2: ~7.8K blocks (mean 539 SNPs/block)
  ├─ Chr3: ~6.2K blocks (mean 548 SNPs/block)
  ─────────────────────────────────
  Total: ~22.5K memory modules
  
LD Decay Characteristics:
  All chromosomes show similar distance-dependent decay:
  r² = 0.8 @ ~5 kb
  r² = 0.5 @ ~45 kb
  r² = 0.2 @ ~350 kb
```

### GenomicBrain Networks
```
3 Trained Neural Networks:

Chr1 Network:
  │ Neurons: 4.3M (one per SNP)
  │ Synapses: 1.3M (LD-weighted connections)
  │ Modules: 8.5K (haplotype blocks)
  │ Convergence: Cycle 2 (loss stable)
  │ Checkpoint: ~/data/checkpoints/brain_chr1.bin
  │
Chr2 Network:
  │ Neurons: 4.2M
  │ Synapses: 1.2M
  │ Modules: 7.8K
  │ Convergence: Cycle 2
  │ Checkpoint: ~/data/checkpoints/brain_chr2.bin
  │
Chr3 Network:
  │ Neurons: 3.4M
  │ Synapses: 1.0M
  │ Modules: 6.2K
  │ Convergence: Cycle 2
  └─ Checkpoint: ~/data/checkpoints/brain_chr3.bin

Total Network Size: 11.8M neurons, 3.5M synapses
```

### Synthetic Genomes (300 individuals)

**Population Breakdown** (100 per chromosome):
```
Ancestry Distribution:
  10 CEU (Northern European)
  10 YRI (Yoruba)
  10 EAS (East Asian)
  10 SAS (South Asian)
  10 AMR (Admixed American)
  50 Mixed ancestry (random blends)
```

**Fitness Metrics**:
```
Diversity:
  ├─ Nucleotide diversity (π): +12-15% vs natural
  ├─ Segregating sites (θ_S): +8-10% vs natural
  └─ Haplotype diversity: +5-8% vs natural

Disease Load:
  ├─ T2D risk: -20.7% (8.2% → 6.5%)
  ├─ CAD risk: -16.4% (3.1% → 2.6%)
  └─ Rare deleterious variants: -10-15%

Trait Distribution:
  ├─ Height PRS: μ=0 (preserved from training data)
  ├─ BMI PRS: μ=0 (preserved)
  └─ IQ proxy PRS: μ=0 (preserved)

Population Structure:
  ├─ PCA: Synthetic samples cluster within empirical populations
  ├─ FST (synthetic vs empirical): < 0.02 (very similar)
  └─ Admixture inference: Accurate ancestry proportions
```

---

## Success Criteria (Week 2 Completion)

### Technical Success:
- [ ] All 3 chromosomes (chr1-3) have VCF files downloaded
- [ ] All 3 converted to CSV format (no corruption, correct genotypes)
- [ ] LD matrices computed with expected statistics (1-1.5M pairs per chr)
- [ ] 3 GenomicBrain networks trained to convergence
- [ ] All 3 checkpoints saved without error
- [ ] 300 synthetic genomes generated (100 per chr)

### Validation Success:
- [ ] Allele frequencies match empirical (AF deviation < 1.5%)
- [ ] LD preservation: r² correlation > 0.85
- [ ] Haplotype block coherence verified (within vs. across-block LD)
- [ ] PRS distributions match expected (KS test p > 0.05)
- [ ] No novel variants created (all AF > 0)
- [ ] No extreme genotypes (no off-scale PRS values)

### Scientific Success:
- [ ] Synthetic genomes show improved fitness metrics (diversity +10-20%)
- [ ] Disease load reduced (T2D risk -20-30%)
- [ ] Population structure preserved (FST < 0.02)
- [ ] Reproducibility: All commands logged, inputs/outputs versioned
- [ ] Scientific documentation complete (roadmap updated)

### Computational Success:
- [ ] Wall-clock time: < 5 hours (parallelization effective)
- [ ] Peak memory: < 8 GB (efficient streaming)
- [ ] No crashes, all tasks completed
- [ ] Checkpoints recoverable (can resume if interrupted)

---

## Risk Mitigation & Contingencies

### Potential Issues & Resolutions:

**Issue 1**: Download failures (network timeout)
- **Mitigation**: Use GNU Parallel or nohup for robust retries
- **Fallback**: wget with resume capability (--continue flag)

**Issue 2**: CSV conversion stalls (memory pressure)
- **Mitigation**: Monitor RAM during conversion
- **Fallback**: Reduce batch size in Python (process 1M SNPs at a time)

**Issue 3**: LD computation exceeds time budget
- **Mitigation**: Use pre-computed LD if available
- **Fallback**: Reduce window size (r² > 0.6 instead of 0.5)

**Issue 4**: KAIROS training doesn't converge
- **Mitigation**: Reduce learning rate (0.005 instead of 0.01)
- **Fallback**: Increase max cycles (10 instead of 5)

**Issue 5**: Synthetic genomes fail QC
- **Mitigation**: Debug LD propagation in activation spreading
- **Fallback**: Use empirical haplotype blocks directly (no synthesis)

---

## Next Checkpoint (Week 3 Preview)

Once Week 2 completes:
- Download chr4-22 (20 chromosomes)
- Queue all 20 in parallel training
- Expected: 22 complete networks (all autosomes)
- Total synthetic individuals: 1000 (100 per chromosome × 10 random seeds)
- Fitness optimization: Personalization framework

---

## Execution Log Template

For tracking actual progress:

```markdown
# Week 2 Execution Log

## Day 1
- [ ] 10:50 UTC: Download chr2 + chr3 started
- [ ] 11:45 UTC: Chr2 download complete (verify size: 1.08 GB)
- [ ] 11:35 UTC: Chr3 download complete (verify size: 0.87 GB)
- [ ] 12:00 UTC: Chr1 full conversion started
- [ ] 12:45 UTC: Chr1 CSV complete (verify size: 2.1 GB, 4.3M lines)

## Day 2
- [ ] 13:00 UTC: Chr1 LD computation started
- [ ] 13:18 UTC: Chr1 LD complete (log high-LD pairs, mean r²)
- [ ] 13:20 UTC: Chr2 CSV conversion started
- [ ] 14:00 UTC: Chr2 CSV complete
- [ ] 14:05 UTC: Chr2 LD computation started
- [ ] 14:20 UTC: Chr2 LD complete

## Day 3
- [ ] 14:25 UTC: Chr3 CSV conversion started
- [ ] 14:55 UTC: Chr3 CSV complete
- [ ] 15:00 UTC: Chr3 LD computation started
- [ ] 15:14 UTC: Chr3 LD complete

[Continue for training, synthesis, QC...]
```

---

**Status**: Ready to Execute  
**Estimated Completion**: 2026-07-19 (Sunday, 20:00 UTC)  
**Contact**: Monitor task output files in `/tasks/` directory
