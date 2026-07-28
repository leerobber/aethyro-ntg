//! Large-scale agent hierarchy manager — 4-tier structure (Super/Sub/Micro/Nano).
//! ADR 0010 §4: Spawning 10K–500K agent cohorts with bounded coordination overhead.

use std::collections::HashMap;

/// Agent tier in the hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentTier {
    Super,  // Top coordinator (1 agent)
    Sub,    // Regional/functional coordinators (10-50)
    Micro,  // Team leaders (100-1K)
    Nano,   // Individual agents (10K-500K)
}

impl AgentTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentTier::Super => "Super",
            AgentTier::Sub => "Sub",
            AgentTier::Micro => "Micro",
            AgentTier::Nano => "Nano",
        }
    }
}

/// Agent identity in hierarchy: tier, tier-local index, parent index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AgentId {
    pub tier: AgentTier,
    pub tier_index: u32,
    pub parent_tier_index: Option<u32>, // None if Super
}

impl AgentId {
    pub fn global_index(&self) -> u64 {
        match self.tier {
            AgentTier::Super => 0,
            AgentTier::Sub => 1 + (self.tier_index as u64),
            AgentTier::Micro => 51 + (self.tier_index as u64),
            AgentTier::Nano => 1051 + (self.tier_index as u64),
        }
    }

    pub fn path(&self) -> String {
        match self.tier {
            AgentTier::Super => "/super/0".to_string(),
            AgentTier::Sub => format!("/super/0/sub/{}", self.tier_index),
            AgentTier::Micro => {
                if let Some(parent) = self.parent_tier_index {
                    format!("/super/0/sub/{}/micro/{}", parent, self.tier_index)
                } else {
                    format!("/micro/{}", self.tier_index)
                }
            }
            AgentTier::Nano => {
                if let Some(parent) = self.parent_tier_index {
                    format!("/super/0/sub/{}/micro/{}/nano/{}", parent, 0, self.tier_index)
                } else {
                    format!("/nano/{}", self.tier_index)
                }
            }
        }
    }
}

/// Agent state snapshot (minimal telemetry).
#[derive(Clone, Debug)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub lifecycle_stage: String, // "Zygote", "Neonate", ..., "Adult"
    pub fitness: f32,
    pub heartbeat_count: u64,
}

/// Hierarchy manager — coordinates multi-tier agent population.
#[derive(Default)]
pub struct AgentHierarchy {
    super_agent: Option<AgentSnapshot>,
    sub_agents: HashMap<u32, AgentSnapshot>,
    micro_agents: HashMap<u32, AgentSnapshot>,
    nano_agents: HashMap<u32, AgentSnapshot>,
}

impl AgentHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn the Super agent (singleton).
    pub fn spawn_super(&mut self) {
        let id = AgentId {
            tier: AgentTier::Super,
            tier_index: 0,
            parent_tier_index: None,
        };
        self.super_agent = Some(AgentSnapshot {
            id,
            lifecycle_stage: "Neonate".to_string(),
            fitness: 0.5,
            heartbeat_count: 0,
        });
    }

    /// Spawn Sub agents (regional coordinators).
    pub fn spawn_sub(&mut self, count: u32) -> Result<(), String> {
        if count > 50 {
            return Err("Sub tier cannot exceed 50 agents".to_string());
        }
        for i in 0..count {
            let id = AgentId {
                tier: AgentTier::Sub,
                tier_index: i,
                parent_tier_index: None,
            };
            self.sub_agents.insert(i, AgentSnapshot {
                id,
                lifecycle_stage: "Neonate".to_string(),
                fitness: 0.5,
                heartbeat_count: 0,
            });
        }
        Ok(())
    }

    /// Spawn Micro agents (team leaders). Parent is a Sub agent index.
    pub fn spawn_micro(&mut self, count: u32, parent_sub: u32) -> Result<(), String> {
        if !self.sub_agents.contains_key(&parent_sub) {
            return Err(format!("Sub agent {} not found", parent_sub));
        }
        if count > 1000 {
            return Err("Micro tier spawn cannot exceed 1000 per call".to_string());
        }

        let micro_base = self.micro_agents.len() as u32;
        for i in 0..count {
            let idx = micro_base + i;
            let id = AgentId {
                tier: AgentTier::Micro,
                tier_index: idx,
                parent_tier_index: Some(parent_sub),
            };
            self.micro_agents.insert(idx, AgentSnapshot {
                id,
                lifecycle_stage: "Neonate".to_string(),
                fitness: 0.5,
                heartbeat_count: 0,
            });
        }
        Ok(())
    }

    /// Spawn Nano agents (individual workers). Parent is a Micro agent index.
    pub fn spawn_nano(&mut self, count: u32, parent_micro: u32) -> Result<(), String> {
        if !self.micro_agents.contains_key(&parent_micro) {
            return Err(format!("Micro agent {} not found", parent_micro));
        }
        if count > 50000 {
            return Err("Nano tier spawn cannot exceed 50K per call".to_string());
        }

        let nano_base = self.nano_agents.len() as u32;
        for i in 0..count {
            let idx = nano_base + i;
            let id = AgentId {
                tier: AgentTier::Nano,
                tier_index: idx,
                parent_tier_index: Some(parent_micro),
            };
            self.nano_agents.insert(idx, AgentSnapshot {
                id,
                lifecycle_stage: "Neonate".to_string(),
                fitness: 0.5,
                heartbeat_count: 0,
            });
        }
        Ok(())
    }

    /// Total agent count across all tiers.
    pub fn total_agents(&self) -> usize {
        let super_count = if self.super_agent.is_some() { 1 } else { 0 };
        super_count + self.sub_agents.len() + self.micro_agents.len() + self.nano_agents.len()
    }

    /// Get agent by tier and index.
    pub fn get_agent(&self, tier: AgentTier, index: u32) -> Option<&AgentSnapshot> {
        match tier {
            AgentTier::Super => self.super_agent.as_ref(),
            AgentTier::Sub => self.sub_agents.get(&index),
            AgentTier::Micro => self.micro_agents.get(&index),
            AgentTier::Nano => self.nano_agents.get(&index),
        }
    }

    /// Get agent snapshot copy.
    pub fn get_agent_mut(&mut self, tier: AgentTier, index: u32) -> Option<&mut AgentSnapshot> {
        match tier {
            AgentTier::Super => self.super_agent.as_mut(),
            AgentTier::Sub => self.sub_agents.get_mut(&index),
            AgentTier::Micro => self.micro_agents.get_mut(&index),
            AgentTier::Nano => self.nano_agents.get_mut(&index),
        }
    }

    /// Tier population summary.
    pub fn summary(&self) -> HashMap<&'static str, usize> {
        let mut map = HashMap::new();
        if self.super_agent.is_some() {
            map.insert("Super", 1);
        }
        map.insert("Sub", self.sub_agents.len());
        map.insert("Micro", self.micro_agents.len());
        map.insert("Nano", self.nano_agents.len());
        map
    }

    /// Broadcast tick to all agents (simulated).
    pub fn broadcast_tick(&mut self, current_tick: u64) {
        if let Some(super_agent) = self.super_agent.as_mut() {
            super_agent.heartbeat_count = current_tick;
        }
        for agent in self.sub_agents.values_mut() {
            agent.heartbeat_count = current_tick;
        }
        for agent in self.micro_agents.values_mut() {
            agent.heartbeat_count = current_tick;
        }
        for agent in self.nano_agents.values_mut() {
            agent.heartbeat_count = current_tick;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_super_global_index() {
        let id = AgentId {
            tier: AgentTier::Super,
            tier_index: 0,
            parent_tier_index: None,
        };
        assert_eq!(id.global_index(), 0);
    }

    #[test]
    fn agent_id_sub_global_index() {
        let id = AgentId {
            tier: AgentTier::Sub,
            tier_index: 5,
            parent_tier_index: None,
        };
        assert_eq!(id.global_index(), 6);
    }

    #[test]
    fn agent_id_path_super() {
        let id = AgentId {
            tier: AgentTier::Super,
            tier_index: 0,
            parent_tier_index: None,
        };
        assert_eq!(id.path(), "/super/0");
    }

    #[test]
    fn agent_id_path_sub() {
        let id = AgentId {
            tier: AgentTier::Sub,
            tier_index: 3,
            parent_tier_index: None,
        };
        assert_eq!(id.path(), "/super/0/sub/3");
    }

    #[test]
    fn hierarchy_spawn_super() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        assert_eq!(hier.total_agents(), 1);
        assert!(hier.super_agent.is_some());
    }

    #[test]
    fn hierarchy_spawn_sub() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(5).unwrap();
        assert_eq!(hier.total_agents(), 6); // 1 super + 5 sub
    }

    #[test]
    fn hierarchy_spawn_sub_exceeds_limit() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        assert!(hier.spawn_sub(51).is_err());
    }

    #[test]
    fn hierarchy_spawn_micro() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(3).unwrap();
        hier.spawn_micro(50, 0).unwrap();
        assert_eq!(hier.total_agents(), 54); // 1 super + 3 sub + 50 micro
    }

    #[test]
    fn hierarchy_spawn_micro_invalid_parent() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        assert!(hier.spawn_micro(10, 999).is_err());
    }

    #[test]
    fn hierarchy_spawn_nano() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(2).unwrap();
        hier.spawn_micro(10, 0).unwrap();
        hier.spawn_nano(1000, 0).unwrap();
        assert_eq!(hier.total_agents(), 1013); // 1 + 2 + 10 + 1000
    }

    #[test]
    fn hierarchy_spawn_nano_large_cohort() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(50).unwrap();
        hier.spawn_micro(1000, 0).unwrap();
        hier.spawn_nano(50000, 0).unwrap();
        assert_eq!(hier.total_agents(), 51051);
    }

    #[test]
    fn hierarchy_summary() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(5).unwrap();
        hier.spawn_micro(20, 0).unwrap();
        hier.spawn_nano(100, 0).unwrap();

        let summary = hier.summary();
        assert_eq!(*summary.get("Super").unwrap_or(&0), 1);
        assert_eq!(*summary.get("Sub").unwrap_or(&0), 5);
        assert_eq!(*summary.get("Micro").unwrap_or(&0), 20);
        assert_eq!(*summary.get("Nano").unwrap_or(&0), 100);
    }

    #[test]
    fn hierarchy_broadcast_tick() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(2).unwrap();
        hier.spawn_micro(5, 0).unwrap();
        hier.spawn_nano(10, 0).unwrap();

        hier.broadcast_tick(42);

        assert_eq!(hier.super_agent.as_ref().unwrap().heartbeat_count, 42);
        assert_eq!(hier.sub_agents.get(&0).unwrap().heartbeat_count, 42);
        assert_eq!(hier.micro_agents.get(&0).unwrap().heartbeat_count, 42);
        assert_eq!(hier.nano_agents.get(&0).unwrap().heartbeat_count, 42);
    }

    #[test]
    fn hierarchy_get_agent() {
        let mut hier = AgentHierarchy::new();
        hier.spawn_super();
        hier.spawn_sub(3).unwrap();

        let super_agent = hier.get_agent(AgentTier::Super, 0);
        assert!(super_agent.is_some());

        let sub_agent = hier.get_agent(AgentTier::Sub, 1);
        assert!(sub_agent.is_some());

        let missing = hier.get_agent(AgentTier::Nano, 999);
        assert!(missing.is_none());
    }
}
