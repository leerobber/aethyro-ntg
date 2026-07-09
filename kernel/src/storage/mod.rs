//! Storage primitives for the Aethyro NTG Engine.
//!
//! This module provides the canonical storage primitives consumed by graph nodes,
//! including the SparseBitSlicedTernary format for efficient ternary value storage.

pub mod sparse_bit_sliced_ternary;

pub use sparse_bit_sliced_ternary::{BitSlicedBlock, SparseBitSlicedTernary};
