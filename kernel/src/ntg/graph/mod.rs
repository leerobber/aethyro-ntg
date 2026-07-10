//! Deterministic graph structure over typed nodes.
//!
//! Node identity is a plain `usize` index into a `Vec`, not a pointer --
//! this keeps the whole structure trivially serializable, which matters
//! once this feeds an audit ledger (ADR 0002) and a self-modification
//! engine (Phase 3) that must log every structural change. Edge order is
//! insertion order, deliberately, so `children()` is deterministic --
//! ADR 0002's replay guarantee starts at this layer.
//!
//! Weighted native-runtime nodes live in [`node::GraphNode`] (sparse
//! ternary weights), separate from the structural [`Node`] below.

pub mod node;

pub use node::GraphNode;

use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::BinaryHeap;
use std::hash::{Hash, Hasher};

use super::error::NtgError;
use super::glyph::{extract_glyph_fingerprint, GlyphFingerprint};
use super::lazyleaf::LazyLeaf;
use super::leafsignal::{extract_leaf_signal, LeafSignal};
use super::ledger::{
    replay::ExecutionTrace, FitnessMeasure, MutationOutcome, TamperEvidentLedger,
};

pub type NodeId = usize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// Real per-character case/punctuation/whitespace signal.
    /// Starts from `label`; updates if lazy body is resolved.
    pub signal: LeafSignal,
    /// Glyph fingerprint v0 from identity (or resolved body) — ADR 0003.
    /// Not trained PIXEL; see `glyph.rs`.
    pub glyph: GlyphFingerprint,
    /// Lazy full-body payload (ADR 0003 content-lazy). `None` until attached.
    pub lazy: Option<LazyLeaf>,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    nodes: Vec<Option<Node>>,
    /// Flat edge list (serialization / audit friendly).
    edges: Vec<(NodeId, NodeId)>,
    /// Adjacency lists parallel to `nodes`: children of `i` in insertion order.
    /// Keeps `children()` O(degree) instead of O(|E|).
    adj_list: Vec<Vec<NodeId>>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, kind: NodeKind, label: impl Into<String>) -> NodeId {
        let label = label.into();
        let signal = extract_leaf_signal(&label);
        let glyph = extract_glyph_fingerprint(&label);
        self.nodes.push(Some(Node {
            kind,
            label,
            signal,
            glyph,
            lazy: None,
        }));
        self.adj_list.push(Vec::new());
        self.nodes.len() - 1
    }

    /// Attach a lazy leaf (identity already on node label; body deferred).
    pub fn attach_lazy_leaf(&mut self, id: NodeId, leaf: LazyLeaf) -> Result<(), NtgError> {
        let node = self.node_mut(id)?;
        node.lazy = Some(leaf);
        Ok(())
    }

    /// Resolve full body bytes for a leaf (ADR 0003 lazy resolution).
    pub fn resolve_leaf_body(
        &mut self,
        id: NodeId,
        body: impl Into<String>,
    ) -> Result<(), NtgError> {
        let node = self.node_mut(id)?;
        let body = body.into();
        if let Some(ref mut lazy) = node.lazy {
            lazy.resolve_body(body.clone());
            node.signal = lazy.signal;
            node.glyph = lazy.glyph;
        } else {
            // Implicit lazy from identity + body
            let mut lazy = LazyLeaf::from_identity(node.label.clone());
            lazy.resolve_body(body);
            node.signal = lazy.signal;
            node.glyph = lazy.glyph;
            node.lazy = Some(lazy);
        }
        Ok(())
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, NtgError> {
        let len = self.nodes.len();
        self.nodes
            .get_mut(id)
            .and_then(|n| n.as_mut())
            .ok_or(NtgError::IndexOutOfBounds { index: id, len })
    }

    /// Run all `Execution` nodes in topo order and ledger-log each run
    /// (ADR 0002 rail 5 for execution-typed nodes). Does not execute
    /// external code — logs the effective leaf text length and fingerprint
    /// as a deterministic audit event. Self-mod remains separate.
    pub fn log_execution_nodes(
        &self,
        ledger: &mut TamperEvidentLedger,
        timestamp: u64,
    ) -> Result<usize, NtgError> {
        let order = self.topological_order()?;
        let mut count = 0usize;
        for id in order {
            let node = self.node(id)?;
            if node.kind != NodeKind::Execution {
                continue;
            }
            let text = node
                .lazy
                .as_ref()
                .map(|l| l.effective_text())
                .unwrap_or(node.label.as_str());
            let mut trace = ExecutionTrace::with_fingerprint(node.glyph.shape_hash);
            trace.record_event(
                id as u32,
                node.glyph.shape_hash,
                text.len() as u64,
                timestamp.saturating_add(count as u64),
            );
            ledger.log_mutation(
                format!(
                    "execute_node id={} label_len={} body_len={} glyph={:016x}",
                    id,
                    node.label.len(),
                    text.len(),
                    node.glyph.shape_hash
                ),
                node.glyph.shape_hash,
                node.glyph.shape_hash,
                FitnessMeasure {
                    latency_us: 0,
                    memory_bytes: text.len() as u64,
                },
                MutationOutcome::Accepted,
                0,
                trace,
                timestamp.saturating_add(count as u64),
            )?;
            count += 1;
        }
        Ok(count)
    }

    pub fn remove_node(&mut self, id: NodeId) -> Result<(), NtgError> {
        let len = self.nodes.len();
        let slot = self
            .nodes
            .get_mut(id)
            .ok_or(NtgError::IndexOutOfBounds { index: id, len })?;
        if slot.is_none() {
            return Err(NtgError::IndexOutOfBounds { index: id, len });
        }
        *slot = None;
        self.edges.retain(|&(a, b)| a != id && b != id);
        // Keep adj_list index alignment with node ids (tombstone slot).
        if let Some(children) = self.adj_list.get_mut(id) {
            children.clear();
        }
        for list in &mut self.adj_list {
            list.retain(|&b| b != id);
        }
        Ok(())
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), NtgError> {
        self.node(from)?;
        self.node(to)?;
        self.edges.push((from, to));
        self.adj_list[from].push(to);
        Ok(())
    }

    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), NtgError> {
        self.node(from)?;
        self.node(to)?;
        let before = self.edges.len();
        self.edges.retain(|&(a, b)| !(a == from && b == to));
        if self.edges.len() == before {
            return Err(NtgError::EdgeNotFound { from, to });
        }
        if let Some(children) = self.adj_list.get_mut(from) {
            if let Some(pos) = children.iter().position(|&b| b == to) {
                children.remove(pos);
            }
        }
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Result<&Node, NtgError> {
        let len = self.nodes.len();
        self.nodes
            .get(id)
            .and_then(|n| n.as_ref())
            .ok_or(NtgError::IndexOutOfBounds { index: id, len })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Children of `id`, in edge-insertion order -- deterministic by
    /// construction. O(degree) via adjacency list.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.adj_list.get(id).cloned().unwrap_or_default()
    }

    /// Ids of every currently-existing node, ascending.
    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().enumerate().filter_map(|(i, n)| n.as_ref().map(|_| i)).collect()
    }

    /// Dataflow-ordered execution order via Kahn's algorithm: a node is
    /// only ready once every node with an edge *into* it has already
    /// been ordered. This is the real mechanism behind "time-irrelevant
    /// execution" -- order is determined by dependency readiness, not a
    /// fixed/insertion-order walk -- and, unlike a naive recursive
    /// descent that assumes a tree, it correctly handles a node with
    /// multiple parents and detects cycles instead of looping forever.
    /// Ties among simultaneously-ready nodes are broken by ascending
    /// `NodeId`, so the result is fully deterministic (ADR 0002 replay).
    pub fn topological_order(&self) -> Result<Vec<NodeId>, NtgError> {
        let existing = self.all_node_ids();
        let mut in_degree = vec![0usize; self.nodes.len()];
        for &(_, to) in &self.edges {
            in_degree[to] += 1;
        }

        // Min-heap on NodeId: among simultaneously-ready nodes this always
        // pops the smallest first, in O(log n) — same tie-break as the
        // previous sorted-Vec implementation, without its O(n) per-step
        // `remove(0)` / sorted `insert` (which made this O(n^2) overall on
        // a path that runs on every forward_pass/fingerprint call).
        let mut ready: BinaryHeap<Reverse<NodeId>> = existing
            .iter()
            .copied()
            .filter(|&id| in_degree[id] == 0)
            .map(Reverse)
            .collect();
        let mut order = Vec::with_capacity(existing.len());

        while let Some(Reverse(id)) = ready.pop() {
            order.push(id);
            for child in self.children(id) {
                in_degree[child] -= 1;
                if in_degree[child] == 0 {
                    ready.push(Reverse(child));
                }
            }
        }

        if order.len() != existing.len() {
            return Err(NtgError::CycleDetected);
        }
        Ok(order)
    }

    /// A minimal, real, deterministic forward pass: visits every node in
    /// dataflow order and aggregates its `LeafSignal` into a running
    /// total. This is not full ternary-tensor compute over the graph --
    /// attaching real ops per node is a separate, larger feature -- it
    /// is the real scheduling mechanism that compute step will run on.
    pub fn forward_pass(&self) -> Result<LeafSignal, NtgError> {
        let order = self.topological_order()?;
        let mut total = LeafSignal::default();
        for id in order {
            total = total.combine(&self.node(id)?.signal);
        }
        Ok(total)
    }

    /// A deterministic content fingerprint: hashes every node's
    /// `(kind, label, signal, child_count)` in dataflow order. Two
    /// graphs built from identical content produce the same
    /// fingerprint; changing any label, kind, or structural shape
    /// changes it. Intended use: Phase 3's ledger can skip logging a
    /// "change" event when the fingerprint didn't actually move.
    ///
    /// **This is not a cryptographic hash.** It uses `std`'s
    /// `DefaultHasher` (SipHash with fixed keys), which is reproducible
    /// run-to-run on a given Rust version but is neither collision-
    /// resistant against an adversary nor guaranteed stable across
    /// standard-library versions forever. It is a change-detection
    /// tool, not the tamper-evidence primitive the real audit ledger
    /// needs -- that still requires a real cryptographic hash
    /// (SHA-256/BLAKE3), a dependency decision not made here.
    pub fn fingerprint(&self) -> Result<u64, NtgError> {
        let order = self.topological_order()?;
        let mut hasher = DefaultHasher::new();
        for id in order {
            let node = self.node(id)?;
            node.kind.hash(&mut hasher);
            node.label.hash(&mut hasher);
            node.signal.hash(&mut hasher);
            self.children(id).len().hash(&mut hasher);
        }
        Ok(hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntg::docparse;

    #[test]
    fn add_and_fetch_node() {
        let mut g = Graph::new();
        let id = g.add_node(NodeKind::Content, "root");
        assert_eq!(g.node(id).unwrap().label, "root");
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn add_node_attaches_real_leaf_signal() {
        let mut g = Graph::new();
        let id = g.add_node(NodeKind::Content, "Hi, World!");
        assert_eq!(g.node(id).unwrap().signal, extract_leaf_signal("Hi, World!"));
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
    fn adj_list_matches_edge_list_after_mutations() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        let b = g.add_node(NodeKind::Content, "b");
        let c = g.add_node(NodeKind::Content, "c");
        g.add_edge(a, b).unwrap();
        g.add_edge(a, c).unwrap();
        g.add_edge(b, c).unwrap();
        assert_eq!(g.children(a), vec![b, c]);
        g.remove_edge(a, b).unwrap();
        assert_eq!(g.children(a), vec![c]);
        g.remove_node(c).unwrap();
        assert_eq!(g.children(a), Vec::<NodeId>::new());
        assert_eq!(g.children(b), Vec::<NodeId>::new());
    }

    #[test]
    fn lazy_body_resolve_updates_glyph() -> Result<(), NtgError> {
        let mut g = Graph::new();
        let id = g.add_node(NodeKind::Content, "leaf");
        let g0 = g.node(id)?.glyph;
        g.resolve_leaf_body(id, "FULL BODY CONTENT HERE")?;
        let n = g.node(id)?;
        assert!(n.lazy.as_ref().unwrap().is_resolved());
        assert_ne!(n.glyph.codepoint_count, g0.codepoint_count);
        Ok(())
    }

    #[test]
    fn log_execution_nodes_writes_ledger() -> Result<(), NtgError> {
        let mut g = Graph::new();
        let root = g.add_node(NodeKind::Content, "doc");
        let ex = g.add_node(NodeKind::Execution, "```rust\nfn main(){}\n```");
        g.add_edge(root, ex)?;
        let mut ledger = TamperEvidentLedger::new(None)?;
        let n = g.log_execution_nodes(&mut ledger, 1000)?;
        assert_eq!(n, 1);
        ledger.verify_full_ledger()?;
        Ok(())
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

    #[test]
    fn topological_order_respects_edges_not_creation_order() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a"); // id 0, created first
        let b = g.add_node(NodeKind::Content, "b"); // id 1, created second
        // b has an edge into a, so b must be scheduled first even
        // though a has the smaller id and was created earlier.
        g.add_edge(b, a).unwrap();
        let order = g.topological_order().unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        assert!(pos_b < pos_a);
    }

    #[test]
    fn topological_order_detects_cycles() {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Content, "a");
        let b = g.add_node(NodeKind::Content, "b");
        g.add_edge(a, b).unwrap();
        g.add_edge(b, a).unwrap();
        assert!(g.topological_order().is_err());
    }

    #[test]
    fn forward_pass_visits_every_node_exactly_once() {
        let mut g = Graph::new();
        docparse::parse_into(&mut g, "doc", "# A\n- hi\n- Bye!\n");
        let total = g.forward_pass().unwrap();

        let mut expected = LeafSignal::default();
        for id in g.all_node_ids() {
            expected = expected.combine(&g.node(id).unwrap().signal);
        }
        assert_eq!(total, expected);
    }

    #[test]
    fn forward_pass_is_deterministic_across_repeated_runs() {
        let mut g = Graph::new();
        docparse::parse_into(&mut g, "doc", "# A\n- hi\n- Bye!\n");
        assert_eq!(g.forward_pass().unwrap(), g.forward_pass().unwrap());
    }

    #[test]
    fn fingerprint_is_deterministic_for_identical_content() {
        let mut g1 = Graph::new();
        let mut g2 = Graph::new();
        docparse::parse_into(&mut g1, "doc", "# A\n- hi\n- Bye!\n");
        docparse::parse_into(&mut g2, "doc", "# A\n- hi\n- Bye!\n");
        assert_eq!(g1.fingerprint().unwrap(), g2.fingerprint().unwrap());
    }

    #[test]
    fn fingerprint_is_stable_across_repeated_calls() {
        let mut g = Graph::new();
        docparse::parse_into(&mut g, "doc", "# A\n- hi\n");
        assert_eq!(g.fingerprint().unwrap(), g.fingerprint().unwrap());
    }

    #[test]
    fn fingerprint_changes_when_a_label_changes() {
        let mut g1 = Graph::new();
        let mut g2 = Graph::new();
        docparse::parse_into(&mut g1, "doc", "# A\n- hi\n");
        docparse::parse_into(&mut g2, "doc", "# A\n- bye\n");
        assert_ne!(g1.fingerprint().unwrap(), g2.fingerprint().unwrap());
    }

    #[test]
    fn fingerprint_changes_when_structure_changes() {
        let mut g1 = Graph::new();
        let mut g2 = Graph::new();
        docparse::parse_into(&mut g1, "doc", "# A\n- hi\n");
        docparse::parse_into(&mut g2, "doc", "# A\n- hi\n- extra\n");
        assert_ne!(g1.fingerprint().unwrap(), g2.fingerprint().unwrap());
    }
}
