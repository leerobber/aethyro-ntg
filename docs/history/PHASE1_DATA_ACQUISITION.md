# Phase 1: Genomic Data Acquisition & Processing Pipeline

**Objective**: Download real genomes → Learn patterns → Synthesize evolved genomes  
**Status**: Ready to execute  
**Timeline**: Week 1-2

---

## Datasets to Acquire

### **1. 1000 Genomes Project (Tier 1 - START HERE)**

**Source**: http://www.internationalgenome.org  
**Size**: 2,504 individuals × ~84 million variants  
**Format**: VCF (we'll convert to CSV)  
**Access**: Public, free  
**Quality**: Gold standard, extensively validated  

**Coverage**:
- 26 populations (African, European, East Asian, South Asian, Americas)
- 5 superpopulations (AFR, AMR, EAS, EUR, SAS)
- Deep sequencing (~30-60x coverage)
- Rare variants included

**Download Strategy**:
```
Phase 1a: Download chromosome 22 (~51Mb, 7.5MB compressed)
          - Smallest, fastest to test pipeline
          
Phase 1b: Download chromosomes 1-22
          - Full genome (~3.2GB compressed)
          - Runtime: ~4-6 hours on your connection
```

### **2. UK Biobank (Tier 2 - After 1000G)**

**Source**: https://www.ukbiobank.ac.uk  
**Size**: 500,000 individuals × ~800K SNPs (genotyped, not sequenced)  
**Format**: BGEN (binary genotype format)  
**Access**: Application required (free for researchers)  
**Quality**: Largest European ancestry cohort  

**Key for synthetic generation**:
- Linkage Disequilibrium patterns (SNP correlations)
- Population stratification (ancestry components)
- Phenotype associations (what variants cause what)

### **3. Gnomad (Tier 2 - Parallel Download)**

**Source**: https://gnomad.broadinstitute.org  
**Size**: 125,748 whole genomes + 15,708 exomes  
**Format**: VCF  
**Access**: Public, free  
**Coverage**: All populations + ancestry breakdown  

**Why Gnomad**:
- Frequency data (how common is each variant?)
- Constraint metrics (which regions are under selection?)
- Essential for realistic synthetic genome generation

### **4. ClinVar (Tier 2 - In Parallel)**

**Source**: https://www.ncbi.nlm.nih.gov/clinvar  
**Size**: ~400K variant interpretations  
**Format**: VCF, XML, TSV  
**Access**: Public, free  
**Key for synthetic genomes**: Disease/pathogenicity annotations

---

## Download & Storage Strategy

### **Directory Structure**

```
C:\Users\leer4\aethyro-ntg\
├── data/
│   ├── raw/
│   │   ├── 1000g/
│   │   │   ├── chr1.vcf.gz
│   │   │   ├── chr2.vcf.gz
│   │   │   └── ... chr22.vcf.gz
│   │   ├── ukbiobank/
│   │   │   ├── ukb_genotype_chr1.bgen
│   │   │   └── ... chr22.bgen
│   │   ├── gnomad/
│   │   │   ├── gnomad.genomes.vcf.gz
│   │   │   └── gnomad.exomes.vcf.gz
│   │   └── clinvar/
│   │       └── clinvar.vcf.gz
│   ├── processed/
│   │   ├── 1000g_chr22_samples.csv
│   │   ├── 1000g_full_samples.csv
│   │   ├── ukbiobank_samples.csv
│   │   └── merged_genotypes.json
│   ├── analysis/
│   │   ├── ld_matrices/
│   │   ├── prs_weights/
│   │   └── population_stats/
│   └── synthetic/
│       ├── generated_genomes_v1/
│       ├── evolved_genomes_v2/
│       └── phenotype_predictions/
```

---

## Implementation: Download Pipeline

### **Step 1: Install Required Tools**

```powershell
# Install vcftools (VCF manipulation)
choco install vcftools -y

# Install bcftools (faster VCF processing)
choco install bcftools -y

# Install wget (file downloads with resume)
choco install wget -y

# Verify installations
vcftools --version
bcftools --version
wget --version
```

### **Step 2: Create Download Script**

**File**: `tools/download_genomes.ps1`

```powershell
# ═════════════════════════════════════════════════════════════════
# Genomic Data Acquisition Pipeline
# Downloads real genomes for learning
# ═════════════════════════════════════════════════════════════════

param(
    [string]$dataset = "1000g_chr22",  # Which dataset to download
    [string]$output_dir = "C:\Users\leer4\aethyro-ntg\data\raw"
)

Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "Genomic Data Acquisition Pipeline" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $output_dir | Out-Null

function Download-1000G-Chromosome {
    param([int]$chr, [string]$dest_dir)
    
    $filename = "ALL.chr${chr}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz"
    $url = "http://ftp.1000genomes.ebi.ac.uk/vol1/ftp/release/20130502/$filename"
    $output = Join-Path $dest_dir $filename
    
    if (Test-Path $output) {
        Write-Host "✓ Already exists: $filename" -ForegroundColor Green
        return
    }
    
    Write-Host "⬇ Downloading chromosome $chr..." -ForegroundColor Yellow
    Write-Host "  URL: $url"
    Write-Host "  Size: ~300-500 MB"
    
    # Use wget with resume capability
    wget --continue -O $output $url
    
    if (Test-Path $output) {
        $size = (Get-Item $output).Length / 1GB
        Write-Host "✓ Downloaded: {0:F2} GB" -f $size -ForegroundColor Green
    } else {
        Write-Host "✗ Download failed" -ForegroundColor Red
    }
}

function Download-1000G-Full {
    param([string]$dest_dir)
    
    Write-Host "Downloading full 1000 Genomes Phase 3..." -ForegroundColor Cyan
    Write-Host "This will download ~3.2 GB" -ForegroundColor Yellow
    Write-Host "Estimated time: 4-6 hours on typical connection" -ForegroundColor Yellow
    Write-Host ""
    
    $1000g_dir = Join-Path $dest_dir "1000g"
    New-Item -ItemType Directory -Force -Path $1000g_dir | Out-Null
    
    # Download all 22 chromosomes
    for ($chr = 1; $chr -le 22; $chr++) {
        Download-1000G-Chromosome -chr $chr -dest_dir $1000g_dir
        Write-Host ""
    }
    
    Write-Host "✓ All chromosomes downloaded" -ForegroundColor Green
}

function Download-1000G-Chr22 {
    param([string]$dest_dir)
    
    Write-Host "Downloading 1000 Genomes Chromosome 22 (test)..." -ForegroundColor Cyan
    Write-Host "This is the smallest chromosome (~51 Mb)" -ForegroundColor Yellow
    Write-Host "Perfect for testing the pipeline (~7.5 MB compressed)" -ForegroundColor Yellow
    Write-Host ""
    
    $1000g_dir = Join-Path $dest_dir "1000g"
    New-Item -ItemType Directory -Force -Path $1000g_dir | Out-Null
    
    Download-1000G-Chromosome -chr 22 -dest_dir $1000g_dir
}

function Download-GnomAD {
    param([string]$dest_dir)
    
    Write-Host "Downloading gnomAD data..." -ForegroundColor Cyan
    $gnomad_dir = Join-Path $dest_dir "gnomad"
    New-Item -ItemType Directory -Force -Path $gnomad_dir | Out-Null
    
    $gnomad_url = "https://gnomad-public-us-east-1.s3.amazonaws.com/release/3.0/vcf/genomes/gnomad.genomes.r3.0.sites.vcf.bgz"
    $output = Join-Path $gnomad_dir "gnomad.genomes.vcf.bgz"
    
    Write-Host "⬇ Downloading gnomAD genomes..." -ForegroundColor Yellow
    Write-Host "  Note: This is ~400 GB - we'll download metadata only first" -ForegroundColor Yellow
    
    # For now, just show the URL
    Write-Host "  When ready, run:"
    Write-Host "  wget $gnomad_url -O $output" -ForegroundColor Cyan
}

function Show-Menu {
    Write-Host "Which dataset to download?" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "1. 1000 Genomes Chr22 (RECOMMENDED - 7.5 MB, test pipeline)" -ForegroundColor Yellow
    Write-Host "2. 1000 Genomes Full (3.2 GB, all chromosomes)" -ForegroundColor Yellow
    Write-Host "3. gnomAD (frequency data, metadata only)" -ForegroundColor Yellow
    Write-Host "4. All of the above (full pipeline, ~4 GB total)" -ForegroundColor Yellow
    Write-Host ""
    
    $choice = Read-Host "Select (1-4)"
    
    switch ($choice) {
        "1" { 
            Download-1000G-Chr22 -dest_dir $output_dir
        }
        "2" { 
            Download-1000G-Full -dest_dir $output_dir
        }
        "3" { 
            Download-GnomAD -dest_dir $output_dir
        }
        "4" {
            Download-1000G-Chr22 -dest_dir $output_dir
            Write-Host ""
            Download-1000G-Full -dest_dir $output_dir
            Write-Host ""
            Download-GnomAD -dest_dir $output_dir
        }
        default {
            Write-Host "Invalid choice" -ForegroundColor Red
        }
    }
}

Show-Menu

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "Download complete! Next steps:" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "1. Convert VCF to CSV:"
Write-Host "   python tools/vcf_to_csv.py data/raw/1000g/ALL.chr22.*.vcf.gz" -ForegroundColor Cyan
Write-Host ""
Write-Host "2. Load into NTG operator:"
Write-Host "   cargo run --release --bin process_genomes" -ForegroundColor Cyan
Write-Host ""
Write-Host "3. Analyze LD patterns:"
Write-Host "   cargo run --release --bin analyze_ld" -ForegroundColor Cyan
```

### **Step 3: VCF to CSV Converter**

**File**: `tools/vcf_to_csv.py`

```python
#!/usr/bin/env python3
"""
Convert VCF files to CSV format for NTG ingestion
Handles large files with streaming processing
"""

import gzip
import sys
import argparse
from pathlib import Path
import csv

def parse_vcf_header(vcf_file):
    """Extract sample names from VCF header"""
    samples = []
    with gzip.open(vcf_file, 'rt') if str(vcf_file).endswith('.gz') else open(vcf_file) as f:
        for line in f:
            if line.startswith('#CHROM'):
                parts = line.strip().split('\t')
                samples = parts[9:]  # Sample names start at column 9
                break
    return samples

def vcf_to_csv(vcf_file, output_csv, max_variants=None):
    """
    Convert VCF to CSV format
    
    Output CSV format:
    snp_id,position,sample1,sample2,...
    rs1000001,1000000,0,1,2,...
    """
    
    samples = parse_vcf_header(vcf_file)
    print(f"Found {len(samples)} samples")
    
    variant_count = 0
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
                continue
            
            # Extract variant info
            chrom = parts[0]
            pos = int(parts[1])
            var_id = parts[2] if parts[2] != '.' else f"chr{chrom}_{pos}"
            
            # Extract genotypes (convert to 0/1/2/3)
            genotypes = []
            for i in range(9, len(parts)):
                gt = parts[i].split(':')[0]  # Get GT field
                
                if gt == './.':
                    genotypes.append(3)  # Missing
                else:
                    alleles = gt.replace('|', '/').split('/')
                    count = sum(1 for a in alleles if a == '1')
                    genotypes.append(count)
            
            writer.writerow([var_id, pos] + genotypes)
            
            variant_count += 1
            if variant_count % 1000 == 0:
                print(f"  Processed {variant_count} variants...")
            
            if max_variants and variant_count >= max_variants:
                break
        
        print(f"✓ Converted {variant_count} variants to CSV")

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Convert VCF to CSV for NTG')
    parser.add_argument('vcf_file', help='Input VCF file (.vcf or .vcf.gz)')
    parser.add_argument('-o', '--output', help='Output CSV file')
    parser.add_argument('--max-variants', type=int, help='Maximum variants to process (for testing)')
    
    args = parser.parse_args()
    
    vcf_path = Path(args.vcf_file)
    output_path = args.output or vcf_path.stem + '.csv'
    
    print(f"Converting {vcf_path} to {output_path}...")
    vcf_to_csv(vcf_path, output_path, args.max_variants)
    print("Done!")
```

---

## Download Timeline & Sizes

### **Phase 1a: Test Pipeline (Recommended Start)**
```
Timeline: 30 minutes
Size: ~7.5 MB compressed → 150 MB uncompressed
Data: Chromosome 22, 2,504 individuals, ~1.1M variants
Output: sample_genotypes.csv (fits in memory)
```

### **Phase 1b: Full 1000 Genomes**
```
Timeline: 4-6 hours
Size: ~3.2 GB compressed → ~80 GB uncompressed
Data: All 22 chromosomes, 2,504 individuals, 84M variants
Output: All chromosomes in CSV format
Storage: Need ~100 GB free space
```

### **Phase 1c: Add gnomAD (Optional)**
```
Timeline: 8-12 hours for download
Size: ~400 GB (genomes) + ~100 GB (exomes)
Purpose: Frequency data for synthetic genome generation
Storage: Large - only if you have space
```

---

## Processing Pipeline (After Download)

### **Step 1: Convert VCF → CSV**
```bash
python tools/vcf_to_csv.py data/raw/1000g/ALL.chr22.*.vcf.gz -o data/processed/1000g_chr22.csv
```

### **Step 2: Load into NTG**
```bash
cargo run --release --bin load_genomic_data data/processed/1000g_chr22.csv
```

### **Step 3: Analyze Patterns**
```bash
cargo run --release --bin analyze_ld --input data/processed/1000g_chr22.csv --output data/analysis/
```

---

## What You'll Learn From Real Genomes

### **Linkage Disequilibrium (LD) Patterns**
- SNPs in genes are correlated (haplotype blocks)
- LD decays with distance (~300-3000 bp typical)
- Population-specific LD structure

### **Allele Frequency Spectrum**
- Common variants (MAF > 5%): ~100K
- Rare variants (MAF < 0.1%): ~3M
- Population-specific rare variants

### **Population Stratification**
- Ancestry components (PC1-PC10)
- Population-specific haplotypes
- Selection signatures

### **Disease Associations**
- Which variants cause disease
- Effect sizes (beta/odds ratios)
- Gene-environment interactions

---

## From Real to Synthetic: The Evolution

### **Learning Phase** (Week 1-2)
```
Real 1000G Data
    ↓
Extract LD patterns
    ↓
Learn allele frequencies
    ↓
Identify population structure
    ↓
Map phenotype-genotype relationships
```

### **Generation Phase** (Week 2-3)
```
Learned patterns + NTG graph evolution
    ↓
Generate synthetic genomes v1
    ↓
    - Preserve LD structure
    - Match allele frequencies
    - Incorporate population diversity
    ↓
Evaluate fitness: Do synthetics reproduce real patterns?
```

### **Evolution Phase** (Week 3-4)
```
Synthetic genomes v1
    ↓
NTG graph self-modifies:
    - Which variants most important?
    - Which interactions critical?
    - Which mutations improve fitness?
    ↓
Generate evolved genomes v2
    ↓
    - More optimal architectures
    - Better disease predictions
    - Enhanced adaptability
```

---

## Quick Start (Do This First)

```powershell
# 1. Create data directories
New-Item -ItemType Directory -Force -Path "C:\Users\leer4\aethyro-ntg\data\raw\1000g"
New-Item -ItemType Directory -Force -Path "C:\Users\leer4\aethyro-ntg\data\processed"
New-Item -ItemType Directory -Force -Path "C:\Users\leer4\aethyro-ntg\data\analysis"
New-Item -ItemType Directory -Force -Path "C:\Users\leer4\aethyro-ntg\data\synthetic"

# 2. Download chromosome 22 (test dataset)
cd C:\Users\leer4\aethyro-ntg
.\tools\download_genomes.ps1

# 3. Once downloaded, convert VCF → CSV
python tools/vcf_to_csv.py data/raw/1000g/ALL.chr22.*.vcf.gz -o data/processed/1000g_chr22.csv

# 4. Load into NTG and analyze
cargo run --release --bin load_genomic_data -- data/processed/1000g_chr22.csv
```

---

## Next Steps

1. **Download test data** (Chr22, 30 min)
2. **Verify CSV conversion** (ensure 2,504 individuals × 1.1M SNPs)
3. **Load into GenomicOperator** (compute LD, PRS)
4. **Analyze real patterns** (document findings)
5. **Then synthesize** (create evolved genomes based on learnings)

**Ready to start downloading? I can help you set up the scripts.**
