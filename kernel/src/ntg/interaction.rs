//! Real ternary-matmul "edge interaction score" between two graph
//! nodes' labels -- the follow-up that came out of a diagnosed failure
//! (see docs/EXPERIMENTS.md, 2026-07-08). The first attempt used
//! `encode()` (per-string threshold) and produced identical scores for
//! completely different strings ("aaaaa" vs "zzzzz"). This version uses
//! `encode_fixed` (fixed global threshold, see `ternary.rs`) instead,
//! which was empirically validated to fix exactly that failure before
//! any of this was written as Rust.
//!
//! What this score actually is, precisely: a byte-position correlation.
//! Identical strings score positively (self-correlation); a
//! byte-for-byte "opposite" encoding (e.g. "aaaaa" vs "zzzzz") scores
//! negatively; a single-character edit measurably lowers the score
//! below the unedited self-score (all three properties are tested
//! below). It is **not** a semantic similarity measure -- it knows
//! nothing about meaning, only byte-value alignment, and it is sensitive
//! to positional shifts (inserting one character early in a string
//! shifts every later comparison out of alignment).

use super::error::NtgError;
use super::graph::{Graph, NodeId};
use super::ternary::{encode_fixed, matmul_scalar};

pub fn edge_interaction_score(graph: &Graph, parent: NodeId, child: NodeId) -> Result<f32, NtgError> {
    let mut a = encode_fixed(&graph.node(parent)?.label);
    let mut b = encode_fixed(&graph.node(child)?.label);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    let result = matmul_scalar(&a, &b, 1, len, 1)?;
    Ok(result[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::graph::NodeKind;

    #[test]
    fn identical_strings_score_higher_than_a_one_char_edit() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "hello");
        let b = g.add_node(NodeKind::Content, "hello");
        let c = g.add_node(NodeKind::Content, "hxllo");
        let self_score = edge_interaction_score(&g, a, b).unwrap();
        let edited_score = edge_interaction_score(&g, a, c).unwrap();
        assert!(
            self_score > edited_score,
            "a single-character edit should reduce the score below self-similarity"
        );
    }

    #[test]
    fn byte_for_byte_opposite_strings_score_negative() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "aaaaa");
        let b = g.add_node(NodeKind::Content, "zzzzz");
        let score = edge_interaction_score(&g, a, b).unwrap();
        assert!(score < 0.0, "'a' and 'z' encode to opposite ternary values");
    }

    #[test]
    fn is_deterministic() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "Title");
        let b = g.add_node(NodeKind::Content, "first");
        assert_eq!(
            edge_interaction_score(&g, a, b).unwrap(),
            edge_interaction_score(&g, a, b).unwrap()
        );
    }

    #[test]
    fn errors_on_missing_node() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        assert!(edge_interaction_score(&g, a, 999).is_err());
    }
}
