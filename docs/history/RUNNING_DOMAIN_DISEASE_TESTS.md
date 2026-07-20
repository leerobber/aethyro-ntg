# Running Domain Disease Detection Tests & Generating Publication-Ready Results

**Version**: 1.0  
**Last Updated**: 2026-07-12  
**Status**: Ready for Execution  

---

## Quick Start (5 minutes)

### Step 1: Compile
```powershell
cd C:\Users\leer4\aethyro-ntg\kernel
cargo build --lib --release
```

### Step 2: Run Test
```powershell
cargo run --bin domain_disease_test --release 2>&1 | Tee-Object -FilePath ../test_output.txt
```

### Step 3: Analyze Results
```powershell
cd ..
.\scripts\analyze_domain_results.ps1 -TestOutput "test_output.txt" -OutputDir "results"
```

### Step 4: View Report
```powershell
# Open HTML report in browser
.\results\report.html
```

---

## Detailed Instructions

### PHASE 1: Build Environment Setup (First Time Only)

#### 1.1 Prerequisites Check
```powershell
# Check Rust installation
rustc --version
cargo --version

# Expected: rustc 1.70+ and cargo 1.70+
# If not installed: https://rustup.rs/
```

#### 1.2 Project Structure Verification
```powershell
cd C:\Users\leer4\aethyro-ntg\kernel

# Verify files exist
Test-Path "src/genomic/domain_agents.rs"         # ✓ Should exist
Test-Path "src/bin/domain_disease_test.rs"       # ✓ Should exist
Test-Path "Cargo.toml"                           # ✓ Should exist
```

#### 1.3 Clean Build
```powershell
# Clean previous build artifacts
cargo clean

# Build library (first time)
cargo build --lib --release

# Expected: Compiles in <30 seconds, 0 errors
```

---

### PHASE 2: Test Execution

#### 2.1 Single Test Run
```powershell
# Run once and save output
cargo run --bin domain_disease_test --release 2>&1 | Tee-Object -FilePath ../test_run_001.txt

# Expected output:
# - 6 domain diagnostics (Genomic, Code, Malware, Injection, Supply, Crypto)
# - Each with: Severity, Score, Patterns, Remediation
# - Total execution time: ~100-500ms
# - No errors or panics
```

#### 2.2 Multiple Test Runs (For Statistical Analysis)
```powershell
# Run 10 times for statistical confidence
for ($i = 1; $i -le 10; $i++) {
    Write-Host "Run $i/10..."
    cargo run --bin domain_disease_test --release 2>&1 | Tee-Object -FilePath "../test_run_$($i.ToString('000')).txt"
    Start-Sleep -Milliseconds 500
}
```

#### 2.3 Verify Test Output
```powershell
# Check first run completed
Get-Content ../test_run_001.txt | Select-Object -First 30

# Expected: Shows all 6 domains with risk scores
```

---

### PHASE 3: Results Analysis

#### 3.1 Parse Single Run
```powershell
# Analyze the test output
.\scripts\analyze_domain_results.ps1 `
    -TestOutput "../test_run_001.txt" `
    -OutputDir "results"

# Generated files:
# - results/metrics.csv
# - results/summary.json
# - results/report.html
```

#### 3.2 Aggregate Multiple Runs
```powershell
# Combine all 10 runs
$allContent = Get-Content ../test_run_*.txt | Out-String

# Save combined output
$allContent | Set-Content -Path "../test_run_all.txt"

# Analyze combined
.\scripts\analyze_domain_results.ps1 `
    -TestOutput "../test_run_all.txt" `
    -OutputDir "results"
```

#### 3.3 View Generated Reports
```powershell
# CSV (open in Excel)
Invoke-Item "results/metrics.csv"

# JSON (text viewer)
Get-Content "results/summary.json" | ConvertFrom-Json | ConvertTo-Json

# HTML (browser)
Invoke-Item "results/report.html"
```

---

### PHASE 4: Publication Preparation

#### 4.1 Generate Research Document
```powershell
# Create publication-ready markdown
Copy-Item "DOMAIN_DISEASE_DETECTION_RESULTS.md" -Destination "results/PUBLICATION_READY.md"

# Add actual results to template
$template = Get-Content "results/PUBLICATION_READY.md" -Raw
$metrics = Get-Content "results/metrics.csv"

# Append metrics
$template += "`n## ACTUAL TEST RESULTS`n"
$template += "```csv`n$metrics`n```"

$template | Set-Content "results/PUBLICATION_READY.md"
```

#### 4.2 Create GitHub Release Bundle
```powershell
# Create directory structure for release
New-Item -ItemType Directory -Path "results/github_release"

Copy-Item "results/metrics.csv" -Destination "results/github_release/"
Copy-Item "results/summary.json" -Destination "results/github_release/"
Copy-Item "results/report.html" -Destination "results/github_release/"
Copy-Item "results/PUBLICATION_READY.md" -Destination "results/github_release/README.md"
Copy-Item "RESEARCH_PROTOCOL_DOMAIN_DISEASE_DETECTION.md" -Destination "results/github_release/"

# Archive for release
Compress-Archive -Path "results/github_release" -DestinationPath "results/domain_disease_v1.0.zip"
```

#### 4.3 Prepare Manuscript Supplementary Materials
```powershell
# Create supplementary materials directory
New-Item -ItemType Directory -Path "results/supplementary"

Copy-Item "src/genomic/domain_agents.rs" -Destination "results/supplementary/domain_agents.rs"
Copy-Item "src/bin/domain_disease_test.rs" -Destination "results/supplementary/domain_disease_test.rs"
Copy-Item "results/metrics.csv" -Destination "results/supplementary/raw_metrics.csv"
Copy-Item "results/summary.json" -Destination "results/supplementary/detailed_results.json"

# Create README for supplementary
@"
# Supplementary Materials

## Domain-Agnostic Disease Detection Framework

### Code
- domain_agents.rs: Core D3 implementation (420 lines)
- domain_disease_test.rs: Test harness (200 lines)

### Data
- raw_metrics.csv: Parsed test metrics
- detailed_results.json: Full result structure

### Reproducibility
1. Extract code files
2. Place in kernel/src/genomic/ and kernel/src/bin/
3. Run: cargo run --bin domain_disease_test --release
4. Results should match included metrics

"@ | Set-Content "results/supplementary/README.txt"
```

---

## Expected Results

### Console Output Example

```
╔═══════════════════════════════════════════════════════════════╗
║  Domain-Agnostic Disease Detection System                    ║
╚═══════════════════════════════════════════════════════════════╝

[Initializing Domain Agents]
✓ Genomic Agent
✓ Code Quality Agent
✓ Malware Detection Agent
✓ Injection Risk Agent
✓ Supply Chain Agent
✓ Cryptographic Agent

╔═════════════════════════════════════════════════════════════╗
║  DOMAIN 1: GENOMIC DISEASE RISK                           ║
╚═════════════════════════════════════════════════════════════╝
[Risk] Severity: MEDIUM | Score: 0.350
  Detected Patterns: 1
  Affected Modules: 2
  Remediation: Genetic counseling recommended...

[... repeats for 5 more domains ...]

╔═══════════════════════════════════════════════════════════════╗
║  UNIFIED RISK DASHBOARD                                     ║
╚═══════════════════════════════════════════════════════════════╝

| Domain           | Severity | Score | Patterns | Status      |
|------------------|----------|-------|----------|-------------|
| Genomic          | MEDIUM   | 0.350 | 1        | 🟡 MEDIUM   |
| Code Quality     | LOW      | 0.100 | 0        | 🟢 LOW      |
| Malware          | CRITICAL | 0.450 | 3        | 🔴 CRITICAL |
| Injection        | HIGH     | 0.380 | 2        | 🟠 HIGH     |
| Supply Chain     | HIGH     | 0.400 | 2        | 🟠 HIGH     |
| Cryptographic    | CRITICAL | 0.600 | 2        | 🔴 CRITICAL |

[OVERALL RISK ASSESSMENT]
  Average Risk Score: 0.380
  Critical Issues: 2
  High-Severity Issues: 2

  ⚠️  ACTION REQUIRED: 2 critical issues detected!
```

### Generated Files

```
results/
├── metrics.csv                  # Parsed domain metrics
├── summary.json                 # Structured results
├── report.html                  # Browser-readable report
├── PUBLICATION_READY.md         # Manuscript-ready document
├── github_release/              # GitHub release package
│   ├── metrics.csv
│   ├── summary.json
│   ├── report.html
│   ├── README.md
│   └── RESEARCH_PROTOCOL_DOMAIN_DISEASE_DETECTION.md
└── supplementary/               # Manuscript supplementary materials
    ├── domain_agents.rs
    ├── domain_disease_test.rs
    ├── raw_metrics.csv
    ├── detailed_results.json
    └── README.txt
```

---

## Validation Checklist

### Pre-Test
- [ ] Rust 1.70+ installed
- [ ] Project directory exists: `C:\Users\leer4\aethyro-ntg\kernel`
- [ ] Files verified:
  - [ ] `src/genomic/domain_agents.rs`
  - [ ] `src/bin/domain_disease_test.rs`
  - [ ] `Cargo.toml`
- [ ] Results directory exists or will be created

### During Test
- [ ] Binary compiles without errors
- [ ] Test runs without panics
- [ ] Output captures all 6 domains
- [ ] Execution completes in <5 seconds
- [ ] All metrics have valid values

### Post-Test
- [ ] CSV file generated with 6 rows
- [ ] JSON file has proper structure
- [ ] HTML report displays correctly
- [ ] All metrics in valid ranges:
  - Scores: [0.0, 1.0]
  - Severity: NONE|LOW|MEDIUM|HIGH|CRITICAL
  - Patterns: non-negative integers
- [ ] Latencies reasonable (<200ms per domain)

---

## Troubleshooting

### Issue: Rust not found
```powershell
# Install Rust
irm https://rustup.rs/ | iex

# Verify
rustc --version
```

### Issue: Compilation errors
```powershell
# Clean and rebuild
cargo clean
cargo build --lib --release

# If still fails, check:
# - Rust version: rustc --version (need 1.70+)
# - Dependencies in Cargo.toml are correct
# - No conflicting features
```

### Issue: Test runs but no output
```powershell
# Try running directly without redirect first
cargo run --bin domain_disease_test --release

# Verify binary exists
cargo build --bin domain_disease_test --release
```

### Issue: Analysis script fails
```powershell
# Check file exists
Test-Path "test_output.txt"

# Try with absolute path
$absPath = (Resolve-Path "test_output.txt").Path
.\scripts\analyze_domain_results.ps1 -TestOutput $absPath
```

---

## Publication Submission Workflow

### Step 1: Run Tests (Multiple Runs for Confidence)
```powershell
# Run 10 times
for ($i = 1; $i -le 10; $i++) {
    cargo run --bin domain_disease_test --release 2>&1 | Tee-Object -FilePath "test_$($i).txt"
}
```

### Step 2: Analyze All Runs
```powershell
# Combine and analyze
$all = Get-Content test_*.txt | Out-String
$all | Set-Content "test_all_combined.txt"
.\scripts\analyze_domain_results.ps1 -TestOutput "test_all_combined.txt"
```

### Step 3: Generate Reports
```powershell
# Publication-ready markdown (results already embedded)
Copy-Item "DOMAIN_DISEASE_DETECTION_RESULTS.md" -Destination "results/MANUSCRIPT.md"

# Protocol documentation
Copy-Item "RESEARCH_PROTOCOL_DOMAIN_DISEASE_DETECTION.md" -Destination "results/SUPPLEMENTARY_PROTOCOL.md"
```

### Step 4: Create Submission Package
```powershell
# Create final submission bundle
New-Item -ItemType Directory -Path "submission" -Force
Copy-Item "results/MANUSCRIPT.md" -Destination "submission/"
Copy-Item "results/metrics.csv" -Destination "submission/"
Copy-Item "results/summary.json" -Destination "submission/"
Copy-Item "results/supplementary/*" -Destination "submission/" -Recurse
```

### Step 5: Submit to Journal
- Upload `submission/MANUSCRIPT.md` as main document
- Upload `submission/metrics.csv` as Table 1
- Upload `submission/supplementary/` as supplementary materials
- Include code availability statement with GitHub link

---

## Publication Ready Commands

### One-liner for Complete Workflow
```powershell
cd C:\Users\leer4\aethyro-ntg\kernel; `
cargo build --lib --release; `
cargo run --bin domain_disease_test --release 2>&1 | Tee-Object -FilePath ../results.txt; `
cd ..; `
.\scripts\analyze_domain_results.ps1 -TestOutput results.txt -OutputDir results; `
Write-Host "`n✓ Complete workflow finished. Results in results/ directory"
```

---

## Support & Questions

### Common Questions

**Q: How long does the test take?**  
A: ~100-500ms for single run. 10 runs: ~5 seconds total.

**Q: Can I run this on other platforms?**  
A: Yes, Rust is cross-platform. Linux/Mac: Same commands with `./scripts/analyze_domain_results.sh` (bash version needed).

**Q: What if I want to modify the test?**  
A: Edit `src/bin/domain_disease_test.rs` and rebuild. Changes take effect immediately.

**Q: How do I cite this work?**  
A: "Domain-Agnostic Disease Detection System v1.0 (2026-07-12), Genomic Intelligence Team"

**Q: Can I integrate this into CI/CD?**  
A: Yes, add to GitHub Actions:
```yaml
- name: Run Domain Disease Tests
  run: |
    cd kernel
    cargo run --bin domain_disease_test --release 2>&1 | tee ../results.txt
```

---

## Success Criteria

✅ Test runs without errors  
✅ All 6 domains produce output  
✅ Risk scores in valid range [0.0, 1.0]  
✅ Severity classifications make sense  
✅ Results can be parsed into CSV/JSON  
✅ HTML report generates successfully  
✅ Results ready for publication  

---

**Document Version**: 1.0  
**Last Updated**: 2026-07-12  
**Status**: READY FOR EXECUTION

