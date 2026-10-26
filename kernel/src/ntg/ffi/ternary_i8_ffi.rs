//! C ABI for the ternary x int8 GEMM, so Python (ctypes) can drive it.
//!
//! Handles are opaque `void*`. Typical use:
//! 1. `ntg_ternary_matrix_new` once per weight matrix (copies and packs the weights),
//! 2. `ntg_ternary_forward_f32` per call: BitNet-style per-row int8 activation
//!    quantisation, exact integer GEMM, dequantisation by `x_scale * weight_scale`,
//! 3. `ntg_ternary_matrix_free` when done.
//!
//! All functions return a negative value (or null) on invalid input and never unwind
//! across the boundary.

use crate::ntg::storage::{Int8Activations, PackedTernaryMatrix, TernaryI8Kernel};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

static POOL: Mutex<Option<Arc<rayon::ThreadPool>>> = Mutex::new(None);

fn pool() -> Option<Arc<rayon::ThreadPool>> {
    POOL.lock().ok().and_then(|g| g.clone())
}

/// Use `threads` worker threads for subsequent forward calls (`<= 1` selects the serial
/// path). Returns 0 on success, -1 if the pool could not be built.
#[no_mangle]
pub extern "C" fn ntg_ternary_set_threads(threads: usize) -> i32 {
    let new_pool = if threads <= 1 {
        None
    } else {
        match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
            Ok(p) => Some(Arc::new(p)),
            Err(_) => return -1,
        }
    };
    match POOL.lock() {
        Ok(mut g) => {
            *g = new_pool;
            0
        }
        Err(_) => -1,
    }
}

/// Fastest kernel on this CPU: 0 = scalar, 1 = AVX2, 2 = AVX-512 VNNI.
#[no_mangle]
pub extern "C" fn ntg_ternary_best_kernel() -> i32 {
    match TernaryI8Kernel::best_available() {
        TernaryI8Kernel::Scalar => 0,
        TernaryI8Kernel::Avx2 => 1,
        TernaryI8Kernel::Avx512Vnni => 2,
    }
}

/// Pack a `rows x cols` row-major ternary matrix. `scale` is the weight dequantisation
/// scale. Returns null if the shape is zero or a weight is outside `{-1, 0, 1}`.
///
/// # Safety
/// `weights` must be valid for reads of `rows * cols` bytes.
#[no_mangle]
pub unsafe extern "C" fn ntg_ternary_matrix_new(
    weights: *const i8,
    rows: usize,
    cols: usize,
    scale: f32,
) -> *mut c_void {
    if weights.is_null() || rows == 0 || cols == 0 {
        return std::ptr::null_mut();
    }
    let w = std::slice::from_raw_parts(weights, rows * cols);
    match PackedTernaryMatrix::from_i8(w, rows, cols) {
        Ok(m) => Box::into_raw(Box::new(m.with_scale(scale))) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Bytes of packed weight storage for a handle, or 0 for null.
///
/// # Safety
/// `m` must be null or a live handle from [`ntg_ternary_matrix_new`].
#[no_mangle]
pub unsafe extern "C" fn ntg_ternary_matrix_bytes(m: *const c_void) -> usize {
    (m as *const PackedTernaryMatrix).as_ref().map_or(0, |m| m.packed_bytes())
}

/// Free a handle. Null is ignored.
///
/// # Safety
/// `m` must be null or a handle from [`ntg_ternary_matrix_new`] that is not used again.
#[no_mangle]
pub unsafe extern "C" fn ntg_ternary_matrix_free(m: *mut c_void) {
    if !m.is_null() {
        drop(Box::from_raw(m as *mut PackedTernaryMatrix));
    }
}

/// `out[b][n] = (sum_k W[n][k] * q(x[b])[k]) * x_scale[b] * weight_scale` for a batch of
/// f32 rows (`batch x cols`), writing `batch x rows` f32 values. Returns 0 on success.
///
/// # Safety
/// `m` must be a live handle; `x` valid for `batch * cols` reads; `out` valid for
/// `batch * rows` writes.
#[no_mangle]
pub unsafe extern "C" fn ntg_ternary_forward_f32(
    m: *const c_void,
    x: *const f32,
    batch: usize,
    out: *mut f32,
) -> i32 {
    let Some(m) = (m as *const PackedTernaryMatrix).as_ref() else { return -1 };
    if x.is_null() || out.is_null() || batch == 0 {
        return -2;
    }
    let x = std::slice::from_raw_parts(x, batch * m.cols());
    let out = std::slice::from_raw_parts_mut(out, batch * m.rows());
    let Ok(acts) = Int8Activations::quantize_f32_bitnet(x, batch, m.cols()) else { return -3 };
    let kernel = TernaryI8Kernel::best_available();
    let mut acc = vec![0i32; batch * m.rows()];
    let ran = match pool() {
        Some(p) => m.gemm_i8_parallel(&acts, &mut acc, kernel, &p),
        None => m.gemm_i8(&acts, &mut acc, kernel),
    };
    if ran.is_err() {
        return -4;
    }
    for b in 0..batch {
        let s = acts.scales()[b] * m.scale();
        for n in 0..m.rows() {
            out[b * m.rows() + n] = acc[b * m.rows() + n] as f32 * s;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_matches_the_rust_api() {
        let (rows, cols, batch) = (40usize, 300usize, 3usize);
        let w: Vec<i8> = (0..rows * cols).map(|i| ((i * 7 + i / 3) % 3) as i8 - 1).collect();
        let x: Vec<f32> = (0..batch * cols).map(|i| ((i * 13) % 97) as f32 / 10.0 - 4.0).collect();
        let mut out = vec![0f32; batch * rows];
        for threads in [1usize, 3] {
            assert_eq!(ntg_ternary_set_threads(threads), 0);
            unsafe {
                let h = ntg_ternary_matrix_new(w.as_ptr(), rows, cols, 0.25);
                assert!(!h.is_null());
                assert_eq!(ntg_ternary_forward_f32(h, x.as_ptr(), batch, out.as_mut_ptr()), 0);
                ntg_ternary_matrix_free(h);
            }
            let m = PackedTernaryMatrix::from_i8(&w, rows, cols).unwrap().with_scale(0.25);
            let acts = Int8Activations::quantize_f32_bitnet(&x, batch, cols).unwrap();
            let mut acc = vec![0i32; batch * rows];
            m.gemm_i8(&acts, &mut acc, TernaryI8Kernel::Scalar).unwrap();
            for b in 0..batch {
                for n in 0..rows {
                    let want = acc[b * rows + n] as f32 * acts.scales()[b] * 0.25;
                    assert_eq!(out[b * rows + n], want, "threads {threads} ({b},{n})");
                }
            }
        }
        ntg_ternary_set_threads(1);
    }

    #[test]
    fn invalid_input_is_rejected_not_unwound() {
        unsafe {
            assert!(ntg_ternary_matrix_new(std::ptr::null(), 4, 4, 1.0).is_null());
            let bad = [0i8, 5, 0, 0];
            assert!(ntg_ternary_matrix_new(bad.as_ptr(), 2, 2, 1.0).is_null());
            assert_eq!(ntg_ternary_forward_f32(std::ptr::null(), std::ptr::null(), 1, std::ptr::null_mut()), -1);
            ntg_ternary_matrix_free(std::ptr::null_mut());
            assert_eq!(ntg_ternary_matrix_bytes(std::ptr::null()), 0);
        }
        assert!((0..=2).contains(&ntg_ternary_best_kernel()));
    }
}
