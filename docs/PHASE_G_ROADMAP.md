# Phase G: Real Genomic Workload Deployment

**Status:** In Progress  
**Target Completion:** 2026-08-15  
**Owner:** Claude (Aethyro-NTG)

---

## Goal

Deploy and validate the complete NTG engine against real 1000 Genomes data:
- Load real VCF genomic data (100K+ SNPs, multi-population)
- Execute Phase 4 calibration on actual genotypes
- Run LD computation at scale and benchmark
- Profile performance with real workload
- Validate self-awareness telemetry instrumentation (Phase F integration)

**Success criteria:**
- ✅ 100K+ SNP LD computation completes in <5 minutes (RTX 5050 GPU)
- ✅ Calibration model roundtrip accuracy ≥ 95%
- ✅ Telemetry flow validates without errors
- ✅ Performance profiling identifies bottlenecks vs. benchmarks

---

## Implementation Phases

### G.1: VCF Data Preparation (Day 1)
**Goal:** Acquire and prepare real 1000 Genomes data subset.

#### Tasks
1. **Data acquisition**
   - Source: 1000 Genomes Project (phase3_shapeit2_mvncall_integrated)
   - Target: 5 populations, 500-1000 samples, chr21 (smallest autosome, ~50M bp)
   - Size: ~100-500 MB compressed
   - Download: `ftp://ftp.1000genomes.ebi.ac.uk/vol1/ftp/release/20130502/`

2. **VCF preprocessing**
   - Extract SNPs only (biallelic, MAF > 0.01)
   - Target: 10K-50K SNPs per population (realistic LD computation size)
   - Format: BGZIP + tabix index
   - Tool: bcftools or vcftools

3. **Validation**
   - Check for missing data
   - Verify allele frequency distribution
   - Generate summary statistics

#### Deliverables
- `data/1kg_chr21_subset.vcf.gz` (indexed)
- VCF validation report
- SNP/sample counts per population

---

### G.2: Phase 4 Calibration on Real Data (Day 2)
**Goal:** Validate model calibration with actual genomic data.

#### Tasks
1. **Load real VCF data**
   - Use kernel VcfParser with 1KG subset
   - Generate BitstreamGenotypes
   - Measure parsing performance

2. **Run phase4_calib binary**
   - Input: Real genotype matrix
   - Output: Calibrated model parameters
   - Validate: Model roundtrip (encode → matmul → decode)

3. **Benchmark calibration**
   - Time per SNP
   - Memory usage
   - Compare to synthetic data (Phase 4 baseline)

4. **Quality assurance**
   - Verify numeric stability (no NaNs/Infs)
   - Check model parameter ranges
   - Validate against known reference (if available)

#### Deliverables
- Calibration results on 1KG data
- Performance comparison (synthetic vs. real)
- Model stability report

---

### G.3: LD Computation at Scale (Day 2-3)
**Goal:** Execute large-scale LD computation and validate results.

#### Tasks
1. **Single-population LD computation**
   - Population: CEU (Utah Europeans, ~99 samples)
   - SNPs: 10K-50K
   - Algorithm: Pearson r² via LdComputer
   - Threshold: 0.5 (standard practice)

2. **Benchmark LD computation**
   - Wall-clock time
   - Peak memory usage
   - Comparisons per second (cache efficiency)
   - GPU vs. CPU (if CUDA enabled)

3. **Cross-population LD comparison**
   - Run LD on 2-3 additional populations
   - Compare LD patterns (should differ by population structure)
   - Validate genetic architecture preservation

4. **Validation against known LD**
   - Download reference LD (1KG Project LD calculations)
   - Correlation of r² values (Pearson/Spearman)
   - Sensitivity/specificity at threshold

#### Deliverables
- LD matrix for each population
- Performance metrics (time, memory, throughput)
- Validation report vs. reference LD
- Identified bottlenecks for Phase 7.5.6 optimization

---

### G.4: Performance Profiling & Optimization (Day 3-4)
**Goal:** Profile real workload and implement targeted optimizations.

#### Tasks
1. **Profile with real data**
   - Run `cargo bench --bench bench_hot_paths` on 1KG data
   - Flamegraph profiling of LD computation
   - Identify actual bottlenecks (vs. synthetic data)

2. **Optimization implementation (if time permits)**
   - Parallel LD computation (rayon)
   - Memory allocator tuning (jemalloc vs. default)
   - SIMD dispatch optimization

3. **Re-benchmark**
   - Measure improvement vs. baseline
   - Target: 20-30% reduction in LD computation time

#### Deliverables
- Flamegraph report
- Optimization recommendations
- Benchmark results (before/after)

---

### G.5: Telemetry & Self-Awareness Validation (Day 4-5)
**Goal:** Validate Phase F telemetry instrumentation end-to-end.

#### Tasks
1. **Self-awareness instrumentation**
   - Enable kernel telemetry module (ADR 0011)
   - Log LD computation stages
   - Capture performance metrics

2. **Telemetry serialization**
   - Generate TelemetryPayload structs
   - Serialize to JSON
   - Validate schema compliance

3. **Hostframe integration (mock)**
   - Simulate Hostframe backend (HTTP mock)
   - Test ingestion flow
   - Verify telemetry persistence

4. **End-to-end validation**
   - Process 100K SNP LD computation with telemetry
   - Verify no data loss
   - Confirm performance impact < 5%

#### Deliverables
- Telemetry validation report
- Mock Hostframe integration test
- Readiness for Phase F GCP deployment

---

## Timeline & Dependencies

```
G.1: Data Preparation       ━━━  Day 1 (2026-08-06)
G.2: Phase 4 Calibration    ━━━  Day 2 (2026-08-07)
G.3: LD Computation         ━━━  Day 2-3 (2026-08-07-08)
  └─ Benchmarking           ━━   (parallel)
G.4: Profiling & Opt        ━━━  Day 3-4 (2026-08-08-09)
G.5: Telemetry Validation   ━━━  Day 4-5 (2026-08-09-10)
```

### Blockers & Assumptions
- **Assumption:** 1000 Genomes FTP access available (public data)
- **Assumption:** Local GPU available on tatortot for LD computation
- **Blocker:** If 1KG access unavailable, use simulated 1M SNP dataset instead

---

## Success Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| VCF parsing | <100 ms/1K SNPs | Time LD computation start |
| Phase 4 calibration | <1 sec on 50K SNPs | Measure phase4_calib runtime |
| LD computation (100K SNPs) | <5 min (GPU) / <30 min (CPU) | Benchmark with perf tools |
| LD validation | r² correlation ≥ 0.95 vs. ref | Pearson correlation test |
| Telemetry ingestion | 0 data loss | Mock backend test |
| Performance impact | Telemetry overhead < 5% | Compare with/without telemetry |

---

## Deliverables Summary

1. **Real genomic workload validation report**
   - VCF parsing performance
   - Phase 4 calibration results
   - LD computation benchmarks
   - Cross-population LD patterns

2. **Performance profile & optimization roadmap**
   - Flamegraph analysis
   - Identified bottlenecks
   - Recommended optimizations for Phase 8

3. **Telemetry integration readiness**
   - End-to-end validation
   - Mock Hostframe test passing
   - Readiness for Phase F GCP deployment

4. **Updated Phase 7.5.6 benchmarks**
   - Actual performance on 1KG data
   - Comparison vs. synthetic benchmarks

---

## Phase F Integration Checkpoints

After Phase G, Phase F (Hostframe GCP backend) can proceed with:
- ✅ Real telemetry data from 1KG workload
- ✅ Performance baselines for BigQuery schema design
- ✅ End-to-end flow tested (local mock backend)
- ✅ Ready for GCP Cloud Run deployment

---

## Notes

- **Data retention:** 1KG subset (~200 MB) can be cached locally; consider cleanup after Phase G
- **Reproducibility:** All runs use seed-based PRNG for deterministic results
- **Documentation:** Each step produces README + validation report for reproducibility
- **Collaboration:** Phase G validates system readiness for multi-agent deployment (Phase 8+)

---

## References

- **Phase 4:** Calibration (docs/PHASE_4_ROADMAP.md)
- **Phase 7.5.6:** Performance Profiling (docs/PHASE_7_5_ROADMAP.md)
- **Phase F:** Hostframe Backend (docs/PHASE_F_ROADMAP.md)
- **ADR 0011:** Self-Awareness Instrumentation (docs/ADR_0011.md)
- **1000 Genomes:** https://www.internationalgenome.org/
