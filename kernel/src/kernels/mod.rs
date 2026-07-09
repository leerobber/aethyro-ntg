//! Vectorized kernels for the Aethyro NTG Engine.
//!
//! This module provides hardware-accelerated implementations of ternary operations
//! using SIMD instructions (AVX-512, NEON, etc.).

#[cfg(target_arch = "x86_64")]
pub mod tobl_avx512;

#[cfg(target_arch = "x86_64")]
pub use tobl_avx512::dot_block_avx512_x8;
