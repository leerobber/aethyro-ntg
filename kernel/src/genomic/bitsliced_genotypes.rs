/// Bitsliced 2-bit genotype storage
/// Genotypes: 0=ref/ref, 1=ref/alt, 2=alt/alt, 3=missing
/// Storage: 2 bits per genotype, packed into u64 words
///
/// For 2504 samples: 5008 bits = 626 bytes per SNP
/// For 4.3M SNPs: 4.3M × 626 bytes = 2.69 GB (uncompressed)
/// After gzip: ~1 GB

use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct BitstreamGenotypes {
    n_samples: usize,
    plane0: Vec<u64>,  // First bit plane (bit 0)
    plane1: Vec<u64>,  // Second bit plane (bit 1)
}

impl BitstreamGenotypes {
    /// Create new bitstream for n_samples
    pub fn new(n_samples: usize) -> Self {
        let words_per_plane = (n_samples + 31) / 32;  // 32 genotypes per u64 word
        BitstreamGenotypes {
            n_samples,
            plane0: vec![0; words_per_plane],
            plane1: vec![0; words_per_plane],
        }
    }

    /// Set genotype for sample at index
    /// Genotype: 0=ref/ref, 1=het, 2=alt/alt, 3=missing
    #[inline]
    pub fn set(&mut self, sample_idx: usize, genotype: u8) {
        if sample_idx >= self.n_samples {
            panic!("Sample index {} out of bounds {}", sample_idx, self.n_samples);
        }

        let word_idx = sample_idx / 32;
        let bit_idx = sample_idx % 32;

        // Extract bits from genotype
        let bit0 = (genotype & 1) as u64;
        let bit1 = ((genotype >> 1) & 1) as u64;

        // Clear old bits
        self.plane0[word_idx] &= !(1u64 << bit_idx);
        self.plane1[word_idx] &= !(1u64 << bit_idx);

        // Set new bits
        self.plane0[word_idx] |= bit0 << bit_idx;
        self.plane1[word_idx] |= bit1 << bit_idx;
    }

    /// Get genotype for sample
    #[inline]
    pub fn get(&self, sample_idx: usize) -> u8 {
        if sample_idx >= self.n_samples {
            return 3;  // Return missing if out of bounds
        }

        let word_idx = sample_idx / 32;
        let bit_idx = sample_idx % 32;

        let bit0 = ((self.plane0[word_idx] >> bit_idx) & 1) as u8;
        let bit1 = ((self.plane1[word_idx] >> bit_idx) & 1) as u8;

        (bit1 << 1) | bit0
    }

    /// Get all genotypes as Vec<u8> (for testing/debugging)
    pub fn to_vec(&self) -> Vec<u8> {
        (0..self.n_samples)
            .map(|i| self.get(i))
            .collect()
    }

    /// Number of samples
    pub fn len(&self) -> usize {
        self.n_samples
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        (self.plane0.len() + self.plane1.len()) * 8
    }

    /// Serialize to binary format
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write header
        writer.write_all(&(self.n_samples as u32).to_le_bytes())?;

        // Write plane0
        for word in &self.plane0 {
            writer.write_all(&word.to_le_bytes())?;
        }

        // Write plane1
        for word in &self.plane1 {
            writer.write_all(&word.to_le_bytes())?;
        }

        Ok(())
    }

    /// Deserialize from binary format
    pub fn read<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        // Read header
        let mut n_bytes = [0u8; 4];
        reader.read_exact(&mut n_bytes)?;
        let n_samples = u32::from_le_bytes(n_bytes) as usize;

        let words_per_plane = (n_samples + 31) / 32;

        // Read plane0
        let mut plane0 = vec![0u64; words_per_plane];
        for word in &mut plane0 {
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            *word = u64::from_le_bytes(bytes);
        }

        // Read plane1
        let mut plane1 = vec![0u64; words_per_plane];
        for word in &mut plane1 {
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            *word = u64::from_le_bytes(bytes);
        }

        Ok(BitstreamGenotypes {
            n_samples,
            plane0,
            plane1,
        })
    }

    /// Mask of the real (non-padding) sample bits within plane word `i`.
    ///
    /// Each word packs only **32** samples, in its low 32 bits (see `set`:
    /// `bit_idx = sample_idx % 32`); bits 32..64 are always-zero padding
    /// in *every* word, and the final word is further trimmed to however
    /// many samples remain. Both kinds of padding read as genotype 0
    /// (valid ref/ref) and would otherwise be miscounted as real samples,
    /// so a moment computation must AND every word against this mask.
    #[inline]
    fn word_sample_mask(&self, i: usize) -> u64 {
        let real_bits = self.n_samples.saturating_sub(i * 32).min(32) as u32;
        // real_bits <= 32, so 1u64 << 32 is well-defined and yields the
        // full low-32 mask; a fully-populated word maps to 0x0000_0000_FFFF_FFFF.
        (1u64 << real_bits) - 1
    }

    /// Squared Pearson correlation (genotypic r²) between this SNP's
    /// dosages and another's, computed word-parallel over the packed bit
    /// planes instead of one `get()` per sample.
    ///
    /// This is the bit-plane form of the exact same statistic
    /// `LdComputer::compute_r_squared` computes scalar-per-sample. Because
    /// a non-missing genotype's numeric value *is* its alt-allele dosage
    /// (0/1/2) and its two plane bits are mutually exclusive there
    /// (only missing = `11` sets both), every moment the correlation needs
    /// is a popcount of AND-ed plane words:
    ///   dosage x  = 1·bit0 + 2·bit1        (so x² = 1·bit0 + 4·bit1)
    ///   Σx        = pop(b0)    + 2·pop(b1)
    ///   Σx²       = pop(b0)    + 4·pop(b1)
    ///   Σxy       = pop(b0x&b0y) + 2·pop(b0x&b1y) + 2·pop(b1x&b0y) + 4·pop(b1x&b1y)
    /// all restricted to samples valid at *both* loci
    /// (`valid = !(missing_x | missing_y)`, missing = `b0 & b1`), with the
    /// final word masked to real samples. The moments are exact integers;
    /// only the closing divide is floating point (in `f64`, matching the
    /// scalar path), so the result agrees with the scalar reference to
    /// within `f32` precision while touching 32 samples per iteration
    /// instead of one. Mirrors the kernel's own
    /// `BitSlicedTernary::dot_product_parallel` popcount idiom.
    ///
    /// Returns `None` when fewer than `min_valid` samples are non-missing
    /// at both loci, or when either locus is monomorphic in that sample
    /// (variance zero -> r² undefined, not zero) -- identical to the
    /// scalar path's guards.
    pub fn pearson_r2_bitparallel(&self, other: &Self, min_valid: usize) -> Option<f32> {
        if self.n_samples != other.n_samples {
            return None;
        }

        let words = self.plane0.len();

        let (mut cx0, mut cx1) = (0i64, 0i64);
        let (mut cy0, mut cy1) = (0i64, 0i64);
        let (mut c00, mut c01, mut c10, mut c11) = (0i64, 0i64, 0i64, 0i64);
        let mut n_valid = 0i64;

        for i in 0..words {
            let b0x = self.plane0[i];
            let b1x = self.plane1[i];
            let b0y = other.plane0[i];
            let b1y = other.plane1[i];

            let miss = (b0x & b1x) | (b0y & b1y);
            let valid = !miss & self.word_sample_mask(i);

            let vx0 = b0x & valid;
            let vx1 = b1x & valid;
            let vy0 = b0y & valid;
            let vy1 = b1y & valid;

            n_valid += valid.count_ones() as i64;
            cx0 += vx0.count_ones() as i64;
            cx1 += vx1.count_ones() as i64;
            cy0 += vy0.count_ones() as i64;
            cy1 += vy1.count_ones() as i64;
            c00 += (vx0 & vy0).count_ones() as i64;
            c01 += (vx0 & vy1).count_ones() as i64;
            c10 += (vx1 & vy0).count_ones() as i64;
            c11 += (vx1 & vy1).count_ones() as i64;
        }

        if (n_valid as usize) < min_valid {
            return None;
        }

        let sum_x = (cx0 + 2 * cx1) as f64;
        let sum_y = (cy0 + 2 * cy1) as f64;
        let sum_x2 = (cx0 + 4 * cx1) as f64;
        let sum_y2 = (cy0 + 4 * cy1) as f64;
        let sum_xy = (c00 + 2 * c01 + 2 * c10 + 4 * c11) as f64;
        let n = n_valid as f64;

        let num = n * sum_xy - sum_x * sum_y;
        let den = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();
        if den <= 0.0 {
            return None;
        }
        let r = num / den;
        Some(((r * r) as f32).clamp(0.0, 1.0))
    }

    /// Compute allele frequency for this SNP
    /// Returns (freq_ref, freq_alt, freq_missing)
    pub fn allele_frequencies(&self) -> (f64, f64, f64) {
        let mut count_ref = 0u64;
        let mut count_missing = 0u64;

        for i in 0..self.n_samples {
            match self.get(i) {
                0 => count_ref += 2, // ref/ref: 2 ref alleles
                1 => count_ref += 1, // ref/alt: 1 ref (alt via 1 - freq_ref)
                2 => {}             // alt/alt: 0 ref alleles; still non-missing
                _ => count_missing += 1,
            }
        }

        let total = (self.n_samples as u64 - count_missing) * 2;
        let freq_ref = if total > 0 {
            count_ref as f64 / total as f64
        } else {
            0.0
        };
        let freq_alt = 1.0 - freq_ref;
        let freq_missing = count_missing as f64 / self.n_samples as f64;

        (freq_ref, freq_alt, freq_missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitsliced_storage() {
        let mut bs = BitstreamGenotypes::new(100);

        // Set some genotypes
        bs.set(0, 0);  // ref/ref
        bs.set(1, 1);  // het
        bs.set(2, 2);  // alt/alt
        bs.set(3, 3);  // missing

        // Read them back
        assert_eq!(bs.get(0), 0);
        assert_eq!(bs.get(1), 1);
        assert_eq!(bs.get(2), 2);
        assert_eq!(bs.get(3), 3);

        // Check memory usage
        // For 100 samples: (100+31)/32 = 4 words per plane = 4*8*2 = 64 bytes
        assert_eq!(bs.memory_bytes(), 64);
    }

    #[test]
    fn test_allele_frequencies() {
        let mut bs = BitstreamGenotypes::new(4);

        // 2 ref/ref, 1 het, 1 alt/alt
        bs.set(0, 0);  // ref/ref
        bs.set(1, 0);  // ref/ref
        bs.set(2, 1);  // het
        bs.set(3, 2);  // alt/alt

        let (freq_ref, freq_alt, freq_missing) = bs.allele_frequencies();

        // Total alleles: 8
        // Ref: 2*2 + 1 = 5
        // Alt: 1 + 1*2 = 3
        // Missing: 0
        assert!((freq_ref - 5.0/8.0).abs() < 0.001);
        assert!((freq_alt - 3.0/8.0).abs() < 0.001);
        assert_eq!(freq_missing, 0.0);
    }
}
