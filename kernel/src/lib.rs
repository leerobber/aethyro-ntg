//! Aethyro NTG (Neural Ternary Graph) Engine -- kernel.
//!
//! Phase 1.1: ternary scalar reference only. See docs/DESIGN.md and
//! docs/architecture/ for the full architecture this grows into: graph
//! topology (Phase 2), bounded self-modification (Phase 3, gated by
//! ADR 0002), SIMD/FFI (later 1.x phases).

pub mod ntg;

pub use ntg::error::NtgError;
pub use ntg::ternary::{encode, matmul_scalar, Ternary};

/// Reports whether this build has a working ternary compute path.
pub fn has_ternary_kernel() -> bool {
    true
}

/// Capability report for future evolution/genome dispatch -- stateless
/// and observable, meant to feed a ledger (ADR 0002) once one exists.
#[derive(Clone, Copy, Debug)]
pub struct TernaryCapability {
    pub scalar_supported: bool,
    pub simd_supported: bool,
    pub version: u32,
}

pub fn ternary_capability() -> TernaryCapability {
    TernaryCapability {
        scalar_supported: true,
        simd_supported: false,
        version: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_reports_scalar_capability() {
        assert!(has_ternary_kernel());
        let cap = ternary_capability();
        assert!(cap.scalar_supported);
        assert!(!cap.simd_supported);
        assert_eq!(cap.version, 1);
    }
}
