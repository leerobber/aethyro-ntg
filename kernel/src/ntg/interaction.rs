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
//!
//! **Tested against this repo's own real ADR/doc graph (2026-07-08),
//! honest negative result:** `edge_interaction_score`'s raw value
//! correlates 0.56-0.60 with `min(len_a, len_b)` across 486 real
//! parent-child pairs -- most of what looked like "real edges score
//! differently from random pairs" was that length confound, not
//! relatedness. `normalized_edge_interaction_score` (below) removes it;
//! after normalizing, real parent-child pairs and random node pairs
//! from the same corpus look statistically similar (means 0.155 vs.
//! 0.132, within one std of each other). **Conclusion: on real
//! documents, this score does not currently distinguish a heading from
//! its own content any better than it distinguishes two unrelated
//! nodes.** See docs/EXPERIMENTS.md for the full numbers. The
//! self-similarity/edit-sensitivity properties tested below remain true
//! and useful for what they are (near-duplicate/edit detection) -- this
//! just isn't evidence they generalize to "structural relatedness."

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

/// `edge_interaction_score` divided by the shorter label's length --
/// removes the length confound diagnosed above (the raw score is
/// dominated by how long the shorter, zero-padded string is, since
/// positions beyond it always contribute zero). Returns `0.0` for a
/// pair of empty labels rather than dividing by zero.
pub fn normalized_edge_interaction_score(
    graph: &Graph,
    parent: NodeId,
    child: NodeId,
) -> Result<f32, NtgError> {
    let raw = edge_interaction_score(graph, parent, child)?;
    let min_len = graph.node(parent)?.label.len().min(graph.node(child)?.label.len());
    if min_len == 0 {
        Ok(0.0)
    } else {
        Ok(raw / min_len as f32)
    }
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

    #[test]
    fn normalized_score_preserves_ordering_for_equal_length_strings() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "hello");
        let b = g.add_node(NodeKind::Content, "hello");
        let c = g.add_node(NodeKind::Content, "hxllo");
        let self_score = normalized_edge_interaction_score(&g, a, b).unwrap();
        let edited_score = normalized_edge_interaction_score(&g, a, c).unwrap();
        assert!(self_score > edited_score);
    }

    #[test]
    fn normalized_score_is_zero_for_empty_labels() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "");
        let b = g.add_node(NodeKind::Content, "");
        assert_eq!(normalized_edge_interaction_score(&g, a, b).unwrap(), 0.0);
    }

    #[test]
    fn normalized_score_removes_length_dependence_of_self_similarity() {
        // Self-similarity of a uniform-character string shouldn't
        // depend on its length once normalized -- the raw score would
        // scale with length; this one should not.
        let mut g = Graph::new();
        let short = g.add_node(NodeKind::Content, "aaa");
        let short2 = g.add_node(NodeKind::Content, "aaa");
        let long = g.add_node(NodeKind::Content, "aaaaaaaaaaaaaaaaaaaa");
        let long2 = g.add_node(NodeKind::Content, "aaaaaaaaaaaaaaaaaaaa");
        let short_norm = normalized_edge_interaction_score(&g, short, short2).unwrap();
        let long_norm = normalized_edge_interaction_score(&g, long, long2).unwrap();
        assert_eq!(short_norm, long_norm);
    }
}
