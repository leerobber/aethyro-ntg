/// Phase C: Evolution Simulation
/// Fitness ranking, selection, and generational advancement
/// Pure Rust implementation

use crate::genomic::synthesis::Genome;

/// Fitness model trait: defines how to evaluate genome fitness
pub trait FitnessModel {
    fn evaluate(&self, genome: &Genome) -> f32;
}

/// Default fitness model: trait heritability simulator
pub struct DefaultFitnessModel {
    pub target_allele_freq: f32,
    pub selection_strength: f32,
}

impl FitnessModel for DefaultFitnessModel {
    fn evaluate(&self, genome: &Genome) -> f32 {
        if genome.genotypes.is_empty() {
            return 0.5;
        }

        let mut mean_freq = 0.0f32;
        for snp_idx in 0..genome.genotypes.len() {
            mean_freq += genome.allele_freq(snp_idx);
        }
        mean_freq /= genome.genotypes.len() as f32;

        // Fitness = proximity to target allele frequency
        let distance = (mean_freq - self.target_allele_freq).abs();
        let fitness = 1.0 - (distance * self.selection_strength);
        fitness.max(0.0).min(1.0)
    }
}

/// Evolution simulator: runs generational cycles
pub struct EvolutionSim {
    pub population: Vec<Genome>,
    pub generation: u32,
    pub fitness_model: Box<dyn FitnessModel>,
}

impl EvolutionSim {
    pub fn new(population: Vec<Genome>, fitness_model: Box<dyn FitnessModel>) -> Self {
        Self {
            population,
            generation: 0,
            fitness_model,
        }
    }

    /// Run one generation cycle: evaluate, rank, select, reproduce
    pub fn step(&mut self, sampler: &crate::genomic::synthesis::GenomeSampler) {
        // Phase 1: Evaluation
        for genome in &mut self.population {
            genome.fitness = self.fitness_model.evaluate(genome);
        }

        // Phase 2: Ranking (sort by fitness descending)
        self.population.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Phase 3: Selection (elitism: keep top 25%)
        let elite_count = (self.population.len() / 4).max(1);
        let mut next_generation = self.population[0..elite_count].to_vec();

        // Phase 4: Reproduction (fill population with new samples)
        let pop_id_start = (self.generation * 1000) as u32;
        while next_generation.len() < self.population.len() {
            let new_genome = sampler.sample(pop_id_start + next_generation.len() as u32);
            next_generation.push(new_genome);
        }

        self.population = next_generation;
        self.generation += 1;
    }

    /// Run multiple generations
    pub fn run_generations(
        &mut self,
        n_generations: u32,
        sampler: &crate::genomic::synthesis::GenomeSampler,
    ) {
        for _ in 0..n_generations {
            self.step(sampler);
        }
    }

    /// Get generation statistics
    pub fn stats(&self) -> GenerationStats {
        if self.population.is_empty() {
            return GenerationStats {
                generation: self.generation,
                mean_fitness: 0.0,
                max_fitness: 0.0,
                min_fitness: 0.0,
                elite_count: 0,
            };
        }

        let mean_fitness = self.population.iter().map(|g| g.fitness).sum::<f32>() / self.population.len() as f32;
        let max_fitness = self
            .population
            .iter()
            .map(|g| g.fitness)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_fitness = self
            .population
            .iter()
            .map(|g| g.fitness)
            .fold(f32::INFINITY, f32::min);
        let elite_count = self.population.iter().filter(|g| g.fitness > 0.7).count() as u32;

        GenerationStats {
            generation: self.generation,
            mean_fitness,
            max_fitness,
            min_fitness,
            elite_count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenerationStats {
    pub generation: u32,
    pub mean_fitness: f32,
    pub max_fitness: f32,
    pub min_fitness: f32,
    pub elite_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstantFitness(f32);

    impl FitnessModel for ConstantFitness {
        fn evaluate(&self, _genome: &Genome) -> f32 {
            self.0
        }
    }

    #[test]
    fn test_evolution_sim() {
        let population = vec![
            Genome::new(0, 5, 10),
            Genome::new(1, 5, 10),
            Genome::new(2, 5, 10),
        ];

        let fitness_model = Box::new(ConstantFitness(0.5));
        let sim = EvolutionSim::new(population, fitness_model);

        assert_eq!(sim.generation, 0);
        assert_eq!(sim.population.len(), 3);
    }

    #[test]
    fn test_generation_stats() {
        let mut population = vec![Genome::new(0, 5, 10), Genome::new(1, 5, 10)];
        population[0].fitness = 0.8;
        population[1].fitness = 0.6;

        let fitness_model = Box::new(ConstantFitness(0.7));
        let sim = EvolutionSim::new(population, fitness_model);
        let stats = sim.stats();

        assert_eq!(stats.max_fitness, 0.8);
        assert_eq!(stats.min_fitness, 0.6);
    }
}
