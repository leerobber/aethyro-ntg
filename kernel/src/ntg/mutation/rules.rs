//! Mutation rule types: concrete topology changes with audit trails.
//!
//! Five core rules for Phase 3:
//! 1. AddNode: introduce a new node with a label
//! 2. RemoveNode: delete a node (if no incoming edges)
//! 3. AddEdge: connect two nodes
//! 4. RemoveEdge: disconnect two nodes
//! 5. RewireEdge: change an edge's target

use super::super::error::NtgError;
use super::super::graph::{Graph, NodeId, NodeKind};

#[derive(Clone, Debug)]
pub enum MutationRuleKind {
    AddNode { label: String },
    RemoveNode { node_id: NodeId },
    AddEdge { from: NodeId, to: NodeId },
    RemoveEdge { from: NodeId, to: NodeId },
    RewireEdge {
        from: NodeId,
        old_to: NodeId,
        new_to: NodeId,
    },
}

/// A versioned, auditable mutation rule.
#[derive(Clone, Debug)]
pub struct MutationRule {
    pub kind: MutationRuleKind,
}

impl MutationRule {
    pub fn description(&self) -> String {
        match &self.kind {
            MutationRuleKind::AddNode { label } => format!("add_node(label='{}')", label),
            MutationRuleKind::RemoveNode { node_id } => format!("remove_node(id={})", node_id),
            MutationRuleKind::AddEdge { from, to } => format!("add_edge({}→{})", from, to),
            MutationRuleKind::RemoveEdge { from, to } => format!("remove_edge({}→{})", from, to),
            MutationRuleKind::RewireEdge {
                from,
                old_to,
                new_to,
            } => format!("rewire_edge({}→{} → {}→{})", from, old_to, from, new_to),
        }
    }

    /// Apply this rule to a graph (creates a test topology).
    /// The graph must be cloned before calling this so the original is unchanged.
    pub fn apply(&self, graph: &mut Graph) -> Result<(), NtgError> {
        match &self.kind {
            MutationRuleKind::AddNode { label } => {
                // Content node by default; Execution nodes are a separate concern.
                let _id = graph.add_node(NodeKind::Content, label.clone());
                Ok(())
            }
            MutationRuleKind::RemoveNode { node_id } => {
                graph.remove_node(*node_id)?;
                Ok(())
            }
            MutationRuleKind::AddEdge { from, to } => {
                graph.add_edge(*from, *to)?;
                Ok(())
            }
            MutationRuleKind::RemoveEdge { from, to } => {
                graph.remove_edge(*from, *to)?;
                Ok(())
            }
            MutationRuleKind::RewireEdge {
                from,
                old_to,
                new_to,
            } => {
                graph.remove_edge(*from, *old_to)?;
                graph.add_edge(*from, *new_to)?;
                Ok(())
            }
        }
    }

    /// Versioned rule ID (for audit trails).
    /// In a real system, this would be a content hash or semantic versioning.
    pub fn rule_version(&self) -> u32 {
        1 // Phase 3.0: all rules versioned as 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_description_add_node() {
        let rule = MutationRule {
            kind: MutationRuleKind::AddNode {
                label: "test_node".to_string(),
            },
        };
        assert_eq!(rule.description(), "add_node(label='test_node')");
    }

    #[test]
    fn rule_description_remove_node() {
        let rule = MutationRule {
            kind: MutationRuleKind::RemoveNode { node_id: 42 },
        };
        assert_eq!(rule.description(), "remove_node(id=42)");
    }

    #[test]
    fn rule_description_add_edge() {
        let rule = MutationRule {
            kind: MutationRuleKind::AddEdge { from: 1, to: 2 },
        };
        assert_eq!(rule.description(), "add_edge(1→2)");
    }

    #[test]
    fn rule_description_remove_edge() {
        let rule = MutationRule {
            kind: MutationRuleKind::RemoveEdge { from: 1, to: 2 },
        };
        assert_eq!(rule.description(), "remove_edge(1→2)");
    }

    #[test]
    fn rule_description_rewire_edge() {
        let rule = MutationRule {
            kind: MutationRuleKind::RewireEdge {
                from: 1,
                old_to: 2,
                new_to: 3,
            },
        };
        assert_eq!(rule.description(), "rewire_edge(1→2 → 1→3)");
    }

    #[test]
    fn rule_version_is_one() {
        let rule = MutationRule {
            kind: MutationRuleKind::AddNode {
                label: "test".to_string(),
            },
        };
        assert_eq!(rule.rule_version(), 1);
    }

    #[test]
    fn apply_add_node() -> Result<(), NtgError> {
        let mut graph = Graph::new();
        let rule = MutationRule {
            kind: MutationRuleKind::AddNode {
                label: "new_node".to_string(),
            },
        };
        rule.apply(&mut graph)?;
        // Verify node was added (basic smoke test)
        assert!(graph.node_count() > 0);
        Ok(())
    }
}
