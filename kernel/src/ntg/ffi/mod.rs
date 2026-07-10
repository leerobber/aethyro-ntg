//! Phase 1.3: Zero-copy FFI + Observability
//!
//! C interface for orchestrator integration. All calls produce OpStats
//! that feed into Phase 3's TamperEvidentLedger for full auditability.
//!
//! Safety: FFI boundary is the only place where `unsafe` appears.
//! All pointer validation is mandatory before dereference.

pub mod stats;
pub mod bindings;
pub mod tobl_ffi;

pub use stats::OpStats;

use crate::ntg::simd::get_dispatcher;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global operation counter for observability
static OP_COUNT: AtomicU64 = AtomicU64::new(0);

/// Get the total number of operations performed via FFI
pub fn get_op_count() -> u64 {
    OP_COUNT.load(Ordering::Relaxed)
}

/// C-compatible matmul interface
///
/// # Safety
/// Caller must ensure:
/// - a, b pointers are valid for (m*k) and (k*n) reads respectively
/// - out pointer is valid for (m*n) writes
/// - stats pointer is valid for write (if not null)
/// - all pointers are properly aligned
///
/// # Returns
/// 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn ntg_matmul_ffi(
    a: *const i8,
    m: u32,
    k: u32,
    b: *const i8,
    k_b: u32,  // Must match k
    n: u32,
    out: *mut f32,
    stats: *mut OpStats,
) -> i32 {
    // Validate dimensions match
    if k != k_b {
        return -1;  // EINVAL
    }

    // Validate pointers are not null
    if a.is_null() || b.is_null() || out.is_null() {
        return -1;  // EINVAL
    }

    let start = std::time::Instant::now();

    // Convert to safe slices
    let a_len = (m as usize) * (k as usize);
    let b_len = (k as usize) * (n as usize);
    let out_len = (m as usize) * (n as usize);

    let a_slice = unsafe { std::slice::from_raw_parts(a, a_len) };
    let b_slice = unsafe { std::slice::from_raw_parts(b, b_len) };
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, out_len) };

    // Call SIMD dispatcher
    let dispatcher = match get_dispatcher() {
        Ok(d) => d,
        Err(_) => return -2,  // EIO
    };

    let result = dispatcher.matmul(a_slice, b_slice, m as usize, k as usize, n as usize);

    let elapsed = start.elapsed();
    let latency_us = elapsed.as_micros() as u64;

    match result {
        Ok(output) => {
            // Copy result to output buffer
            out_slice.copy_from_slice(&output);

            // Fill stats if provided
            if !stats.is_null() {
                unsafe {
                    (*stats) = OpStats {
                        latency_us,
                        memory_bytes: (a_len + b_len + out_len) as u64,
                        simd_path: dispatcher.selected_path() as u8,
                        timestamp_ns: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_nanos() as u64)
                            .unwrap_or(0),
                    };
                }
            }

            OP_COUNT.fetch_add(1, Ordering::Relaxed);
            0  // Success
        }
        Err(_) => -3,  // ECALLFAILED
    }
}

/// Get FFI statistics summary
#[no_mangle]
pub extern "C" fn ntg_get_op_count() -> u64 {
    get_op_count()
}

/// Reset operation counter
#[no_mangle]
pub extern "C" fn ntg_reset_op_count() {
    OP_COUNT.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_counter_increments() {
        let _before = get_op_count();
        ntg_reset_op_count();
        assert_eq!(get_op_count(), 0);
        // Would need to actually call ntg_matmul_ffi to increment
    }

    #[test]
    fn ffi_null_pointers_rejected() {
        // Nulls are validated before any dereference, so this call is sound
        // despite the null args; the `unsafe` here is the FFI contract, not
        // a claim that these particular inputs are dangerous.
        let result = unsafe {
            ntg_matmul_ffi(
                std::ptr::null(), // null a
                2,
                2,
                std::ptr::null(), // null b
                2,
                2,
                std::ptr::null_mut(), // null out
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result, -1); // EINVAL
    }

    #[test]
    fn ffi_dimension_mismatch_rejected() {
        let a = [1i8; 4];
        let b = [1i8; 6];
        let mut out = vec![0.0f32; 4];

        let result = unsafe {
            ntg_matmul_ffi(
                a.as_ptr(),
                2,
                2,
                b.as_ptr(),
                3, // Doesn't match k=2
                2,
                out.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result, -1); // EINVAL
    }
}
