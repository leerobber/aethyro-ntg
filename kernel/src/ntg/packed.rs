//! Bit-packed ternary storage: 2 bits per value, 4 values per byte.
//!
//! Phase 1.1's `Vec<i8>` spends a full byte per ternary value -- this is
//! the actual memory-density mechanism ternary quantization exists for
//! (see docs/DESIGN.md, docs/ROADMAP.md Phase 1.2). Slot encoding:
//! `0b00` = Zero, `0b01` = Pos, `0b11` = Neg (`0b10` is unused/reserved
//! and only reachable if a byte is corrupted outside this API).

use super::error::NtgError;
use super::ternary::Ternary;

const SLOT_ZERO: u8 = 0b00;
const SLOT_POS: u8 = 0b01;
const SLOT_NEG: u8 = 0b11;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedTernary {
    len: usize,
    bytes: Vec<u8>,
}

impl PackedTernary {
    pub fn from_values(values: &[i8]) -> Result<Self, NtgError> {
        let mut bytes = vec![0u8; values.len().div_ceil(4)];
        for (i, &v) in values.iter().enumerate() {
            let t = Ternary::from_i8(v)?;
            let slot = match t {
                Ternary::Zero => SLOT_ZERO,
                Ternary::Pos => SLOT_POS,
                Ternary::Neg => SLOT_NEG,
            };
            let byte_idx = i / 4;
            let shift = (i % 4) * 2;
            bytes[byte_idx] |= slot << shift;
        }
        Ok(Self { len: values.len(), bytes })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Result<i8, NtgError> {
        if index >= self.len {
            return Err(NtgError::IndexOutOfBounds { index, len: self.len });
        }
        let byte_idx = index / 4;
        let shift = (index % 4) * 2;
        let slot = (self.bytes[byte_idx] >> shift) & 0b11;
        match slot {
            SLOT_ZERO => Ok(0),
            SLOT_POS => Ok(1),
            SLOT_NEG => Ok(-1),
            other => Err(NtgError::InvalidTernaryValue(other as i8)),
        }
    }

    pub fn to_values(&self) -> Result<Vec<i8>, NtgError> {
        (0..self.len).map(|i| self.get(i)).collect()
    }

    /// Bytes actually used for storage -- makes the memory-density claim
    /// checkable in a test instead of asserted in a doc.
    pub fn packed_byte_len(&self) -> usize {
        self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_values() {
        let values = vec![-1i8, 0, 1, 0, 1, -1, -1, 0, 1];
        let packed = PackedTernary::from_values(&values).unwrap();
        assert_eq!(packed.to_values().unwrap(), values);
    }

    #[test]
    fn packing_density_is_4x() {
        let values = vec![1i8; 16];
        let packed = PackedTernary::from_values(&values).unwrap();
        // 16 values at 2 bits each = 32 bits = 4 bytes, vs. 16 bytes unpacked.
        assert_eq!(packed.packed_byte_len(), 4);
    }

    #[test]
    fn non_multiple_of_four_length_roundtrips() {
        let values = vec![1i8, -1, 0];
        let packed = PackedTernary::from_values(&values).unwrap();
        assert_eq!(packed.packed_byte_len(), 1);
        assert_eq!(packed.to_values().unwrap(), values);
    }

    #[test]
    fn empty_input_is_empty() {
        let packed = PackedTernary::from_values(&[]).unwrap();
        assert!(packed.is_empty());
        assert_eq!(packed.packed_byte_len(), 0);
    }

    #[test]
    fn rejects_invalid_value() {
        assert!(PackedTernary::from_values(&[2]).is_err());
    }

    #[test]
    fn get_out_of_bounds_errors() {
        let packed = PackedTernary::from_values(&[1, 0, -1]).unwrap();
        assert!(packed.get(3).is_err());
    }
}
