//! NTG Genomic FFI: C-compatible interface for OmniSynth-X
//!
//! Exposes genomic operator to FFI for Python/C integration
//! Handles: LD computation, PRS scoring, LD clustering

use crate::ntg::operators::genomic::GenomicOperator;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global genomic operation counter
static GENOMIC_OP_COUNT: AtomicU64 = AtomicU64::new(0);

/// Opaque handle for C code (wraps GenomicOperator)
pub struct GenomicHandle {
    inner: GenomicOperator,
}

// ═══════════════════════════════════════════════════════════════
// LIFECYCLE
// ═══════════════════════════════════════════════════════════════

/// Create new GenomicOperator (C interface)
///
/// # Safety
/// Caller must call `ntg_genomic_drop` to free allocated memory.
#[no_mangle]
pub extern "C" fn ntg_genomic_new(
    num_individuals: usize,
    num_snps: usize,
) -> *mut GenomicHandle {
    let handle = Box::new(GenomicHandle {
        inner: GenomicOperator::new(num_individuals, num_snps),
    });
    Box::into_raw(handle)
}

/// Destroy GenomicOperator (C interface)
///
/// # Safety
/// `handle` must be either null or a pointer previously returned by
/// `ntg_genomic_new` that has not already been passed to this function.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_drop(handle: *mut GenomicHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// GENOTYPE I/O
// ═══════════════════════════════════════════════════════════════

/// Set genotype value
/// val: 0=ref/ref, 1=ref/alt, 2=alt/alt, 3=missing
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_set(
    handle: *mut GenomicHandle,
    snp_idx: usize,
    ind_idx: usize,
    val: u8,
) -> i32 {
    if handle.is_null() {
        return -1; // EINVAL
    }

    if snp_idx >= (*handle).inner.num_snps || ind_idx >= (*handle).inner.num_individuals {
        return -2; // out of bounds
    }
    (*handle).inner.set(snp_idx, ind_idx, val);
    0
}

/// Get genotype value
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_get(
    handle: *const GenomicHandle,
    snp_idx: usize,
    ind_idx: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    if snp_idx >= (*handle).inner.num_snps || ind_idx >= (*handle).inner.num_individuals {
        return -2;
    }
    (*handle).inner.get(snp_idx, ind_idx) as i32
}

// ═══════════════════════════════════════════════════════════════
// STATISTICS
// ═══════════════════════════════════════════════════════════════

/// Compute means and standard deviations
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_compute_statistics(handle: *mut GenomicHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }

    (*handle).inner.compute_statistics();
    GENOMIC_OP_COUNT.fetch_add(1, Ordering::Relaxed);
    0
}

/// Get mean for a SNP
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_get_mean(handle: *const GenomicHandle, snp_idx: usize) -> f64 {
    if handle.is_null() {
        return 0.0;
    }

    let inner = &(*handle).inner;
    if snp_idx >= inner.num_snps {
        return 0.0;
    }
    inner.means.get(snp_idx).copied().unwrap_or(0.0)
}

/// Get std dev for a SNP
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_get_std_dev(handle: *const GenomicHandle, snp_idx: usize) -> f64 {
    if handle.is_null() {
        return 0.0;
    }

    let inner = &(*handle).inner;
    if snp_idx >= inner.num_snps {
        return 0.0;
    }
    inner.std_devs.get(snp_idx).copied().unwrap_or(1.0)
}

// ═══════════════════════════════════════════════════════════════
// LINKAGE DISEQUILIBRIUM
// ═══════════════════════════════════════════════════════════════

/// Compute LD matrix (returns into pre-allocated buffer)
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed. `out` must point to at
/// least (num_snps * num_snps) f64 values.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_compute_ld(
    handle: *mut GenomicHandle,
    out: *mut f64,
    out_len: usize,
) -> i32 {
    if handle.is_null() || out.is_null() {
        return -1;
    }

    let expected_len = (*handle).inner.num_snps * (*handle).inner.num_snps;
    if out_len < expected_len {
        return -2; // buffer too small
    }

    let ld_matrix = (*handle).inner.compute_ld_matrix();

    for (i, val) in ld_matrix.iter().enumerate() {
        if i < out_len {
            *out.add(i) = *val;
        }
    }

    GENOMIC_OP_COUNT.fetch_add(1, Ordering::Relaxed);
    ld_matrix.len() as i32
}

/// Get single LD value (r)
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_get_ld(
    handle: *mut GenomicHandle,
    snp_i: usize,
    snp_j: usize,
) -> f64 {
    if handle.is_null() {
        return 0.0;
    }

    if snp_i >= (*handle).inner.num_snps || snp_j >= (*handle).inner.num_snps {
        return 0.0;
    }

    let ld_matrix = (*handle).inner.compute_ld_matrix();
    ld_matrix[snp_i * (*handle).inner.num_snps + snp_j]
}

// ═══════════════════════════════════════════════════════════════
// POLYGENIC RISK SCORES
// ═══════════════════════════════════════════════════════════════

/// Compute PRS for all individuals
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed. `weights` must point to at
/// least num_snps f64 values. `out` must point to at least
/// num_individuals f64 values.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_compute_prs(
    handle: *const GenomicHandle,
    weights: *const f64,
    weights_len: usize,
    out: *mut f64,
    out_len: usize,
) -> i32 {
    if handle.is_null() || weights.is_null() || out.is_null() {
        return -1;
    }

    if weights_len != (*handle).inner.num_snps {
        return -2; // dimension mismatch
    }
    if out_len < (*handle).inner.num_individuals {
        return -3; // output buffer too small
    }

    let weights_slice = std::slice::from_raw_parts(weights, weights_len);
    let prs = (*handle).inner.compute_prs(weights_slice);

    for (i, val) in prs.iter().enumerate() {
        if i < out_len {
            *out.add(i) = *val;
        }
    }

    GENOMIC_OP_COUNT.fetch_add(1, Ordering::Relaxed);
    prs.len() as i32
}

// ═══════════════════════════════════════════════════════════════
// METADATA
// ═══════════════════════════════════════════════════════════════

/// Get number of individuals
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_num_individuals(handle: *const GenomicHandle) -> usize {
    if handle.is_null() {
        return 0;
    }

    (*handle).inner.num_individuals
}

/// Get number of SNPs
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_num_snps(handle: *const GenomicHandle) -> usize {
    if handle.is_null() {
        return 0;
    }

    (*handle).inner.num_snps
}

/// Get missing data rate (0.0 to 1.0) - stub implementation
#[no_mangle]
pub extern "C" fn ntg_genomic_missing_rate(_handle: *const GenomicHandle) -> f64 {
    // TODO: implement missing rate estimation in GenomicOperator
    0.0
}

/// Get total genomic operation count (for profiling)
#[no_mangle]
pub extern "C" fn ntg_genomic_op_count() -> u64 {
    GENOMIC_OP_COUNT.load(Ordering::Relaxed)
}

// ═══════════════════════════════════════════════════════════════
// UTILITY
// ═══════════════════════════════════════════════════════════════

/// Bulk load genotypes from array
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed. `genotypes` must point to
/// (num_snps * num_individuals) u8 values, in row-major order
/// (SNP-major).
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_load_bulk(
    handle: *mut GenomicHandle,
    genotypes: *const u8,
    num_genotypes: usize,
) -> i32 {
    if handle.is_null() || genotypes.is_null() {
        return -1;
    }

    let expected = (*handle).inner.num_snps * (*handle).inner.num_individuals;
    if num_genotypes != expected {
        return -2; // size mismatch
    }

    let geno_slice = std::slice::from_raw_parts(genotypes, num_genotypes);
    let mut idx = 0;

    for snp in 0..(*handle).inner.num_snps {
        for ind in 0..(*handle).inner.num_individuals {
            if idx < geno_slice.len() {
                (*handle).inner.set(snp, ind, geno_slice[idx]);
                idx += 1;
            }
        }
    }

    GENOMIC_OP_COUNT.fetch_add(1, Ordering::Relaxed);
    0
}

/// Export all genotypes (for serialization)
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `ntg_genomic_new` that has not been freed. `out` must point to at
/// least (num_snps * num_individuals) u8 values.
#[no_mangle]
pub unsafe extern "C" fn ntg_genomic_export_bulk(
    handle: *const GenomicHandle,
    out: *mut u8,
    out_len: usize,
) -> i32 {
    if handle.is_null() || out.is_null() {
        return -1;
    }

    let expected = (*handle).inner.num_snps * (*handle).inner.num_individuals;
    if out_len < expected {
        return -2; // buffer too small
    }

    let mut idx = 0;
    for snp in 0..(*handle).inner.num_snps {
        for ind in 0..(*handle).inner.num_individuals {
            let val = (*handle).inner.get(snp, ind);
            *out.add(idx) = val;
            idx += 1;
        }
    }

    idx as i32
}
