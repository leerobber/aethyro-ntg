//! Runtime-dispatched SIMD acceleration for ternary matmul.
//!
//! Phase 1.2 implementation:
//! - CPU feature detection at startup
//! - Profile each available SIMD path (AVX2, NEON, Scalar)
//! - Runtime dispatch selection based on measured performance
//! - Adaptive re-profiling if environment changes
//!
//! All paths must produce **bit-identical** output to scalar reference.
//! Performance deltas are measured on CI and recorded honestly.

pub mod dispatcher;
pub mod avx2;
pub mod neon;
pub mod profiler;

pub use dispatcher::SIMDDispatcher;
pub use profiler::{BenchmarkResult, ProfileResult};

use super::error::NtgError;

/// Global SIMD dispatcher instance (lazy-initialized at first use).
static DISPATCHER_INSTANCE: std::sync::OnceLock<SIMDDispatcher> = std::sync::OnceLock::new();

/// Get or initialize the global SIMD dispatcher.
pub fn get_dispatcher() -> Result<&'static SIMDDispatcher, NtgError> {
    // OnceLock::get_or_try_init is still unstable; init once and map errors.
    if DISPATCHER_INSTANCE.get().is_none() {
        let dispatcher = SIMDDispatcher::new().map_err(|e| {
            NtgError::InvalidInput(format!("Failed to initialize SIMD dispatcher: {}", e))
        })?;
        let _ = DISPATCHER_INSTANCE.set(dispatcher);
    }
    Ok(DISPATCHER_INSTANCE
        .get()
        .expect("SIMD dispatcher set above"))
}

/// High-level matmul that uses optimal SIMD path for this hardware.
pub fn matmul_auto(
    a: &[i8],
    b: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>, NtgError> {
    let dispatcher = get_dispatcher()?;
    dispatcher.matmul(a, b, m, k, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_initializes() -> Result<(), NtgError> {
        let _ = get_dispatcher()?;
        Ok(())
    }

    #[test]
    fn matmul_auto_works() -> Result<(), NtgError> {
        let a = vec![1i8, -1, 0, 1];
        let b = vec![1i8, 0, -1, 1];
        let result = matmul_auto(&a, &b, 2, 2, 2)?;
        assert_eq!(result.len(), 4);
        Ok(())
    }
}
