/// Genomic Data Loader
/// Handles ingestion of genomic data from various formats
/// (VCF, PLINK, CSV, JSON)

use super::genomic::GenomicNode;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Genomic data formats supported
#[derive(Debug, Clone, Copy)]
pub enum GenomicFormat {
    Vcf,           // VCF (Variant Call Format)
    Plink,         // PLINK format
    Csv,           // CSV (SNP-by-sample)
    Json,          // JSON array format
}

/// Raw genotype record (before loading into operator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenotypeRecord {
    pub snp_id: String,
    pub position: u32,
    pub genotypes: Vec<u8>, // 0, 1, 2, or 3 (missing)
}

/// Genomic data batch for ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicBatch {
    pub num_individuals: usize,
    pub num_snps: usize,
    pub records: Vec<GenotypeRecord>,
}

impl GenomicBatch {
    /// Load from CSV format
    /// Expected format: SNP_ID,POS,IND1_GENOTYPE,IND2_GENOTYPE,...
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        let mut num_individuals = 0;

        for (line_no, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Line {}: {}", line_no, e))?;

            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                continue;
            }

            let snp_id = parts[0].to_string();
            let position = parts[1]
                .parse::<u32>()
                .map_err(|_| format!("Invalid position at line {}", line_no))?;

            let genotypes: Result<Vec<u8>, _> = parts[2..]
                .iter()
                .map(|g| {
                    g.trim().parse::<u8>().map_err(|_| {
                        format!("Invalid genotype '{}' at line {}", g, line_no)
                    })
                })
                .collect();

            let genotypes = genotypes?;

            if num_individuals == 0 {
                num_individuals = genotypes.len();
            } else if genotypes.len() != num_individuals {
                return Err(format!(
                    "Inconsistent number of individuals at line {}",
                    line_no
                ));
            }

            records.push(GenotypeRecord {
                snp_id,
                position,
                genotypes,
            });
        }

        Ok(GenomicBatch {
            num_individuals,
            num_snps: records.len(),
            records,
        })
    }

    /// Load from JSON format
    pub fn from_json<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let batch: GenomicBatch =
            serde_json::from_reader(file).map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(batch)
    }

    /// Load into GenomicOperator
    pub fn load_into_operator(&self) -> Result<GenomicNode, String> {
        if self.num_individuals == 0 || self.num_snps == 0 {
            return Err("Empty batch".to_string());
        }

        let mut node = GenomicNode::new(0, self.num_individuals, self.num_snps);

        for (snp_idx, record) in self.records.iter().enumerate() {
            node.operator.variant_names[snp_idx] = record.snp_id.clone();
            node.operator.positions[snp_idx] = record.position;

            for (ind_idx, &genotype) in record.genotypes.iter().enumerate() {
                node.operator.set(snp_idx, ind_idx, genotype);
            }
        }

        // Compute statistics
        node.operator.compute_statistics();

        Ok(node)
    }

    /// Save to JSON
    pub fn to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        serde_json::to_writer_pretty(file, self)
            .map_err(|e| format!("JSON write error: {}", e))?;
        Ok(())
    }
}

/// Genomic data ingestion pipeline
pub struct GenomicPipeline {
    pub batches: Vec<GenomicBatch>,
}

impl GenomicPipeline {
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
        }
    }

    /// Load a batch from file
    pub fn load_batch<P: AsRef<Path>>(
        &mut self,
        path: P,
        format: GenomicFormat,
    ) -> Result<(), String> {
        let batch = match format {
            GenomicFormat::Csv => GenomicBatch::from_csv(path)?,
            GenomicFormat::Json => GenomicBatch::from_json(path)?,
            _ => return Err("Format not yet implemented".to_string()),
        };

        self.batches.push(batch);
        Ok(())
    }

    /// Merge all batches into single operator
    pub fn merge_batches(&self) -> Result<GenomicNode, String> {
        if self.batches.is_empty() {
            return Err("No batches loaded".to_string());
        }

        // Check all batches have same number of individuals
        let num_individuals = self.batches[0].num_individuals;
        for batch in &self.batches {
            if batch.num_individuals != num_individuals {
                return Err("Batch size mismatch".to_string());
            }
        }

        let total_snps: usize = self.batches.iter().map(|b| b.num_snps).sum();
        let mut merged = GenomicNode::new(0, num_individuals, total_snps);

        let mut snp_offset = 0;
        for batch in &self.batches {
            for (batch_snp_idx, record) in batch.records.iter().enumerate() {
                let global_snp_idx = snp_offset + batch_snp_idx;
                merged.operator.variant_names[global_snp_idx] = record.snp_id.clone();
                merged.operator.positions[global_snp_idx] = record.position;

                for (ind_idx, &genotype) in record.genotypes.iter().enumerate() {
                    merged.operator.set(global_snp_idx, ind_idx, genotype);
                }
            }
            snp_offset += batch.num_snps;
        }

        merged.operator.compute_statistics();
        Ok(merged)
    }

    /// Get summary of loaded data
    pub fn summary(&self) -> GenomicLoadSummary {
        GenomicLoadSummary {
            num_batches: self.batches.len(),
            total_snps: self.batches.iter().map(|b| b.num_snps).sum(),
            total_individuals: self
                .batches
                .first()
                .map(|b| b.num_individuals)
                .unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicLoadSummary {
    pub num_batches: usize,
    pub total_snps: usize,
    pub total_individuals: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genomic_batch_creation() {
        let batch = GenomicBatch {
            num_individuals: 10,
            num_snps: 5,
            records: vec![],
        };

        assert_eq!(batch.num_individuals, 10);
        assert_eq!(batch.num_snps, 5);
    }

    #[test]
    fn test_pipeline_summary() {
        let mut pipeline = GenomicPipeline::new();
        let batch = GenomicBatch {
            num_individuals: 100,
            num_snps: 50,
            records: vec![],
        };

        pipeline.batches.push(batch);
        let summary = pipeline.summary();

        assert_eq!(summary.num_batches, 1);
        assert_eq!(summary.total_snps, 50);
        assert_eq!(summary.total_individuals, 100);
    }
}
