//! Filesystem path structural parsing (ADR 0003) -- the "paths as
//! intelligence graphs" half, as a real, testable transform: a path
//! string becomes typed graph nodes, one per directory segment, with
//! the final segment typed by its extension.
//!
//! This module does not watch a real filesystem -- see `fsevents.rs`
//! for the pure event-application layer built on top of it. Wiring
//! either of these to actual OS filesystem notifications needs a new
//! external crate (e.g. `notify`), which is a separate dependency
//! decision tracked in ROADMAP.md, not made here.

use super::graph::{Graph, NodeId, NodeKind};

/// Extensions treated as "executable" for graph-typing purposes --
/// source code the self-modification engine (Phase 3) would eventually
/// care about, as opposed to plain data/docs.
const EXECUTABLE_EXTENSIONS: &[&str] = &["rs", "py", "sh", "js", "ts"];

fn kind_for_extension(ext: Option<&str>) -> NodeKind {
    match ext {
        Some(e) if EXECUTABLE_EXTENSIONS.contains(&e) => NodeKind::Execution,
        _ => NodeKind::Content,
    }
}

/// Parse a `/`-or-`\`-separated path into `graph` under `parent`,
/// returning the id of the final (deepest) node. Reuses whatever
/// segment nodes already exist with a matching label under their
/// parent, so parsing two paths that share a directory prefix doesn't
/// create duplicate directory nodes.
pub fn parse_path_into(graph: &mut Graph, parent: NodeId, path: &str) -> NodeId {
    let mut current = parent;
    let segments: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        if let Some(existing) = find_child_by_label(graph, current, segment) {
            current = existing;
            continue;
        }
        let kind = if is_last {
            kind_for_extension(segment.rsplit_once('.').map(|(_, e)| e))
        } else {
            NodeKind::Content
        };
        let node = graph.add_node(kind, segment.to_string());
        graph
            .add_edge(current, node)
            .expect("current and node were just verified to exist in this graph");
        current = node;
    }

    current
}

/// Look up an existing path without creating anything. Returns `None`
/// as soon as any segment along the way doesn't already exist.
pub fn find_path(graph: &Graph, parent: NodeId, path: &str) -> Option<NodeId> {
    let mut current = parent;
    for segment in path.split(['/', '\\']).filter(|s| !s.is_empty()) {
        current = find_child_by_label(graph, current, segment)?;
    }
    Some(current)
}

fn find_child_by_label(graph: &Graph, parent: NodeId, label: &str) -> Option<NodeId> {
    graph
        .children(parent)
        .into_iter()
        .find(|&child| graph.node(child).map(|n| n.label == label).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nests_directories_and_types_the_leaf_by_extension() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let leaf = parse_path_into(&mut g, root, "kernel/src/ntg/graph.rs");
        assert_eq!(g.node(leaf).unwrap().kind, NodeKind::Execution);
        assert_eq!(g.node(leaf).unwrap().label, "graph.rs");
    }

    #[test]
    fn shared_prefix_reuses_directory_nodes() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        parse_path_into(&mut g, root, "kernel/src/ntg/graph.rs");
        parse_path_into(&mut g, root, "kernel/src/ntg/packed.rs");

        let kernel = g.children(root);
        assert_eq!(kernel.len(), 1, "kernel/ should only be created once");
        let src = g.children(kernel[0]);
        assert_eq!(src.len(), 1, "src/ should only be created once");
        let ntg = g.children(src[0]);
        assert_eq!(ntg.len(), 1, "ntg/ should only be created once");
        assert_eq!(g.children(ntg[0]).len(), 2, "graph.rs and packed.rs both under ntg/");
    }

    #[test]
    fn non_executable_extension_is_content() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let leaf = parse_path_into(&mut g, root, "docs/DESIGN.md");
        assert_eq!(g.node(leaf).unwrap().kind, NodeKind::Content);
    }

    #[test]
    fn extensionless_file_is_content() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let leaf = parse_path_into(&mut g, root, "LICENSE");
        assert_eq!(g.node(leaf).unwrap().kind, NodeKind::Content);
    }

    #[test]
    fn find_path_does_not_create_anything() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        assert!(find_path(&g, root, "does/not/exist.rs").is_none());
        assert_eq!(g.node_count(), 1, "a failed lookup must not create nodes");
    }

    #[test]
    fn find_path_locates_an_existing_leaf() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let created = parse_path_into(&mut g, root, "kernel/src/lib.rs");
        let found = find_path(&g, root, "kernel/src/lib.rs");
        assert_eq!(found, Some(created));
    }
}
