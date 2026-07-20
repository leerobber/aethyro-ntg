//! SIMD path dispatcher: CPU feature detection + profiling + selection.
//!
//! At startup:
//! 1. Detect available CPU features (AVX2, NEON, etc.)
//! 2. Profile each available path: real correctness check against scalar,
//!    real wall-clock timing (see `profiler::profile_simd_path`)
//! 3. Select the fastest path that passed its correctness check
//! 4. Route all matmul calls through the selected path
//!
//! Gracefully falls back to scalar if SIMD unavailable, fails to compile
//! its correctness check, or wasn't profiled at all.
//!
//! Earlier version of this file computed a profile result in `profile_all`
//! and then discarded it (own comment: "we'd ideally mutate here... use
//! interior mutability"), and `select_best` never looked at profiling data
//! at all -- it just hardcoded "prefer AVX2 if the CPU claims to support
//! it," with no correctness gating. That made "self-tuning" fictional:
//! there was no tuning signal anywhere in the loop. Fixed by storing
//! per-path results in fixed-size atomic arrays (four SIMDPath variants,
//! known at compile time) that `select_best` now actually reads.

use super::profiler::{ProfileResult, profile_simd_path};
use super::super::ternary::matmul_scalar;
use super::super::error::NtgError;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Number of SIMDPath variants (Scalar, AVX2, NEON, SSE41). Used to size
/// the fixed per-path result arrays below.
const NUM_PATHS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SIMDPath {
    Scalar = 0,
    AVX2 = 1,
    NEON = 2,
    SSE41 = 3,
}

impl SIMDPath {
    pub fn name(&self) -> &'static str {
        match self {
            SIMDPath::Scalar => "Scalar",
            SIMDPath::AVX2 => "AVX2",
            SIMDPath::NEON => "NEON",
            SIMDPath::SSE41 => "SSE4.1",
        }
    }
}

#[derive(Clone, Debug)]
struct PathProfile {
    path: SIMDPath,
    available: bool,
}

pub struct SIMDDispatcher {
    profiles: Vec<PathProfile>,
    selected_path: AtomicUsize,  // Index into profiles
    last_profile_ns: std::sync::atomic::AtomicU64,
    /// Real profiling results, indexed by `SIMDPath as usize`. Latency is
    /// stored as `f64::to_bits()` since there's no AtomicF64 in std. The
    /// initial placeholder bit pattern is never a meaningful float (and
    /// doesn't need to be) -- `path_profiled` is always checked first, so
    /// an unprofiled path's latency bits are never read.
    path_latency_us_bits: [AtomicU64; NUM_PATHS],
    path_passed_correctness: [AtomicBool; NUM_PATHS],
    path_profiled: [AtomicBool; NUM_PATHS],
}

impl SIMDDispatcher {
    pub fn new() -> Result<Self, String> {
        // Detect available paths
        let mut profiles = vec![
            PathProfile {
                path: SIMDPath::Scalar,
                available: true,  // Always available
            },
        ];

        // Check for AVX2 (x86_64)
        #[cfg(target_arch = "x86_64")]
        {
            profiles.push(PathProfile {
                path: SIMDPath::AVX2,
                available: is_x86_feature_detected!("avx2"),
            });
        }

        // Check for NEON (ARM)
        #[cfg(target_arch = "aarch64")]
        {
            profiles.push(PathProfile {
                path: SIMDPath::NEON,
                available: cfg!(target_feature = "neon"),
            });
        }

        let dispatcher = Self {
            profiles,
            selected_path: AtomicUsize::new(0),  // Start with scalar
            last_profile_ns: std::sync::atomic::AtomicU64::new(0),
            path_latency_us_bits: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            path_passed_correctness: std::array::from_fn(|_| AtomicBool::new(false)),
            path_profiled: std::array::from_fn(|_| AtomicBool::new(false)),
        };

        // Profile all available paths
        dispatcher.profile_all()?;

        // Select best path (fastest among those that passed correctness)
        dispatcher.select_best();

        Ok(dispatcher)
    }

    fn profile_all(&self) -> Result<(), String> {
        for profile in &self.profiles {
            if !profile.available {
                continue;
            }

            // profile_simd_path now does a real correctness check against
            // the scalar reference and real wall-clock timing (see
            // profiler.rs) instead of returning a hardcoded placeholder.
            match profile_simd_path(profile.path, 1000) {
                Ok(result) => {
                    let path_idx = profile.path as usize;
                    self.path_latency_us_bits[path_idx]
                        .store(result.latency_us.to_bits(), Ordering::Relaxed);
                    self.path_passed_correctness[path_idx]
                        .store(result.passed_correctness, Ordering::Relaxed);
                    self.path_profiled[path_idx].store(true, Ordering::Relaxed);
                }
                Err(_) => {
                    // Profiling itself failed to run (not the same as
                    // failing correctness) -- leave path_profiled false so
                    // select_best treats it as ineligible, same as a path
                    // that ran but produced wrong output.
                }
            }
        }
        Ok(())
    }

    /// Select the fastest path that actually passed its correctness check.
    /// Falls back to Scalar (index 0, always available and always its own
    /// correctness reference) if nothing else qualifies -- including if
    /// profiling itself never ran for some reason.
    fn select_best(&self) {
        let mut best_idx = 0usize; // Scalar, guaranteed present at index 0
        let mut best_latency_us = f64::INFINITY;

        for (idx, profile) in self.profiles.iter().enumerate() {
            if !profile.available {
                continue;
            }
            let path_idx = profile.path as usize;
            if !self.path_profiled[path_idx].load(Ordering::Relaxed) {
                continue;
            }
            if !self.path_passed_correctness[path_idx].load(Ordering::Relaxed) {
                continue; // never select a path that produced wrong output
            }
            let latency_us =
                f64::from_bits(self.path_latency_us_bits[path_idx].load(Ordering::Relaxed));
            if latency_us < best_latency_us {
                best_latency_us = latency_us;
                best_idx = idx;
            }
        }

        self.selected_path.store(best_idx, Ordering::Relaxed);
    }

    pub fn selected_path(&self) -> SIMDPath {
        let idx = self.selected_path.load(Ordering::Relaxed);
        if idx < self.profiles.len() {
            self.profiles[idx].path
        } else {
            SIMDPath::Scalar
        }
    }

    pub fn matmul(
        &self,
        a: &[i8],
        b: &[i8],
        m: usize,
        k: usize,
        n: usize,
    ) -> Result<Vec<f32>, NtgError> {
        // SIMDPath is not cfg-gated per architecture, so this match must stay
        // exhaustive on every target -- cfg only decides which *body* runs,
        // never which *arm* exists. (A prior version cfg-gated whole arms,
        // which silently compiled out the NEON arm and its scalar-fallback
        // catch-all on x86_64, leaving no handler at all for that variant.)
        match self.selected_path() {
            SIMDPath::Scalar => matmul_scalar(a, b, m, k, n),

            SIMDPath::AVX2 => {
                #[cfg(target_arch = "x86_64")]
                {
                    if is_x86_feature_detected!("avx2") {
                        return super::avx2::matmul_avx2(a, b, m, k, n);
                    }
                }
                matmul_scalar(a, b, m, k, n)
            }

            SIMDPath::NEON => {
                #[cfg(target_arch = "aarch64")]
                {
                    if cfg!(target_feature = "neon") {
                        return super::neon::matmul_neon(a, b, m, k, n);
                    }
                }
                matmul_scalar(a, b, m, k, n)
            }

            SIMDPath::SSE41 => {
                // SSE4.1 not yet implemented, fall back to scalar
                matmul_scalar(a, b, m, k, n)
            }
        }
    }

    pub fn available_paths(&self) -> Vec<SIMDPath> {
        self.profiles
            .iter()
            .filter(|p| p.available)
            .map(|p| p.path)
            .collect()
    }
}

impl Default for SIMDDispatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create SIMD dispatcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_selects_available_path() -> Result<(), String> {
        let dispatcher = SIMDDispatcher::new()?;
        let available = dispatcher.available_paths();
        assert!(!available.is_empty());
        assert!(available.contains(&SIMDPath::Scalar));
        Ok(())
    }

    #[test]
    fn dispatcher_has_selected_path() -> Result<(), String> {
        let dispatcher = SIMDDispatcher::new()?;
        let selected = dispatcher.selected_path();
        assert_ne!(selected, SIMDPath::SSE41);  // SSE41 not yet implemented
        Ok(())
    }
}
