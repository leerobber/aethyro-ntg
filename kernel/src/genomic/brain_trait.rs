//! Unified trait for brain implementations.
//! Enables swapping between ChromosomeBrain, SovereignBrain, and other implementations.

/// Trait for genomic neural networks (brains).
/// Provides a unified interface for different brain architectures.
pub trait Brain: Clone + Send + Sync {
    /// Unique identifier for this brain instance.
    fn id(&self) -> u64;

    /// Current generation/epoch counter.
    fn generation(&self) -> u64;

    /// Measure structure size (neuron count, synapse count).
    fn measure_structure(&self) -> StructureMeasurement;

    /// Score this brain's fitness on a given task (0.0 to 1.0).
    fn score_fitness(&self) -> f32;

    /// Get a human-readable summary of the brain state.
    fn describe(&self) -> BrainDescription;
}

/// Measurements of brain structure complexity.
#[derive(Clone, Debug)]
pub struct StructureMeasurement {
    pub n_neurons: u32,
    pub n_synapses: u32,
    pub n_layers: u32,
}

/// Human-readable brain state description.
#[derive(Clone, Debug)]
pub struct BrainDescription {
    pub brain_type: String,
    pub generation: u64,
    pub neurons: u32,
    pub synapses: u32,
    pub fitness: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockBrain {
        id: u64,
        generation: u64,
        fitness: f32,
    }

    impl Brain for MockBrain {
        fn id(&self) -> u64 {
            self.id
        }

        fn generation(&self) -> u64 {
            self.generation
        }

        fn measure_structure(&self) -> StructureMeasurement {
            StructureMeasurement {
                n_neurons: 100,
                n_synapses: 500,
                n_layers: 3,
            }
        }

        fn score_fitness(&self) -> f32 {
            self.fitness
        }

        fn describe(&self) -> BrainDescription {
            BrainDescription {
                brain_type: "MockBrain".to_string(),
                generation: self.generation,
                neurons: 100,
                synapses: 500,
                fitness: self.fitness,
            }
        }
    }

    #[test]
    fn mock_brain_implements_trait() {
        let brain = MockBrain {
            id: 1,
            generation: 42,
            fitness: 0.85,
        };
        assert_eq!(brain.generation(), 42);
        assert_eq!(brain.score_fitness(), 0.85);
        let structure = brain.measure_structure();
        assert_eq!(structure.n_neurons, 100);
    }
}
