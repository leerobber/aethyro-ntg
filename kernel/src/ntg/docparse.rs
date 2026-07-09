//! Minimal structural parser: Markdown-shaped documents -> typed `Graph`
//! nodes (ADR 0003).
//!
//! This implements the "docs parsed into the graph" half of ADR 0003
//! using literal document structure -- headings, list items, fenced
//! code blocks -- the same mechanism GraphMD/"Literate Execution" uses
//! (see docs/LITERATURE.md), not an invented punctuation-to-opcode
//! table. Byte-exact leaf content and any glyph-level feature extraction
//! are a deliberately separate, later concern -- this module only
//! builds structure, so it can be tested and trusted on its own.

use super::graph::{Graph, NodeId, NodeKind};

/// Parse `text` into `graph`, returning the id of the document's root
/// node. Headings nest by level; fenced code blocks become
/// `NodeKind::Execution` leaves; everything else (paragraphs, bullets,
/// numbered items) becomes `NodeKind::Content` children of the current
/// section.
pub fn parse_into(graph: &mut Graph, doc_label: &str, text: &str) -> NodeId {
    let root = graph.add_node(NodeKind::Content, doc_label.to_string());
    // Stack of (heading_level, node_id); level 0 is the document root.
    let mut section_stack: Vec<(usize, NodeId)> = vec![(0, root)];
    let mut in_fence = false;
    let mut fence_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            if in_fence {
                let parent = current_section(&section_stack);
                let content = fence_lines.join("\n");
                let node = graph.add_node(NodeKind::Execution, content);
                link(graph, parent, node);
                fence_lines.clear();
                in_fence = false;
            } else {
                in_fence = true;
            }
            continue;
        }

        if in_fence {
            fence_lines.push(line);
            continue;
        }

        if let Some(level) = heading_level(trimmed) {
            let label = trimmed.trim_start_matches('#').trim().to_string();
            while section_stack.len() > 1 && section_stack.last().unwrap().0 >= level {
                section_stack.pop();
            }
            let parent = current_section(&section_stack);
            let node = graph.add_node(NodeKind::Content, label);
            link(graph, parent, node);
            section_stack.push((level, node));
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        let parent = current_section(&section_stack);
        let label = if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            rest.to_string()
        } else if let Some(rest) = numbered_item(trimmed) {
            rest.to_string()
        } else {
            trimmed.to_string()
        };
        let node = graph.add_node(NodeKind::Content, label);
        link(graph, parent, node);
    }

    root
}

fn current_section(stack: &[(usize, NodeId)]) -> NodeId {
    stack.last().expect("section_stack always has the document root at index 0").1
}

fn link(graph: &mut Graph, parent: NodeId, child: NodeId) {
    graph
        .add_edge(parent, child)
        .expect("parent and child were just created in this graph, so both exist")
}

fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes > 0 && line.as_bytes().get(hashes) == Some(&b' ') {
        Some(hashes)
    } else {
        None
    }
}

fn numbered_item(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    line[digits..].strip_prefix(". ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_nest_by_level() {
        let mut g = Graph::new();
        let doc = "# Title\n## Section A\ntext under A\n## Section B\ntext under B\n";
        let root = parse_into(&mut g, "doc", doc);
        let title_children = g.children(root);
        assert_eq!(title_children.len(), 1);
        let sections = g.children(title_children[0]);
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn fenced_code_becomes_execution_node() {
        let mut g = Graph::new();
        let doc = "# Title\n```\nprintln!(\"hi\");\n```\n";
        let root = parse_into(&mut g, "doc", doc);
        let title = g.children(root)[0];
        let children = g.children(title);
        assert_eq!(children.len(), 1);
        assert_eq!(g.node(children[0]).unwrap().kind, NodeKind::Execution);
        assert_eq!(g.node(children[0]).unwrap().label, "println!(\"hi\");");
    }

    #[test]
    fn bullets_and_numbered_items_become_content_nodes() {
        let mut g = Graph::new();
        let doc = "# Title\n- first\n- second\n1. one\n2. two\n";
        let root = parse_into(&mut g, "doc", doc);
        let title = g.children(root)[0];
        let children = g.children(title);
        assert_eq!(children.len(), 4);
        for c in &children {
            assert_eq!(g.node(*c).unwrap().kind, NodeKind::Content);
        }
        let labels: Vec<&str> = children.iter().map(|&c| g.node(c).unwrap().label.as_str()).collect();
        assert_eq!(labels, vec!["first", "second", "one", "two"]);
    }

    #[test]
    fn empty_doc_has_only_root() {
        let mut g = Graph::new();
        let root = parse_into(&mut g, "doc", "");
        assert!(g.children(root).is_empty());
    }

    #[test]
    fn deeper_heading_returns_to_shallower_parent() {
        let mut g = Graph::new();
        let doc = "# A\n## B\n### C\n## D\n";
        let root = parse_into(&mut g, "doc", doc);
        let a = g.children(root)[0];
        // B and D are both direct children of A; C is nested under B only.
        let a_children = g.children(a);
        assert_eq!(a_children.len(), 2);
        let b = a_children[0];
        assert_eq!(g.children(b).len(), 1);
    }
}
