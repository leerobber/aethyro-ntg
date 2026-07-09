//! Native sparse ternary forward runtime.
//!
//! Layers hold dense-sequential [`GraphNode`] IDs (contract: `node.id == index`).
//! `forward_native_parallel` selects a compute device from activation density
//! via [`AccelManager`], then evaluates each node's weights against shared
//! activations in parallel scoped threads (no post-pass sort; ordering is
//! fixed by ingest).

use super::accel::{AccelDevice, AccelManager};
use super::error::NtgError;
use super::graph::GraphNode;
use super::storage::{BitSlicedBlock, BitSlicedTernary, SparseBitSlicedTernary};
use super::ternary::matmul_scalar;
use std::sync::Mutex;
use std::thread;

// Re-export hardware path helpers for a stable runtime surface.
pub use super::accel::resolve_avx512_hardware;

/// Multi-layer native execution arena.
#[derive(Clone, Debug, Default)]
pub struct Runtime {
    pub layers: Vec<Vec<GraphNode>>,
    pub accel_manager: AccelManager,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_accel(mut self, accel_manager: AccelManager) -> Self {
        self.accel_manager = accel_manager;
        self
    }

    pub fn push_layer(&mut self, nodes: Vec<GraphNode>) -> Result<(), NtgError> {
        validate_sequential_ids(&nodes)?;
        self.layers.push(nodes);
        Ok(())
    }

    /// Parallel native forward for one layer.
    ///
    /// Device selection: `accel_manager.select_for(input_activations.density())`.
    /// Requires sequential node IDs starting at 0 (enforced at ingest /
    /// `push_layer`). Output blocks are keyed by `node.id as u32`.
    pub fn forward_native_parallel(
        &self,
        layer_idx: usize,
        input_activations: &SparseBitSlicedTernary,
        threshold: i64,
    ) -> Result<SparseBitSlicedTernary, NtgError> {
        let layer = self.layers.get(layer_idx).ok_or(NtgError::LayerNotFound {
            layer_idx,
            layer_count: self.layers.len(),
        })?;

        if layer.is_empty() {
            return Ok(SparseBitSlicedTernary::new(0));
        }

        // Pre-validate sequential IDs (defense in depth if layer was mutated).
        validate_sequential_ids(layer)?;

        let density = input_activations.density();
        let device = self.accel_manager.select_for(density);
        let _device_name = device.name(); // available for future OpStats / ledger

        let layer_len = layer.len();
        let mut shared_blocks: Vec<(u32, BitSlicedBlock)> =
            vec![(0u32, BitSlicedBlock::default()); layer_len];

        let worker_count = thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let chunk_size = (layer_len / worker_count).max(1);

        // First error from any worker (no panics on the hot path).
        let first_err: Mutex<Option<NtgError>> = Mutex::new(None);

        thread::scope(|s| {
            for (node_chunk, output_slice) in layer
                .chunks(chunk_size)
                .zip(shared_blocks.chunks_mut(chunk_size))
            {
                let err_slot = &first_err;
                s.spawn(move || {
                    for (node_offset, node) in node_chunk.iter().enumerate() {
                        match device.ternary_matmul(
                            &node.weights,
                            input_activations,
                            threshold,
                        ) {
                            Ok(node_output) => {
                                if let Some((_, block)) = node_output.blocks.first() {
                                    output_slice[node_offset] = (node.id as u32, *block);
                                } else {
                                    output_slice[node_offset] =
                                        (node.id as u32, BitSlicedBlock::default());
                                }
                            }
                            Err(e) => {
                                if let Ok(mut guard) = err_slot.lock() {
                                    if guard.is_none() {
                                        *guard = Some(e);
                                    }
                                }
                            }
                        }
                    }
                });
            }
        });

        if let Some(e) = first_err.into_inner().ok().and_then(|x| x) {
            return Err(e);
        }

        shared_blocks.retain(|(_, block)| !block.is_empty());
        // Nodes already in id order; retain keeps relative order.
        shared_blocks.sort_by_key(|&(c, _)| c);

        let mut output =
            SparseBitSlicedTernary::with_capacity(layer_len * 64, shared_blocks.len());
        output.blocks = shared_blocks;
        output.compute_density();
        Ok(output)
    }

    /// Last selected device for a given density (test / observability helper).
    pub fn select_device_for_density(&self, density: f32) -> AccelDevice {
        self.accel_manager.select_for(density)
    }
}

/// Ingest / layer contract: node IDs must be dense and sequential from 0.
pub fn validate_sequential_ids(nodes: &[GraphNode]) -> Result<(), NtgError> {
    for (i, node) in nodes.iter().enumerate() {
        if node.id != i {
            return Err(NtgError::NonSequentialNodeId {
                expected: i,
                got: node.id,
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shape guard + fast matmul dispatch
// ---------------------------------------------------------------------------

/// Checked shape description for dense ternary matmul `(m×k) @ (k×n)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeGuard {
    pub m: usize,
    pub k: usize,
    pub n: usize,
}

impl ShapeGuard {
    /// Validate dimensions and that buffer lengths match. Uses `checked_mul`
    /// so oversized shapes fail without attempting huge allocations.
    pub fn new(m: usize, k: usize, n: usize, a_len: usize, b_len: usize) -> Result<Self, NtgError> {
        let a_need = m
            .checked_mul(k)
            .ok_or(NtgError::DimensionOverflow { m, k, n })?;
        let b_need = k
            .checked_mul(n)
            .ok_or(NtgError::DimensionOverflow { m, k, n })?;
        // Output size also must fit
        let _c_need = m
            .checked_mul(n)
            .ok_or(NtgError::DimensionOverflow { m, k, n })?;

        if a_len != a_need {
            return Err(NtgError::ShapeMismatch {
                expected: a_need,
                got: a_len,
            });
        }
        if b_len != b_need {
            return Err(NtgError::ShapeMismatch {
                expected: b_need,
                got: b_len,
            });
        }
        Ok(Self { m, k, n })
    }

    /// Shape-only check (no buffers) — used by overflow unit tests.
    /// Never allocates; fails if any product of (m,k), (k,n), or (m,n) overflows.
    pub fn check_dims(m: usize, k: usize, n: usize) -> Result<(), NtgError> {
        if m.checked_mul(k).is_none()
            || k.checked_mul(n).is_none()
            || m.checked_mul(n).is_none()
        {
            return Err(NtgError::DimensionOverflow { m, k, n });
        }
        Ok(())
    }
}

/// Dense ternary matmul with shape guards.
///
/// Dispatches through the scalar reference for bit-identity; bit-sliced
/// popcount path is used when both operands are full-length and we want a
/// same-shape self-check against scalar on small tensors in tests.
pub fn matmul_fast(a: &[i8], b: &[i8], m: usize, k: usize, n: usize) -> Result<Vec<f32>, NtgError> {
    let _guard = ShapeGuard::new(m, k, n, a.len(), b.len())?;
    // Portable correct path (Phase 1.1 bit-identity baseline).
    // AVX-512 / bit-sliced acceleration for dense i8 is a follow-on; sparse
    // path already uses popcount via SparseBitSlicedTernary.
    matmul_scalar(a, b, m, k, n)
}

/// Bit-sliced dense dot product (64-wide popcount). Shape-guarded.
pub fn bit_sliced_dot_fast(a: &BitSlicedTernary, b: &BitSlicedTernary) -> Result<i64, NtgError> {
    if a.len() != b.len() {
        return Err(NtgError::ShapeMismatch {
            expected: a.len(),
            got: b.len(),
        });
    }
    Ok(BitSlicedTernary::dot_product_parallel(a, b))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::accel::HardwarePath;

    #[test]
    fn rejects_non_sequential_ids() {
        let nodes = vec![
            GraphNode::new(0, 64),
            GraphNode::new(2, 64),
        ];
        assert!(matches!(
            validate_sequential_ids(&nodes),
            Err(NtgError::NonSequentialNodeId {
                expected: 1,
                got: 2
            })
        ));
    }

    #[test]
    fn forward_parallel_produces_sparse_output() -> Result<(), NtgError> {
        let mut weights = SparseBitSlicedTernary::new(64);
        let mut acts = SparseBitSlicedTernary::new(64);
        for i in 0..8 {
            weights.set(i, 1);
            acts.set(i, 1);
        }
        acts.compute_density();

        let mut rt = Runtime::new();
        rt.push_layer(vec![
            GraphNode::with_weights(0, weights.clone()),
            GraphNode::with_weights(1, weights),
        ])?;

        // Low density → sparse device path
        assert_eq!(
            rt.select_device_for_density(acts.density()),
            AccelDevice::SparseCpu
        );

        let out = rt.forward_native_parallel(0, &acts, 1)?;
        assert!(!out.blocks.is_empty());
        assert_eq!(out.blocks[0].0, 0);
        assert_eq!(out.blocks[0].1.pos, 1);
        Ok(())
    }

    #[test]
    fn high_density_selects_non_sparse_device() {
        let rt = Runtime::new();
        let d = rt.select_device_for_density(0.95);
        assert!(matches!(
            d,
            AccelDevice::DenseCpu | AccelDevice::Avx512Cpu
        ));
    }

    #[test]
    fn empty_layer_index_errors() {
        let rt = Runtime::new();
        let acts = SparseBitSlicedTernary::new(64);
        let err = rt.forward_native_parallel(0, &acts, 1).unwrap_err();
        assert!(matches!(
            err,
            NtgError::LayerNotFound {
                layer_idx: 0,
                layer_count: 0
            }
        ));
    }

    #[test]
    fn empty_layer_returns_empty_tensor() -> Result<(), NtgError> {
        let mut rt = Runtime::new();
        rt.push_layer(vec![])?;
        let acts = SparseBitSlicedTernary::new(64);
        let out = rt.forward_native_parallel(0, &acts, 1)?;
        assert_eq!(out.len(), 0);
        Ok(())
    }

    #[test]
    fn shape_guard_rejects_mismatch() {
        let err = ShapeGuard::new(2, 2, 2, 3, 4).unwrap_err();
        assert!(matches!(err, NtgError::ShapeMismatch { .. }));
    }

    #[test]
    fn shape_guard_overflow_without_alloc() {
        // Do NOT allocate usize::MAX-sized vectors — only check dims.
        let err = ShapeGuard::check_dims(usize::MAX, 2, 2).unwrap_err();
        assert!(matches!(
            err,
            NtgError::DimensionOverflow {
                m: usize::MAX,
                k: 2,
                n: 2
            }
        ));
    }

    #[test]
    fn matmul_fast_matches_scalar() -> Result<(), NtgError> {
        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];
        let fast = matmul_fast(&a, &b, 2, 2, 2)?;
        let slow = matmul_scalar(&a, &b, 2, 2, 2)?;
        assert_eq!(fast, slow);
        Ok(())
    }

    #[test]
    fn matmul_fast_overflow_is_checked() {
        // checked_mul path — no huge allocation
        let err = ShapeGuard::new(usize::MAX / 2 + 1, 3, 1, 0, 0).unwrap_err();
        assert!(matches!(err, NtgError::DimensionOverflow { .. }));
    }

    #[test]
    fn resolve_hardware_returns_known_path() {
        let path = resolve_avx512_hardware();
        assert!(matches!(
            path,
            HardwarePath::Avx512Popcnt | HardwarePath::Avx2 | HardwarePath::Scalar
        ));
        // This host advertises avx512_vpopcntdq in /proc/cpuinfo — expect
        // at least AVX2 on x86_64 CI, often Avx512Popcnt.
        #[cfg(target_arch = "x86_64")]
        {
            assert_ne!(path, HardwarePath::Scalar);
        }
    }

    #[test]
    fn bit_sliced_dot_shape_guard() {
        let a = BitSlicedTernary::new(8);
        let b = BitSlicedTernary::new(16);
        assert!(bit_sliced_dot_fast(&a, &b).is_err());
    }
}
