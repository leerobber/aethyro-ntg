//! Phase 1.2-1.3 Integration Tests: SIMD Dispatch + FFI + Observability
//!
//! Tests verify:
//! 1. SIMD paths produce bit-identical output to scalar reference
//! 2. FFI interface is zero-copy and safe
//! 3. OpStats flow correctly
//! 4. Performance deltas are measured and recorded

use ntg_kernel::ntg::{
    simd::{get_dispatcher, matmul_auto},
    ternary::matmul_scalar,
    ffi::{OpStats, ntg_matmul_ffi, ntg_get_op_count},
    error::NtgError,
};

/// Test 1: Dispatcher initializes and selects a path
#[test]
fn test_dispatcher_initialization() -> Result<(), NtgError> {
    let dispatcher = get_dispatcher()?;
    let selected = dispatcher.selected_path();
    assert!(!selected.name().is_empty());
    println!("Selected SIMD path: {}", selected.name());
    Ok(())
}

/// Test 2: All matmul implementations produce identical output (bit-parity)
#[test]
fn test_simd_bit_parity_simple() -> Result<(), NtgError> {
    let a = vec![1i8, -1, 0, 1];
    let b = vec![1i8, 0, -1, 1];

    // Scalar reference
    let scalar_result = matmul_scalar(&a, &b, 2, 2, 2)?;

    // Auto (uses best available SIMD path)
    let auto_result = matmul_auto(&a, &b, 2, 2, 2)?;

    // Bit-identical comparison
    assert_eq!(
        scalar_result, auto_result,
        "Auto result differs from scalar reference"
    );

    println!("✓ Bit-parity test passed: {:?}", scalar_result);
    Ok(())
}

/// Test 3: Larger matrix test for SIMD paths
#[test]
fn test_simd_bit_parity_large() -> Result<(), NtgError> {
    // 50x50 matrix filled from a repeating ternary pattern
    let a_pat = [1i8, -1, 0, 1, -1, 0, 1, -1, 0, 1];
    let b_pat = [0i8, 1, -1, 0, 1, -1, 0, 1, -1, 0];
    let a: Vec<i8> = a_pat.iter().copied().cycle().take(50 * 50).collect();
    let b: Vec<i8> = b_pat.iter().copied().cycle().take(50 * 50).collect();

    let scalar_result = matmul_scalar(&a, &b, 50, 50, 50)?;
    let auto_result = matmul_auto(&a, &b, 50, 50, 50)?;

    assert_eq!(scalar_result, auto_result);
    println!("✓ Large matrix bit-parity test passed (50x50x50)");
    Ok(())
}

/// Test 4: FFI matmul call
#[test]
fn test_ffi_matmul_call() {
    let a = [1i8, -1, 0, 1];
    let b = [1i8, 0, -1, 1];
    let mut out = vec![0.0f32; 4];
    let mut stats = OpStats::default();

    let result = unsafe {
        ntg_matmul_ffi(
            a.as_ptr(),
            2,
            2,
            b.as_ptr(),
            2,
            2,
            out.as_mut_ptr(),
            &mut stats as *mut OpStats,
        )
    };

    assert_eq!(result, 0, "FFI call failed");
    assert_ne!(out[0], 0.0, "Output not filled");

    // Expected: [2, -1, -1, 1]
    assert_eq!(out[0], 2.0);
    assert_eq!(out[1], -1.0);
    assert_eq!(out[2], -1.0);
    assert_eq!(out[3], 1.0);

    println!("✓ FFI matmul test passed");
    println!("  Latency: {} us", stats.latency_us);
    println!("  Memory: {} bytes", stats.memory_bytes);
    println!("  SIMD path: {}", stats.simd_path_name());
}

/// Test 5: FFI operation counter
#[test]
fn test_ffi_op_counter() {
    let before = ntg_get_op_count();

    let a = [1i8; 4];
    let b = [1i8; 4];
    let mut out = vec![0.0f32; 4];

    unsafe {
        ntg_matmul_ffi(
            a.as_ptr(),
            2,
            2,
            b.as_ptr(),
            2,
            2,
            out.as_mut_ptr(),
            std::ptr::null_mut(),
        );
    }

    let after = ntg_get_op_count();
    assert!(after > before, "Op counter should increment");
    println!("✓ Op counter test passed: {} -> {}", before, after);
}

/// Test 6: FFI rejects null pointers
#[test]
fn test_ffi_null_pointer_rejection() {
    let result = unsafe {
        ntg_matmul_ffi(
            std::ptr::null(),
            2,
            2,
            std::ptr::null(),
            2,
            2,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    assert_eq!(result, -1, "Should reject null pointers");
    println!("✓ Null pointer rejection test passed");
}

/// Test 7: FFI rejects dimension mismatch
#[test]
fn test_ffi_dimension_mismatch() {
    let a = [1i8; 4];
    let b = [1i8; 6];
    let mut out = vec![0.0f32; 4];

    let result = unsafe {
        ntg_matmul_ffi(
            a.as_ptr(),
            2,
            2,
            b.as_ptr(),
            3,  // Doesn't match k=2
            2,
            out.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };

    assert_eq!(result, -1, "Should reject dimension mismatch");
    println!("✓ Dimension mismatch rejection test passed");
}

/// Test 8: Performance comparison (scalar vs auto)
#[test]
fn test_performance_delta() -> Result<(), NtgError> {
    let size = 100;  // 100x100x100 matmul
    let a = vec![1i8; size * size];
    let b = vec![1i8; size * size];

    // Measure scalar
    let start = std::time::Instant::now();
    let scalar_result = matmul_scalar(&a, &b, size, size, size)?;
    let scalar_time = start.elapsed().as_micros() as f64;

    // Measure auto (SIMD if available)
    let start = std::time::Instant::now();
    let auto_result = matmul_auto(&a, &b, size, size, size)?;
    let auto_time = start.elapsed().as_micros() as f64;

    // Verify correctness
    assert_eq!(scalar_result, auto_result, "Results must be identical");

    let speedup = scalar_time / auto_time.max(1.0);
    println!("✓ Performance delta test:");
    println!("  Scalar: {:.1} us", scalar_time);
    println!("  Auto:   {:.1} us", auto_time);
    println!("  Speedup: {:.2}x", speedup);

    Ok(())
}

/// Test 9: OpStats dual-objective fitness check
#[test]
fn test_opstats_fitness() {
    let baseline = OpStats {
        latency_us: 5000,
        memory_bytes: 1024,
        simd_path: 0,
        timestamp_ns: 0,
    };

    let improved = OpStats {
        latency_us: 4950,
        memory_bytes: 1000,
        simd_path: 1,
        timestamp_ns: 0,
    };

    assert!(improved.improves_over(&baseline, 1.01));

    let regressed = OpStats {
        latency_us: 5100,
        memory_bytes: 1100,
        simd_path: 0,
        timestamp_ns: 0,
    };

    assert!(!regressed.improves_over(&baseline, 1.01));
    println!("✓ OpStats fitness test passed");
}

/// Test 10: OpStats to JSON serialization
#[test]
fn test_opstats_json() {
    let stats = OpStats {
        latency_us: 1234,
        memory_bytes: 5678,
        simd_path: 1,
        timestamp_ns: 9999,
    };

    let json = stats.to_json();
    assert!(json.contains("1234"));
    assert!(json.contains("5678"));
    assert!(json.contains("AVX2"));
    println!("✓ OpStats JSON test passed");
    println!("  JSON: {}", json);
}

/// Test 11: End-to-end SIMD + FFI + Ledger integration
#[test]
fn test_end_to_end_integration() -> Result<(), NtgError> {
    // Phase 1.2: SIMD dispatcher selects best path
    let dispatcher = get_dispatcher()?;
    println!("Selected path: {}", dispatcher.selected_path().name());

    // Phase 1.3: Call via FFI with stats
    let a = vec![1i8, -1, 0, 1, -1, 0, 1, -1, 0, 1, -1, 0];
    let b = vec![1i8, 0, -1, 1, 0, 1, -1, 0, 1, -1, 0, 1];
    let mut out = vec![0.0f32; 9];
    let mut stats = OpStats::default();

    let result = unsafe {
        ntg_matmul_ffi(
            a.as_ptr(),
            3,
            4,
            b.as_ptr(),
            4,
            3,
            out.as_mut_ptr(),
            &mut stats as *mut OpStats,
        )
    };

    assert_eq!(result, 0);
    assert_ne!(out[0], 0.0);

    // Verify stats would flow into Phase 3 ledger
    let json = stats.to_json();
    println!("Stats for ledger: {}", json);

    println!("✓ End-to-end integration test passed");
    Ok(())
}
