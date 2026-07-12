/// Phase C: Phenotype & GxE (Genotype × Environment)
/// Trait prediction and gene-environment interactions
/// Pure Rust implementation

use crate::genomic::synthesis::Genome;
use std::collections::HashMap;

/// Phenotype head: predicts trait from genomic embeddings
#[derive(Clone, Debug)]
pub struct PhenotypeHead {
    pub trait_name: String,
    pub weights: Vec<f32>,
    pub bias: f32,
}

impl PhenotypeHead {
    pub fn new(trait_name: String, n_features: usize) -> Self {
        Self {
            trait_name,
            weights: vec![0.1f32; n_features],
            bias: 0.5f32,
        }
    }

    /// Predict trait from feature vector
    pub fn predict(&self, features: &[f32]) -> f32 {
        let mut output = self.bias;
        for i in 0..features.len().min(self.weights.len()) {
            output += features[i] * self.weights[i];
        }
        output.max(0.0).min(1.0) // Clamp to [0, 1]
    }
}

/// Environment features affecting phenotype
#[derive(Clone, Debug)]
pub struct Environment {
    pub features: Vec<f32>,
}

impl Environment {
    pub fn new(n_features: usize) -> Self {
        Self {
            features: vec![0.5f32; n_features],
        }
    }

    /// Standard environment (neutral)
    pub fn default_env() -> Self {
        Self {
            features: vec![0.5, 0.5, 0.5],
        }
    }

    /// Stress environment
    pub fn stress_env() -> Self {
        Self {
            features: vec![0.8, 0.2, 0.3],
        }
    }

    /// Rich environment
    pub fn rich_env() -> Self {
        Self {
            features: vec![0.3, 0.8, 0.7],
        }
    }
}

/// Gene × Environment interaction engine
#[derive(Clone, Debug)]
pub struct GxEEngine {
    pub phenotype_heads: HashMap<String, PhenotypeHead>,
    pub env_weights: Vec<f32>,
}

impl GxEEngine {
    pub fn new() -> Self {
        Self {
            phenotype_heads: HashMap::new(),
            env_weights: vec![0.1, 0.05, 0.02], // Decreasing env effect
        }
    }

    /// Register a phenotype head
    pub fn register_trait(&mut self, head: PhenotypeHead) {
        self.phenotype_heads.insert(head.trait_name.clone(), head);
    }

    /// Predict trait under genotype + environment
    pub fn predict_trait(
        &self,
        trait_name: &str,
        genotype_features: &[f32],
        environment: &Environment,
    ) -> f32 {
        if let Some(head) = self.phenotype_heads.get(trait_name) {
            let genetic_component = head.predict(genotype_features);

            // Environmental contribution
            let mut env_component = 0.0f32;
            for i in 0..environment.features.len().min(self.env_weights.len()) {
                env_component += environment.features[i] * self.env_weights[i];
            }

            // G×E interaction (multiplicative term)
            let interaction = genetic_component * env_component * 0.1;

            (genetic_component + env_component + interaction).max(0.0).min(1.0)
        } else {
            0.5 // Default if trait not found
        }
    }

    /// Compute all phenotypes for a genome
    pub fn compute_phenotypes(
        &self,
        genome: &Genome,
        environment: &Environment,
    ) -> HashMap<String, f32> {
        let mut phenotypes = HashMap::new();

        // Extract genotype features (allele frequencies)
        let mut features = Vec::new();
        for snp_idx in 0..genome.genotypes.len().min(512) {
            features.push(genome.allele_freq(snp_idx));
        }

        // Predict all traits
        for trait_name in self.phenotype_heads.keys() {
            let value = self.predict_trait(trait_name, &features, environment);
            phenotypes.insert(trait_name.clone(), value);
        }

        phenotypes
    }
}

impl Default for GxEEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phenotype_head() {
        let head = PhenotypeHead::new("Height".to_string(), 5);
        let features = vec![0.5, 0.6, 0.4, 0.7, 0.3];
        let prediction = head.predict(&features);
        assert!(prediction >= 0.0 && prediction <= 1.0);
    }

    #[test]
    fn test_environment() {
        let env = Environment::default_env();
        assert_eq!(env.features.len(), 3);

        let stress = Environment::stress_env();
        let rich = Environment::rich_env();

        assert!(stress.features[0] > rich.features[0]); // Stress higher on first dim
    }

    #[test]
    fn test_gxe_engine() {
        let mut engine = GxEEngine::new();
        let head = PhenotypeHead::new("Cognition".to_string(), 5);
        engine.register_trait(head);

        let features = vec![0.5, 0.5, 0.5, 0.5, 0.5];
        let env = Environment::default_env();

        let prediction = engine.predict_trait("Cognition", &features, &env);
        assert!(prediction >= 0.0 && prediction <= 1.0);
    }

    #[test]
    fn test_gxe_interaction() {
        let mut engine = GxEEngine::new();
        let head = PhenotypeHead::new("Trait".to_string(), 3);
        engine.register_trait(head);

        let features = vec![0.5, 0.5, 0.5];

        // Same genetics, different environments
        let default_env = Environment::default_env();
        let stress_env = Environment::stress_env();

        let pred_default = engine.predict_trait("Trait", &features, &default_env);
        let pred_stress = engine.predict_trait("Trait", &features, &stress_env);

        // Stress should produce different phenotype
        assert_ne!(
            (pred_stress * 1000.0).floor(),
            (pred_default * 1000.0).floor()
        );
    }
}
