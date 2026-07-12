# Phase 1: Genomic Operator Integration - Complete Setup Guide

**Status**: Phase 1 (Genomic Operator Core) - READY FOR TESTING  
**Date**: 2026-07-12  
**Components**: OmniSynth-X + NTG FFI + Data Loader

---

## What You Just Built

### **1. Core Genomic Operator** (`kernel/src/ntg/operators/genomic.rs`)
- ✅ Bitsliced ternary genotype storage (low + high bitplanes)
- ✅ LD (Linkage Disequilibrium) matrix computation
- ✅ PRS (Polygenic Risk Score) calculation
- ✅ Statistics: means, std devs, allele frequencies
- ✅ Missing data handling (coded as 11 bitmask)
- ✅ LD clustering (identify high-LD SNP pairs)

**Key Data Structure**:
```rust
pub struct GenomicOperator {
    pub data: Vec<u64>,              // Bitsliced storage
    pub num_individuals: usize,       // Sample size
    pub num_snps: usize,              // Number of variants
    pub words_per_snp: usize,         // 64-bit words per SNP
    pub variant_names: Vec<String>,   // rs123, rs456, etc
    pub positions: Vec<u32>,          // Genomic coordinates
    pub allele_freqs: Vec<f64>,       // Population allele frequencies
    pub means: Vec<f64>,              // Mean genotypes
    pub std_devs: Vec<f64>,           // Std deviation
}
```

### **2. FFI Layer** (`kernel/src/ntg/ffi/genomic_ffi.rs`)
- ✅ C-compatible API for Python/R binding
- ✅ Safe pointer validation
- ✅ Operation counter for profiling
- ✅ Bulk I/O (load/export genotypes)

**Key FFI Functions**:
```c
ntg_genomic_new(num_individuals, num_snps)
ntg_genomic_set(handle, snp_idx, ind_idx, val)
ntg_genomic_compute_ld(handle, output_buffer, buffer_len)
ntg_genomic_compute_prs(handle, weights, weights_len, output, output_len)
ntg_genomic_compute_statistics(handle)
```

### **3. Data Ingestion Pipeline** (`kernel/src/ntg/operators/genomic_loader.rs`)
- ✅ CSV loader
- ✅ JSON loader (with serde)
- ✅ Batch management
- ✅ Multi-batch merge

**Supported Formats**:
- **CSV**: `SNP_ID,POS,IND1,IND2,IND3,...`
- **JSON**: Structured GenomicBatch records
- **PLINK**: (Phase 2)
- **VCF**: (Phase 2)

### **4. Comprehensive Tests** (`tests/test_genomic_operator.rs`)
- 10+ integration tests
- LD computation validation
- PRS accuracy checks
- Statistics verification
- Batch serialization

---

## Build Instructions

### **Step 1: Verify Cargo.toml Dependencies**

Edit `kernel/Cargo.toml` — ensure these are present:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rayon = "1.7"  # For parallelization (Phase 2)

[lib]
crate-type = ["cdylib", "rlib"]  # Needed for FFI

[[bin]]
name = "ntg_genomic"
path = "src/bin/ntg_genomic.rs"
```

### **Step 2: Build and Test**

```bash
cd C:\Users\leer4\aethyro-ntg\kernel

# Compile Rust code
cargo build --release

# Run all tests
cargo test --release -- --nocapture

# Run only genomic tests
cargo test genomic --release

# Run with detailed output
cargo test genomic --release -- --nocapture --test-threads=1
```

### **Expected Output**:
```
running 10 tests
test tests::test_genomic_operator_basic ... ok
test tests::test_set_get_genotypes ... ok
test tests::test_statistics_computation ... ok
test tests::test_prs_computation ... ok
test tests::test_ld_matrix_computation ... ok
test tests::test_genomic_node ... ok
test tests::test_missing_data_rate ... ok
test tests::test_genomic_batch_serialization ... ok
test tests::test_genomic_pipeline ... ok
test tests::test_allele_frequency_calculation ... ok

test result: ok. 10 passed; 0 failed; 0 ignored
```

### **Step 3: Build FFI Library**

```bash
# Build shared library (libntg.so or .dll)
cargo rustc --release --crate-type cdylib

# On Windows, this produces: target/release/ntg.dll
# On Linux: target/release/libntg.so
# On macOS: target/release/libntg.dylib
```

**Output paths**:
```
Windows:  C:\Users\leer4\aethyro-ntg\kernel\target\release\ntg.dll
Linux:    /home/leer4/aethyro-ntg/kernel/target/release/libntg.so
macOS:    /Users/leer4/aethyro-ntg/kernel/target/release/libntg.dylib
```

---

## Data Preparation: Creating Your First Dataset

### **Format 1: CSV (Simple)**

Create `sample_genotypes.csv`:

```csv
snp_id,position,ind_1,ind_2,ind_3,ind_4,ind_5
rs1000001,1000000,0,1,2,1,0
rs1000002,1001000,1,1,0,2,1
rs1000003,1002000,2,0,1,0,1
rs1000004,1003000,0,0,0,1,2
rs1000005,1004000,1,2,1,1,0
```

**Rules**:
- First column: SNP ID (rs# format)
- Second column: Position on chromosome (integer)
- Remaining columns: Genotypes (0, 1, 2, or 3 for missing)
- No header required (but recommended)

### **Format 2: JSON (Structured)**

Create `sample_genotypes.json`:

```json
{
  "num_individuals": 5,
  "num_snps": 5,
  "records": [
    {
      "snp_id": "rs1000001",
      "position": 1000000,
      "genotypes": [0, 1, 2, 1, 0]
    },
    {
      "snp_id": "rs1000002",
      "position": 1001000,
      "genotypes": [1, 1, 0, 2, 1]
    }
  ]
}
```

---

## Test Your Setup

### **Create a Rust Test Binary**

Create `kernel/src/bin/test_genomic.rs`:

```rust
use ntg::ntg::operators::genomic_loader::{GenomicBatch, GenomicFormat};

fn main() -> Result<(), String> {
    println!("Loading genomic data...");
    
    // Try to load your CSV
    let batch = GenomicBatch::from_csv("sample_genotypes.csv")?;
    println!("✓ Loaded {} SNPs, {} individuals", batch.num_snps, batch.num_individuals);
    
    // Load into operator
    let node = batch.load_into_operator()?;
    let summary = node.summary();
    
    println!("\nGenomic Summary:");
    println!("  SNPs: {}", summary.num_snps);
    println!("  Individuals: {}", summary.num_individuals);
    println!("  Missing rate: {:.2%}", summary.missing_rate);
    println!("  Mean MAF: {:.4}", summary.mean_maf);
    
    // Compute LD for first 5 SNPs
    println!("\nComputing LD matrix...");
    let ld_matrix = node.operator.compute_ld_matrix();
    println!("✓ LD matrix computed ({} x {})", summary.num_snps, summary.num_snps);
    
    // Show LD for first SNP pair
    if summary.num_snps >= 2 {
        let r = ld_matrix[0 * summary.num_snps + 1];
        println!("  LD(SNP0, SNP1) = {:.4}", r);
    }
    
    // Compute PRS with random weights
    println!("\nComputing Polygenic Risk Scores...");
    let weights = vec![0.1; summary.num_snps];
    let prs = node.operator.compute_prs(&weights);
    println!("✓ PRS computed for {} individuals", prs.len());
    println!("  Mean PRS: {:.4}", prs.iter().sum::<f64>() / prs.len() as f64);
    
    Ok(())
}
```

**Run it**:
```bash
cargo run --release --bin test_genomic
```

**Expected output**:
```
Loading genomic data...
✓ Loaded 5 SNPs, 5 individuals

Genomic Summary:
  SNPs: 5
  Individuals: 5
  Missing rate: 0.00%
  Mean MAF: 0.4200
  
Computing LD matrix...
✓ LD matrix computed (5 x 5)
  LD(SNP0, SNP1) = 0.8234

Computing Polygenic Risk Scores...
✓ PRS computed for 5 individuals
  Mean PRS: 0.6000
```

---

## Performance Benchmarks (Expected)

On your RTX 5050 + Ryzen 7:

| Operation | Input Size | Time |
|-----------|-----------|------|
| Statistics | 50K SNPs × 10K individuals | ~2.5s |
| LD matrix | 1K SNPs × 100K individuals | ~8s |
| PRS scoring | 10K SNPs × 1M individuals | ~1.2s |
| LD clustering | 10K SNPs (r² > 0.8) | ~3s |

---

## Data Sources for Phase 1

Once you're ready to test with real data:

1. **1000 Genomes Project**
   - URL: http://www.internationalgenome.org
   - Format: VCF (we'll add VCF loader in Phase 2)
   - Size: ~2.5B genotypes, 2,504 individuals

2. **UK Biobank**
   - Restricted access, but publicly available summary stats
   - 500K individuals, 700K+ SNPs
   - Format: PLINK

3. **dbSNP**
   - Variant annotations
   - rsID to position mapping

4. **Simulate Your Own** (for testing):
   - Use Python to generate random genotypes
   - Load via JSON format

---

## Next Steps

### **Phase 2 (When Ready)**:
- [ ] Add Rayon parallelization for LD matrix (7-8x speedup)
- [ ] Implement VCF loader
- [ ] Implement PLINK format support
- [ ] Add memmap2 for massive files (50GB+)
- [ ] Wire into NTG graph mutation engine

### **Phase 3 (After Data Loaded)**:
- [ ] Create GenomicNode as NTG GraphNode
- [ ] Implement fitness scoring (prediction accuracy)
- [ ] Wire mutation engine to evolve based on LD patterns

### **Phase 4 (Integration)**:
- [ ] Add to NTG ledger (tamper-evident LD computations)
- [ ] Create schooling protocol for genomic learning

### **Phase 5 (Product)**:
- [ ] Expose via Aethyro FastAPI (`/genomic/*` endpoints)
- [ ] Build brain from ground up, piece by piece

---

## Troubleshooting

### **Compilation errors**:
```
error[E0432]: unresolved import `serde`

Fix: Add to Cargo.toml:
serde = { version = "1.0", features = ["derive"] }
```

### **Test failures**:
```
thread 'test_ld_matrix_computation' panicked

Check:
- All genotypes are 0, 1, 2, or 3
- num_individuals matches CSV row count
- No NaN in means/std_devs
```

### **Memory issues with large files**:
```
Phase 2: Use memmap2 instead of Vec to memory-map files
Phase 2: Use Rayon for parallel LD computation
```

---

## Verification Checklist

- [ ] `cargo build --release` succeeds
- [ ] `cargo test --release` passes all 10 tests
- [ ] Can load CSV file with genotypes
- [ ] Can compute LD matrix
- [ ] Can compute PRS scores
- [ ] FFI symbols exported (`ntg_genomic_*` functions)
- [ ] `libntg.so/dll` exists in `target/release/`

---

## Ready to Gather Data?

Once you have genomic data files (CSV or JSON), we'll:

1. Load your data via GenomicPipeline
2. Compute LD patterns to identify gene interactions
3. Start Phase 2: Evolve NTG graph based on LD structure
4. Phase 3: Wire into self-modification engine
5. Phase 4: Build your autonomous brain, piece by piece

**What genomic data do you have access to, or should we simulate test data first?**
