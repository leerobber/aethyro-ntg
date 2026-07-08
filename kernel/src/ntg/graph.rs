//! Deterministic graph structure over typed nodes.
//!
//! Node identity is a plain `usize` index into a `Vec`, not a pointer --
//! this keeps the whole structure trivially serializable, which matters
//! once this feeds an audit ledger (ADR 0002) and a self-modification
//! engine (Phase 3) that must log every structural change. Edge order is
//! insertion order, deliberately, so `children()` is deterministic --
//! ADR 0002's replay guarantee starts at this layer.

use super::error::NtgError;

pub type NodeId = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Plain content -- data, not something the engine runs.
    Content,
    /// Executable, per ADR 0003. This type alone doesn't execute
    /// anything yet -- it marks a node as the kind that Phase 3's
    /// self-modification engine and its ledger will treat specially.
    Execution,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    nodes: Vec<Option<Node>>,
    edges: Vec<(NodeId, NodeId)>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, kind: NodeKind, label: impl Into<String>) -> NodeId {
        self.nodes.push(Some(Node { kind, label: label.into() }));
        self.nodes.len() - 1
    }

    pub fn remove_node(&mut self, id: NodeId) -> Result<(), NtgError> {
        let len = self.nodes.len();
        let slot = self
            .nodes
            .get_mut(id)
            .ok_or_else(|| NtgError::IndexOutOfBounds { index: id, len })?;
        if slot.is_none() {
            return Err(NtgError::IndexOutOfBounds { index: id, len });
        }
        *slot = None;
        self.edges.retain(|&(a, b)| a != id && b != id);
        Ok(())
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), NtgError> {
        self.node(from)?;
        self.node(to)?;
        self.edges.push((from, to));
        Ok(())
    }

    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), NtgError> {
        let before = self.edges.len();
        self.edges.retain(|&(a, b)| !(a == from && b == to));
        if self.edges.len() == before {
            return Err(NtgError::EdgeNotFound { from, to });
        }
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Result<&Node, NtgError> {
        let len = self.nodes.len();
        self.nodes
            .get(id)
            .and_then(|n| n.as_ref())
            .ok_or_else(|| NtgError::IndexOutOfBounds { index: id, len })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Children of `id`, in edge-insertion order -- deterministic by
    /// construction.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.edges.iter().filter(|&&(a, _)| a == id).map(|&(_, b)| b).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_fetch_node() {
        let mut g = Graph::new();
        let id = g.add_node(NodeKind::Content, "root");
        assert_eq!(g.node(id).unwrap().label, "root");
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn edges_require_existing_nodes() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        assert!(g.add_edge(a, 99).is_err());
    }

    #[test]
    fn children_are_deterministic_order() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let c1 = g.add_node(NodeKind::Content, "c1");
        let c2 = g.add_node(NodeKind::Content, "c2");
        g.add_edge(root, c1).unwrap();
        g.add_edge(root, c2).unwrap();
        assert_eq!(g.children(root), vec![c1, c2]);
    }

    #[test]
    fn remove_node_drops_its_edges() {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "root");
        let c1 = g.add_node(NodeKind::Content, "c1");
        g.add_edge(root, c1).unwrap();
        g.remove_node(c1).unwrap();
        assert_eq!(g.children(root), Vec::<NodeId>::new());
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn remove_missing_node_errors() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        g.remove_node(a).unwrap();
        assert!(g.remove_node(a).is_err());
    }

    #[test]
    fn remove_missing_edge_errors() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        let b = g.add_node(NodeKind::Content, "b");
        assert!(g.remove_edge(a, b).is_err());
    }
}
