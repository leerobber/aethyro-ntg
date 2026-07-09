//! Self-referential test: `docparse` parses this repo's own ADRs and
//! design docs. This is the real, buildable version of "self-referential
//! kernel" -- not recursive self-awareness, just dogfooding: the
//! parser's own documentation becomes a test fixture for the parser,
//! and CI fails if parsing this repo's real docs ever panics or
//! produces something structurally wrong.
//!
//! Fenced-code-block counts below were checked directly against the
//! real files on 2026-07-08 (`grep -F -c '```' <file>`), not assumed --
//! see docs/ROADMAP.md's "self-parse test" item.

use ntg_kernel::{parse_into, Graph, NodeKind};

const ADR_0001: &str = include_str!("../../docs/architecture/0001-vision-and-pivot.md");
const ADR_0002: &str =
    include_str!("../../docs/architecture/0002-safety-rails-for-self-modification.md");
const ADR_0003: &str = include_str!("../../docs/architecture/0003-sis-frontend.md");
const DESIGN: &str = include_str!("../../docs/DESIGN.md");
const ROADMAP: &str = include_str!("../../docs/ROADMAP.md");

fn execution_node_count(graph: &Graph, root: usize) -> usize {
    fn walk(graph: &Graph, id: usize, count: &mut usize) {
        if graph.node(id).map(|n| n.kind == NodeKind::Execution).unwrap_or(false) {
            *count += 1;
        }
        for child in graph.children(id) {
            walk(graph, child, count);
        }
    }
    let mut count = 0;
    walk(graph, root, &mut count);
    count
}

#[test]
fn parses_every_real_doc_without_panicking() {
    for (label, text) in [
        ("adr-0001", ADR_0001),
        ("adr-0002", ADR_0002),
        ("adr-0003", ADR_0003),
        ("design", DESIGN),
        ("roadmap", ROADMAP),
    ] {
        let mut g = Graph::new();
        let root = parse_into(&mut g, label, text);
        assert!(g.node(root).is_ok(), "{label}: root node should exist");
        assert!(g.node_count() > 1, "{label}: should produce more than just the root");
    }
}

#[test]
fn adrs_without_fenced_code_produce_zero_execution_nodes() {
    // ADRs 0001-0003 are prose + links, no fenced code blocks in the
    // real files (grep -F -c '```' returns 0 for all three).
    for (label, text) in [("adr-0001", ADR_0001), ("adr-0002", ADR_0002), ("adr-0003", ADR_0003)] {
        let mut g = Graph::new();
        let root = parse_into(&mut g, label, text);
        assert_eq!(
            execution_node_count(&g, root),
            0,
            "{label}: real file has no fenced code blocks, so no Execution nodes expected"
        );
    }
}

#[test]
fn design_doc_produces_at_least_one_execution_node() {
    // DESIGN.md has exactly one fenced block (the ASCII architecture
    // diagram) in the real file (grep -F -c '```' returns 2, i.e. one
    // open/close pair).
    let mut g = Graph::new();
    let root = parse_into(&mut g, "design", DESIGN);
    assert!(
        execution_node_count(&g, root) >= 1,
        "DESIGN.md's fenced diagram block should parse as an Execution node"
    );
}
