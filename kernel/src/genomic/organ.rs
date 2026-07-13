//! Shared organ interface for Sovereign multi-structure stack (ADR 0009).
//!
//! Every structure type (genomic chromosome map, language/SIS, future code
//! graphs) should expose the same lifecycle so selection and persistence
//! do not grow one-off APIs.

use crate::genomic::language_organ::LanguageOrgan;
use crate::genomic::sovereign_brain::SovereignBrain;

/// Lifecycle operations shared by cognitive organs.
pub trait Organ {
    /// Human-readable organ kind (e.g. "sovereign_genomic", "language_sis").
    fn kind(&self) -> &'static str;

    /// Approximate live footprint in bytes (honest estimate, not RSS).
    fn approx_memory_bytes(&self) -> u64;

    /// Fingerprint of structural identity for ledger / cache keys.
    fn structure_fingerprint(&self) -> u64;

    /// How many addressable units (neurons, nodes, …) are live.
    fn unit_count(&self) -> usize;
}

impl Organ for SovereignBrain {
    fn kind(&self) -> &'static str {
        "sovereign_genomic"
    }

    fn approx_memory_bytes(&self) -> u64 {
        self.measure_structure().approx_memory_bytes
    }

    fn structure_fingerprint(&self) -> u64 {
        let s = self.measure_structure();
        let mut h = 0xcbf29ce484222325u64;
        for x in [
            s.n_chromosomes as u64,
            s.n_neurons as u64,
            s.n_synapses as u64,
            s.n_blocks as u64,
            s.n_ltm_motifs as u64,
            s.approx_memory_bytes,
            (s.mean_synapse_weight * 1_000_000.0) as u64,
            self.generation,
        ] {
            h ^= x.wrapping_mul(0x100000001b3);
            h = h.rotate_left(13);
        }
        h
    }

    fn unit_count(&self) -> usize {
        self.measure_structure().n_neurons as usize
    }
}

impl Organ for LanguageOrgan {
    fn kind(&self) -> &'static str {
        "language_sis"
    }

    fn approx_memory_bytes(&self) -> u64 {
        // Rough: nodes * 128B + model weights if present.
        let nodes = self.node_count() as u64 * 128;
        let model = self
            .model
            .as_ref()
            .map(|m| m.weights.len() as u64)
            .unwrap_or(0);
        nodes + model
    }

    fn structure_fingerprint(&self) -> u64 {
        let mut h = 0x811c9dc5u64;
        h ^= self.node_count() as u64;
        h = h.rotate_left(7);
        h ^= self.docs_ingested as u64;
        h = h.rotate_left(7);
        h ^= self.execution_node_count() as u64;
        if let Some(m) = &self.model {
            h ^= m.threshold as u64;
            h ^= m.nonzero_count() as u64;
        }
        h
    }

    fn unit_count(&self) -> usize {
        self.node_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::language_organ::fixture_docs;
    use crate::genomic::sovereign_brain::synthetic_test_brain;

    #[test]
    fn organ_impls_report_kind() {
        let brain = synthetic_test_brain();
        assert_eq!(brain.kind(), "sovereign_genomic");
        assert!(brain.unit_count() > 0);
        assert!(brain.structure_fingerprint() != 0);

        let mut lang = LanguageOrgan::new();
        lang.ingest_documents(&fixture_docs());
        assert_eq!(lang.kind(), "language_sis");
        assert!(lang.unit_count() > 0);
    }
}
