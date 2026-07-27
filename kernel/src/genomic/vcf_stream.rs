//! VCF Streaming Parser
//! Handles gzipped (BGZF multi-member) VCF files with streaming genotype
//! encoding. Measured on real 1000 Genomes chr1-3/22 VCFs (2504 samples):
//! ~14K variants/sec parsing; LD computation downstream is the actual
//! bottleneck at ~500 SNPs/sec (sliding-window r² over 2504 samples/SNP).

use std::io::{BufRead, BufReader};
use flate2::read::MultiGzDecoder;
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
    /// Per-SNP, per-sample allele on haplotype copy A (0=ref, 1=alt,
    /// 3=missing). Empty unless parsed via `parse_vcf_phased_limited`.
    /// Real 1000 Genomes phase3 VCFs are phased ("0|1", not "0/1"), so
    /// "copy A" / "copy B" are the actual maternal/paternal chromosome
    /// copies as called, not an arbitrary split.
    pub hap_a: Vec<BitstreamGenotypes>,
    /// Same as `hap_a` for haplotype copy B.
    pub hap_b: Vec<BitstreamGenotypes>,
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
        self.parse_vcf_limited(vcf_path, chr_id, None)
    }

    /// Same as `parse_vcf`, but stops after `max_variants` records for the
    /// target chromosome. A full 1000-Genomes chromosome file can carry
    /// millions of variants; this bounds a real-data run to a tractable
    /// slice without needing a separate synthetic path.
    pub fn parse_vcf_limited<P: AsRef<Path>>(
        &self,
        vcf_path: P,
        chr_id: u8,
        max_variants: Option<usize>,
    ) -> Result<VcfChromosome, String> {
        self.parse_vcf_impl(vcf_path, chr_id, max_variants, false)
    }

    /// Same as `parse_vcf_limited`, but also captures per-haplotype
    /// allele calls (`VcfChromosome::hap_a`/`hap_b`) needed for
    /// haplotype-block-based synthesis (see `synthesis::HaplotypePool`).
    /// Costs roughly 2x the memory/parse time of `parse_vcf_limited`
    /// since it stores two extra per-SNP bit-vectors; only worth it when
    /// phase-resolved data is actually going to be used.
    pub fn parse_vcf_phased_limited<P: AsRef<Path>>(
        &self,
        vcf_path: P,
        chr_id: u8,
        max_variants: Option<usize>,
    ) -> Result<VcfChromosome, String> {
        self.parse_vcf_impl(vcf_path, chr_id, max_variants, true)
    }

    fn parse_vcf_impl<P: AsRef<Path>>(
        &self,
        vcf_path: P,
        chr_id: u8,
        max_variants: Option<usize>,
        capture_phase: bool,
    ) -> Result<VcfChromosome, String> {
        let path = vcf_path.as_ref();

        if self.verbose {
            println!("[*] Opening VCF: {}", path.display());
        }

        // Open and decompress. Real VCF.gz distributions (including
        // 1000 Genomes) are BGZF: many concatenated gzip members, not one
        // stream. A plain GzDecoder silently stops at the end of the
        // first member (a few KB in, right after the header) and reports
        // success with only a handful of records parsed — MultiGzDecoder
        // is required to read the whole file.
        let file = File::open(path)
            .map_err(|e| format!("Failed to open VCF: {}", e))?;
        let decoder = MultiGzDecoder::new(file);
        let reader = BufReader::new(decoder);

        let mut sample_names = Vec::new();
        let mut snps = Vec::new();
        let mut all_genotypes = Vec::new();
        let mut all_hap_a = Vec::new();
        let mut all_hap_b = Vec::new();
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
                if capture_phase {
                    all_hap_a = vec![BitstreamGenotypes::new(sample_names.len()); 10000];
                    all_hap_b = vec![BitstreamGenotypes::new(sample_names.len()); 10000];
                }

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
                if capture_phase {
                    all_hap_a.push(BitstreamGenotypes::new(sample_names.len()));
                    all_hap_b.push(BitstreamGenotypes::new(sample_names.len()));
                }
            }

            let mut snp_genotypes = BitstreamGenotypes::new(sample_names.len());
            let mut snp_hap_a = if capture_phase {
                Some(BitstreamGenotypes::new(sample_names.len()))
            } else {
                None
            };
            let mut snp_hap_b = if capture_phase {
                Some(BitstreamGenotypes::new(sample_names.len()))
            } else {
                None
            };

            // Parse genotypes (starting at column 9)
            for (sample_idx, sample_data) in parts[9..].iter().enumerate() {
                let genotype = self.parse_genotype(sample_data);
                snp_genotypes.set(sample_idx, genotype);

                if let (Some(hap_a), Some(hap_b)) = (snp_hap_a.as_mut(), snp_hap_b.as_mut()) {
                    let (a, b) = self.parse_genotype_phased(sample_data);
                    hap_a.set(sample_idx, a);
                    hap_b.set(sample_idx, b);
                }
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
            if capture_phase {
                all_hap_a[snp_count as usize] = snp_hap_a.unwrap();
                all_hap_b[snp_count as usize] = snp_hap_b.unwrap();
            }
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

            if let Some(limit) = max_variants {
                if snp_count as usize >= limit {
                    if self.verbose {
                        println!("[*] Reached max_variants={}, stopping early", limit);
                    }
                    break;
                }
            }
        }

        // Trim unused genotype storage
        all_genotypes.truncate(snp_count as usize);
        if capture_phase {
            all_hap_a.truncate(snp_count as usize);
            all_hap_b.truncate(snp_count as usize);
        }

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
            hap_a: all_hap_a,
            hap_b: all_hap_b,
        })
    }

    /// Parse genotype from VCF format (e.g., "0/1", "1|1", "./.", etc.)
    fn parse_genotype(&self, gt_str: &str) -> u8 {
        if gt_str.is_empty() || gt_str.starts_with(".") {
            return 3;  // Missing
        }

        let parts: Vec<&str> = gt_str.split(['/', '|']).collect();
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

    /// Parse a genotype into its two haplotype-resolved alleles
    /// (allele_on_hap_a, allele_on_hap_b), each 0 (ref), 1 (alt), or 3
    /// (missing on that copy). "hap_a"/"hap_b" follow the order the two
    /// alleles are written in the GT field -- for phased calls ("0|1")
    /// that is the true maternal/paternal chromosome copy; for unphased
    /// calls ("0/1") there is no real phase information to recover, and
    /// this just assigns first-written/second-written, which does not
    /// correspond to a real chromosome copy. Real 1000 Genomes phase3
    /// VCFs are fully phased, so this caveat doesn't affect that data.
    fn parse_genotype_phased(&self, gt_str: &str) -> (u8, u8) {
        if gt_str.is_empty() || gt_str.starts_with('.') {
            return (3, 3);
        }

        let parts: Vec<&str> = gt_str.split(['/', '|']).collect();
        if parts.len() < 2 {
            return (3, 3);
        }

        let allele1: u8 = parts[0].parse().unwrap_or(255);
        let allele2: u8 = parts[1].parse().unwrap_or(255);

        if allele1 == 255 || allele2 == 255 {
            return (3, 3);
        }

        let a = if allele1 > 0 { 1 } else { 0 };
        let b = if allele2 > 0 { 1 } else { 0 };
        (a, b)
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

    fn write_gz_fixture(text: &str) -> std::path::PathBuf {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};

        // cargo test runs tests in parallel threads within one process, so
        // process id alone is not a unique filename -- two tests racing on
        // the same path would each see a mix of the other's content.
        static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut buf = Vec::new();
        {
            let mut enc = GzEncoder::new(&mut buf, Compression::default());
            enc.write_all(text.as_bytes()).unwrap();
            enc.finish().unwrap();
        }
        let path = std::env::temp_dir().join(format!(
            "ntg_kernel_test_fixture_{}_{}.vcf.gz",
            std::process::id(),
            unique
        ));
        std::fs::write(&path, &buf).unwrap();
        path
    }

    #[test]
    fn test_parse_vcf_phased_limited_captures_haplotypes() {
        let vcf = "##fileformat=VCFv4.1\n\
             #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\n\
             1\t100\trs1\tA\tG\t100\tPASS\t.\tGT\t0|1\t1|0\n\
             1\t200\trs2\tA\tG\t100\tPASS\t.\tGT\t1|1\t0|0\n";
        let path = write_gz_fixture(vcf);

        let parser = VcfParser::new(false);
        let chromosome = parser.parse_vcf_phased_limited(&path, 1, None).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(chromosome.snps.len(), 2);
        assert_eq!(chromosome.hap_a.len(), 2);
        assert_eq!(chromosome.hap_b.len(), 2);

        // rs1: S1 = 0|1, S2 = 1|0
        assert_eq!(chromosome.hap_a[0].get(0), 0);
        assert_eq!(chromosome.hap_b[0].get(0), 1);
        assert_eq!(chromosome.hap_a[0].get(1), 1);
        assert_eq!(chromosome.hap_b[0].get(1), 0);
        // Collapsed genotype still agrees: both are het (allele count 1).
        assert_eq!(chromosome.genotypes[0].get(0), 1);
        assert_eq!(chromosome.genotypes[0].get(1), 1);

        // rs2: S1 = 1|1, S2 = 0|0
        assert_eq!(chromosome.hap_a[1].get(0), 1);
        assert_eq!(chromosome.hap_b[1].get(0), 1);
        assert_eq!(chromosome.hap_a[1].get(1), 0);
        assert_eq!(chromosome.hap_b[1].get(1), 0);
        assert_eq!(chromosome.genotypes[1].get(0), 2);
        assert_eq!(chromosome.genotypes[1].get(1), 0);
    }

    #[test]
    fn test_parse_vcf_limited_leaves_haplotypes_empty() {
        let vcf = "##fileformat=VCFv4.1\n\
             #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\n\
             1\t100\trs1\tA\tG\t100\tPASS\t.\tGT\t0|1\n";
        let path = write_gz_fixture(vcf);

        let parser = VcfParser::new(false);
        let chromosome = parser.parse_vcf_limited(&path, 1, None).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(chromosome.snps.len(), 1);
        assert!(chromosome.hap_a.is_empty(), "unphased parse should not pay for haplotype storage");
        assert!(chromosome.hap_b.is_empty());
    }

    /// Real VCF.gz distributions (1000 Genomes included) are BGZF: many
    /// concatenated gzip members, not a single stream. A GzDecoder alone
    /// silently stops after the first member; this constructs a two-member
    /// gzip file the same way and asserts both members get read.
    #[test]
    fn test_multi_member_gzip_is_fully_decoded() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let header = "##fileformat=VCFv4.1\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\n";
        let data_line = "1\t100\trs1\tA\tG\t100\tPASS\t.\tGT\t0/1\n";

        let mut member1 = Vec::new();
        {
            let mut enc = GzEncoder::new(&mut member1, Compression::default());
            enc.write_all(header.as_bytes()).unwrap();
            enc.finish().unwrap();
        }

        let mut member2 = Vec::new();
        {
            let mut enc = GzEncoder::new(&mut member2, Compression::default());
            enc.write_all(data_line.as_bytes()).unwrap();
            enc.finish().unwrap();
        }

        let mut combined = member1;
        combined.extend_from_slice(&member2);

        let tmp_path = std::env::temp_dir().join("ntg_kernel_test_multimember.vcf.gz");
        std::fs::write(&tmp_path, &combined).unwrap();

        let parser = VcfParser::new(false);
        let chromosome = parser.parse_vcf(&tmp_path, 1).unwrap();

        std::fs::remove_file(&tmp_path).ok();

        assert_eq!(chromosome.sample_names, vec!["S1".to_string()]);
        assert_eq!(chromosome.snps.len(), 1, "data line from the second gzip member was not read");
        assert_eq!(chromosome.snps[0].id, "rs1");
    }

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
