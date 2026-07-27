//! Domain-Agnostic Agent System
//! Handles genomic diseases, code diseases, data diseases, malware, poisonous injections, etc.
//! Architecture: Patterns + Connectivity + Blocks → Risk Scores

use crate::genomic::chromosome_brain::{ChromosomeBrain, NeuronId};
use std::collections::HashMap;

/// Disease domain classification
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainType {
    /// Genomic diseases (SNPs → risk)
    Genomic,
    /// Code quality diseases (bad patterns, anti-patterns)
    CodeQuality,
    /// Malware signatures (virus, trojan, ransomware patterns)
    Malware,
    /// Data injection diseases (SQL injection, XSS, prompt injection)
    InjectionRisk,
    /// Dependency/supply chain poisoning
    SupplyChain,
    /// Cryptographic weakness
    Cryptographic,
}

/// Risk severity levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskSeverity {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for RiskSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskSeverity::None => write!(f, "NONE"),
            RiskSeverity::Low => write!(f, "LOW"),
            RiskSeverity::Medium => write!(f, "MEDIUM"),
            RiskSeverity::High => write!(f, "HIGH"),
            RiskSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Pattern signature for disease detection
#[derive(Clone, Debug)]
pub struct PatternSignature {
    pub pattern_id: u32,
    pub name: String,
    pub domain: DomainType,
    pub severity: RiskSeverity,
    pub confidence: f32,           // [0, 1] detection confidence
    pub prevalence: f32,           // [0, 1] how common in population
    pub affected_modules: Vec<u32>, // Connected component indices
}

/// Disease diagnosis result
#[derive(Clone, Debug)]
pub struct DiseaseDiagnosis {
    pub domain: DomainType,
    pub primary_risk: RiskSeverity,
    pub risk_score: f32,           // [0, 1]
    pub detected_patterns: Vec<PatternSignature>,
    pub affected_modules: Vec<u32>,
    pub remediation: String,
    pub signal_vector: [f32; 32],
}

/// Extended domain-agnostic query system
#[derive(Clone, Debug, PartialEq)]
pub enum DomainQuery {
    /// Genomic disease risk (SNP indices)
    GenomicRisk { snp_indices: Vec<u32> },
    /// Code quality diseases (bad patterns, anti-patterns)
    CodeDisease { module_ids: Vec<u32> },
    /// Malware detection (file/signature analysis)
    MalwareRisk { signatures: Vec<String> },
    /// Injection vulnerability scan (data flow analysis)
    InjectionVulnerability { entry_points: Vec<u32> },
    /// Supply chain risk (dependency poisoning)
    SupplyChainRisk { dependency_indices: Vec<u32> },
    /// Cryptographic weakness analysis
    CryptoRisk { algorithm_ids: Vec<u32> },
}

/// Domain-agnostic risk assessment engine
pub struct DomainAgent {
    pub brain: ChromosomeBrain,
    pub domain_type: DomainType,
    pub pattern_library: HashMap<String, PatternSignature>,
}

impl DomainAgent {
    /// Create new domain agent from brain
    pub fn new(brain: ChromosomeBrain, domain_type: DomainType) -> Self {
        Self {
            brain,
            domain_type,
            pattern_library: HashMap::new(),
        }
    }

    /// Register pattern signature for detection
    pub fn register_pattern(&mut self, name: String, pattern: PatternSignature) {
        self.pattern_library.insert(name, pattern);
    }

    /// Handle domain-specific query
    pub fn diagnose(&self, query: &DomainQuery) -> DiseaseDiagnosis {
        match query {
            DomainQuery::GenomicRisk { snp_indices } => {
                self.assess_genomic_risk(snp_indices)
            }
            DomainQuery::CodeDisease { module_ids } => {
                self.assess_code_quality(module_ids)
            }
            DomainQuery::MalwareRisk { signatures } => {
                self.assess_malware_risk(signatures)
            }
            DomainQuery::InjectionVulnerability { entry_points } => {
                self.assess_injection_risk(entry_points)
            }
            DomainQuery::SupplyChainRisk { dependency_indices } => {
                self.assess_supply_chain_risk(dependency_indices)
            }
            DomainQuery::CryptoRisk { algorithm_ids } => {
                self.assess_crypto_risk(algorithm_ids)
            }
        }
    }

    /// GENOMIC: SNP-based disease risk
    fn assess_genomic_risk(&self, snp_indices: &[u32]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        for &idx in snp_indices {
            if let Some(neuron) = self.brain.neurons.iter().find(|n| n.snp_index == idx) {
                risk_score += 0.15;

                // Find blocks (modules) containing this pattern
                for (b_idx, block) in self.brain.blocks.iter().enumerate() {
                    if block.snp_indices.contains(&idx) {
                        affected_modules.push(b_idx as u32);
                        risk_score += block.mean_r_squared * 0.05;
                    }
                }

                // Pattern: rare allele = higher disease risk
                if neuron.is_rare {
                    detected_patterns.push(PatternSignature {
                        pattern_id: idx,
                        name: format!("RareSNP_{}", idx),
                        domain: DomainType::Genomic,
                        severity: RiskSeverity::Medium,
                        confidence: 0.8,
                        prevalence: 0.05,
                        affected_modules: affected_modules.clone(),
                    });
                }
            }
        }

        let severity = if risk_score > 0.7 {
            RiskSeverity::Critical
        } else if risk_score > 0.5 {
            RiskSeverity::High
        } else if risk_score > 0.3 {
            RiskSeverity::Medium
        } else if risk_score > 0.1 {
            RiskSeverity::Low
        } else {
            RiskSeverity::None
        };

        let remediation = format!(
            "Genetic counseling recommended. Affected {} blocks. Consider {} screening.",
            affected_modules.len(),
            if severity == RiskSeverity::Critical {
                "immediate medical"
            } else {
                "regular"
            }
        );

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;
        signal[2] = severity as i32 as f32;

        DiseaseDiagnosis {
            domain: DomainType::Genomic,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// CODE QUALITY: Bad patterns, anti-patterns, code smells
    fn assess_code_quality(&self, module_ids: &[u32]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        // Pattern 1: High cyclomatic complexity (proxy: high synapse density)
        let avg_synapses_per_neuron = if self.brain.neurons.is_empty() {
            0.0
        } else {
            self.brain.synapses.len() as f32 / self.brain.neurons.len() as f32
        };

        if avg_synapses_per_neuron > 3.0 {
            risk_score += 0.3;
            detected_patterns.push(PatternSignature {
                pattern_id: 1001,
                name: "HighCyclomaticComplexity".to_string(),
                domain: DomainType::CodeQuality,
                severity: RiskSeverity::Medium,
                confidence: 0.85,
                prevalence: 0.3,
                affected_modules: module_ids.to_vec(),
            });
        }

        // Pattern 2: Large module size (proxy: many neurons per block)
        for block in &self.brain.blocks {
            if block.snp_indices.len() > 50 {
                risk_score += 0.2;
                affected_modules.push(block.id);
                detected_patterns.push(PatternSignature {
                    pattern_id: 1002,
                    name: "LargeModuleSize".to_string(),
                    domain: DomainType::CodeQuality,
                    severity: RiskSeverity::Low,
                    confidence: 0.9,
                    prevalence: 0.2,
                    affected_modules: vec![block.id],
                });
            }
        }

        // Pattern 3: Weak coupling detection (low average LD = low coupling)
        let avg_weight = if self.brain.synapses.is_empty() {
            0.0
        } else {
            self.brain.synapses.iter().map(|s| s.weight).sum::<f32>() / self.brain.synapses.len() as f32
        };

        if avg_weight < 0.2 {
            detected_patterns.push(PatternSignature {
                pattern_id: 1003,
                name: "WeakModuleCoupling".to_string(),
                domain: DomainType::CodeQuality,
                severity: RiskSeverity::Low,
                confidence: 0.7,
                prevalence: 0.4,
                affected_modules: module_ids.to_vec(),
            });
        }

        let severity = if risk_score > 0.6 { RiskSeverity::High }
                      else if risk_score > 0.3 { RiskSeverity::Medium }
                      else { RiskSeverity::Low };

        let remediation = format!(
            "Refactor {} modules. Extract {} methods. Reduce cyclomatic complexity via decomposition.",
            affected_modules.len(),
            (avg_synapses_per_neuron * 5.0) as u32
        );

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;
        signal[2] = avg_synapses_per_neuron;

        DiseaseDiagnosis {
            domain: DomainType::CodeQuality,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// MALWARE: Virus, trojan, ransomware signature detection
    fn assess_malware_risk(&self, signatures: &[String]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        for (sig_idx, sig) in signatures.iter().enumerate() {
            // Heuristic: signature matching via pattern library lookup
            if let Some(pattern) = self.pattern_library.get(sig) {
                risk_score += pattern.confidence * 0.5;
                detected_patterns.push(pattern.clone());
                affected_modules.extend(pattern.affected_modules.clone());
            } else {
                // Generic malware scoring
                let entropy_score = self.compute_entropy(sig);
                if entropy_score > 0.8 {
                    risk_score += 0.4; // High entropy = suspicious
                    detected_patterns.push(PatternSignature {
                        pattern_id: 2000 + sig_idx as u32,
                        name: format!("MalwareSignature_{}", sig_idx),
                        domain: DomainType::Malware,
                        severity: RiskSeverity::High,
                        confidence: entropy_score,
                        prevalence: 0.01,
                        affected_modules: vec![],
                    });
                }
            }
        }

        let severity = if risk_score > 0.8 {
            RiskSeverity::Critical
        } else if risk_score > 0.5 {
            RiskSeverity::High
        } else if risk_score > 0.2 {
            RiskSeverity::Medium
        } else {
            RiskSeverity::Low
        };

        let remediation = if risk_score > 0.5 {
            "CRITICAL: Isolate system immediately. Quarantine {} signatures. Run full antivirus scan.".to_string()
        } else {
            "Monitor {} suspicious signatures. Update signatures and rescan.".to_string()
        };

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;

        DiseaseDiagnosis {
            domain: DomainType::Malware,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// INJECTION: SQL injection, XSS, prompt injection, command injection
    fn assess_injection_risk(&self, entry_points: &[u32]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        for &ep_idx in entry_points {
            // Pattern: entry point with high outgoing synapses = data sink risk
            let outgoing = self
                .brain
                .synapses_for_neuron(NeuronId(ep_idx))
                .iter()
                .filter(|s| s.from == NeuronId(ep_idx))
                .count();

            if outgoing > 5 {
                risk_score += 0.3;
                detected_patterns.push(PatternSignature {
                    pattern_id: 3000 + ep_idx,
                    name: format!("HighOutflowEntry_{}", ep_idx),
                    domain: DomainType::InjectionRisk,
                    severity: RiskSeverity::High,
                    confidence: 0.75,
                    prevalence: 0.15,
                    affected_modules: vec![ep_idx],
                });
            }

            // Pattern: entry point without validation (proxy: isolated neuron)
            if outgoing < 2 && !self.brain.synapses_for_neuron(NeuronId(ep_idx)).is_empty() {
                risk_score += 0.25;
                detected_patterns.push(PatternSignature {
                    pattern_id: 3100 + ep_idx,
                    name: format!("UnvalidatedEntry_{}", ep_idx),
                    domain: DomainType::InjectionRisk,
                    severity: RiskSeverity::High,
                    confidence: 0.8,
                    prevalence: 0.2,
                    affected_modules: vec![ep_idx],
                });
            }
        }

        // Pattern: Chain of sinks (connected high-risk nodes)
        let chains = self.detect_injection_chains(entry_points);
        risk_score += chains.len() as f32 * 0.2;
        affected_modules.extend(chains);

        let severity = if risk_score > 0.8 {
            RiskSeverity::Critical
        } else if risk_score > 0.5 {
            RiskSeverity::High
        } else if risk_score > 0.2 {
            RiskSeverity::Medium
        } else {
            RiskSeverity::Low
        };

        let remediation = format!(
            "Sanitize {} entry points. Add input validation. Implement parameterized queries. {} chains require immediate patching.",
            entry_points.len(),
            affected_modules.len()
        );

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;
        signal[2] = affected_modules.len() as f32;

        DiseaseDiagnosis {
            domain: DomainType::InjectionRisk,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// SUPPLY CHAIN: Dependency poisoning, compromised packages
    fn assess_supply_chain_risk(&self, dependency_indices: &[u32]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        for &dep_idx in dependency_indices {
            // Pattern: high LD = high dependency (tight coupling = supply chain risk)
            let connected_weight: f32 = self
                .brain
                .synapses_for_neuron(NeuronId(dep_idx))
                .iter()
                .map(|s| s.weight)
                .sum();

            if connected_weight > 0.7 {
                risk_score += 0.35;
                affected_modules.push(dep_idx);
                detected_patterns.push(PatternSignature {
                    pattern_id: 4000 + dep_idx,
                    name: format!("TightDependency_{}", dep_idx),
                    domain: DomainType::SupplyChain,
                    severity: RiskSeverity::High,
                    confidence: 0.85,
                    prevalence: 0.1,
                    affected_modules: vec![dep_idx],
                });
            }
        }

        let severity = if risk_score > 0.7 {
            RiskSeverity::Critical
        } else if risk_score > 0.4 {
            RiskSeverity::High
        } else {
            RiskSeverity::Medium
        };

        let remediation = format!(
            "Verify {} dependencies. Pin versions. Implement dependency scanning. {} have tight coupling—decouple or isolate.",
            dependency_indices.len(),
            affected_modules.len()
        );

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;

        DiseaseDiagnosis {
            domain: DomainType::SupplyChain,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// CRYPTOGRAPHIC: Weak algorithms, short keys, deprecated ciphers
    fn assess_crypto_risk(&self, algorithm_ids: &[u32]) -> DiseaseDiagnosis {
        let mut risk_score = 0.0f32;
        let mut detected_patterns = Vec::new();
        let mut affected_modules = Vec::new();

        for &alg_idx in algorithm_ids {
            // Heuristic: weak algorithms marked by low SNP position (old = weak)
            if let Some(neuron) = self.brain.neurons.get(alg_idx as usize) {
                if neuron.position_bp < 1000 {
                    // Proxy for deprecated/weak algorithm
                    risk_score += 0.4;
                    affected_modules.push(alg_idx);
                    detected_patterns.push(PatternSignature {
                        pattern_id: 5000 + alg_idx,
                        name: format!("WeakCryptoAlgorithm_{}", alg_idx),
                        domain: DomainType::Cryptographic,
                        severity: RiskSeverity::High,
                        confidence: 0.8,
                        prevalence: 0.05,
                        affected_modules: vec![alg_idx],
                    });
                }
            }
        }

        let severity = if risk_score > 0.6 {
            RiskSeverity::Critical
        } else if risk_score > 0.3 {
            RiskSeverity::High
        } else {
            RiskSeverity::Medium
        };

        let remediation = format!(
            "Upgrade {} cryptographic algorithms. Use AES-256, SHA-3, or TLS 1.3+. Rotate keys immediately.",
            affected_modules.len()
        );

        let mut signal = [0.0f32; 32];
        signal[0] = risk_score;
        signal[1] = detected_patterns.len() as f32;

        DiseaseDiagnosis {
            domain: DomainType::Cryptographic,
            primary_risk: severity,
            risk_score: risk_score.min(1.0),
            detected_patterns,
            affected_modules,
            remediation,
            signal_vector: signal,
        }
    }

    /// Helper: Detect injection chains (connected vulnerable nodes)
    fn detect_injection_chains(&self, entry_points: &[u32]) -> Vec<u32> {
        let mut chains = Vec::new();
        for &ep in entry_points {
            let reachable = self.bfs_reachable(NeuronId(ep), 5); // 5-hop max
            if reachable.len() > 3 {
                chains.extend(reachable);
            }
        }
        chains.sort();
        chains.dedup();
        chains
    }

    /// BFS reachability analysis
    fn bfs_reachable(&self, start: NeuronId, max_depth: usize) -> Vec<u32> {
        let mut reachable = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start, 0));
        visited.insert(start);

        while let Some((node, depth)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }
            reachable.push(node.0);

            for syn in self.brain.synapses_for_neuron(node) {
                let next = if syn.from == node { syn.to } else { syn.from };
                if !visited.contains(&next) {
                    visited.insert(next);
                    queue.push_back((next, depth + 1));
                }
            }
        }
        reachable
    }

    /// Compute entropy heuristic for signatures
    fn compute_entropy(&self, data: &str) -> f32 {
        let mut freq = [0u32; 256];
        for &byte in data.as_bytes() {
            freq[byte as usize] += 1;
        }

        let len = data.len() as f32;
        let mut entropy = 0.0f32;
        for count in freq.iter() {
            if *count > 0 {
                let p = *count as f32 / len;
                entropy -= p * p.log2();
            }
        }
        entropy / 8.0 // Normalize to [0, 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::chromosome_brain::{
        ChromosomeBrain, ChromosomeId, EmbeddingLayer, KairosState,
    };

    #[test]
    fn test_domain_agent_creation() {
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

        let agent = DomainAgent::new(brain, DomainType::Genomic);
        assert_eq!(agent.domain_type, DomainType::Genomic);
    }

    #[test]
    fn test_risk_severity_ordering() {
        assert!(RiskSeverity::Critical > RiskSeverity::High);
        assert!(RiskSeverity::High > RiskSeverity::Medium);
        assert!(RiskSeverity::Medium > RiskSeverity::Low);
        assert!(RiskSeverity::Low > RiskSeverity::None);
    }
}
