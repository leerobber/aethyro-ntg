/// Genomic data processing modules
/// Phase A: VCF → LD → Blocks
/// Phase B: Chromosome Brain Architecture + Domain-Agnostic Disease Detection
/// Phase C: Genome Synthesis & Evolution with G×E
/// Phase D: Quality Control & Validation
/// Phase E: Extended Validation (all 22 chromosomes, multi-population, locus power)
/// Report Generation: Pure Rust CSV/JSON/HTML generation

pub mod bitsliced_genotypes;
pub mod vcf_stream;
pub mod ld_compute;
pub mod haplotype_blocks;
pub mod chromosome_brain;
pub mod agents;
pub mod domain_agents;
pub mod report_gen;
pub mod synthesis;
pub mod evolution;
pub mod phenotype;
pub mod quality_control;
pub mod validation;
pub mod extended_validation;

pub use bitsliced_genotypes::BitstreamGenotypes;
pub use vcf_stream::{VcfParser, VcfChromosome, SnpRecord};
pub use ld_compute::{LdComputer, LdMatrix, LdPair};
pub use haplotype_blocks::{BlockDetector, HaplotypeBlock, BlockStatistics, compute_block_statistics};
pub use chromosome_brain::{ChromosomeBrain, ChromosomeId, NeuronId, Synapse, GenomicNeuron, KairosState, BrainSummary, EmbeddingLayer, init_chromosome_brain};
pub use agents::{ChromosomeAgent, AgentQuery, AgentResponse, AgentCoordinator, CoordinatorResponse};
pub use domain_agents::{DomainAgent, DomainType, DomainQuery, DiseaseDiagnosis, PatternSignature, RiskSeverity};
pub use report_gen::{TestResults, DomainResult, AggregateStats};
pub use synthesis::{Genome, GenomeSampler};
pub use evolution::{EvolutionSim, FitnessModel, DefaultFitnessModel, GenerationStats};
pub use phenotype::{PhenotypeHead, Environment, GxEEngine};
pub use quality_control::{GenomeValidator, LocusStats, PopulationStats, QCMetrics};
pub use validation::{GenomeComparator, ReferenceGenome, SyntheticGenome, ValidationResults, PowerAnalysis};
pub use extended_validation::{
    Population, MultiPopulationReference, ChromosomeValidation, GenomeWideValidation,
    LocusPower, LocusPowerAnalyzer, ExtendedValidationReport,
};
