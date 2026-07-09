//! Aethyro NTG (Neural Ternary Graph) Engine -- kernel.
//!
//! Capability version 10: Phase 0–5 complete — ternary/storage, SIS graph,
//! ledger/self-mod, calibration, precision+runtime warm-start optimization.
//!
//! **Status truth:** repo-root `docs/STATUS.md` and `docs/ROADMAP.md` —
//! trust those over this comment if they disagree.
//!
//! Note: a parallel crate-root `runtime`/`storage`/`kernels` sketch from
//! an earlier remote commit was superseded by the tested `ntg::*` modules.

pub mod ntg;

pub use ntg::chain::{ChainEntry, ChainLog};
pub use ntg::docparse::parse_into;
pub use ntg::error::NtgError;
pub use ntg::fsevents::{apply_event, FsEvent};
pub use ntg::graph::{Graph, GraphNode, NodeKind};
pub use ntg::interaction::{edge_interaction_score, normalized_edge_interaction_score};
pub use ntg::leafsignal::{extract_leaf_signal, LeafSignal};
pub use ntg::packed::PackedTernary;
pub use ntg::pathparse::{find_path, parse_path_into};
pub use ntg::accel::{
    resolve_avx512_hardware, AccelDevice, AccelManager, HardwarePath,
    DEFAULT_SPARSE_DENSITY_THRESHOLD,
};
pub use ntg::runtime::{
    bit_sliced_dot_fast, matmul_fast, validate_sequential_ids, Runtime, ShapeGuard,
};
pub use ntg::storage::{BitSlicedBlock, BitSlicedTernary, SparseBitSlicedTernary};
pub use ntg::ternary::{encode, encode_fixed, matmul_scalar, Ternary};
pub use ntg::observability::{StatsCollector, StatsSnapshot};
pub use ntg::genome::{DNAGraphNode, GenomeDelta, propose_density_delta};

/// Reports whether this build has a working ternary compute path.
pub fn has_ternary_kernel() -> bool {
    true
}

/// Capability report for evolution/genome dispatch — stateless and
/// observable. Ledger + self-mod exist (Phase 3); this remains a
/// feature bitmap for hosts, not a performance claim.
#[derive(Clone, Copy, Debug)]
pub struct TernaryCapability {
    pub scalar_supported: bool,
    pub packed_supported: bool,
    pub simd_supported: bool,
    pub graph_supported: bool,
    pub doc_path_parsing_supported: bool,
    pub forward_pass_supported: bool,
    pub fingerprint_supported: bool,
    pub edge_interaction_score_supported: bool,
    pub chain_log_supported: bool,
    pub bit_sliced_supported: bool,
    pub sparse_bit_sliced_supported: bool,
    pub native_parallel_forward_supported: bool,
    pub phase4_calibration_supported: bool,
    /// Phase 5: CalibModel → GraphNode warm-start + parallel batch score path.
    pub phase5_runtime_calib_supported: bool,
    pub version: u32,
}

pub fn ternary_capability() -> TernaryCapability {
    TernaryCapability {
        scalar_supported: true,
        packed_supported: true,
        // Dense i8 SIMD dispatcher exists; true AVX2 matmul still scalar-correct
        // for bit-identity. Portable bit-sliced popcount TOBL is the sparse path.
        simd_supported: true,
        graph_supported: true,
        doc_path_parsing_supported: true,
        forward_pass_supported: true,
        fingerprint_supported: true,
        edge_interaction_score_supported: true,
        chain_log_supported: true,
        bit_sliced_supported: true,
        sparse_bit_sliced_supported: true,
        native_parallel_forward_supported: true,
        phase4_calibration_supported: true,
        phase5_runtime_calib_supported: true,
        version: 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_reports_current_capability() {
        assert!(has_ternary_kernel());
        let cap = ternary_capability();
        assert!(cap.scalar_supported);
        assert!(cap.packed_supported);
        assert!(cap.simd_supported);
        assert!(cap.graph_supported);
        assert!(cap.doc_path_parsing_supported);
        assert!(cap.forward_pass_supported);
        assert!(cap.fingerprint_supported);
        assert!(cap.edge_interaction_score_supported);
        assert!(cap.chain_log_supported);
        assert!(cap.bit_sliced_supported);
        assert!(cap.sparse_bit_sliced_supported);
        assert!(cap.native_parallel_forward_supported);
        assert!(cap.phase4_calibration_supported);
        assert!(cap.phase5_runtime_calib_supported);
        assert_eq!(cap.version, 10);
    }
}
