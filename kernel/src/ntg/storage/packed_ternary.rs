//! Cache-aligned PackedTernary: 2-bit ternary encoding for TOBL kernels.
//!
//! Format: -1 → 0b01, 0 → 0b00, 1 → 0b10 (32 elements per u64 word)
//! Alignment: 64 bytes (L1 cache line)
//! Observability: density + cycle tracking for Phase 3+ mutations

use crate::ntg::error::NtgError;

/// Cache-aligned ternary storage: 32 ternary elements per u64 word.
#[repr(C, align(64))]
pub struct PackedTernary {
    /// Packed words: 2 bits per ternary value (-1, 0, 1)
    words: Vec<u64>,
    /// Total ternary elements (not word count)
    len: usize,
    /// Observability: non-zero density for structural evolution detection
    pub density: f32,
    /// Observability: cycle count for TOBL calibration loop
    pub last_op_cycles: u64,
    /// Observability: generation counter for mutation tracking
    pub generation: u32,
}

impl PackedTernary {
    /// Create new packed ternary buffer of given ternary element count.
    pub fn new(len: usize) -> Self {
        let word_count = len.div_ceil(32);
        Self {
            words: vec![0u64; word_count],
            len,
            density: 0.0,
            last_op_cycles: 0,
            generation: 0,
        }
    }

    /// Get total element count.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Set ternary value at index (-1, 0, 1).
    #[inline]
    pub fn set(&mut self, idx: usize, val: i8) {
        debug_assert!(idx < self.len, "index {} out of bounds {}", idx, self.len);
        debug_assert!(
            (-1..=1).contains(&val),
            "ternary value {} out of range [-1, 1]",
            val
        );

        let word_idx = idx / 32;
        let bit_offset = (idx % 32) * 2;
        let mask = 0b11u64 << bit_offset;

        let packed = match val {
            -1 => 0b01,
            0 => 0b00,
            1 => 0b10,
            _ => unreachable!(),
        };

        self.words[word_idx] = (self.words[word_idx] & !mask) | (packed << bit_offset);
    }

    /// Get ternary value at index.
    #[inline]
    pub fn get(&self, idx: usize) -> i8 {
        debug_assert!(idx < self.len, "index {} out of bounds {}", idx, self.len);

        let word_idx = idx / 32;
        let bit_offset = (idx % 32) * 2;
        let packed = (self.words[word_idx] >> bit_offset) & 0b11;

        match packed {
            0b01 => -1,
            0b00 => 0,
            0b10 => 1,
            _ => unreachable!(),
        }
    }

    /// Bulk set from slice of i8.
    pub fn set_from_slice(&mut self, vals: &[i8]) -> Result<(), NtgError> {
        if vals.len() > self.len {
            return Err(NtgError::InvalidInput(format!(
                "slice len {} exceeds capacity {}",
                vals.len(),
                self.len
            )));
        }

        for (i, &val) in vals.iter().enumerate() {
            self.set(i, val);
        }
        self.generation += 1;
        Ok(())
    }

    /// Bulk get into mutable slice.
    pub fn get_into_slice(&self, out: &mut [i8]) -> Result<(), NtgError> {
        if out.len() > self.len {
            return Err(NtgError::InvalidInput(format!(
                "output slice len {} exceeds data len {}",
                out.len(),
                self.len
            )));
        }

        for (i, val) in out.iter_mut().enumerate() {
            *val = self.get(i);
        }
        Ok(())
    }

    /// Compute non-zero density for structural evolution heuristics.
    /// Non-blocking; called by Reflexive Fitness Evaluator (Phase 3+).
    pub fn compute_density(&mut self) -> f32 {
        let mut nz = 0usize;
        for &w in &self.words {
            // Count 1 bits (each ternary has 1-2 bits set)
            // Density = occupied bits / total bits
            nz += w.count_ones() as usize;
        }
        // Each ternary occupies 2 bits; density = nonzero_bits / (len * 2)
        self.density = nz as f32 / (self.len as f32 * 2.0);
        self.density
    }

    /// Record operation cycle count (set by TOBL kernel after execution).
    #[inline]
    pub fn record_cycles(&mut self, cycles: u64) {
        self.last_op_cycles = cycles;
    }

    /// Clear to all zeros.
    pub fn clear(&mut self) {
        self.words.iter_mut().for_each(|w| *w = 0);
        self.generation += 1;
    }

    /// Raw word access for TOBL kernels (unsafe FFI boundary).
    #[inline]
    pub fn word_ptr(&self) -> *const u64 {
        self.words.as_ptr()
    }

    /// Raw word access for TOBL kernels (unsafe FFI boundary).
    #[inline]
    pub fn word_ptr_mut(&mut self) -> *mut u64 {
        self.words.as_mut_ptr()
    }

    /// Word count for kernel iteration.
    #[inline]
    pub fn word_count(&self) -> usize {
        self.len.div_ceil(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_ternary_set_get() {
        let mut pt = PackedTernary::new(10);
        pt.set(0, -1);
        pt.set(1, 0);
        pt.set(2, 1);
        assert_eq!(pt.get(0), -1);
        assert_eq!(pt.get(1), 0);
        assert_eq!(pt.get(2), 1);
    }

    #[test]
    fn packed_ternary_from_slice() {
        let mut pt = PackedTernary::new(5);
        let vals = vec![1i8, -1, 0, 1, -1];
        pt.set_from_slice(&vals).unwrap();
        for (i, &expected) in vals.iter().enumerate() {
            assert_eq!(pt.get(i), expected);
        }
    }

    #[test]
    fn packed_ternary_density() {
        let mut pt = PackedTernary::new(100);
        let data = vec![1i8; 50];
        pt.set_from_slice(&data).unwrap();
        let d = pt.compute_density();
        assert!(d > 0.0 && d < 1.0);
    }

    #[test]
    fn packed_ternary_generation_tracking() {
        let mut pt = PackedTernary::new(10);
        assert_eq!(pt.generation, 0);
        pt.set_from_slice(&[1i8]).unwrap();
        assert_eq!(pt.generation, 1);
        pt.clear();
        assert_eq!(pt.generation, 2);
    }

    #[test]
    fn packed_ternary_word_count() {
        let pt = PackedTernary::new(100);
        assert_eq!(pt.word_count(), 4); // ceil(100 / 32) = 4
    }
}
