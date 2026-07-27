//! Phase B: Chromosome Agents
//! Local intelligence handlers for genome queries and decisions
//! Agents operate on chromosome brains and coordinate multi-brain responses

use crate::genomic::chromosome_brain::{ChromosomeBrain, ChromosomeId};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum AgentQuery {
    /// Risk assessment for specific SNPs
    DiseaseRisk { snp_indices: Vec<u32> },
    /// Trait pattern analysis
    TraitPattern { trait_name: String },
    /// Population-level signals
    PopulationSignal,
    /// Evolutionary/adaptation hints
    EvolutionHint,
}

#[derive(Clone, Debug)]
pub struct AgentResponse {
    pub query_type: String,
    pub explanation: String,
    pub score: f32,
    pub affected_snps: Vec<u32>,
    pub affected_blocks: Vec<u32>,
    pub signal_vector: [f32; 32],
}

#[derive(Clone, Debug)]
pub struct ChromosomeAgent {
    pub brain: ChromosomeBrain,
    pub capabilities: Vec<String>,
}

impl ChromosomeAgent {
    /// Create new agent from trained brain
    pub fn new(brain: ChromosomeBrain) -> Self {
        let capabilities = vec![
            "NeuroGenomics".to_string(),
            "RiskAssessment".to_string(),
            "TraitPrediction".to_string(),
            "PopulationAnalysis".to_string(),
        ];
        Self { brain, capabilities }
    }

    /// Handle query on this chromosome
    pub fn handle_query(&self, query: &AgentQuery) -> AgentResponse {
        match query {
            AgentQuery::DiseaseRisk { snp_indices } => self.assess_disease_risk(snp_indices),
            AgentQuery::TraitPattern { trait_name } => self.analyze_trait_pattern(trait_name),
            AgentQuery::PopulationSignal => self.compute_population_signal(),
            AgentQuery::EvolutionHint => self.infer_evolution_hint(),
        }
    }

    fn assess_disease_risk(&self, snp_indices: &[u32]) -> AgentResponse {
        let mut score = 0.0f32;
        let mut affected_snps = Vec::new();
        let mut affected_blocks = Vec::new();
        let mut explanation = format!("Chr{} Disease Risk Assessment: ", self.brain.chr.0);

        for &target_idx in snp_indices {
            if let Some(neuron) = self.brain.neurons.iter().find(|n| n.snp_index == target_idx) {
                affected_snps.push(target_idx);
                score += 0.15;
                explanation.push_str(&format!("SNP#{} ", target_idx));

                // Find blocks containing this SNP
                for (b_idx, block) in self.brain.blocks.iter().enumerate() {
                    if block.snp_indices.contains(&target_idx) {
                        affected_blocks.push(b_idx as u32);
                        score += block.mean_r_squared * 0.05;
                    }
                }

                // Check synapses from this neuron
                let connected = self
                    .brain
                    .synapses_for_neuron(neuron.id)
                    .iter()
                    .map(|s| s.ld_r2)
                    .sum::<f32>();
                score += (connected / (self.brain.synapses.len() as f32 + 1.0)) * 0.1;
            }
        }

        if affected_snps.is_empty() {
            explanation.push_str("No matching SNPs found in this chromosome.");
            score = 0.0;
        } else {
            explanation.push_str(&format!(
                " Total Risk Score: {:.3} (affected {} SNPs, {} blocks)",
                score,
                affected_snps.len(),
                affected_blocks.len()
            ));
        }

        let mut signal = [0.0f32; 32];
        signal[0] = score;
        signal[1] = affected_snps.len() as f32;
        signal[2] = self.brain.kairos_state.convergence_score;

        AgentResponse {
            query_type: "DiseaseRisk".to_string(),
            explanation,
            score: score.min(1.0),
            affected_snps,
            affected_blocks,
            signal_vector: signal,
        }
    }

    fn analyze_trait_pattern(&self, trait_name: &str) -> AgentResponse {
        let mut score = 0.42f32;
        let mut explanation = format!(
            "Chr{} Trait Analysis for '{}': ",
            self.brain.chr.0, trait_name
        );

        // Heuristic: LD density in this chromosome indicates trait complexity
        let avg_weight = if self.brain.synapses.is_empty() {
            0.0
        } else {
            self.brain.synapses.iter().map(|s| s.weight).sum::<f32>() / self.brain.synapses.len() as f32
        };
        score += avg_weight * 0.3;

        explanation.push_str(&format!(
            "Detected {} LD edges (avg weight {:.3}). ",
            self.brain.synapses.len(),
            avg_weight
        ));

        // Block count as proxy for genetic architecture complexity
        explanation.push_str(&format!(
            "{} haplotype blocks suggest {} complexity.",
            self.brain.blocks.len(),
            if self.brain.blocks.len() > 10 {
                "high"
            } else if self.brain.blocks.len() > 3 {
                "moderate"
            } else {
                "low"
            }
        ));

        let mut signal = [0.0f32; 32];
        signal[0] = score.min(1.0);
        signal[1] = self.brain.blocks.len() as f32;
        signal[2] = avg_weight;

        AgentResponse {
            query_type: "TraitPattern".to_string(),
            explanation,
            score: score.min(1.0),
            affected_snps: vec![],
            affected_blocks: (0..self.brain.blocks.len() as u32).collect(),
            signal_vector: signal,
        }
    }

    fn compute_population_signal(&self) -> AgentResponse {
        let n_rare = self.brain.neurons.iter().filter(|n| n.is_rare).count();
        let rare_frac = n_rare as f32 / (self.brain.neurons.len() as f32 + 1.0);

        let score = rare_frac;
        let mut explanation = format!(
            "Chr{} Population Signal: {:.2}% rare alleles ",
            self.brain.chr.0,
            rare_frac * 100.0
        );

        // LD decay as population history proxy
        let strong_ld = self.brain.synapses.iter().filter(|s| s.ld_r2 > 0.7).count();
        explanation.push_str(&format!(
            "with {} strong LD edges. ",
            strong_ld
        ));

        if strong_ld > 100 {
            explanation.push_str("High LD density suggests recent population bottleneck.");
        } else if strong_ld > 20 {
            explanation.push_str("Moderate LD density indicates stable population.");
        } else {
            explanation.push_str("Low LD density suggests high recombination.");
        }

        let mut signal = [0.0f32; 32];
        signal[0] = score;
        signal[1] = n_rare as f32;
        signal[2] = strong_ld as f32;

        AgentResponse {
            query_type: "PopulationSignal".to_string(),
            explanation,
            score,
            affected_snps: (0..self.brain.neurons.len() as u32)
                .filter(|idx| self.brain.neurons[*idx as usize].is_rare)
                .collect(),
            affected_blocks: vec![],
            signal_vector: signal,
        }
    }

    fn infer_evolution_hint(&self) -> AgentResponse {
        let mut explanation = format!("Chr{} Evolution Hint: ", self.brain.chr.0);
        let mut score = 0.1f32;

        // High LD in clusters suggests recent sweeps
        let mut ld_clusters = 0;
        for block in &self.brain.blocks {
            if block.mean_r_squared > 0.6 && block.snp_indices.len() > 3 {
                ld_clusters += 1;
                score += 0.05;
            }
        }

        if ld_clusters > 5 {
            explanation.push_str("Multiple strong LD blocks detected: likely targets of recent selection.");
            score = score.min(1.0);
        } else if ld_clusters > 1 {
            explanation.push_str("Moderate LD clustering: possible historical selection pressures.");
        } else {
            explanation.push_str("Weak LD clustering: suggests neutral evolution dominates.");
        }

        let mut signal = [0.0f32; 32];
        signal[0] = score;
        signal[1] = ld_clusters as f32;
        signal[2] = self.brain.kairos_state.convergence_score;

        AgentResponse {
            query_type: "EvolutionHint".to_string(),
            explanation,
            score,
            affected_snps: vec![],
            affected_blocks: (0..ld_clusters).collect(),
            signal_vector: signal,
        }
    }
}

/// Multi-chromosome agent coordinator
pub struct AgentCoordinator {
    pub agents: Vec<ChromosomeAgent>,
}

#[derive(Clone, Debug)]
pub struct CoordinatorResponse {
    pub per_chromosome: Vec<(ChromosomeId, AgentResponse)>,
    pub summary: String,
    pub fused_signal: [f32; 512],
}

impl AgentCoordinator {
    /// Create coordinator from list of agents
    pub fn new(agents: Vec<ChromosomeAgent>) -> Self {
        Self { agents }
    }

    /// Route query to all agents and fuse results
    pub fn coordinate_query(&self, query: &AgentQuery) -> CoordinatorResponse {
        let mut per_chromosome = Vec::new();
        let mut chr_scores = HashMap::new();

        for agent in &self.agents {
            let response = agent.handle_query(query);
            chr_scores.insert(agent.brain.chr.0, response.score);
            per_chromosome.push((agent.brain.chr, response));
        }

        let summary = self.synthesize_summary(&per_chromosome, query);
        let fused_signal = self.fuse_signals(&per_chromosome);

        CoordinatorResponse {
            per_chromosome,
            summary,
            fused_signal,
        }
    }

    fn synthesize_summary(
        &self,
        responses: &[(ChromosomeId, AgentResponse)],
        query: &AgentQuery,
    ) -> String {
        let query_type = match query {
            AgentQuery::DiseaseRisk { .. } => "Disease Risk",
            AgentQuery::TraitPattern { .. } => "Trait Pattern",
            AgentQuery::PopulationSignal => "Population Signal",
            AgentQuery::EvolutionHint => "Evolution Hint",
        };

        let avg_score = responses
            .iter()
            .map(|(_, r)| r.score)
            .sum::<f32>()
            / (responses.len() as f32 + 1e-6);

        let significant_chrs: Vec<_> = responses
            .iter()
            .filter(|(_, r)| r.score > 0.2)
            .map(|(chr, _)| chr.0)
            .collect();

        format!(
            "{} Analysis across {} chromosomes. Average signal: {:.3}. Significant contributors: Chr{}.",
            query_type,
            responses.len(),
            avg_score,
            if significant_chrs.is_empty() {
                "none".to_string()
            } else {
                significant_chrs
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }
        )
    }

    fn fuse_signals(&self, responses: &[(ChromosomeId, AgentResponse)]) -> [f32; 512] {
        let mut fused = [0.0f32; 512];

        for (chr_idx, (_, response)) in responses.iter().enumerate() {
            if chr_idx >= 16 {
                break; // Max 16 chromosomes, 32 dims each
            }
            let offset = chr_idx * 32;
            for i in 0..32 {
                if i < 32 && offset + i < 512 {
                    fused[offset + i] = response.signal_vector[i];
                }
            }
        }

        fused
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::chromosome_brain::{ChromosomeBrain, KairosState, EmbeddingLayer};

    #[test]
    fn test_agent_creation() {
        let brain = ChromosomeBrain {
            chr: ChromosomeId(1),
            neurons: vec![],
            synapses: vec![],
            blocks: vec![],
            embeddings: EmbeddingLayer {
                snp_embeddings: vec![],
                block_embeddings: vec![],
                consolidated: vec![],
            },
            training_cycles: 0,
            kairos_state: KairosState::default(),
        };

        let agent = ChromosomeAgent::new(brain);
        assert_eq!(agent.capabilities.len(), 4);
    }

    #[test]
    fn test_coordinator() {
        let brain = ChromosomeBrain {
            chr: ChromosomeId(1),
            neurons: vec![],
            synapses: vec![],
            blocks: vec![],
            embeddings: EmbeddingLayer {
                snp_embeddings: vec![],
                block_embeddings: vec![],
                consolidated: vec![],
            },
            training_cycles: 0,
            kairos_state: KairosState::default(),
        };

        let agent = ChromosomeAgent::new(brain);
        let coordinator = AgentCoordinator::new(vec![agent]);

        let query = AgentQuery::PopulationSignal;
        let response = coordinator.coordinate_query(&query);

        assert!(!response.summary.is_empty());
        assert_eq!(response.per_chromosome.len(), 1);
    }
}
