//! Ternary storage layer: dense packed, dual-stream bit-sliced, and sparse COO.
//!
//! - [`PackedTernary`] — 2-bit sequential packing (Phase 1.2 / TOBL FFI path)
//! - [`BitSlicedTernary`] — dual-stream pos/neg for popcount TOBL dots
//! - [`SparseBitSlicedTernary`] — flat COO sparse blocks + ledgered compact
//!
//! **Naming note:** there are two types historically called `PackedTernary`:
//! 1. `crate::ntg::packed::PackedTernary` — early Phase 1.2, 4 values/byte `Vec<u8>`
//! 2. `crate::ntg::storage::PackedTernary` — TOBL/FFI path, 32 values/u64, density hooks
//!
//! Prefer **storage::PackedTernary** for new TOBL/FFI work and
//! **BitSlicedTernary / SparseBitSlicedTernary** for native runtime compute.
//! Unification into one type is tracked as a ROADMAP open item; do not
//! silently swap encodings (slot bit patterns differ).
//!
//! All self-evolving metadata (density, cycles, fitness_signal) is initialized
//! for Phase 3+ Reflexive Fitness integration.

pub mod bit_sliced_ternary;
pub mod packed_ternary;
pub mod sparse_bit_sliced_ternary;
pub mod tobl_kernel;

pub use bit_sliced_ternary::BitSlicedTernary;
pub use packed_ternary::PackedTernary;
pub use sparse_bit_sliced_ternary::{
    BitSlicedBlock, SparseBitSlicedTernary, COMPACT_THRESHOLD,
};
pub use tobl_kernel::{tobl_dot_product, ToblKernelPath};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_module_loads() {
        let _ = PackedTernary::new(100);
        let _ = BitSlicedTernary::new(100);
        let _ = SparseBitSlicedTernary::new(100);
    }
}
