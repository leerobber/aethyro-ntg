//! TOBL (Ternary Operations with Bitwise Logic) AVX-512 kernels.
//!
//! Vectorized implementations of ternary operations using AVX-512
//! and AVX-512VPOPCNTDQ instructions for maximum throughput.

use std::arch::x86_64::*;
use crate::storage::sparse_bit_sliced_ternary::BitSlicedBlock;

/// 8-Way Vectorized TOBL dot product evaluating 512 elements in parallel.
/// Processes 8 BitSlicedBlock pairs per invocation.
/// Caller must guarantee that the 8 pairs have already been validated as matching chunk IDs.
#[target_feature(enable = "avx512f", enable = "avx512vpopcntdq")]
pub unsafe fn dot_block_avx512_x8(a: &[BitSlicedBlock; 8], b: &[BitSlicedBlock; 8]) -> i64 {
    // Pack 8 individual 64-bit streams into contiguous 512-bit hardware layouts
    // Note: _mm512_set_epi64 takes arguments from highest lane to lowest
    let a_pos = _mm512_set_epi64(
        a[7].pos as i64, a[6].pos as i64, a[5].pos as i64, a[4].pos as i64,
        a[3].pos as i64, a[2].pos as i64, a[1].pos as i64, a[0].pos as i64,
    );
    let a_neg = _mm512_set_epi64(
        a[7].neg as i64, a[6].neg as i64, a[5].neg as i64, a[4].neg as i64,
        a[3].neg as i64, a[2].neg as i64, a[1].neg as i64, a[0].neg as i64,
    );
    let b_pos = _mm512_set_epi64(
        b[7].pos as i64, b[6].pos as i64, b[5].pos as i64, b[4].pos as i64,
        b[3].pos as i64, b[2].pos as i64, b[1].pos as i64, b[0].pos as i64,
    );
    let b_neg = _mm512_set_epi64(
        b[7].neg as i64, b[6].neg as i64, b[5].neg as i64, b[4].neg as i64,
        b[3].neg as i64, b[2].neg as i64, b[1].neg as i64, b[0].neg as i64,
    );

    // Concurrent evaluation of sign matches across all 8 blocks
    let pp = _mm512_and_epi64(a_pos, b_pos);
    let mm = _mm512_and_epi64(a_neg, b_neg);
    let pm = _mm512_and_epi64(a_pos, b_neg);
    let mp = _mm512_and_epi64(a_neg, b_pos);

    // Vectorized population count executed across each independent 64-bit lane
    let pp_cnt = _mm512_popcnt_epi64(pp);
    let mm_cnt = _mm512_popcnt_epi64(mm);
    let pm_cnt = _mm512_popcnt_epi64(pm);
    let mp_cnt = _mm512_popcnt_epi64(mp);

    // Zero-allocation data extraction arrays for lane calculation
    let mut buf_pp = [0i64; 8];
    let mut buf_mm = [0i64; 8];
    let mut buf_pm = [0i64; 8];
    let mut buf_mp = [0i64; 8];

    // Toolchain-safe vector store
    _mm512_storeu_si512(buf_pp.as_mut_ptr() as *mut __m512i, pp_cnt);
    _mm512_storeu_si512(buf_mm.as_mut_ptr() as *mut __m512i, mm_cnt);
    _mm512_storeu_si512(buf_pm.as_mut_ptr() as *mut __m512i, pm_cnt);
    _mm512_storeu_si512(buf_mp.as_mut_ptr() as *mut __m512i, mp_cnt);

    // Linear reduction loop
    let mut total_score: i64 = 0;
    for i in 0..8 {
        total_score += (buf_pp[i] + buf_mm[i]) - (buf_pm[i] + buf_mp[i]);
    }

    total_score
}
