//! Dual-stream bit-sliced dense ternary storage.
//!
//! Layout: separate `pos_bits` (+1) and `neg_bits` (-1) vectors. Zeros are
//! implicit (bit clear in both streams). Dot products evaluate 64 elements
//! per word via AND + popcount — no scalar unpack/shift loops.
//!
//! This is the dense baseline. Sparse / arena-backed traffic uses
//! [`super::sparse_bit_sliced_ternary::SparseBitSlicedTernary`].

/// Dense dual-stream ternary tensor: 64 ternary values per u64 word pair.
#[derive(Clone, Debug)]
pub struct BitSlicedTernary {
    /// Bit set iff element is +1
    pub pos_bits: Vec<u64>,
    /// Bit set iff element is -1
    pub neg_bits: Vec<u64>,
    pub len: usize,
    /// Non-zero density in [0, 1] (updated by `compute_density`)
    pub density: f32,
    /// Last measured / estimated op cost (cycles or µs — caller convention)
    pub last_op_cycles: u64,
}

impl BitSlicedTernary {
    pub fn new(len: usize) -> Self {
        let words = len.div_ceil(64);
        Self {
            pos_bits: vec![0u64; words],
            neg_bits: vec![0u64; words],
            len,
            density: 0.0,
            last_op_cycles: 0,
        }
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
    pub fn word_count(&self) -> usize {
        self.pos_bits.len()
    }

    #[inline]
    pub fn set(&mut self, idx: usize, val: i8) {
        debug_assert!(idx < self.len);
        debug_assert!((-1..=1).contains(&val));
        let w = idx / 64;
        let bit = idx % 64;
        let mask = 1u64 << bit;
        match val {
            1 => {
                self.pos_bits[w] |= mask;
                self.neg_bits[w] &= !mask;
            }
            -1 => {
                self.neg_bits[w] |= mask;
                self.pos_bits[w] &= !mask;
            }
            0 => {
                self.pos_bits[w] &= !mask;
                self.neg_bits[w] &= !mask;
            }
            _ => unreachable!("ternary value out of range"),
        }
    }

    #[inline]
    pub fn get(&self, idx: usize) -> i8 {
        debug_assert!(idx < self.len);
        let w = idx / 64;
        let bit = idx % 64;
        let mask = 1u64 << bit;
        if self.pos_bits[w] & mask != 0 {
            1
        } else if self.neg_bits[w] & mask != 0 {
            -1
        } else {
            0
        }
    }

    /// Build from a dense i8 slice of ternary values.
    pub fn from_slice(vals: &[i8]) -> Self {
        let mut t = Self::new(vals.len());
        for (i, &v) in vals.iter().enumerate() {
            t.set(i, v);
        }
        t.compute_density();
        t
    }

    /// Density of non-zero elements (self-evolving / fitness hook).
    pub fn compute_density(&mut self) -> f32 {
        if self.len == 0 {
            self.density = 0.0;
            return 0.0;
        }
        let mut nz = 0u32;
        for i in 0..self.pos_bits.len() {
            nz += self.pos_bits[i].count_ones() + self.neg_bits[i].count_ones();
        }
        // Mask off padding bits past `len` in the last word
        let rem = self.len % 64;
        if rem != 0 {
            let last = self.pos_bits.len() - 1;
            let pad_mask = !((1u64 << rem) - 1);
            nz -= (self.pos_bits[last] & pad_mask).count_ones()
                + (self.neg_bits[last] & pad_mask).count_ones();
        }
        self.density = nz as f32 / self.len as f32;
        self.density
    }

    /// Dot product over 64-wide bit-gates (no scalar unpack).
    ///
    /// `Σ a_i * b_i` where each product is in {-1,0,1}:
    ///   (+,+) and (-,-) contribute +1; (+,-) and (-,+) contribute -1.
    #[inline]
    pub fn dot_product_parallel(a: &Self, b: &Self) -> i64 {
        debug_assert_eq!(a.len, b.len);
        let words = a.pos_bits.len().min(b.pos_bits.len());
        let mut total: i64 = 0;
        for i in 0..words {
            let pp = (a.pos_bits[i] & b.pos_bits[i]).count_ones() as i64;
            let mm = (a.neg_bits[i] & b.neg_bits[i]).count_ones() as i64;
            let pm = (a.pos_bits[i] & b.neg_bits[i]).count_ones() as i64;
            let mp = (a.neg_bits[i] & b.pos_bits[i]).count_ones() as i64;
            total += (pp + mm) - (pm + mp);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_roundtrip() {
        let mut t = BitSlicedTernary::new(130);
        t.set(0, 1);
        t.set(1, -1);
        t.set(63, 1);
        t.set(64, -1);
        t.set(129, 1);
        assert_eq!(t.get(0), 1);
        assert_eq!(t.get(1), -1);
        assert_eq!(t.get(2), 0);
        assert_eq!(t.get(63), 1);
        assert_eq!(t.get(64), -1);
        assert_eq!(t.get(129), 1);
    }

    #[test]
    fn overwriting_sign_clears_other_stream() {
        let mut t = BitSlicedTernary::new(8);
        t.set(3, 1);
        t.set(3, -1);
        assert_eq!(t.get(3), -1);
        t.set(3, 0);
        assert_eq!(t.get(3), 0);
    }

    #[test]
    fn dot_product_matches_scalar() {
        let a = BitSlicedTernary::from_slice(&[1, -1, 0, 1, 1, -1, 0, 0]);
        let b = BitSlicedTernary::from_slice(&[1, -1, 1, -1, 0, 1, 0, 1]);
        // 1*1 + (-1)*(-1) + 0 + 1*(-1) + 1*0 + (-1)*1 + 0 + 0
        // = 1 + 1 + 0 - 1 + 0 - 1 = 0
        assert_eq!(BitSlicedTernary::dot_product_parallel(&a, &b), 0);

        let mut expected = 0i64;
        for i in 0..a.len() {
            expected += a.get(i) as i64 * b.get(i) as i64;
        }
        assert_eq!(BitSlicedTernary::dot_product_parallel(&a, &b), expected);
    }

    #[test]
    fn density_counts_nonzero_only() {
        let mut t = BitSlicedTernary::from_slice(&[1, 0, -1, 0]);
        let d = t.compute_density();
        assert!((d - 0.5).abs() < 1e-6);
    }
}
