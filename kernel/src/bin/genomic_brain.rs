/// GenomicBrain: Bio-inspired neural architecture from chromosome LD patterns
/// Learns memory techniques from genetic diversity

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::Instant;

/// Core neuron: represents a genetic variant (SNP)
#[derive(Clone, Debug)]
struct GenomicNeuron {
    snp_id: String,
    position: u32,
    activation: f64,        // Current activation level
    memory_strength: f64,   // Synaptic weight
    connections: Vec<usize>, // Connected neuron indices
}

/// Memory module: represents a haplotype block
#[derive(Clone, Debug)]
struct MemoryModule {
    block_id: String,
    neurons: Vec<usize>,    // Indices of neurons in this block
    coherence: f64,         // How tightly linked (from LD)
    context: String,        // Population context
}

/// The GenomicBrain: learns from chromosome LD patterns
#[derive(Debug)]
pub struct GenomicBrain {
    neurons: Vec<GenomicNeuron>,
    modules: Vec<MemoryModule>,
    synapses: HashMap<(usize, usize), f64>, // (neuron_i, neuron_j) -> weight
    learning_rate: f64,
    chromosome: String,
    checkpoint_path: String,
}

impl GenomicBrain {
    /// Create new brain for a chromosome
    pub fn new(chromosome: &str, num_neurons: usize) -> Self {
        println!("[GenomicBrain] Initializing {} neurons for {}", num_neurons, chromosome);

        let neurons = (0..num_neurons)
            .map(|i| GenomicNeuron {
                snp_id: format!("snp_{}", i),
                position: (i as u32) * 1000, // Dummy positions
                activation: 0.0,
                memory_strength: 0.0,
                connections: Vec::new(),
            })
            .collect();

        Self {
            neurons,
            modules: Vec::new(),
            synapses: HashMap::new(),
            learning_rate: 0.01,
            chromosome: chromosome.to_string(),
            checkpoint_path: format!("data/checkpoints/brain_{}.checkpoint", chromosome),
        }
    }

    /// Learn from LD patterns: create synapses from high-LD pairs
    pub fn learn_from_ld(&mut self, ld_pairs: Vec<(usize, usize, f64)>) {
        println!("[GenomicBrain] Learning from {} LD pairs", ld_pairs.len());

        for (i, j, ld_strength) in ld_pairs {
            // Weight = LD strength (r squared)
            self.synapses.insert((i, j), ld_strength);
            self.synapses.insert((j, i), ld_strength);

            // Update neuron connections
            if i < self.neurons.len() {
                self.neurons[i].connections.push(j);
            }
            if j < self.neurons.len() {
                self.neurons[j].connections.push(i);
            }

            // Update memory strength based on LD
            if i < self.neurons.len() {
                self.neurons[i].memory_strength = (self.neurons[i].memory_strength + ld_strength) / 2.0;
            }
        }

        println!("[GenomicBrain] Created {} synaptic connections", self.synapses.len());
    }

    /// Create memory module from haplotype block (contiguous high-LD region)
    pub fn create_memory_module(&mut self, block_id: &str, neuron_indices: Vec<usize>, ld_coherence: f64) {
        let module = MemoryModule {
            block_id: block_id.to_string(),
            neurons: neuron_indices.clone(),
            coherence: ld_coherence,
            context: self.chromosome.clone(),
        };

        self.modules.push(module);
        println!("[GenomicBrain] Created memory module {} with {} neurons (coherence: {:.3})",
                 block_id, neuron_indices.len(), ld_coherence);
    }

    /// Spread activation through LD-based synapses
    pub fn activate(&mut self, neuron_idx: usize, strength: f64) {
        if neuron_idx >= self.neurons.len() {
            return;
        }

        self.neurons[neuron_idx].activation = strength;

        // Spread to connected neurons via synapses
        let connections = self.neurons[neuron_idx].connections.clone();
        for connected_idx in connections {
            if let Some(&weight) = self.synapses.get(&(neuron_idx, connected_idx)) {
                let spread = strength * weight * self.learning_rate;
                self.neurons[connected_idx].activation += spread;
            }
        }
    }

    /// Recall: retrieve correlated memories (high-LD neurons)
    pub fn recall(&self, neuron_idx: usize) -> Vec<(String, f64)> {
        if neuron_idx >= self.neurons.len() {
            return Vec::new();
        }

        let mut recalled = Vec::new();
        let neuron = &self.neurons[neuron_idx];

        // Retrieve connected neurons with weights
        for &connected_idx in &neuron.connections {
            if let Some(&weight) = self.synapses.get(&(neuron_idx, connected_idx)) {
                if weight > 0.5 {  // Only high-LD connections (r² > 0.5)
                    recalled.push((
                        self.neurons[connected_idx].snp_id.clone(),
                        weight,
                    ));
                }
            }
        }

        // Sort by strength (descending)
        recalled.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recalled
    }

    /// Save checkpoint
    pub fn checkpoint(&self) -> std::io::Result<()> {
        println!("[GenomicBrain] Saving checkpoint to {}", self.checkpoint_path);

        let mut file = File::create(&self.checkpoint_path)?;
        writeln!(file, "GenomicBrain Checkpoint")?;
        writeln!(file, "Chromosome: {}", self.chromosome)?;
        writeln!(file, "Neurons: {}", self.neurons.len())?;
        writeln!(file, "Synapses: {}", self.synapses.len())?;
        writeln!(file, "Modules: {}", self.modules.len())?;

        Ok(())
    }

    /// Summary statistics
    pub fn summary(&self) {
        println!("\n[GenomicBrain] Summary for {}", self.chromosome);
        println!("  Neurons: {}", self.neurons.len());
        println!("  Synapses: {}", self.synapses.len());
        println!("  Memory modules: {}", self.modules.len());

        let avg_connections = self.neurons.iter()
            .map(|n| n.connections.len() as f64)
            .sum::<f64>() / self.neurons.len() as f64;
        println!("  Avg connections/neuron: {:.2}", avg_connections);

        let avg_weight = self.synapses.values()
            .sum::<f64>() / self.synapses.len() as f64;
        println!("  Avg synaptic weight: {:.3}", avg_weight);
    }
}

fn main() {
    println!("\n{}", "=".repeat(64));
    println!("GenomicBrain v1.0 - Bio-inspired Neural Architecture");
    println!("{}", "=".repeat(64));

    let start = Instant::now();

    // Create brain for chromosome 1 (placeholder: 100 neurons for demo)
    let mut brain = GenomicBrain::new("chr1", 100);

    // Simulate LD patterns (normally loaded from computed LD matrix)
    let mut demo_ld_pairs = Vec::new();
    for i in 0..50 {
        for j in (i + 1)..60.min(i + 10) {
            // Simulate LD strength decreasing with distance
            let distance = (j - i) as f64;
            let ld_strength = (1.0 / (1.0 + distance * 0.1)).max(0.1);
            demo_ld_pairs.push((i, j, ld_strength));
        }
    }

    // Learn from simulated LD patterns
    brain.learn_from_ld(demo_ld_pairs);

    // Create a memory module (haplotype block)
    brain.create_memory_module("block_001", vec![0, 1, 2, 3, 4], 0.92);

    // Demonstrate recall
    println!("\n[Demo] Recalling from neuron 0:");
    let recalled = brain.recall(0);
    for (snp, weight) in recalled.iter().take(5) {
        println!("  {} (weight: {:.3})", snp, weight);
    }

    // Summary
    brain.summary();

    // Checkpoint
    let _ = brain.checkpoint();

    let elapsed = start.elapsed().as_secs_f64();
    println!("\n[OK] GenomicBrain initialized in {:.2}s", elapsed);
    println!("     Ready for chr1 LD data\n");
}
