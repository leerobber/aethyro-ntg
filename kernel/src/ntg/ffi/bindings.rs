//! C bindings and FFI safety documentation.
//!
//! This module documents the FFI safety contract. All FFI functions
//! are defined in mod.rs with #[no_mangle] pub extern "C".
//!
//! C header equivalent is in kernel/include/ntg.h.

// FFI Safety Contract
//
// All FFI functions assume caller is responsible for:
// 1. Pointer validity (not null, properly aligned)
// 2. Buffer sizing (a: m*k, b: k*n, out: m*n)
// 3. Dimension consistency
//
// Violations result in -1 (EINVAL) return code.
//
// Thread Safety: Each FFI call is independent and thread-safe.
// OpStats are thread-local and safe to pass between threads.
//
// Memory Management: Output buffer (out) is caller-allocated.
// No dynamic allocation happens inside FFI functions.

pub mod error_codes {
    /// Success
    pub const NTG_OK: i32 = 0;
    /// Invalid argument (null pointer, dimension mismatch)
    pub const NTG_EINVAL: i32 = -1;
    /// IO error (dispatcher initialization failed)
    pub const NTG_EIO: i32 = -2;
    /// Operation failed
    pub const NTG_ECALLFAILED: i32 = -3;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_distinct() {
        assert_ne!(error_codes::NTG_OK, error_codes::NTG_EINVAL);
        assert_ne!(error_codes::NTG_EINVAL, error_codes::NTG_EIO);
        assert_ne!(error_codes::NTG_EIO, error_codes::NTG_ECALLFAILED);
    }
}
