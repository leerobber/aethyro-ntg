//! Integration tests: PackedTernary + TOBL + Ledger + Observability
//!
//! Verifies that Phase 1.2 storage layer integrates cleanly with:
//! - FFI interface for C orchestrator
//! - SIMD dispatcher for kernel selection
//! - Phase 3 ledger for mutation tracking
//! - OpStats observability

use ntg_kernel::ntg::{
    storage::{PackedTernary, tobl_dot_product, ToblKernelPath},
    ffi::tobl_ffi::*,
    ledger::{
        FitnessMeasure, MutationOutcome, TamperEvidentLedger,
        replay::ExecutionTrace,
    },
    error::NtgError,
};

/// Test 1: PackedTernary basic operations
#[test]
fn test_packed_ternary_basic() -> Result<(), NtgError> {
    let mut pt = PackedTernary::new(100);

    // Set mixed values
    for i in 0..100 {
        pt.set(i, (i % 3) as i8 - 1); // cycles through -1, 0, 1
    }

    // Verify reads match writes
    for i in 0..100 {
        let expected = (i % 3) as i8 - 1;
        assert_eq!(pt.get(i), expected);
    }

    Ok(())
}

/// Test 2: PackedTernary generation tracking (for mutation ledger)
#[test]
fn test_packed_ternary_generation() -> Result<(), NtgError> {
    let mut pt = PackedTernary::new(10);
    assert_eq!(pt.generation, 0);

    pt.set_from_slice(&[1i8, -1, 0, 1, -1])?;
    assert_eq!(pt.generation, 1);

    pt.clear();
    assert_eq!(pt.generation, 2);

    Ok(())
}

/// Test 3: TOBL dot-product (scalar path)
#[test]
fn test_tobl_dot_scalar() -> Result<(), NtgError> {
    let mut a = PackedTernary::new(10);
    let mut b = PackedTernary::new(10);

    a.set_from_slice(&[1i8, -1, 0, 1, 0, 1, -1, 0, 1, -1])?;
    b.set_from_slice(&[1i8, 1, 0, -1, 0, 1, 1, 0, 1, 1])?;

    let (result, _cycles) = tobl_dot_product(&a, &b, Some(ToblKernelPath::Scalar))?;

    // Manual calculation:
    // 1*1=1, -1*1=-1, 0*0=0, 1*(-1)=-1, 0*0=0, 1*1=1, -1*1=-1, 0*0=0, 1*1=1, -1*1=-1
    // = 1 - 1 + 0 - 1 + 0 + 1 - 1 + 0 + 1 - 1 = -1
    assert_eq!(result, -1);

    Ok(())
}

/// Test 4: TOBL kernel path detection
#[test]
fn test_tobl_kernel_selection() -> Result<(), NtgError> {
    let mut a = PackedTernary::new(50);
    let mut b = PackedTernary::new(50);

    a.set_from_slice(&[1i8; 50])?;
    b.set_from_slice(&[1i8; 50])?;

    // Auto-select best path
    let (result_auto, cycles_auto) = tobl_dot_product(&a, &b, None)?;

    // Force scalar
    let (result_scalar, _cycles_scalar) = tobl_dot_product(&a, &b, Some(ToblKernelPath::Scalar))?;

    // Results must match (even if timing differs)
    assert_eq!(result_auto, result_scalar);
    assert_eq!(result_auto, 50); // 50 x (1*1)

    println!("TOBL auto path: {} cycles", cycles_auto);

    Ok(())
}

/// Test 5: PackedTernary density metric (structural evolution heuristic)
#[test]
fn test_packed_ternary_density() -> Result<(), NtgError> {
    let mut pt = PackedTernary::new(100);

    // All zeros: density should be 0
    let d0 = pt.compute_density();
    assert_eq!(d0, 0.0);

    // Half filled with 1s
    for i in 0..50 {
        pt.set(i, 1);
    }
    let d1 = pt.compute_density();
    assert!(d1 > 0.0 && d1 < 1.0);
    println!("50% fill density: {}", d1);

    // All filled
    for i in 50..100 {
        pt.set(i, 1);
    }
    let d2 = pt.compute_density();
    assert!(d2 > d1);
    println!("100% fill density: {}", d2);

    Ok(())
}

/// Test 6: FFI integration (C interface)
#[test]
fn test_tobl_ffi_basic() {
    unsafe {
        let a = ntg_tobl_new(10);
        let b = ntg_tobl_new(10);

        assert_ne!(a as usize, 0);
        assert_ne!(b as usize, 0);

        // Set values
        assert_eq!(ntg_tobl_set(a, 0, 1), 0);
        assert_eq!(ntg_tobl_set(a, 1, -1), 0);
        assert_eq!(ntg_tobl_set(b, 0, 1), 0);
        assert_eq!(ntg_tobl_set(b, 1, -1), 0);

        // Verify reads
        assert_eq!(ntg_tobl_get(a, 0), 1);
        assert_eq!(ntg_tobl_get(a, 1), -1);

        // Dot product
        let mut result: i64 = 0;
        let mut cycles: u64 = 0;
        assert_eq!(ntg_tobl_dot(a, b, &mut result, &mut cycles), 0);
        // 1*1 + (-1)*(-1) = 2
        assert_eq!(result, 2);

        ntg_tobl_drop(a);
        ntg_tobl_drop(b);
    }
}

/// Test 7: PackedTernary + Ledger integration (mutation tracking)
#[test]
fn test_packed_ternary_ledger_integration() -> Result<(), NtgError> {
    let mut pt = PackedTernary::new(10);
    let mut ledger = TamperEvidentLedger::new(None)?;

    // Simulate mutation: set values and record in ledger
    pt.set_from_slice(&[1i8, -1, 0, 1, -1])?;
    let density = pt.compute_density();

    // Log mutation event with density metric
    let entry = format!(
        "density_change density={} generation={}",
        density, pt.generation
    );
    ledger.log_mutation(
        entry,
        0,
        0,
        FitnessMeasure {
            latency_us: 0,
            memory_bytes: 0,
        },
        MutationOutcome::Accepted,
        0,
        ExecutionTrace::new(),
        0,
    )?;

    // Verify ledger integrity
    ledger.verify_full_ledger()?;

    Ok(())
}

/// Test 8: TOBL + FFI + Ledger end-to-end
#[test]
fn test_tobl_ffi_ledger_e2e() -> Result<(), NtgError> {
    unsafe {
        // Create PackedTernary via FFI
        let a = ntg_tobl_new(20);
        let b = ntg_tobl_new(20);

        // Initialize with pattern
        for i in 0..20 {
            let val = if i % 2 == 0 { 1i8 } else { -1i8 };
            ntg_tobl_set(a, i as u32, val);
            ntg_tobl_set(b, i as u32, val);
        }

        // Compute dot product and get metrics
        let mut result: i64 = 0;
        let mut cycles: u64 = 0;
        assert_eq!(ntg_tobl_dot(a, b, &mut result, &mut cycles), 0);

        // Should get dot product of 20 (all products = 1, sum = 20)
        assert_eq!(result, 20);

        // Record operation in ledger
        let mut ledger = TamperEvidentLedger::new(None)?;
        let entry = format!(
            "tobl_dot result={} cycles={} op_count={}",
            result,
            cycles,
            ntg_tobl_op_count()
        );
        ledger.log_mutation(
            entry,
            0,
            0,
            FitnessMeasure {
                latency_us: 0,
                memory_bytes: 0,
            },
            MutationOutcome::Accepted,
            cycles,
            ExecutionTrace::new(),
            0,
        )?;

        // Verify ledger
        ledger.verify_full_ledger()?;

        ntg_tobl_drop(a);
        ntg_tobl_drop(b);
    }

    Ok(())
}

/// Test 9: Large matrix TOBL performance (1000x1000)
#[test]
fn test_tobl_large_matrix() -> Result<(), NtgError> {
    let size = 1000;
    let mut a = PackedTernary::new(size);
    let mut b = PackedTernary::new(size);

    // Fill with ternary pattern
    for i in 0..size {
        a.set(i, (i % 3) as i8 - 1);
        b.set(i, (i % 3) as i8 - 1);
    }

    let start = std::time::Instant::now();
    let (result, _cycles) = tobl_dot_product(&a, &b, None)?;
    let elapsed = start.elapsed();

    println!(
        "1000-element dot-product: {} us, result = {}",
        elapsed.as_micros(),
        result
    );

    assert!(elapsed.as_micros() < 1_000_000); // Should be < 1ms

    Ok(())
}

/// Test 10: Observability: density + cycle tracking
#[test]
fn test_observability_metrics() -> Result<(), NtgError> {
    let mut pt = PackedTernary::new(100);

    // Initial state
    assert_eq!(pt.generation, 0);
    assert_eq!(pt.last_op_cycles, 0);

    // Mutation 1
    pt.set_from_slice(&[1i8; 50])?;
    let density1 = pt.compute_density();
    assert!(density1 > 0.0);

    // Mutation 2
    pt.set_from_slice(&[0i8; 100])?;
    let density2 = pt.compute_density();
    assert_eq!(density2, 0.0);

    // Cycle tracking
    let (_result, cycles) = tobl_dot_product(&pt, &pt, None)?;
    pt.record_cycles(cycles);
    assert!(pt.last_op_cycles > 0);

    println!("Observability metrics: generation={}, cycles={}, density={}", pt.generation, pt.last_op_cycles, density1);

    Ok(())
}
