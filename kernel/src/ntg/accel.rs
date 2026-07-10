//! Density-aware compute device selection for native ternary forward.
//!
//! `AccelManager` picks a device from activation density (and optional
//! hardware hints). Today all devices share the correct sparse
//! bit-sliced merge-join kernel; the manager is the hook for future
//! AVX-512 / NEON / GPU paths without rewriting the runtime loop.

use super::error::NtgError;
use super::storage::SparseBitSlicedTernary;

/// Below this density, prefer the sparse merge-join path (memory-bus bypass).
pub const DEFAULT_SPARSE_DENSITY_THRESHOLD: f32 = 0.35;

/// Hardware capability for popcount-accelerated TOBL paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwarePath {
    /// AVX-512 foundation + VPOPCNTDQ (64-bit popcount per lane)
    Avx512Popcnt,
    /// AVX2 available (x86_64)
    Avx2,
    /// Portable scalar / software popcount
    Scalar,
}

/// Detect best available TOBL-related hardware path.
///
/// Feature names: `avx512f` + `avx512vpopcntdq` (not the invalid
/// `avx512popcnt` token).
pub fn resolve_avx512_hardware() -> HardwarePath {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512vpopcntdq") {
            return HardwarePath::Avx512Popcnt;
        }
        if is_x86_feature_detected!("avx2") {
            return HardwarePath::Avx2;
        }
    }
    HardwarePath::Scalar
}

/// Selected compute backend for one forward pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccelDevice {
    /// Sparse COO merge-join (popcount TOBL). Best when activations are sparse.
    SparseCpu,
    /// Dense-oriented path (currently same kernel; reserved for packed/dense TOBL).
    DenseCpu,
    /// Hardware prefers AVX-512 VPOPCNTDQ-class paths when density is mid/high.
    Avx512Cpu,
}

impl AccelDevice {
    pub fn name(self) -> &'static str {
        match self {
            AccelDevice::SparseCpu => "sparse_cpu",
            AccelDevice::DenseCpu => "dense_cpu",
            AccelDevice::Avx512Cpu => "avx512_cpu",
        }
    }

    /// Chunk-level ternary interaction used by native parallel forward.
    /// Infallible for valid same-kind tensors; returns Result for API symmetry
    /// and future fallible accelerators.
    pub fn ternary_matmul(
        self,
        weights: &SparseBitSlicedTernary,
        activations: &SparseBitSlicedTernary,
        threshold: i64,
    ) -> Result<SparseBitSlicedTernary, NtgError> {
        // Allow zero-len empty tensors through; otherwise require a length match.
        // Sparse matmul itself only asserts in debug; enforce in release too.
        if weights.len() != activations.len() && !weights.is_empty() && !activations.is_empty() {
            return Err(NtgError::ShapeMismatch {
                expected: weights.len(),
                got: activations.len(),
            });
        }
        // All devices use the proven sparse kernel today. Device tag is
        // recorded by the runtime for observability / future dispatch.
        let _ = self;
        Ok(SparseBitSlicedTernary::ternary_matmul(
            weights,
            activations,
            threshold,
        ))
    }
}

/// Chooses an [`AccelDevice`] from density + host hardware.
#[derive(Clone, Debug)]
pub struct AccelManager {
    /// Use sparse path when activation density is strictly below this.
    pub sparse_density_threshold: f32,
    /// Cached host capability (from [`resolve_avx512_hardware`]).
    pub host_path: HardwarePath,
}

impl Default for AccelManager {
    fn default() -> Self {
        Self {
            sparse_density_threshold: DEFAULT_SPARSE_DENSITY_THRESHOLD,
            host_path: resolve_avx512_hardware(),
        }
    }
}

impl AccelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(mut self, sparse_density_threshold: f32) -> Self {
        self.sparse_density_threshold = sparse_density_threshold;
        self
    }

    /// Select device for a forward pass given activation density in `[0, 1]`.
    pub fn select_for(&self, density: f32) -> AccelDevice {
        if density < self.sparse_density_threshold {
            return AccelDevice::SparseCpu;
        }
        match self.host_path {
            HardwarePath::Avx512Popcnt => AccelDevice::Avx512Cpu,
            HardwarePath::Avx2 | HardwarePath::Scalar => AccelDevice::DenseCpu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_density_selects_sparse() {
        let m = AccelManager::new();
        assert_eq!(m.select_for(0.1), AccelDevice::SparseCpu);
    }

    #[test]
    fn high_density_selects_dense_or_avx512() {
        let m = AccelManager::new();
        let d = m.select_for(0.9);
        assert!(matches!(
            d,
            AccelDevice::DenseCpu | AccelDevice::Avx512Cpu
        ));
    }

    #[test]
    fn ternary_matmul_via_device() -> Result<(), NtgError> {
        let mut w = SparseBitSlicedTernary::new(64);
        let mut a = SparseBitSlicedTernary::new(64);
        for i in 0..4 {
            w.set(i, 1);
            a.set(i, 1);
        }
        a.compute_density();
        let device = AccelManager::new().select_for(a.density());
        let out = device.ternary_matmul(&w, &a, 1)?;
        assert!(!out.blocks.is_empty());
        Ok(())
    }
}
