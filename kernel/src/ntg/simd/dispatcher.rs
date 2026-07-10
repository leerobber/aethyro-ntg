//! SIMD path dispatcher: CPU feature detection + selection.
//!
//! At startup, detects available CPU features (AVX2, NEON, etc.) and
//! selects a path by static preference (AVX2 > NEON > scalar) — matmul()
//! also re-checks the feature at call time before dispatching.
//!
//! **Not yet real:** `profile_all` calls [`profile_simd_path`] per
//! available path, but `profile_simd_path` is a stub that returns a fixed
//! placeholder (`latency_us: 0.0`) rather than measuring anything, and
//! `profile_all`'s own results are discarded before `select_best` runs
//! (see the comment in `profile_all`) — path selection is feature
//! detection only, not benchmark-driven, despite this module's name.
//! Wiring real measurement into selection (e.g. via interior mutability
//! on `profiles`) is open work, not yet done.
//!
//! Gracefully falls back to scalar if SIMD unavailable or fails.

use super::profiler::{ProfileResult, profile_simd_path};
use super::super::ternary::matmul_scalar;
use super::super::error::NtgError;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    /// Reserved for interior-mutability profiling (see `profile_all`).
    #[allow(dead_code)]
    benchmark: Option<ProfileResult>,
}

pub struct SIMDDispatcher {
    profiles: Vec<PathProfile>,
    selected_path: AtomicUsize, // Index into profiles
    /// Wall time of last profile run (ns); reserved for adaptive re-profile.
    #[allow(dead_code)]
    last_profile_ns: std::sync::atomic::AtomicU64,
}

impl SIMDDispatcher {
    pub fn new() -> Result<Self, String> {
        // Detect available paths
        let mut profiles = vec![
            PathProfile {
                path: SIMDPath::Scalar,
                available: true,  // Always available
                benchmark: None,
            },
        ];

        // Check for AVX2 (x86_64)
        #[cfg(target_arch = "x86_64")]
        {
            profiles.push(PathProfile {
                path: SIMDPath::AVX2,
                available: is_x86_feature_detected!("avx2"),
                benchmark: None,
            });
        }

        // Check for NEON (ARM)
        #[cfg(target_arch = "aarch64")]
        {
            profiles.push(PathProfile {
                path: SIMDPath::NEON,
                available: cfg!(target_feature = "neon"),
                benchmark: None,
            });
        }

        let dispatcher = Self {
            profiles,
            selected_path: AtomicUsize::new(0),  // Start with scalar
            last_profile_ns: std::sync::atomic::AtomicU64::new(0),
        };

        // Profile all available paths
        dispatcher.profile_all()?;

        // Select best path
        dispatcher.select_best();

        Ok(dispatcher)
    }

    fn profile_all(&self) -> Result<(), String> {
        for profile in &self.profiles {
            if !profile.available {
                continue;
            }

            match profile_simd_path(profile.path, 1000) {
                Ok(_result) => {
                    // Store benchmark result
                    let idx = self.profiles
                        .iter()
                        .position(|p| p.path == profile.path)
                        .unwrap();
                    let _ = &self.profiles[idx];
                    // Note: We'd ideally mutate here, but Rust's borrow checker
                    // makes this difficult. In a real implementation, use interior
                    // mutability (RefCell, Mutex, or Arc<Mutex<>>).
                }
                Err(_) => {
                    // Profiling failed, mark as unavailable
                }
            }
        }
        Ok(())
    }

    fn select_best(&self) {
        // Select the fastest available path
        // For now: prefer SIMD if available, else scalar
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                self.selected_path.store(1, Ordering::Relaxed);  // AVX2
                return;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if cfg!(target_feature = "neon") {
                self.selected_path.store(2, Ordering::Relaxed);  // NEON
                return;
            }
        }

        self.selected_path.store(0, Ordering::Relaxed);  // Scalar fallback
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
        match self.selected_path() {
            SIMDPath::Scalar | SIMDPath::SSE41 => matmul_scalar(a, b, m, k, n),

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
