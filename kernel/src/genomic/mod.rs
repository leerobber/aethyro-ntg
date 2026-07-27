//! Genomic data processing modules
//! Phase A: VCF → LD → Blocks
//! Phase B: Chromosome Brain Architecture + Domain-Agnostic Disease Detection
//! Phase C: Genome Synthesis & Evolution with G×E
//! Phase D: Quality Control & Validation
//! Phase E: Extended Validation (all 22 chromosomes, multi-population, locus power)
//! Report Generation: Pure Rust CSV/JSON/HTML generation

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
pub mod real_pipeline;
pub mod epigenetic_engine;
pub mod optimized_core;
pub mod sovereign_brain;
pub mod sovereign_fitness;
pub mod language_organ;
pub mod organ;
pub mod selection_loop;
pub mod sovereign_persist;
pub mod vitascale;

pub use bitsliced_genotypes::BitstreamGenotypes;
pub use vcf_stream::{VcfParser, VcfChromosome, SnpRecord};
pub use ld_compute::{LdComputer, LdMatrix, LdPair};
pub use haplotype_blocks::{BlockDetector, HaplotypeBlock, BlockStatistics, compute_block_statistics};
pub use chromosome_brain::{ChromosomeBrain, ChromosomeId, NeuronId, Synapse, GenomicNeuron, KairosState, BrainSummary, EmbeddingLayer, init_chromosome_brain};
pub use agents::{ChromosomeAgent, AgentQuery, AgentResponse, AgentCoordinator, CoordinatorResponse};
pub use domain_agents::{DomainAgent, DomainType, DomainQuery, DiseaseDiagnosis, PatternSignature, RiskSeverity};
pub use report_gen::{TestResults, DomainResult, AggregateStats};
pub use synthesis::{Genome, GenomeSampler, HaplotypePool};
pub use evolution::{EvolutionSim, FitnessModel, DefaultFitnessModel, GenerationStats};
pub use phenotype::{PhenotypeHead, Environment, GxEEngine};
pub use quality_control::{GenomeValidator, LocusStats, PopulationStats, QCMetrics};
pub use validation::{GenomeComparator, ReferenceGenome, SyntheticGenome, ValidationResults, PowerAnalysis};
pub use extended_validation::{
    Population, MultiPopulationReference, ChromosomeValidation, GenomeWideValidation,
    LocusPower, LocusPowerAnalyzer, ExtendedValidationReport,
    RecombinationMap, RecombinationComparator, RecombinationComparison,
    HaplotypeBlockComparator, HaplotypeBlockComparison,
};
pub use real_pipeline::{RealChromosomeData, build_real_chromosome, snp_key};
pub use epigenetic_engine::{EpigeneticEngine, GeneticExpressionBlock, StrategyFn};
pub use optimized_core::{
    SovereignEpigeneticEngine, GeneTable, GeneIndex, MAX_GENES, FitnessBenchmark,
    SelectionOutcome, Telemetry4D, kimura_fixation_probability,
};
pub use sovereign_brain::{
    SovereignBrain, WorkingSet, LtmMotif, LtmStats, GlobalNeuronRef, StructuralMetrics,
    ConsolidateReport,
};
pub use sovereign_fitness::{
    SovereignFitnessContext, reference_from_brain, synthetic_from_brain, ld_coverage,
};
pub use language_organ::{LanguageOrgan, fixture_docs};
pub use organ::Organ;
pub use selection_loop::{run_selection_loop, format_summary, LoopSummary, LoopStepRecord};
pub use sovereign_persist::{save_snapshot, load_snapshot_into, SnapshotReport};
pub use vitascale::{
    Kairos, KairosReport, NurseryGenomeSpec, NeonateCareReport, LifeStage, LifeCourse,
    StagePermissions, StageGateResult, DevelopmentalJournalEntry, Pulsewire, PulseEvent,
    VitalMeters, PulseHandles, Guardian, DisciplineEthos, BirthImprint, FIRST_WORDS,
    GUARDIAN_NAME, GUARDIAN_ROLE, TrajectoryCharter, TrajectoryPillar,
};
