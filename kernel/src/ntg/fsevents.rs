//! Pure filesystem-event -> graph-mutation translation (ADR 0003,
//! "paths as intelligence graphs" -- the mutation half).
//!
//! This module deliberately does **not** watch a real filesystem.
//! Doing that needs an OS-level notification mechanism (e.g. the
//! `notify` crate), which is a new external dependency -- a separate
//! decision from the pure, deterministic, fully-testable logic here:
//! given an event, what graph operation does it become. Wiring this to
//! real OS events is tracked in ROADMAP.md, not done in this module.

use super::graph::{Graph, NodeId};
use super::pathparse::{find_path, parse_path_into};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsEvent {
    Created(String),
    Removed(String),
    Renamed(String, String),
}

/// Apply a single filesystem event to `graph`, rooted at `root`.
/// Returns the affected node id for `Created`/`Renamed`; `None` for
/// `Removed` and for a `Removed`/rename-source that didn't exist (a
/// no-op, not an error -- a watcher replaying stale events shouldn't
/// crash the graph).
pub fn apply_event(graph: &mut Graph, root: NodeId, event: &FsEvent) -> Option<NodeId> {
    match event {
        FsEvent::Created(path) => Some(parse_path_into(graph, root, path)),
        FsEvent::Removed(path) => {
            if let Some(id) = find_path(graph, root, path) {
                graph.remove_node(id).ok();
            }
            None
        }
        FsEvent::Renamed(from, to) => {
            if let Some(old) = find_path(graph, root, from) {
                graph.remove_node(old).ok();
            }
            Some(parse_path_into(graph, root, to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::graph::NodeKind;

    #[test]
    fn created_event_adds_the_path() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let id = apply_event(&mut g, root, &FsEvent::Created("src/lib.rs".to_string())).unwrap();
        assert_eq!(g.node(id).unwrap().label, "lib.rs");
    }

    #[test]
    fn removed_event_drops_an_existing_path() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        apply_event(&mut g, root, &FsEvent::Created("src/lib.rs".to_string()));
        let result = apply_event(&mut g, root, &FsEvent::Removed("src/lib.rs".to_string()));
        assert!(result.is_none());
        let src = g.children(root)[0];
        assert!(g.children(src).is_empty());
    }

    #[test]
    fn removed_event_on_missing_path_is_a_no_op() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let before = g.node_count();
        apply_event(&mut g, root, &FsEvent::Removed("does/not/exist.rs".to_string()));
        assert_eq!(g.node_count(), before, "removing a nonexistent path must not create nodes");
    }

    #[test]
    fn renamed_event_moves_the_leaf() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        apply_event(&mut g, root, &FsEvent::Created("src/old.rs".to_string()));
        let new_id = apply_event(
            &mut g,
            root,
            &FsEvent::Renamed("src/old.rs".to_string(), "src/new.rs".to_string()),
        )
        .unwrap();
        assert_eq!(g.node(new_id).unwrap().label, "new.rs");
        let src = g.children(root)[0];
        let labels: Vec<String> =
            g.children(src).iter().map(|&c| g.node(c).unwrap().label.clone()).collect();
        assert_eq!(labels, vec!["new.rs".to_string()]);
    }
}
