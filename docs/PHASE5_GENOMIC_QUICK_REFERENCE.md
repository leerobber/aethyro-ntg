# NTG Phase 5 Genomic Training: Quick Reference

**One-page checklist for execution** | **4 weeks to doctorate degree**

---

## The Mission

Train NTG engine on GenomicBrain genomic data (869.5k SNPs, 18.5M LD pairs, 3,000 genomes) to prove:
1. Correct data ingest & representation ✓
2. 10x+ compression ✓
3. Parallel scoring ≡ serial ✓
4. Deterministic reproducibility ✓

**Pass bar:** Composite score ≥ 75% on 5 independent runs

---

## 4-Week Timeline

| Week | Goal | Checkpoint | Must Pass |
|------|------|-----------|-----------|
| **1** | Data prep & ingest | 4/8 items | SNPs, LD, genotypes, topo-sort |
| **2** | Correctness & compression | 8/8 items + ≥10x | Bit-identity + compression |
| **3** | Parallel & determinism | Parallel ✓ Determinism ✓ | 95%+ batches match, ledger identical |
| **4** | Doctorate exam | Composite ≥75% | Generate diploma certificate |

---

## 8 Test Items (Each Required)

| Item | Test | Success Criteria |
|------|------|------------------|
| **1** | Load 869.5k SNPs | SNP count == 869,500 |
| **2** | Load 18.5M LD pairs | LD matrix nnz == 18.5M |
| **3** | Load 3,000 genotypes | Genotypes: 3000×869.5k intact |
| **4** | Chromosome topo-sort | All nodes in chr order (1..22) |
| **5** | LD reconstruction | 100 random pairs: exact match |
| **6** | Disease annotation | All 186 loci attached |
| **7** | Pathway coverage | All 23 pathways traversable |
| **8** | Ledger completeness | ≥18.5M entries logged |

**Pass condition:** 8/8 items passing

---

## Composite Score Formula

```
Score = 0.35×ItemPass + 0.25×Compression + 0.20×Parallel + 0.20×Determinism

ItemPass = 8/8 passing = 1.0
Compression = min(1.0, ratio/10.0) [target: ≥10x]
Parallel = 1.0 if batch_serial ≡ batch_parallel else 0.5
Determinism = 1.0 if run2_ledger_hash == run1_ledger_hash else 0.0

Example (all perfect):
Score = 0.35×1.0 + 0.25×1.0 + 0.20×1.0 + 0.20×1.0 = 1.0 (100%)
Result: ✅ PASS

Threshold: ≥ 0.75 (75%)
```

---

## Input Data (GenomicBrain Phase 4)

| File | Size | Items Tested |
|------|------|-------------|
| `snp_metadata.csv` | 8 MB | Item 1 |
| `ld_matrix_18.5m.coo` | 37 MB | Items 2, 5, 8 |
| `genotypes_3000.ternary` | 65 MB | Item 3 |
| `loci_186.csv` | 2 KB | Item 6 |
| `pathways_23.json` | 100 KB | Item 7 |
| **Total** | **~690 MB** | **8 items** |

**Target compression:** ≤ 72 MB (10x ratio)

---

## Build & Run

```bash
# Setup
cd /c/Users/leer4/aethyro-ntg/kernel
cargo build --release --bin ntg_school_genomic

# Execute (all 5 runs)
cargo run --release --bin ntg_school_genomic -- \
  --genomic-data ../data/genomic_data \
  --out ../docs/schooling/runs/genomic \
  --runs 5

# Check results
ls ../docs/schooling/runs/genomic/
# Expected: RUN_0*.md + DIPLOMA_CERTIFICATE.md (if ≥75%)
```

---

## Success Indicators

### Week 1 (Data Prep)
- [ ] Load complete: all 869.5k SNPs + 18.5M LD + 3000 genomes
- [ ] Items 1-4 passing (ingest + topo-sort)
- [ ] No data corruption, checksums valid

### Week 2 (Correctness)
- [ ] Item 5 passing: LD reconstruction 100% bit-identical
- [ ] Items 6-7 passing: all annotations attached
- [ ] Item 8 passing: ≥18.5M ledger entries
- [ ] Compression ≥ 10.0x verified

### Week 3 (Parallelization)
- [ ] Batch_parallel ≡ Batch_serial (100 batch tests)
- [ ] Run 1 ledger hash == Run 2 ledger hash (determinism)
- [ ] All benchmarks logged (latency, throughput)

### Week 4 (Exam)
- [ ] Run 0: Composite ≥ 75% ✅
- [ ] Run 1: Composite ≥ 75% ✅
- [ ] Run 2: Composite ≥ 75% ✅
- [ ] Run 3: Composite ≥ 75% ✅
- [ ] Run 4: Composite ≥ 75% ✅
- [ ] DIPLOMA_CERTIFICATE.md generated ✅

---

## Hard Gates (All Required)

| Gate | Requirement | Consequence of Failure |
|------|-------------|-----|
| **Data ingest** | All 869.5k + 18.5M + 3000 loaded | FAIL week 1 |
| **Bit-identity** | 100% of LD pairs reconstruct exactly | FAIL item 5 |
| **Compression** | ≥ 10.0x on combined LD+genotypes | FAIL composite |
| **Parallel** | batch_serial ≡ batch_parallel | FAIL composite |
| **Determinism** | run2_hash == run1_hash | FAIL composite |
| **Composite** | Score ≥ 75% on 5 runs | REDO (max 5 attempts) |

---

## Failure Mode (If Composite < 75%)

1. **Diagnose:** Which dimension failed?
   - ItemPass < 0.75? → Fix failing test items
   - Compression < 7.5x? → Optimize encoding
   - Parallel < 0.75? → Debug batch divergence
   - Determinism = 0? → Fix random seeds

2. **Re-execute:**
   - Re-run full study phase (all item tests)
   - Re-run full advanced exam (1 schooling pass)
   - Re-compute composite score
   - Increment attempt #N

3. **Stop after:** 5 attempts (hard limit)

---

## Output Artifacts

**Notebooks (per run):**
```
docs/schooling/runs/genomic/
├── RUN_00_NOTEBOOK.md  ← Detailed exam results
├── RUN_01_NOTEBOOK.md
├── ...
├── RUN_04_NOTEBOOK.md
├── DIPLOMA_CERTIFICATE.md  ← Degree (if PASS)
└── DATASET_MANIFEST.md  ← Data inventory
```

**Data artifacts:**
```
artifacts/phase5_genomic/
├── genomic_ntg_model.calib  ← Frozen graph
├── genomic_genotypes.ternary  ← Compressed genotypes
├── genomic_ledger.db  ← Cryptographic ledger
└── metadata.json  ← Training parameters
```

**Reports:**
```
docs/reports/phase5_genomic/
├── compression_report.json
├── performance_report.json
└── correctness_summary.csv
```

---

## Key Metrics to Track

| Metric | Target | Measured | Pass? |
|--------|--------|----------|-------|
| SNPs loaded | 869,500 | [ ] | [ ] |
| LD pairs loaded | 18,500,000 | [ ] | [ ] |
| Genotypes intact | 3,000 × 869.5k | [ ] | [ ] |
| LD bit-identity | 100% | [ ] % | [ ] |
| Compression ratio | ≥ 10.0x | [ ]x | [ ] |
| Parallel speedup | ≥ 4.0x (8 cores) | [ ]x | [ ] |
| Ledger entries | ≥ 18.5M | [ ] | [ ] |
| Determinism | Hash match | [ ] | [ ] |
| Composite score | ≥ 75% | [ ]% | [ ] |

---

## Troubleshooting Quick Fixes

| Problem | Likely Cause | Fix |
|---------|--------------|-----|
| SNP load fails | File missing/corrupt | Verify path, re-download if needed |
| LD reconstruction fails | Sparse encoding wrong | Check COO format, test on golden data first |
| Compression <10x | Sparse matrix too dense | Reduce stored pairs, test threshold |
| Parallel diverges | SIMD rounding difference | Use scalar path for final exam, add epsilon tolerance |
| Ledger not deterministic | Random seed in compute | Lock all RNG seeds before exam |
| Composite <75% on all runs | Multiple dimensions failing | Focus on worst performer (ItemPass usually) |

---

## Phase 5 → Phase 7 Handoff

After Phase 5 PASS (diploma issued):

1. **Freeze model** (`--write-model`)
2. **Archive ledger** (backup cryptographic chain)
3. **Document findings** (what worked, what didn't)
4. **Prepare Phase 7 integration:**
   - Replace GenomicBrain bitslicing with NTG ternary
   - Wire sparse/dense selection
   - Validate 609 integration tests
   - Production deployment

**Timeline:** Phase 7 = 1-2 weeks after Phase 5 PASS

---

## Sign-Off Checklist

- [ ] Framework reviewed and approved
- [ ] Rust build environment ready
- [ ] Genomic data staged and validated
- [ ] Week 1 data ingest beginning
- [ ] Weekly gates defined and understood
- [ ] Failure redo protocol reviewed
- [ ] Success criteria approved
- [ ] Phase 5 PASS = Phase 7 green light

---

## Documentation

| Document | Purpose | Read First? |
|----------|---------|------------|
| **PHASE5_GENOMIC_TRAINING_FRAMEWORK.md** | Complete methodology | ✅ YES |
| **PHASE5_GENOMIC_EXECUTION_STATUS.md** | Week-by-week plan | ✅ YES |
| **PHASE5_GENOMIC_IMPLEMENTATION_SPEC.md** | Technical code reference | Dev only |
| **PHASE5_GENOMIC_EXECUTION_SUMMARY.md** | Executive summary | ✅ YES |
| **PHASE5_GENOMIC_QUICK_REFERENCE.md** | This document | ✅ Quick ref |

---

**STATUS: READY FOR LAUNCH** 🚀

**Start Week 1 when:**
- Rust toolchain verified ✓
- Genomic data staged ✓
- Framework approved ✓

**Contact:** Refer to framework docs for technical questions
**Timeline:** 4 weeks to diploma (or redo for <75% score)

---

**NTG Phase 5 Genomic Training Framework v1.0**  
**Prepared:** 2026-07-16  
**Approval Gate:** Project Lead Sign-Off Required

