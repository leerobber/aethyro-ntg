/// VCF Streaming Parser
/// Handles gzipped VCF files with streaming genotype encoding
/// Target: 201K SNPs/sec throughput

use std::io::{BufRead, BufReader};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct VcfChromosome {
    pub chr: u8,
    pub snps: Vec<SnpRecord>,
    pub sample_names: Vec<String>,
    pub genotypes: Vec<BitstreamGenotypes>,
}

#[derive(Debug, Clone)]
pub struct SnpRecord {
    pub id: String,
    pub position: u32,
    pub ref_allele: String,
    pub alt_allele: String,
    pub qual: f32,
    pub info: String,
}

pub struct VcfParser {
    verbose: bool,
}

impl VcfParser {
    pub fn new(verbose: bool) -> Self {
        VcfParser { verbose }
    }

    /// Parse gzipped VCF file and return chromosome data
    /// Streams through the file without loading it all into memory
    pub fn parse_vcf<P: AsRef<Path>>(
        &self,
        vcf_path: P,
        chr_id: u8,
    ) -> Result<VcfChromosome, String> {
        let path = vcf_path.as_ref();

        if self.verbose {
            println!("[*] Opening VCF: {}", path.display());
        }

        // Open and decompress
        let file = File::open(path)
            .map_err(|e| format!("Failed to open VCF: {}", e))?;
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        let mut sample_names = Vec::new();
        let mut snps = Vec::new();
        let mut all_genotypes = Vec::new();
        let mut line_count = 0u64;
        let mut snp_count = 0u64;

        let start = Instant::now();
        let mut last_progress = Instant::now();

        for line_result in reader.lines() {
            let line = line_result
                .map_err(|e| format!("Failed to read VCF line: {}", e))?;

            if line.starts_with("##") {
                // Skip meta lines
                continue;
            }

            if line.starts_with("#CHROM") {
                // Parse header line
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() < 10 {
                    return Err("Invalid VCF header: fewer than 10 columns".to_string());
                }

                // Sample names start at column 9
                sample_names = parts[9..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                if self.verbose {
                    println!("[OK] Found {} samples", sample_names.len());
                }

                // Initialize genotype storage
                all_genotypes = vec![BitstreamGenotypes::new(sample_names.len()); 10000];

                continue;
            }

            if line.starts_with("#") {
                // Skip other comment lines
                continue;
            }

            // Parse data line
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 10 {
                continue;
            }

            // Extract VCF fields
            // Parse chromosome number (handle "chr1" and "1" formats)
            let chrom_str = parts[0].trim_start_matches("chr");
            let chrom: u8 = chrom_str.parse().unwrap_or(0);

            if chrom != chr_id {
                continue;  // Skip if wrong chromosome
            }

            let pos: u32 = parts[1]
                .parse()
                .map_err(|_| "Failed to parse position".to_string())?;
            let id = parts[2].to_string();
            let ref_allele = parts[3].to_string();
            let alt_allele = parts[4].to_string();
            let qual: f32 = parts[5]
                .parse()
                .unwrap_or(0.0);

            // Initialize genotypes for this SNP if needed
            if snp_count >= all_genotypes.len() as u64 {
                all_genotypes.push(BitstreamGenotypes::new(sample_names.len()));
            }

            let mut snp_genotypes = BitstreamGenotypes::new(sample_names.len());

            // Parse genotypes (starting at column 9)
            for (sample_idx, sample_data) in parts[9..].iter().enumerate() {
                let genotype = self.parse_genotype(sample_data);
                snp_genotypes.set(sample_idx, genotype);
            }

            // Store SNP record
            snps.push(SnpRecord {
                id,
                position: pos,
                ref_allele,
                alt_allele,
                qual,
                info: String::new(),
            });

            all_genotypes[snp_count as usize] = snp_genotypes;
            snp_count += 1;

            // Progress reporting
            if self.verbose && last_progress.elapsed().as_secs() >= 5 {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = snp_count as f64 / elapsed;
                println!(
                    "  [Progress] {} SNPs processed ({:.0} SNPs/sec)",
                    snp_count, rate
                );
                last_progress = Instant::now();
            }

            line_count += 1;
        }

        // Trim unused genotype storage
        all_genotypes.truncate(snp_count as usize);

        if self.verbose {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = snp_count as f64 / elapsed;
            println!(
                "[OK] Parsed {} variants in {:.1}s ({:.0} SNPs/sec)",
                snp_count, elapsed, rate
            );

            // Calculate memory usage
            let total_memory: usize = all_genotypes.iter().map(|g| g.memory_bytes()).sum();
            let memory_mb = total_memory / (1024 * 1024);
            println!("[*] Genotype storage: {} MB", memory_mb);
        }

        Ok(VcfChromosome {
            chr: chr_id,
            snps,
            sample_names,
            genotypes: all_genotypes,
        })
    }

    /// Parse genotype from VCF format (e.g., "0/1", "1|1", "./.", etc.)
    fn parse_genotype(&self, gt_str: &str) -> u8 {
        if gt_str.is_empty() || gt_str.starts_with(".") {
            return 3;  // Missing
        }

        let parts: Vec<&str> = gt_str.split(|c| c == '/' || c == '|').collect();
        if parts.len() < 2 {
            return 3;  // Missing
        }

        let allele1: u8 = parts[0].parse().unwrap_or(255);
        let allele2: u8 = parts[1].parse().unwrap_or(255);

        if allele1 == 255 || allele2 == 255 {
            return 3;  // Missing
        }

        // Count alternate alleles
        let alt_count = (if allele1 > 0 { 1 } else { 0 }) +
                       (if allele2 > 0 { 1 } else { 0 });

        alt_count as u8
    }
}

impl VcfChromosome {
    /// Validate chromosome data
    pub fn validate(&self) -> Result<(), String> {
        if self.snps.is_empty() {
            return Err("No SNPs parsed".to_string());
        }

        if self.sample_names.is_empty() {
            return Err("No samples found".to_string());
        }

        if self.genotypes.len() != self.snps.len() {
            return Err(format!(
                "Genotype count {} != SNP count {}",
                self.genotypes.len(),
                self.snps.len()
            ));
        }

        // Check positions are monotonically increasing
        for i in 1..self.snps.len() {
            if self.snps[i].position < self.snps[i - 1].position {
                return Err(format!(
                    "Positions not sorted: {} after {}",
                    self.snps[i].position,
                    self.snps[i - 1].position
                ));
            }
        }

        Ok(())
    }

    /// Get summary statistics
    pub fn summary(&self) -> String {
        let total_memory: usize = self.genotypes.iter().map(|g| g.memory_bytes()).sum();
        let memory_mb = total_memory / (1024 * 1024);

        format!(
            "Chr{}: {} SNPs, {} samples, {:.1} MB memory",
            self.chr,
            self.snps.len(),
            self.sample_names.len(),
            memory_mb as f64
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genotype_parsing() {
        let parser = VcfParser::new(false);

        assert_eq!(parser.parse_genotype("0/0"), 0);  // ref/ref
        assert_eq!(parser.parse_genotype("0/1"), 1);  // het
        assert_eq!(parser.parse_genotype("1/0"), 1);  // het (reversed)
        assert_eq!(parser.parse_genotype("1/1"), 2);  // alt/alt
        assert_eq!(parser.parse_genotype("./."), 3);  // missing
        assert_eq!(parser.parse_genotype("0|1"), 1);  // phased het
        assert_eq!(parser.parse_genotype("1|1"), 2);  // phased alt/alt
    }
}
