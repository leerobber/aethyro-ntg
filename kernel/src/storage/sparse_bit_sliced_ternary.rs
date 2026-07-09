//! Sparse Bit-Sliced Ternary storage primitive.
//!
//! Canonical storage for ternary {-1, 0, 1} values using dual-stream
//! bit-slicing: separate pos_bits and neg_bits vectors track +1 and -1
//! values respectively. Zero values are implicit (both bits 0).
//!
//! This layout enables:
//! - 2 bits per ternary value (memory efficient)
//! - Branch-free bitwise operations for dot products
//! - Hardware popcount acceleration
//! - Sparse block representation (only non-zero chunks allocated)
//! - Ledgered mutations for tamper-evidence

/// A 64-element block of ternary values in bit-sliced format.
/// Each bit in pos represents a +1 value, each bit in neg represents a -1 value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BitSlicedBlock {
    pub pos: u64,
    pub neg: u64,
}

const TOMBSTONE_COMPACT_THRESHOLD: usize = 32;

/// Sparse Bit-Sliced Ternary tensor.
///
/// Only non-zero 64-element chunks are stored in the blocks Vec.
/// Chunks are sorted by chunk_id for efficient merge operations.
#[derive(Clone, Debug)]
pub struct SparseBitSlicedTernary {
    pub blocks: Vec<(u32, BitSlicedBlock)>,
    pub len: usize,
    pub density: f32,
    pub last_op_cycles: u64,
    pub tombstone_count: usize,
    active_bits: usize,
}

impl SparseBitSlicedTernary {
    pub fn with_capacity(len: usize, max_active_chunks: usize) -> Self {
        Self {
            blocks: Vec::with_capacity(max_active_chunks),
            len,
            density: 0.0,
            last_op_cycles: 0,
            tombstone_count: 0,
            active_bits: 0,
        }
    }

    pub fn new(len: usize) -> Self {
        Self::with_capacity(len, 1024)
    }

    #[inline(always)]
    pub fn set(&mut self, idx: usize, val: i8) {
        debug_assert!(idx < self.len);
        let chunk = (idx / 64) as u32;
        let bit = idx % 64;
        let mask = 1u64 << bit;

        match self.blocks.binary_search_by_key(&chunk, |&(c, _)| c) {
            Ok(i) => {
                let entry = &mut self.blocks[i].1;
                let was_tombstone = entry.pos == 0 && entry.neg == 0;

                let old_pop = entry.pos.count_ones() + entry.neg.count_ones();

                match val {
                    1 => { entry.pos |= mask; entry.neg &= !mask; }
                    -1 => { entry.neg |= mask; entry.pos &= !mask; }
                    0 => { entry.pos &= !mask; entry.neg &= !mask; }
                    _ => unreachable!(),
                }

                let is_tombstone = entry.pos == 0 && entry.neg == 0;
                let new_pop = entry.pos.count_ones() + entry.neg.count_ones();

                self.active_bits = (self.active_bits + new_pop as usize) - old_pop as usize;

                if !was_tombstone && is_tombstone {
                    self.tombstone_count += 1;
                } else if was_tombstone && !is_tombstone {
                    self.tombstone_count = self.tombstone_count.saturating_sub(1);
                }
            }
            Err(i) if val != 0 => {
                let mut entry = BitSlicedBlock::default();
                if val == 1 { entry.pos |= mask; } else { entry.neg |= mask; }
                self.active_bits += (entry.pos.count_ones() + entry.neg.count_ones()) as usize;
                self.blocks.insert(i, (chunk, entry));
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn density(&self) -> f32 {
        if self.len == 0 { 0.0 } else { self.active_bits as f32 / self.len as f32 }
    }

    #[inline(always)]
    pub fn fitness_signal(&self) -> f32 {
        1.0 - self.density()
    }

    pub fn compute_density(&mut self) -> f32 {
        let mut nz = 0usize;
        for &(_, ref b) in &self.blocks {
            nz += (b.pos.count_ones() + b.neg.count_ones()) as usize;
        }
        self.density = nz as f32 / self.len as f32;
        self.density
    }

    /// Branch-free on matching-chunk path; tombstones naturally contribute 0 via bitwise ops.
    pub fn dot_product_sparse(a: &Self, b: &Self) -> i64 {
        let mut total: i64 = 0;
        let mut a_idx = 0;
        let mut b_idx = 0;

        // Zero-allocation staging buffers (stack only)
        let mut a_stage = [BitSlicedBlock::default(); 8];
        let mut b_stage = [BitSlicedBlock::default(); 8];
        let mut stage_count = 0usize;

        // Runtime feature detection (cached, cheap)
        let has_avx512 = Self::resolve_avx512_hardware();

        while a_idx < a.blocks.len() && b_idx < b.blocks.len() {
            let (a_chunk, a_block) = &a.blocks[a_idx];
            let (b_chunk, b_block) = &b.blocks[b_idx];

            if a_chunk == b_chunk {
                a_stage[stage_count] = *a_block;
                b_stage[stage_count] = *b_block;
                stage_count += 1;
                a_idx += 1;
                b_idx += 1;

                if stage_count == 8 {
                    if has_avx512 {
                        #[cfg(target_arch = "x86_64")]
                        unsafe {
                            total += crate::kernels::tobl_avx512::dot_block_avx512_x8(&a_stage, &b_stage);
                        }
                    } else {
                        for i in 0..8 {
                            total += evaluate_scalar_block(&a_stage[i], &b_stage[i]);
                        }
                    }
                    stage_count = 0;
                }
            } else if a_chunk < b_chunk {
                a_idx += 1;
            } else {
                b_idx += 1;
            }
        }

        // Drain remaining staged blocks
        for i in 0..stage_count {
            total += evaluate_scalar_block(&a_stage[i], &b_stage[i]);
        }

        total
    }

    /// Sparse × Sparse → Sparse ternary matrix multiplication.
    pub fn ternary_matmul(
        weights: &Self,
        activations: &Self,
        threshold: i64,
    ) -> Self {
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

                if score >= threshold {
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

        let mut result = Self::with_capacity(
            weights.len.max(activations.len),
            output_blocks.len(),
        );
        result.blocks = output_blocks;
        result.compute_density();
        result
    }

    /// Branch-friendly union merge for residual connections.
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
                if combined.pos != 0 || combined.neg != 0 {
                    result_blocks.push((*a_chunk, combined));
                }
                a_idx += 1;
                b_idx += 1;
            } else if a_chunk < b_chunk {
                result_blocks.push((*a_chunk, *a_block));
                a_idx += 1;
            } else {
                result_blocks.push((*b_chunk, *b_block));
                b_idx += 1;
            }
        }

        // Drain remainders
        while a_idx < a.blocks.len() {
            result_blocks.push(a.blocks[a_idx]);
            a_idx += 1;
        }
        while b_idx < b.blocks.len() {
            result_blocks.push(b.blocks[b_idx]);
            b_idx += 1;
        }

        let mut result = Self::with_capacity(a.len.max(b.len), result_blocks.len());
        result.blocks = result_blocks;
        result.compute_density();
        result
    }

    pub fn compact(&mut self, ledger: &mut ChronosLedger) -> LedgerEntry {
        let before = self.blocks.len();
        self.blocks.retain(|(_, b)| b.pos != 0 || b.neg != 0);
        self.tombstone_count = 0;
        self.compute_density();

        let hash = MutationHash::from_compact(before, self.blocks.len());
        ledger.append(hash)
    }

    pub fn apply_structural_mutation(&mut self, idx: usize, val: i8, ledger: &mut ChronosLedger) -> LedgerEntry {
        self.set(idx, val);

        if self.tombstone_count > TOMBSTONE_COMPACT_THRESHOLD {
            self.compact(ledger)
        } else {
            let hash = MutationHash::from_state(self);
            ledger.append(hash)
        }
    }

    #[inline(always)]
    fn resolve_avx512_hardware() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512vpopcntdq")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    }
}

#[inline(always)]
fn evaluate_scalar_block(a_block: &BitSlicedBlock, b_block: &BitSlicedBlock) -> i64 {
    let pp = (a_block.pos & b_block.pos).count_ones() as i64;
    let mm = (a_block.neg & b_block.neg).count_ones() as i64;
    let pm = (a_block.pos & b_block.neg).count_ones() as i64;
    let mp = (a_block.neg & b_block.pos).count_ones() as i64;
    (pp + mm) - (pm + mp)
}

// Placeholder types for ledger integration
pub struct LedgerEntry;
pub struct MutationHash;
pub struct ChronosLedger {
    pub entries: Vec<LedgerEntry>,
}

impl ChronosLedger {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
    pub fn append(&mut self, _hash: MutationHash) -> LedgerEntry {
        LedgerEntry
    }
}

impl MutationHash {
    pub fn from_compact(_before: usize, _after: usize) -> Self {
        Self
    }
    pub fn from_state(_state: &SparseBitSlicedTernary) -> Self {
        Self
    }
}
