//! Sparse bit-sliced ternary: flat COO blocks with lazy tombstone compact.
//!
//! Only non-zero 64-element chunks are stored as `Vec<(u32, BitSlicedBlock)>`
//! sorted by chunk id. Zero regions never touch the memory bus during merge
//! joins. Structural mutations log to [`TamperEvidentLedger`].
//!
//! Compaction is lazy: zeroing a weight leaves a tombstone until
//! `tombstone_count` crosses [`COMPACT_THRESHOLD`] (or an explicit
//! `compact` call).

use crate::ntg::error::NtgError;
use crate::ntg::ledger::{
    replay::ExecutionTrace, FitnessMeasure, MutationOutcome, TamperEvidentLedger,
};

/// Lazy-compact trigger: prune tombstones after this many dead blocks.
pub const COMPACT_THRESHOLD: usize = 32;

/// One 64-wide dual-stream block (pos = +1 bits, neg = -1 bits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BitSlicedBlock {
    pub pos: u64,
    pub neg: u64,
}

impl BitSlicedBlock {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos == 0 && self.neg == 0
    }

    #[inline]
    pub fn nonzero_count(&self) -> u32 {
        self.pos.count_ones() + self.neg.count_ones()
    }
}

/// Sparse arena-backed ternary tensor (flat COO, sorted by chunk id).
#[derive(Clone, Debug)]
pub struct SparseBitSlicedTernary {
    /// Sorted by chunk id; empty blocks are tombstones until compact.
    pub blocks: Vec<(u32, BitSlicedBlock)>,
    pub len: usize,
    pub density: f32,
    pub last_op_cycles: u64,
    /// Number of zeroed blocks still present in `blocks`.
    pub tombstone_count: usize,
}

impl SparseBitSlicedTernary {
    pub fn with_capacity(len: usize, max_active_chunks: usize) -> Self {
        Self {
            blocks: Vec::with_capacity(max_active_chunks),
            len,
            density: 0.0,
            last_op_cycles: 0,
            tombstone_count: 0,
        }
    }

    pub fn new(len: usize) -> Self {
        Self::with_capacity(len, 1024)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn active_block_count(&self) -> usize {
        self.blocks.len().saturating_sub(self.tombstone_count)
    }

    /// Cached density in `[0, 1]`. Call [`compute_density`] after mutations
    /// that should refresh the value used by accel selection.
    #[inline]
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Set a single ternary weight. Common case: in-place update / tombstone.
    /// New chunks insert at the sorted position (O(n) shift — rare path).
    #[inline]
    pub fn set(&mut self, idx: usize, val: i8) {
        debug_assert!(idx < self.len);
        debug_assert!((-1..=1).contains(&val));
        let chunk = (idx / 64) as u32;
        let bit = idx % 64;
        let mask = 1u64 << bit;

        match self.blocks.binary_search_by_key(&chunk, |&(c, _)| c) {
            Ok(i) => {
                let entry = &mut self.blocks[i].1;
                let was_tombstone = entry.is_empty();

                match val {
                    1 => {
                        entry.pos |= mask;
                        entry.neg &= !mask;
                    }
                    -1 => {
                        entry.neg |= mask;
                        entry.pos &= !mask;
                    }
                    0 => {
                        entry.pos &= !mask;
                        entry.neg &= !mask;
                    }
                    _ => unreachable!(),
                }

                let is_tombstone = entry.is_empty();
                if !was_tombstone && is_tombstone {
                    self.tombstone_count += 1;
                } else if was_tombstone && !is_tombstone {
                    self.tombstone_count = self.tombstone_count.saturating_sub(1);
                }
            }
            Err(i) if val != 0 => {
                let mut entry = BitSlicedBlock::default();
                if val == 1 {
                    entry.pos |= mask;
                } else {
                    entry.neg |= mask;
                }
                self.blocks.insert(i, (chunk, entry));
            }
            _ => {}
        }
    }

    #[inline]
    pub fn get(&self, idx: usize) -> i8 {
        debug_assert!(idx < self.len);
        let chunk = (idx / 64) as u32;
        let bit = idx % 64;
        let mask = 1u64 << bit;
        match self.blocks.binary_search_by_key(&chunk, |&(c, _)| c) {
            Ok(i) => {
                let b = &self.blocks[i].1;
                if b.pos & mask != 0 {
                    1
                } else if b.neg & mask != 0 {
                    -1
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    pub fn from_slice(vals: &[i8]) -> Self {
        let mut t = Self::new(vals.len());
        for (i, &v) in vals.iter().enumerate() {
            if v != 0 {
                t.set(i, v);
            }
        }
        t.compute_density();
        t
    }

    /// Sparse merge-join dot product. Unmatched / zero chunks contribute 0.
    /// Tombstones evaluate to 0 without an explicit branch (AND of zeros).
    pub fn dot_product_sparse(a: &Self, b: &Self) -> i64 {
        debug_assert_eq!(a.len, b.len);
        let mut total: i64 = 0;
        let mut a_idx = 0;
        let mut b_idx = 0;

        while a_idx < a.blocks.len() && b_idx < b.blocks.len() {
            let (a_chunk, a_block) = &a.blocks[a_idx];
            let (b_chunk, b_block) = &b.blocks[b_idx];

            if a_chunk == b_chunk {
                let pp = (a_block.pos & b_block.pos).count_ones() as i64;
                let mm = (a_block.neg & b_block.neg).count_ones() as i64;
                let pm = (a_block.pos & b_block.neg).count_ones() as i64;
                let mp = (a_block.neg & b_block.pos).count_ones() as i64;
                total += (pp + mm) - (pm + mp);
                a_idx += 1;
                b_idx += 1;
            } else if a_chunk < b_chunk {
                a_idx += 1;
            } else {
                b_idx += 1;
            }
        }
        total
    }

    /// Chunk-level ternary interaction: for each common chunk, score the
    /// 64-wide bit-gate product; emit +1 / -1 / omit based on `threshold`.
    ///
    /// This is the native sparse "matmul" primitive used by
    /// [`crate::ntg::runtime::Runtime::forward_native_parallel`].
    pub fn ternary_matmul(weights: &Self, activations: &Self, threshold: i64) -> Self {
        let mut output_blocks: Vec<(u32, BitSlicedBlock)> = Vec::new();
        let mut w_idx = 0;
        let mut a_idx = 0;

        while w_idx < weights.blocks.len() && a_idx < activations.blocks.len() {
            let (w_chunk, w_block) = &weights.blocks[w_idx];
            let (a_chunk, a_block) = &activations.blocks[a_idx];

            if w_chunk == a_chunk {
                let pp = (w_block.pos & a_block.pos).count_ones() as i64;
                let mm = (w_block.neg & a_block.neg).count_ones() as i64;
                let pm = (w_block.pos & a_block.neg).count_ones() as i64;
                let mp = (w_block.neg & a_block.pos).count_ones() as i64;
                let score = (pp + mm) - (pm + mp);

                // Architecture gate: |score| >= threshold → ±1 (threshold 0 → any nonzero)
                if threshold <= 0 {
                    if score > 0 {
                        output_blocks.push((*w_chunk, BitSlicedBlock { pos: 1, neg: 0 }));
                    } else if score < 0 {
                        output_blocks.push((*w_chunk, BitSlicedBlock { pos: 0, neg: 1 }));
                    }
                } else if score >= threshold {
                    output_blocks.push((*w_chunk, BitSlicedBlock { pos: 1, neg: 0 }));
                } else if score <= -threshold {
                    output_blocks.push((*w_chunk, BitSlicedBlock { pos: 0, neg: 1 }));
                }

                w_idx += 1;
                a_idx += 1;
            } else if w_chunk < a_chunk {
                w_idx += 1;
            } else {
                a_idx += 1;
            }
        }

        let out_len = weights.len.max(activations.len);
        let mut result = Self::with_capacity(out_len, output_blocks.len());
        result.blocks = output_blocks;
        result.compute_density();
        result
    }

    /// Union residual: merge two sparse streams (OR of pos/neg per chunk).
    /// Intentionally not a ternary full-adder; carry path is a later optimization.
    pub fn sparse_residual_add(a: &Self, b: &Self) -> Self {
        let mut result_blocks: Vec<(u32, BitSlicedBlock)> = Vec::new();
        let mut a_idx = 0;
        let mut b_idx = 0;

        while a_idx < a.blocks.len() && b_idx < b.blocks.len() {
            let (a_chunk, a_block) = &a.blocks[a_idx];
            let (b_chunk, b_block) = &b.blocks[b_idx];

            if a_chunk == b_chunk {
                let combined = BitSlicedBlock {
                    pos: a_block.pos | b_block.pos,
                    neg: a_block.neg | b_block.neg,
                };
                if !combined.is_empty() {
                    result_blocks.push((*a_chunk, combined));
                }
                a_idx += 1;
                b_idx += 1;
            } else if a_chunk < b_chunk {
                if !a_block.is_empty() {
                    result_blocks.push((*a_chunk, *a_block));
                }
                a_idx += 1;
            } else {
                if !b_block.is_empty() {
                    result_blocks.push((*b_chunk, *b_block));
                }
                b_idx += 1;
            }
        }

        while a_idx < a.blocks.len() {
            let (c, b) = a.blocks[a_idx];
            if !b.is_empty() {
                result_blocks.push((c, b));
            }
            a_idx += 1;
        }
        while b_idx < b.blocks.len() {
            let (c, b) = b.blocks[b_idx];
            if !b.is_empty() {
                result_blocks.push((c, b));
            }
            b_idx += 1;
        }

        let mut result = Self::with_capacity(a.len.max(b.len), result_blocks.len());
        result.blocks = result_blocks;
        result.compute_density();
        result
    }

    pub fn compute_density(&mut self) -> f32 {
        if self.len == 0 {
            self.density = 0.0;
            return 0.0;
        }
        let mut nz = 0usize;
        for (_, b) in &self.blocks {
            nz += b.nonzero_count() as usize;
        }
        self.density = nz as f32 / self.len as f32;
        self.density
    }

    /// Higher sparsity → higher fitness for edge / multi-agent workloads.
    pub fn fitness_signal(&self) -> f32 {
        if self.density > 0.0 {
            1.0 - self.density
        } else {
            1.0
        }
    }

    /// Static arena primitive — sole place that shrinks the block vector.
    /// Always ledgered when called from the structural mutation path.
    pub fn compact(&mut self, ledger: &mut TamperEvidentLedger) -> Result<u64, NtgError> {
        let before = self.blocks.len();
        self.blocks.retain(|(_, b)| !b.is_empty());
        // retain preserves sort order — no re-sort needed
        self.tombstone_count = 0;
        self.compute_density();

        ledger.log_mutation(
            format!(
                "sparse_compact before={} after={} density={}",
                before,
                self.blocks.len(),
                self.density
            ),
            before as u64,
            self.blocks.len() as u64,
            FitnessMeasure {
                latency_us: 0,
                memory_bytes: self.blocks.len() as u64 * 16,
            },
            MutationOutcome::Accepted,
            0,
            ExecutionTrace::new(),
            0,
        )
    }

    /// Mutation boundary: set + optional lazy compact + ledger entry.
    pub fn apply_structural_mutation(
        &mut self,
        idx: usize,
        val: i8,
        ledger: &mut TamperEvidentLedger,
    ) -> Result<u64, NtgError> {
        self.set(idx, val);

        if self.tombstone_count > COMPACT_THRESHOLD {
            return self.compact(ledger);
        }

        self.compute_density();
        ledger.log_mutation(
            format!(
                "sparse_set idx={} val={} density={} tombstones={}",
                idx, val, self.density, self.tombstone_count
            ),
            0,
            idx as u64,
            FitnessMeasure {
                latency_us: 0,
                memory_bytes: self.blocks.len() as u64 * 16,
            },
            MutationOutcome::Accepted,
            0,
            ExecutionTrace::new(),
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_set_get_and_zero_bypass() {
        let mut t = SparseBitSlicedTernary::new(200);
        t.set(0, 1);
        t.set(100, -1);
        assert_eq!(t.get(0), 1);
        assert_eq!(t.get(100), -1);
        assert_eq!(t.get(50), 0);
        assert_eq!(t.active_block_count(), 2);

        t.set(0, 0);
        assert_eq!(t.get(0), 0);
        assert_eq!(t.tombstone_count, 1);
    }

    #[test]
    fn sparse_dot_matches_dense_bit_sliced() {
        use super::super::BitSlicedTernary;

        let vals_a = [1i8, -1, 0, 1, 0, 0, 1, -1];
        let vals_b = [1i8, 1, 0, -1, 0, 0, 1, 1];
        let dense_a = BitSlicedTernary::from_slice(&vals_a);
        let dense_b = BitSlicedTernary::from_slice(&vals_b);
        let sparse_a = SparseBitSlicedTernary::from_slice(&vals_a);
        let sparse_b = SparseBitSlicedTernary::from_slice(&vals_b);

        assert_eq!(
            SparseBitSlicedTernary::dot_product_sparse(&sparse_a, &sparse_b),
            BitSlicedTernary::dot_product_parallel(&dense_a, &dense_b)
        );
    }

    #[test]
    fn sparse_dot_skips_non_overlapping_chunks() {
        let mut a = SparseBitSlicedTernary::new(256);
        let mut b = SparseBitSlicedTernary::new(256);
        a.set(0, 1); // chunk 0
        b.set(128, 1); // chunk 2
        assert_eq!(SparseBitSlicedTernary::dot_product_sparse(&a, &b), 0);
    }

    #[test]
    fn ternary_matmul_threshold() {
        let mut w = SparseBitSlicedTernary::new(64);
        let mut a = SparseBitSlicedTernary::new(64);
        // Full agreement on 3 positions → score 3
        w.set(0, 1);
        w.set(1, 1);
        w.set(2, 1);
        a.set(0, 1);
        a.set(1, 1);
        a.set(2, 1);

        let out = SparseBitSlicedTernary::ternary_matmul(&w, &a, 2);
        assert_eq!(out.blocks.len(), 1);
        assert_eq!(out.blocks[0].1.pos, 1);

        let out_high = SparseBitSlicedTernary::ternary_matmul(&w, &a, 10);
        assert!(out_high.blocks.is_empty());
    }

    #[test]
    fn residual_union_merges_chunks() {
        let mut a = SparseBitSlicedTernary::new(128);
        let mut b = SparseBitSlicedTernary::new(128);
        a.set(0, 1);
        b.set(64, -1);
        let r = SparseBitSlicedTernary::sparse_residual_add(&a, &b);
        assert_eq!(r.blocks.len(), 2);
        assert_eq!(r.get(0), 1);
        assert_eq!(r.get(64), -1);
    }

    #[test]
    fn lazy_compact_via_ledger() -> Result<(), NtgError> {
        let mut t = SparseBitSlicedTernary::with_capacity(4096, 64);
        let mut ledger = TamperEvidentLedger::new(None)?;

        // Fill then zero many chunks to create tombstones past threshold
        for i in 0..40 {
            t.set(i * 64, 1);
        }
        for i in 0..40 {
            t.set(i * 64, 0);
        }
        assert!(t.tombstone_count > COMPACT_THRESHOLD);

        let mid = t.apply_structural_mutation(0, 0, &mut ledger)?;
        assert!(mid < 100);
        // compact should have run
        assert_eq!(t.tombstone_count, 0);
        ledger.verify_full_ledger()?;
        Ok(())
    }

    #[test]
    fn fitness_prefers_sparsity() {
        let mut dense = SparseBitSlicedTernary::from_slice(&[1i8; 64]);
        dense.compute_density();
        let mut sparse = SparseBitSlicedTernary::from_slice(&[1i8, 0, 0, 0]);
        sparse.compute_density();
        assert!(sparse.fitness_signal() > dense.fitness_signal());
    }
}
