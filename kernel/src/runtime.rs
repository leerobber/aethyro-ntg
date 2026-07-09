//! Runtime execution engine for the Aethyro NTG (Neural Ternary Graph) Engine.
//!
//! Provides the core forward pass execution with native parallel processing
//! using pre-partitioned arenas for zero-synchronization multi-core scaling.
//!
//! Key invariants:
//! - Node IDs within a layer must be dense and sequential starting at 0
//! - This enables pre-partitioned arena allocation without post-parallel sorting
//! - All mutations flow through ledgered paths for tamper-evidence

use std::thread;

use crate::storage::sparse_bit_sliced_ternary::{BitSlicedBlock, SparseBitSlicedTernary};

/// A layer in the neural ternary graph.
#[derive(Clone, Debug)]
pub struct Layer {
    pub nodes: Vec<GraphNode>,
}

impl Layer {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }
}

/// A graph node with ternary weights.
#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: u64,
    pub weights: SparseBitSlicedTernary,
}

impl GraphNode {
    pub fn new(id: u64, weight_len: usize, max_active_chunks: usize) -> Self {
        Self {
            id,
            weights: SparseBitSlicedTernary::with_capacity(weight_len, max_active_chunks),
        }
    }

    /// Primary mutation entry point for self-evolving topology / weight updates.
    pub fn update_weight(&mut self, idx: usize, val: i8, ledger: &mut ChronosLedger) {
        let _entry = self.weights.apply_structural_mutation(idx, val, ledger);
    }

    /// Telemetry exposure for orchestration loop and Reflexive Fitness Evaluators
    #[inline(always)]
    pub fn tombstone_count(&self) -> usize {
        self.weights.tombstone_count
    }

    #[inline(always)]
    pub fn density(&self) -> f32 {
        self.weights.density()
    }

    #[inline(always)]
    pub fn fitness_signal(&self) -> f32 {
        self.weights.fitness_signal()
    }

    /// Read-only sparse scoring path used by TRA / multi-agent orchestration
    pub fn score_against(&self, other: &Self) -> i64 {
        SparseBitSlicedTernary::dot_product_sparse(&self.weights, &other.weights)
    }

    /// Batch scoring surface for multi-agent / large-context orchestration
    pub fn score_against_batch(&self, others: &[&GraphNode]) -> Vec<i64> {
        others.iter()
            .map(|other| SparseBitSlicedTernary::dot_product_sparse(&self.weights, &other.weights))
            .collect()
    }

    /// Parallel batch scoring across independent nodes.
    pub fn par_score_against_batch(&self, others: &[&GraphNode]) -> Vec<i64> {
        if others.is_empty() {
            return Vec::new();
        }

        // Single master allocation matching the target batch dimension
        let mut results = vec![0i64; others.len()];
        let worker_count = thread::available_parallelism().map(|p| p.get()).unwrap_or(4);
        let chunk_size = (others.len() / worker_count).max(1);

        thread::scope(|s| {
            // Divide input arrays and mutable outputs into matching, non-overlapping slices
            let others_chunks = others.chunks(chunk_size);
            let results_chunks = results.chunks_mut(chunk_size);

            for (other_chunk, result_chunk) in others_chunks.zip(results_chunks) {
                s.spawn(move || {
                    for (other, res) in other_chunk.iter().zip(result_chunk.iter_mut()) {
                        *res = SparseBitSlicedTernary::dot_product_sparse(&self.weights, &other.weights);
                    }
                });
            }
        }); // Implicit scoped barrier ensures all threads synchronize here

        results
    }
}

/// The main runtime engine for executing the neural ternary graph.
#[derive(Clone, Debug)]
pub struct Runtime {
    pub layers: Vec<Layer>,
}

impl Runtime {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Native parallel forward pass using pre-partitioned arena.
    ///
    /// This is the primary execution path for the NTG Engine.
    /// It leverages the contract that node.id == node_idx within each layer
    /// to enable zero-synchronization parallel execution.
    ///
    /// # Arguments
    /// * `layer_idx` - Index of the layer to execute
    /// * `input_activations` - Input activations as a sparse bit-sliced ternary tensor
    /// * `threshold` - Activation threshold for ternary matmul
    ///
    /// # Returns
    /// Output activations as a sparse bit-sliced ternary tensor
    pub fn forward_native_parallel(
        &self,
        layer_idx: usize,
        input_activations: &SparseBitSlicedTernary,
        threshold: i64,
    ) -> SparseBitSlicedTernary {
        let layer = &self.layers[layer_idx];
        if layer.is_empty() {
            return SparseBitSlicedTernary::new(0);
        }

        // Pre-partitioned arena: one slot per node in topological order
        let mut shared_blocks: Vec<(u32, BitSlicedBlock)> =
            vec![(0u32, BitSlicedBlock::default()); layer.len()];

        let worker_count = thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let chunk_size = (layer.len() / worker_count).max(1);

        thread::scope(|s| {
            for (node_chunk, output_chunk) in
                layer.nodes.chunks(chunk_size).zip(shared_blocks.chunks_mut(chunk_size))
            {
                s.spawn(move || {
                    for (local_idx, node) in node_chunk.iter().enumerate() {
                        let node_output = SparseBitSlicedTernary::ternary_matmul(
                            &node.weights,
                            input_activations,
                            threshold,
                        );

                        let slot = &mut output_chunk[local_idx];
                        slot.0 = node.id as u32;

                        if let Some((_, block)) = node_output.blocks.first() {
                            slot.1 = *block;
                        } else {
                            slot.1 = BitSlicedBlock::default();
                        }
                    }
                });
            }
        });

        // Linear tombstone removal (no sort required if node IDs are dense + sequential)
        shared_blocks.retain(|(_, block)| block.pos != 0 || block.neg != 0);

        let mut output = SparseBitSlicedTernary::with_capacity(
            layer.len() * 64,
            shared_blocks.len(),
        );
        output.blocks = shared_blocks;
        output.compute_density();
        output
    }

    /// Execute a forward pass with residual connections.
    ///
    /// This combines the standard forward pass with a residual addition,
    /// enabling ResNet-style architectures.
    pub fn forward_with_residual(
        &self,
        layer_idx: usize,
        input_activations: &SparseBitSlicedTernary,
        threshold: i64,
        residual: &SparseBitSlicedTernary,
    ) -> SparseBitSlicedTernary {
        let output = self.forward_native_parallel(layer_idx, input_activations, threshold);
        SparseBitSlicedTernary::sparse_residual_add(&output, residual)
    }
}

// Re-export ledger types for convenience
pub use crate::storage::sparse_bit_sliced_ternary::{ChronosLedger, LedgerEntry, MutationHash};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(runtime.layers.is_empty());
    }

    #[test]
    fn test_layer_creation() {
        let layer = Layer::new();
        assert!(layer.is_empty());
        assert_eq!(layer.len(), 0);
    }

    #[test]
    fn test_graph_node_creation() {
        let node = GraphNode::new(0, 100, 16);
        assert_eq!(node.id, 0);
        assert_eq!(node.weights.len, 100);
    }

    #[test]
    fn test_empty_forward_pass() {
        let mut runtime = Runtime::new();
        let layer = Layer::new();
        runtime.add_layer(layer);
        
        let input = SparseBitSlicedTernary::new(100);
        let output = runtime.forward_native_parallel(0, &input, 0);
        
        assert_eq!(output.len, 0);
    }
}
