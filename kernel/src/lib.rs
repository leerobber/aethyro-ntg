//! Aethyro NTG (Neural Ternary Graph) Engine -- kernel.
//!
//! Phase 1.1 (ternary scalar reference) and Phase 1.2 (bit-packed
//! storage) are implemented. Phase 2 (graph structure + ADR 0003's
//! docs/paths/glyph-signal front-end) is in progress: document
//! structure parsing, filesystem path parsing, pure fs-event mutation,
//! and a leaf case/punctuation signal extractor are implemented. See
//! docs/DESIGN.md and docs/architecture/ for the full architecture, and
//! docs/ROADMAP.md for current phase status and what's explicitly still
//! not done -- don't assume this comment is up to date, check ROADMAP.md.

pub mod ntg;

pub use ntg::chain::{ChainEntry, ChainLog};
pub use ntg::docparse::parse_into;
pub use ntg::error::NtgError;
pub use ntg::fsevents::{apply_event, FsEvent};
pub use ntg::graph::{Graph, NodeKind};
pub use ntg::interaction::{edge_interaction_score, normalized_edge_interaction_score};
pub use ntg::leafsignal::{extract_leaf_signal, LeafSignal};
pub use ntg::packed::PackedTernary;
pub use ntg::pathparse::{find_path, parse_path_into};
pub use ntg::ternary::{encode, encode_fixed, matmul_scalar, Ternary};

/// Reports whether this build has a working ternary compute path.
pub fn has_ternary_kernel() -> bool {
    true
}

/// Capability report for future evolution/genome dispatch -- stateless
/// and observable, meant to feed a ledger (ADR 0002) once one exists.
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
    pub version: u32,
}

pub fn ternary_capability() -> TernaryCapability {
    TernaryCapability {
        scalar_supported: true,
        packed_supported: true,
        simd_supported: false,
        graph_supported: true,
        doc_path_parsing_supported: true,
        forward_pass_supported: true,
        fingerprint_supported: true,
        edge_interaction_score_supported: true,
        chain_log_supported: true,
        version: 7,
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
        assert!(!cap.simd_supported);
        assert!(cap.graph_supported);
        assert!(cap.doc_path_parsing_supported);
        assert!(cap.forward_pass_supported);
        assert!(cap.fingerprint_supported);
        assert!(cap.edge_interaction_score_supported);
        assert!(cap.chain_log_supported);
        assert_eq!(cap.version, 7);
    }
}
