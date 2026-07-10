//! Phase 1.3: TOBL FFI extensions for PackedTernary operations
//!
//! C-compatible interface for ternary dot-product operations.
//! Bridges storage layer (PackedTernary) with orchestrator observability.

use crate::ntg::storage::{PackedTernary, tobl_dot_product};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global TOBL operation counter
static TOBL_OP_COUNT: AtomicU64 = AtomicU64::new(0);

/// Opaque handle for C code (wraps PackedTernary)
pub struct ToblHandle {
    inner: PackedTernary,
}

/// Create new PackedTernary (C interface)
///
/// # Safety
/// Caller must call `ntg_tobl_drop` to free allocated memory.
#[no_mangle]
pub extern "C" fn ntg_tobl_new(len: u32) -> *mut ToblHandle {
    let handle = Box::new(ToblHandle {
        inner: PackedTernary::new(len as usize),
    });
    Box::into_raw(handle)
}

/// Destroy PackedTernary (C interface)
///
/// # Safety
/// `handle` must be either null or a pointer previously returned by
/// `ntg_tobl_new` that has not already been passed to `ntg_tobl_drop`.
#[no_mangle]
pub unsafe extern "C" fn ntg_tobl_drop(handle: *mut ToblHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Set single ternary value
///
/// # Safety
/// `handle` must be either null or a valid, live pointer returned by
/// `ntg_tobl_new`.
#[no_mangle]
pub unsafe extern "C" fn ntg_tobl_set(handle: *mut ToblHandle, idx: u32, val: i8) -> i32 {
    if handle.is_null() {
        return -1; // EINVAL
    }

    unsafe {
        if idx as usize >= (*handle).inner.len() {
            return -1; // out of bounds
        }
        (*handle).inner.set(idx as usize, val);
        0
    }
}

/// Get single ternary value
///
/// # Safety
/// `handle` must be either null or a valid, live pointer returned by
/// `ntg_tobl_new`.
#[no_mangle]
pub unsafe extern "C" fn ntg_tobl_get(handle: *const ToblHandle, idx: u32) -> i8 {
    if handle.is_null() {
        return 0;
    }

    unsafe {
        if idx as usize >= (*handle).inner.len() {
            return 0;
        }
        (*handle).inner.get(idx as usize)
    }
}

/// Ternary dot-product: C interface
///
/// # Safety
/// `a` and `b` must be either null or valid, live pointers returned by
/// `ntg_tobl_new`, with equal length. `result` and `cycles` must be either
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn ntg_tobl_dot(
    a: *const ToblHandle,
    b: *const ToblHandle,
    result: *mut i64,
    cycles: *mut u64,
) -> i32 {
    if a.is_null() || b.is_null() {
        return -1; // EINVAL
    }

    unsafe {
        let a_ref = &(*a).inner;
        let b_ref = &(*b).inner;

        match tobl_dot_product(a_ref, b_ref, None) {
            Ok((dot, cyc)) => {
                if !result.is_null() {
                    *result = dot;
                }
                if !cycles.is_null() {
                    *cycles = cyc;
                }
                TOBL_OP_COUNT.fetch_add(1, Ordering::Relaxed);
                0
            }
            Err(_) => -2, // EIO
        }
    }
}

/// Get density metric for structural evolution heuristics
///
/// # Safety
/// `handle` must be either null or a valid, live pointer returned by
/// `ntg_tobl_new`.
#[no_mangle]
pub unsafe extern "C" fn ntg_tobl_density(handle: *mut ToblHandle) -> f32 {
    if handle.is_null() {
        return 0.0;
    }

    unsafe { (*handle).inner.compute_density() }
}

/// Get total TOBL operations
#[no_mangle]
pub extern "C" fn ntg_tobl_op_count() -> u64 {
    TOBL_OP_COUNT.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tobl_ffi_new_drop() {
        unsafe {
            let handle = ntg_tobl_new(100);
            assert!(!handle.is_null());
            ntg_tobl_drop(handle);
        }
    }

    #[test]
    fn tobl_ffi_set_get() {
        unsafe {
            let handle = ntg_tobl_new(10);
            assert_eq!(ntg_tobl_set(handle, 0, 1), 0);
            assert_eq!(ntg_tobl_get(handle, 0), 1);
            ntg_tobl_drop(handle);
        }
    }

    #[test]
    fn tobl_ffi_dot() {
        unsafe {
            let a = ntg_tobl_new(10);
            let b = ntg_tobl_new(10);

            // Set values: [1, -1, 0, ...]
            ntg_tobl_set(a, 0, 1);
            ntg_tobl_set(a, 1, -1);
            ntg_tobl_set(b, 0, 1);
            ntg_tobl_set(b, 1, -1);

            let mut result: i64 = 0;
            let mut cycles: u64 = 0;
            assert_eq!(ntg_tobl_dot(a, b, &mut result, &mut cycles), 0);
            // 1*1 + (-1)*(-1) = 2
            assert!(result > 0);

            ntg_tobl_drop(a);
            ntg_tobl_drop(b);
        }
    }
}
