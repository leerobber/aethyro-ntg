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

    /// Compute allele frequency for this SNP
    /// Returns (freq_ref, freq_alt, freq_missing)
    pub fn allele_frequencies(&self) -> (f64, f64, f64) {
        let mut count_ref = 0u64;
        let mut count_alt = 0u64;
        let mut count_missing = 0u64;

        for i in 0..self.n_samples {
            match self.get(i) {
                0 => count_ref += 2,      // ref/ref: 2 ref alleles
                1 => count_ref += 1,      // ref/alt: 1 ref, 1 alt
                2 => count_alt += 2,      // alt/alt: 2 alt alleles
                _ => count_missing += 1,  // missing
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
