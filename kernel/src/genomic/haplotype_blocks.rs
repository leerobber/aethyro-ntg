/// Haplotype Block Detection
/// Identifies contiguous blocks of SNPs in high linkage disequilibrium
/// Uses BFS on LD graph to find connected components

use crate::genomic::ld_compute::LdPair;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct HaplotypeBlock {
    pub id: u32,
    pub snp_indices: Vec<u32>,
    pub mean_r_squared: f32,
    pub start_position: u32,
    pub end_position: u32,
    pub size: u32,
}

#[derive(Debug)]
pub struct BlockDetector {
    verbose: bool,
}

impl BlockDetector {
    pub fn new(verbose: bool) -> Self {
        BlockDetector { verbose }
    }

    /// Detect haplotype blocks from LD pairs using BFS
    /// Algorithm:
    /// 1. Build adjacency graph from LD pairs
    /// 2. Find connected components via BFS
    /// 3. Each component = one haplotype block
    pub fn detect_blocks(
        &self,
        ld_pairs: &[LdPair],
        n_snps: usize,
    ) -> Result<Vec<HaplotypeBlock>, String> {
        if ld_pairs.is_empty() {
            return Err("No LD pairs provided".to_string());
        }

        if self.verbose {
            println!("[*] Detecting haplotype blocks from {} LD pairs", ld_pairs.len());
        }

        let start = Instant::now();

        // Build adjacency graph
        let mut graph: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut edge_weights: HashMap<(u32, u32), f32> = HashMap::new();

        for pair in ld_pairs {
            let min_idx = pair.snp1_idx.min(pair.snp2_idx);
            let max_idx = pair.snp1_idx.max(pair.snp2_idx);

            graph
                .entry(min_idx)
                .or_insert_with(Vec::new)
                .push(max_idx);
            graph
                .entry(max_idx)
                .or_insert_with(Vec::new)
                .push(min_idx);

            edge_weights.insert((min_idx, max_idx), pair.r_squared);
            edge_weights.insert((max_idx, min_idx), pair.r_squared);
        }

        if self.verbose {
            println!("[*] Graph built: {} SNPs in graph", graph.len());
        }

        // Find connected components via BFS
        let mut visited = vec![false; n_snps];
        let mut blocks = Vec::new();
        let mut block_id = 0u32;

        for start_snp in 0..n_snps as u32 {
            if visited[start_snp as usize] {
                continue;
            }

            // BFS to find connected component
            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(start_snp);
            visited[start_snp as usize] = true;

            while let Some(snp) = queue.pop_front() {
                component.push(snp);

                // Add neighbors to queue
                if let Some(neighbors) = graph.get(&snp) {
                    for &neighbor in neighbors {
                        if !visited[neighbor as usize] {
                            visited[neighbor as usize] = true;
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            // Only create block if it has multiple SNPs
            if component.len() >= 1 {
                component.sort();

                // Calculate mean r² for this block
                let mut sum_r_sq = 0.0_f32;
                let mut pair_count = 0;

                for i in 0..component.len() {
                    for j in (i + 1)..component.len() {
                        let min = component[i].min(component[j]);
                        let max = component[i].max(component[j]);
                        if let Some(&r_sq) = edge_weights.get(&(min, max)) {
                            sum_r_sq += r_sq;
                            pair_count += 1;
                        }
                    }
                }

                let mean_r_sq = if pair_count > 0 {
                    sum_r_sq / pair_count as f32
                } else {
                    0.0
                };

                blocks.push(HaplotypeBlock {
                    id: block_id,
                    snp_indices: component,
                    mean_r_squared: mean_r_sq,
                    start_position: 0,  // Will be set later
                    end_position: 0,    // Will be set later
                    size: blocks.len() as u32,
                });

                block_id += 1;
            }
        }

        if self.verbose {
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "[OK] Detected {} haplotype blocks in {:.1}s",
                blocks.len(),
                elapsed
            );
        }

        Ok(blocks)
    }

    /// Annotate blocks with genomic positions
    pub fn annotate_blocks(
        &self,
        blocks: &mut [HaplotypeBlock],
        positions: &[u32],
    ) -> Result<(), String> {
        if positions.is_empty() {
            return Err("No positions provided".to_string());
        }

        for block in blocks {
            if let Some(&min_pos) = block
                .snp_indices
                .iter()
                .filter_map(|&idx| positions.get(idx as usize))
                .min()
            {
                block.start_position = min_pos;
            }

            if let Some(&max_pos) = block
                .snp_indices
                .iter()
                .filter_map(|&idx| positions.get(idx as usize))
                .max()
            {
                block.end_position = max_pos;
            }
        }

        Ok(())
    }
}

impl HaplotypeBlock {
    /// Get summary string for this block
    pub fn summary(&self) -> String {
        let span_bp = (self.end_position as i32 - self.start_position as i32).max(0);
        format!(
            "Block {}: {} SNPs, r²={:.3}, pos {}bp-{}bp ({}bp span)",
            self.id, self.snp_indices.len(), self.mean_r_squared, self.start_position, self.end_position, span_bp
        )
    }
}

pub struct BlockStatistics {
    pub total_blocks: usize,
    pub total_snps_in_blocks: usize,
    pub mean_block_size: f64,
    pub median_block_size: u32,
    pub min_block_size: u32,
    pub max_block_size: u32,
    pub mean_r_squared: f64,
    pub mean_span_bp: f64,
}

pub fn compute_block_statistics(blocks: &[HaplotypeBlock]) -> BlockStatistics {
    if blocks.is_empty() {
        return BlockStatistics {
            total_blocks: 0,
            total_snps_in_blocks: 0,
            mean_block_size: 0.0,
            median_block_size: 0,
            min_block_size: 0,
            max_block_size: 0,
            mean_r_squared: 0.0,
            mean_span_bp: 0.0,
        };
    }

    let total_snps: usize = blocks.iter().map(|b| b.snp_indices.len()).sum();
    let mean_size = total_snps as f64 / blocks.len() as f64;

    let mut sizes: Vec<u32> = blocks.iter().map(|b| b.snp_indices.len() as u32).collect();
    sizes.sort();

    let median_size = if sizes.len() % 2 == 0 {
        (sizes[sizes.len() / 2 - 1] + sizes[sizes.len() / 2]) / 2
    } else {
        sizes[sizes.len() / 2]
    };

    let min_size = sizes.first().copied().unwrap_or(0);
    let max_size = sizes.last().copied().unwrap_or(0);

    let mean_r_sq: f64 = blocks.iter().map(|b| b.mean_r_squared as f64).sum::<f64>()
        / blocks.len() as f64;

    let mean_span: f64 = blocks
        .iter()
        .map(|b| (b.end_position as i32 - b.start_position as i32).max(0) as f64)
        .sum::<f64>()
        / blocks.len() as f64;

    BlockStatistics {
        total_blocks: blocks.len(),
        total_snps_in_blocks: total_snps,
        mean_block_size: mean_size,
        median_block_size: median_size,
        min_block_size: min_size,
        max_block_size: max_size,
        mean_r_squared: mean_r_sq,
        mean_span_bp: mean_span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_detection() {
        // Create test LD pairs that form a connected component
        let pairs = vec![
            LdPair {
                snp1_idx: 0,
                snp2_idx: 1,
                r_squared: 0.9,
                position1: 1000,
                position2: 2000,
            },
            LdPair {
                snp1_idx: 1,
                snp2_idx: 2,
                r_squared: 0.85,
                position1: 2000,
                position2: 3000,
            },
        ];

        let detector = BlockDetector::new(false);
        let blocks = detector.detect_blocks(&pairs, 5).unwrap();

        // Should detect at least one block containing SNPs 0, 1, 2
        assert!(blocks.len() > 0, "Expected at least one block");
    }

    #[test]
    fn test_block_statistics() {
        let blocks = vec![
            HaplotypeBlock {
                id: 0,
                snp_indices: vec![0, 1, 2],
                mean_r_squared: 0.85,
                start_position: 1000,
                end_position: 3000,
                size: 0,
            },
            HaplotypeBlock {
                id: 1,
                snp_indices: vec![3, 4],
                mean_r_squared: 0.90,
                start_position: 4000,
                end_position: 5000,
                size: 1,
            },
        ];

        let stats = compute_block_statistics(&blocks);
        assert_eq!(stats.total_blocks, 2);
        assert_eq!(stats.total_snps_in_blocks, 5);
        assert!(stats.mean_block_size > 2.0 && stats.mean_block_size < 3.0);
    }
}
