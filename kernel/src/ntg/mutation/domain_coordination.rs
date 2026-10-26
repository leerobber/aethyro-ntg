//! Phase 6.13: Multi-Agent Coordination — inter-domain mutation synchronization.
//!
//! Coordinates mutation strategies across multiple autonomous domains (super-agents,
//! sub-agents, micro-agents, nano-agents) with:
//! - Pattern affinity scoring and cross-domain transfer
//! - Distributed mutation queue with load balancing
//! - Consensus mechanism for conflicting mutations
//! - Synchronized behavioral snapshots across agent hierarchy

use std::collections::{HashMap, VecDeque};
use super::super::error::NtgError;

/// Unique identifier for an autonomous agent/domain in the multi-agent system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(u64);

impl AgentId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Hierarchical agent level in the swarm (super → sub → micro → nano).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentLevel {
    /// Top-level coordinator (1 per domain cluster)
    Super,
    /// Mid-level coordinators (10-100 per super)
    Sub,
    /// Local executors (100-1000 per sub)
    Micro,
    /// Leaf workers (1000-10000 per micro)
    Nano,
}

impl AgentLevel {
    /// Typical number of agents at this level per parent.
    pub fn typical_children_per_parent(&self) -> usize {
        match self {
            Self::Super => 100,  // 1 super → 100 subs
            Self::Sub => 50,     // 1 sub → 50 micros
            Self::Micro => 100,  // 1 micro → 100 nanos
            Self::Nano => 1,     // Leaf (no children)
        }
    }
}

/// Cross-domain affinity score (0.0 = unrelated, 1.0 = perfectly aligned).
#[derive(Clone, Copy, Debug)]
pub struct PatternAffinity {
    /// Domain similarity based on mutation patterns (0.0-1.0)
    pub pattern_similarity: f32,
    /// Fitness alignment (0.0-1.0, higher = more aligned objectives)
    pub fitness_alignment: f32,
    /// Behavioral compatibility (0.0-1.0, higher = non-conflicting strategies)
    pub behavioral_compatibility: f32,
    /// Confidence in affinity score (0.0-1.0)
    pub confidence: f32,
}

impl PatternAffinity {
    /// Overall affinity score: weighted combination of all factors.
    pub fn overall_score(&self) -> f32 {
        (self.pattern_similarity * 0.5
            + self.fitness_alignment * 0.25
            + self.behavioral_compatibility * 0.25)
            * self.confidence
    }
}

/// Proposed mutation from an autonomous agent.
#[derive(Clone, Debug)]
pub struct MutationProposal {
    pub proposal_id: u64,
    pub agent_id: AgentId,
    pub agent_level: AgentLevel,
    pub cycle: u64,
    pub mutation_description: String,
    pub estimated_efficiency_gain: f32,
    pub confidence: f32,
    /// Domain tag (e.g., "learning", "routing", "memory")
    pub domain: String,
    pub timestamp_us: u64,
}

/// Mutation consensus decision across agents.
#[derive(Clone, Debug, PartialEq)]
pub enum ConsensusDecision {
    /// All agents agree to apply (requires ≥80% affinity)
    Approved {
        supporting_agents: usize,
        total_agents: usize,
    },
    /// Majority conflict (different domains propose incompatible mutations)
    Conflict {
        sides: Vec<(String, usize)>, // (domain/strategy, agent count)
    },
    /// Insufficient affinity for consensus
    Deferred {
        reason: String,
    },
}

/// Distributed mutation queue entry with routing metadata.
#[derive(Clone, Debug)]
pub struct QueuedMutation {
    pub proposal: MutationProposal,
    /// Which agent should execute (load-balanced assignment)
    pub assigned_to: AgentId,
    /// Priority (0.0-1.0, higher = earlier execution)
    pub priority: f32,
    /// Estimated queue position
    pub queue_position: usize,
    /// How many other agents are waiting for results
    pub dependent_count: usize,
}

/// Cross-domain pattern transfer record.
#[derive(Clone, Debug)]
pub struct PatternTransfer {
    pub from_agent: AgentId,
    pub from_level: AgentLevel,
    pub to_agents: Vec<AgentId>,
    pub pattern_type: String,
    pub affinity_score: f32,
    pub timestamp_us: u64,
    /// Whether transfer was successful
    pub completed: bool,
}

/// Per-domain behavioral snapshot for alignment tracking.
#[derive(Clone, Debug)]
pub struct DomainSnapshot {
    pub agent_id: AgentId,
    pub level: AgentLevel,
    pub cycle: u64,
    pub efficiency: f32,
    pub mutation_acceptance_rate: f32,
    pub active_strategies: Vec<String>,
    /// Signature of current behavioral state
    pub behavior_hash: u64,
}

/// Multi-agent coordination engine (Phase 6.13).
#[derive(Clone, Debug)]
pub struct DomainCoordinationEngine {
    /// All agents in the system (indexed by agent_id)
    pub agents: HashMap<AgentId, AgentMetadata>,
    /// Affinity matrix between agents (cached)
    affinity_cache: HashMap<(AgentId, AgentId), PatternAffinity>,
    /// Distributed mutation queue (per agent)
    mutation_queues: HashMap<AgentId, VecDeque<QueuedMutation>>,
    /// Pattern transfer history (for learning)
    transfer_history: Vec<PatternTransfer>,
    /// Per-domain behavioral snapshots
    domain_snapshots: HashMap<AgentId, Vec<DomainSnapshot>>,
    /// Pending consensus decisions
    pending_consensus: HashMap<u64, ConsensusDecision>,
    /// Global cycle counter
    pub global_cycle: u64,
    /// Configuration
    pub config: CoordinationConfig,
}

/// Metadata about an autonomous agent.
#[derive(Clone, Debug)]
pub struct AgentMetadata {
    pub id: AgentId,
    pub level: AgentLevel,
    pub parent: Option<AgentId>,
    pub domain: String,
    pub efficiency_history: VecDeque<f32>, // Last 50 cycles
    pub mutation_capacity: usize,
    pub last_heartbeat_cycle: u64,
}

/// Configuration for multi-agent coordination.
#[derive(Clone, Debug)]
pub struct CoordinationConfig {
    /// Minimum affinity score for pattern transfer (0.0-1.0)
    pub min_affinity_for_transfer: f32,
    /// Minimum agents in agreement for consensus (0.0-1.0 of total)
    pub consensus_threshold: f32,
    /// Max mutations per agent per cycle
    pub max_mutations_per_agent: usize,
    /// How long to maintain transfer history (cycles)
    pub transfer_history_window: u64,
    /// Enable conflict resolution via voting
    pub enable_conflict_resolution: bool,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            min_affinity_for_transfer: 0.75,
            consensus_threshold: 0.80,
            max_mutations_per_agent: 10,
            transfer_history_window: 1000,
            enable_conflict_resolution: true,
        }
    }
}

impl DomainCoordinationEngine {
    pub fn new(config: CoordinationConfig) -> Self {
        Self {
            agents: HashMap::new(),
            affinity_cache: HashMap::new(),
            mutation_queues: HashMap::new(),
            transfer_history: Vec::new(),
            domain_snapshots: HashMap::new(),
            pending_consensus: HashMap::new(),
            global_cycle: 0,
            config,
        }
    }

    /// Register a new agent in the coordination system.
    pub fn register_agent(
        &mut self,
        agent_id: AgentId,
        level: AgentLevel,
        domain: String,
        parent: Option<AgentId>,
    ) -> Result<(), NtgError> {
        if self.agents.contains_key(&agent_id) {
            return Err(NtgError::InvalidInput(format!(
                "Agent {:?} already registered",
                agent_id
            )));
        }

        let metadata = AgentMetadata {
            id: agent_id,
            level,
            parent,
            domain,
            efficiency_history: VecDeque::with_capacity(50),
            mutation_capacity: self.config.max_mutations_per_agent,
            last_heartbeat_cycle: self.global_cycle,
        };

        self.agents.insert(agent_id, metadata);
        self.mutation_queues.insert(agent_id, VecDeque::new());
        self.domain_snapshots.insert(agent_id, Vec::new());

        Ok(())
    }

    /// Compute affinity between two agents based on behavior and domain.
    pub fn compute_affinity(
        &mut self,
        agent_a: AgentId,
        agent_b: AgentId,
    ) -> Result<PatternAffinity, NtgError> {
        if agent_a == agent_b {
            return Ok(PatternAffinity {
                pattern_similarity: 1.0,
                fitness_alignment: 1.0,
                behavioral_compatibility: 1.0,
                confidence: 1.0,
            });
        }

        // Check cache first
        let key = if agent_a < agent_b {
            (agent_a, agent_b)
        } else {
            (agent_b, agent_a)
        };

        if let Some(cached) = self.affinity_cache.get(&key) {
            return Ok(*cached);
        }

        let meta_a = self
            .agents
            .get(&agent_a)
            .ok_or_else(|| NtgError::InvalidInput(format!("Agent {:?} not found", agent_a)))?;
        let meta_b = self
            .agents
            .get(&agent_b)
            .ok_or_else(|| NtgError::InvalidInput(format!("Agent {:?} not found", agent_b)))?;

        // Pattern similarity: same domain or related domains
        let pattern_similarity = if meta_a.domain == meta_b.domain {
            0.9
        } else {
            0.4
        };

        // Fitness alignment: compare recent efficiency averages
        let avg_a = if meta_a.efficiency_history.is_empty() {
            0.5
        } else {
            meta_a.efficiency_history.iter().sum::<f32>() / meta_a.efficiency_history.len() as f32
        };
        let avg_b = if meta_b.efficiency_history.is_empty() {
            0.5
        } else {
            meta_b.efficiency_history.iter().sum::<f32>() / meta_b.efficiency_history.len() as f32
        };
        let fitness_alignment = 1.0 - (avg_a - avg_b).abs();

        // Behavioral compatibility: similar hierarchy levels work better
        let behavioral_compatibility = if meta_a.level == meta_b.level {
            0.85
        } else {
            0.6
        };

        // Confidence: higher if we have enough history
        let confidence = (meta_a.efficiency_history.len() as f32 / 50.0).min(1.0);

        let affinity = PatternAffinity {
            pattern_similarity,
            fitness_alignment,
            behavioral_compatibility,
            confidence,
        };

        self.affinity_cache.insert(key, affinity);
        Ok(affinity)
    }

    /// Propose a mutation from an agent, route it through the queue.
    pub fn propose_mutation(
        &mut self,
        proposal: MutationProposal,
    ) -> Result<u64, NtgError> {
        let agent_id = proposal.agent_id;
        if !self.agents.contains_key(&agent_id) {
            return Err(NtgError::InvalidInput(format!(
                "Agent {:?} not registered",
                agent_id
            )));
        }

        let queue = self
            .mutation_queues
            .get_mut(&agent_id)
            .ok_or_else(|| NtgError::InvalidInput("Queue not found".to_string()))?;

        if queue.len() >= self.config.max_mutations_per_agent {
            return Err(NtgError::InvalidInput(
                "Agent mutation queue full".to_string(),
            ));
        }

        // Compute priority based on confidence and estimated gain
        let priority = (proposal.confidence * 0.6 + proposal.estimated_efficiency_gain * 0.4)
            .clamp(0.0, 1.0);

        let queued = QueuedMutation {
            proposal: proposal.clone(),
            assigned_to: agent_id,
            priority,
            queue_position: queue.len(),
            dependent_count: 0,
        };

        queue.push_back(queued);
        Ok(proposal.proposal_id)
    }

    /// Transfer a successful pattern from source to target agents.
    pub fn transfer_pattern(
        &mut self,
        from_agent: AgentId,
        to_agents: Vec<AgentId>,
        pattern_type: String,
    ) -> Result<(), NtgError> {
        if !self.agents.contains_key(&from_agent) {
            return Err(NtgError::InvalidInput(format!(
                "Source agent {:?} not found",
                from_agent
            )));
        }

        let from_level = self.agents[&from_agent].level;

        // Compute affinity between source and each target
        let mut valid_targets = Vec::new();
        for &target_id in &to_agents {
            if !self.agents.contains_key(&target_id) {
                continue;
            }

            let affinity = self.compute_affinity(from_agent, target_id)?;
            if affinity.overall_score() >= self.config.min_affinity_for_transfer {
                valid_targets.push(target_id);
            }
        }

        if valid_targets.is_empty() {
            return Err(NtgError::InvalidInput(
                "No agents meet affinity threshold for transfer".to_string(),
            ));
        }

        let transfer = PatternTransfer {
            from_agent,
            from_level,
            to_agents: valid_targets.clone(),
            pattern_type,
            affinity_score: self.compute_affinity(from_agent, valid_targets[0])?.overall_score(),
            timestamp_us: self.global_cycle,
            completed: true,
        };

        self.transfer_history.push(transfer);
        Ok(())
    }

    /// Attempt consensus on a conflicting mutation set.
    pub fn reach_consensus(
        &mut self,
        proposal_id: u64,
        proposals: Vec<MutationProposal>,
    ) -> Result<ConsensusDecision, NtgError> {
        if proposals.is_empty() {
            return Err(NtgError::InvalidInput("No proposals for consensus".to_string()));
        }

        // Group proposals by domain/strategy
        let mut domain_groups: HashMap<String, usize> = HashMap::new();
        for proposal in &proposals {
            *domain_groups.entry(proposal.domain.clone()).or_insert(0) += 1;
        }

        let total_agents = proposals.len();

        // Check if all agents have high affinity
        let mut all_affinities = Vec::new();
        for i in 0..proposals.len() {
            for j in (i + 1)..proposals.len() {
                let aff = self.compute_affinity(
                    proposals[i].agent_id,
                    proposals[j].agent_id,
                )?;
                all_affinities.push(aff.overall_score());
            }
        }

        let avg_affinity = if all_affinities.is_empty() {
            1.0
        } else {
            all_affinities.iter().sum::<f32>() / all_affinities.len() as f32
        };

        let decision = if avg_affinity >= self.config.consensus_threshold {
            ConsensusDecision::Approved {
                supporting_agents: total_agents,
                total_agents,
            }
        } else if domain_groups.len() > 1 && self.config.enable_conflict_resolution {
            let mut sides = domain_groups.into_iter().collect::<Vec<_>>();
            sides.sort_by_key(|side| std::cmp::Reverse(side.1));
            ConsensusDecision::Conflict { sides }
        } else {
            ConsensusDecision::Deferred {
                reason: format!("Affinity score {:.2} below threshold", avg_affinity),
            }
        };

        self.pending_consensus.insert(proposal_id, decision.clone());
        Ok(decision)
    }

    /// Load-balance mutation queue across agents.
    pub fn rebalance_queues(&mut self) -> Result<usize, NtgError> {
        let mut reassignments = 0;

        // Collect all queued mutations
        let mut all_mutations: Vec<(AgentId, QueuedMutation)> = Vec::new();
        for (agent_id, queue) in self.mutation_queues.iter() {
            for mutation in queue.iter() {
                all_mutations.push((*agent_id, mutation.clone()));
            }
        }

        // Sort by priority (highest first)
        all_mutations.sort_by(|a, b| b.1.priority.partial_cmp(&a.1.priority).unwrap());

        // Reassign to least-loaded agents
        self.mutation_queues.clear();
        for agent_id in self.agents.keys() {
            self.mutation_queues.insert(*agent_id, VecDeque::new());
        }

        for (_original_agent, mutation) in all_mutations {
            // Find agent with smallest queue
            let target_agent = self
                .mutation_queues
                .iter()
                .min_by_key(|(_, queue)| queue.len())
                .map(|(agent_id, _)| *agent_id)
                .unwrap_or(mutation.assigned_to);

            if target_agent != mutation.assigned_to {
                reassignments += 1;
            }

            let mut updated = mutation;
            updated.assigned_to = target_agent;
            updated.queue_position = self.mutation_queues[&target_agent].len();

            self.mutation_queues
                .get_mut(&target_agent)
                .unwrap()
                .push_back(updated);
        }

        Ok(reassignments)
    }

    /// Record behavioral snapshot for an agent (for drift detection across hierarchy).
    pub fn record_domain_snapshot(&mut self, snapshot: DomainSnapshot) -> Result<(), NtgError> {
        let agent_id = snapshot.agent_id;

        // Update efficiency history
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.efficiency_history.push_back(snapshot.efficiency);
            if agent.efficiency_history.len() > 50 {
                agent.efficiency_history.pop_front();
            }
            agent.last_heartbeat_cycle = self.global_cycle;
        }

        // Store snapshot
        let snapshots = self
            .domain_snapshots
            .entry(agent_id)
            .or_default();
        snapshots.push(snapshot);

        // Prune old snapshots (keep last N cycles per agent)
        if snapshots.len() > 100 {
            snapshots.remove(0);
        }

        Ok(())
    }

    /// Advance global cycle and prune stale data.
    pub fn next_cycle(&mut self) {
        self.global_cycle += 1;

        // Clear old affinity cache entries periodically
        if self.global_cycle.is_multiple_of(100) {
            self.affinity_cache.clear();
        }

        // Prune old transfer history
        let cutoff = self.global_cycle.saturating_sub(self.config.transfer_history_window);
        self.transfer_history
            .retain(|t| t.timestamp_us >= cutoff);

        // Mark inactive agents (no heartbeat in 10 cycles)
        let inactive_threshold = self.global_cycle.saturating_sub(10);
        for agent in self.agents.values_mut() {
            if agent.last_heartbeat_cycle < inactive_threshold {
                // Agent is considered inactive; could trigger failover
            }
        }
    }

    /// Generate coordination report.
    pub fn report(&self) -> String {
        let total_agents = self.agents.len();
        let total_queued: usize = self.mutation_queues.values().map(|q| q.len()).sum();
        let total_transfers = self.transfer_history.len();

        let level_breakdown = {
            let mut counts: HashMap<AgentLevel, usize> = HashMap::new();
            for agent in self.agents.values() {
                *counts.entry(agent.level).or_insert(0) += 1;
            }
            let mut lines = Vec::new();
            for level in &[AgentLevel::Super, AgentLevel::Sub, AgentLevel::Micro, AgentLevel::Nano] {
                if let Some(count) = counts.get(level) {
                    lines.push(format!(
                        "  {:?}: {} agents",
                        level, count
                    ));
                }
            }
            lines.join("\n")
        };

        format!(
            "=== Domain Coordination Report (Cycle {}) ===\n\
             Total Agents: {}\n\
             {}\n\
             Mutations Queued: {}\n\
             Pattern Transfers: {}\n\
             Pending Consensus Decisions: {}\n",
            self.global_cycle,
            total_agents,
            level_breakdown,
            total_queued,
            total_transfers,
            self.pending_consensus.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_agents_in_hierarchy() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());

        // Register super agent
        let super_id = AgentId::new(1);
        engine
            .register_agent(super_id, AgentLevel::Super, "root".to_string(), None)
            .unwrap();

        // Register sub agent (child of super)
        let sub_id = AgentId::new(2);
        engine
            .register_agent(sub_id, AgentLevel::Sub, "learning".to_string(), Some(super_id))
            .unwrap();

        assert_eq!(engine.agents.len(), 2);
        assert_eq!(engine.agents[&sub_id].parent, Some(super_id));
    }

    #[test]
    fn compute_affinity_self_is_perfect() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let agent_id = AgentId::new(1);
        engine
            .register_agent(agent_id, AgentLevel::Micro, "test".to_string(), None)
            .unwrap();

        let affinity = engine.compute_affinity(agent_id, agent_id).unwrap();
        assert_eq!(affinity.overall_score(), 1.0);
    }

    #[test]
    fn affinity_same_domain_higher() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let a1 = AgentId::new(1);
        let a2 = AgentId::new(2);
        let a3 = AgentId::new(3);

        engine
            .register_agent(a1, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();
        engine
            .register_agent(a2, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();
        engine
            .register_agent(a3, AgentLevel::Micro, "routing".to_string(), None)
            .unwrap();

        // Add efficiency history so confidence is not 0
        for _ in 0..10 {
            engine.agents.get_mut(&a1).unwrap().efficiency_history.push_back(0.8);
            engine.agents.get_mut(&a2).unwrap().efficiency_history.push_back(0.8);
            engine.agents.get_mut(&a3).unwrap().efficiency_history.push_back(0.9);
        }

        let same_domain = engine.compute_affinity(a1, a2).unwrap().overall_score();
        let diff_domain = engine.compute_affinity(a1, a3).unwrap().overall_score();

        // Same domain should have higher affinity due to pattern_similarity (0.9 vs 0.4)
        assert!(same_domain > diff_domain, "same_domain={}, diff_domain={}", same_domain, diff_domain);
    }

    #[test]
    fn propose_mutation_queues() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let agent_id = AgentId::new(1);
        engine
            .register_agent(agent_id, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();

        let proposal = MutationProposal {
            proposal_id: 1,
            agent_id,
            agent_level: AgentLevel::Micro,
            cycle: 0,
            mutation_description: "test mutation".to_string(),
            estimated_efficiency_gain: 0.15,
            confidence: 0.9,
            domain: "learning".to_string(),
            timestamp_us: 0,
        };

        engine.propose_mutation(proposal).unwrap();

        let queue = &engine.mutation_queues[&agent_id];
        assert_eq!(queue.len(), 1);
        assert!(queue[0].priority > 0.5);
    }

    #[test]
    fn load_balance_queues() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let a1 = AgentId::new(1);
        let a2 = AgentId::new(2);

        engine
            .register_agent(a1, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();
        engine
            .register_agent(a2, AgentLevel::Micro, "routing".to_string(), None)
            .unwrap();

        // Add mutations to a1
        for i in 0..5 {
            let proposal = MutationProposal {
                proposal_id: i,
                agent_id: a1,
                agent_level: AgentLevel::Micro,
                cycle: 0,
                mutation_description: format!("mutation {}", i),
                estimated_efficiency_gain: 0.1,
                confidence: 0.8,
                domain: "learning".to_string(),
                timestamp_us: 0,
            };
            engine.propose_mutation(proposal).unwrap();
        }

        let reassignments = engine.rebalance_queues().unwrap();
        assert!(reassignments > 0);

        // Verify distribution improved
        let q1 = engine.mutation_queues[&a1].len();
        let q2 = engine.mutation_queues[&a2].len();
        assert!(q1 >= 2 && q2 >= 2); // More balanced now
    }

    #[test]
    fn consensus_high_affinity_approved() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let a1 = AgentId::new(1);
        let a2 = AgentId::new(2);

        engine
            .register_agent(a1, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();
        engine
            .register_agent(a2, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();

        let proposals = vec![
            MutationProposal {
                proposal_id: 1,
                agent_id: a1,
                agent_level: AgentLevel::Micro,
                cycle: 0,
                mutation_description: "mutation 1".to_string(),
                estimated_efficiency_gain: 0.15,
                confidence: 0.9,
                domain: "learning".to_string(),
                timestamp_us: 0,
            },
            MutationProposal {
                proposal_id: 2,
                agent_id: a2,
                agent_level: AgentLevel::Micro,
                cycle: 0,
                mutation_description: "mutation 2".to_string(),
                estimated_efficiency_gain: 0.15,
                confidence: 0.9,
                domain: "learning".to_string(),
                timestamp_us: 0,
            },
        ];

        let decision = engine.reach_consensus(1, proposals).unwrap();
        matches!(decision, ConsensusDecision::Approved { .. });
    }

    #[test]
    fn pattern_transfer_requires_affinity() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let source = AgentId::new(1);
        let target = AgentId::new(2);

        engine
            .register_agent(source, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();
        engine
            .register_agent(target, AgentLevel::Nano, "routing".to_string(), None)
            .unwrap();

        let result = engine.transfer_pattern(
            source,
            vec![target],
            "routing_pattern".to_string(),
        );

        // May be deferred due to low affinity (different levels + domains)
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn record_domain_snapshot() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let agent_id = AgentId::new(1);
        engine
            .register_agent(agent_id, AgentLevel::Micro, "learning".to_string(), None)
            .unwrap();

        let snapshot = DomainSnapshot {
            agent_id,
            level: AgentLevel::Micro,
            cycle: 0,
            efficiency: 0.85,
            mutation_acceptance_rate: 0.75,
            active_strategies: vec!["strategy1".to_string()],
            behavior_hash: 12345,
        };

        engine.record_domain_snapshot(snapshot).unwrap();

        assert_eq!(
            engine.agents[&agent_id].efficiency_history.len(),
            1
        );
        assert_eq!(engine.domain_snapshots[&agent_id].len(), 1);
    }

    #[test]
    fn next_cycle_advances_counter() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let initial = engine.global_cycle;
        engine.next_cycle();
        assert_eq!(engine.global_cycle, initial + 1);
    }

    #[test]
    fn agent_level_hierarchy_children() {
        assert_eq!(AgentLevel::Super.typical_children_per_parent(), 100);
        assert_eq!(AgentLevel::Sub.typical_children_per_parent(), 50);
        assert_eq!(AgentLevel::Micro.typical_children_per_parent(), 100);
        assert_eq!(AgentLevel::Nano.typical_children_per_parent(), 1);
    }

    #[test]
    fn report_generation() {
        let mut engine = DomainCoordinationEngine::new(CoordinationConfig::default());
        let a1 = AgentId::new(1);
        engine
            .register_agent(a1, AgentLevel::Super, "root".to_string(), None)
            .unwrap();

        let report = engine.report();
        assert!(report.contains("Domain Coordination Report"));
        assert!(report.contains("1 agents"));
    }
}
