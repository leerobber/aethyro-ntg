//! Real multi-axis scorers for SovereignBrain (next step after Rung 1–2 proxies).
//!
//! Replaces structure-only proxies with:
//! - **Biology** — Phase D [`GenomeComparator`] vs frozen reference panels
//!   (allele-frequency fidelity + LD-pair coverage after structural mutate)
//! - **Task** — [`ChromosomeAgent`] queries (disease risk + population signal)
//! - **Safety** — [`TamperEvidentLedger`] verify + per-decision mutation log
//!
//! Proxies in `ntg::mutation::multi_axis` remain available for unit micro-tests;
//! production selection should use [`SovereignFitnessContext`].

use crate::genomic::chromosome_brain::ChromosomeBrain;
use crate::genomic::language_organ::{fixture_docs, LanguageOrgan};
use crate::genomic::organ::Organ;
use crate::genomic::real_pipeline::{snp_key, RealChromosomeData};
use crate::genomic::sovereign_brain::SovereignBrain;
use crate::genomic::validation::{
    GenomeComparator, ReferenceGenome, SyntheticGenome, ValidationResults,
};
use crate::ntg::calib::{samples_from_documents, CalibModel, Sample};
use crate::ntg::ledger::replay::ExecutionTrace;
use crate::ntg::ledger::{
    FitnessMeasure, MutationOutcome, TamperEvidentLedger,
};
use crate::ntg::mutation::multi_axis::{
    MultiAxisEvaluator, MultiAxisFitness, SelectionOutcome,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Holds frozen biological references + ledger for real axis scoring.
pub struct SovereignFitnessContext {
    /// Per-chromosome reference panels (allele freqs + LD at ingest time).
    pub references: BTreeMap<u8, ReferenceGenome>,
    /// Tamper-evident decision log (ADR 0002 rail 5).
    pub ledger: TamperEvidentLedger,
    /// Evaluator thresholds.
    pub evaluator: MultiAxisEvaluator,
    /// Last biology validation details (for demos / experiments).
    pub last_validation: BTreeMap<u8, ValidationResults>,
    /// Last biology coverage fraction (shared LD pairs / reference LD pairs).
    pub last_ld_coverage: f32,
    /// Phase 4 calib model for language task axis (optional).
    pub calib_model: Option<CalibModel>,
    /// Holdout samples for calib task scoring.
    pub calib_holdout: Vec<Sample>,
    /// Last calib balanced accuracy component of task score.
    pub last_calib_task: f32,
    /// Last genomic agent component of task score.
    pub last_genomic_task: f32,
}

impl SovereignFitnessContext {
    /// Empty context with a fresh in-memory ledger.
    pub fn new() -> Result<Self, String> {
        let ledger = TamperEvidentLedger::new(None).map_err(|e| e.to_string())?;
        Ok(Self {
            references: BTreeMap::new(),
            ledger,
            // bio_slack 0.08: allow mild LD-coverage loss when structure/task improve.
            evaluator: MultiAxisEvaluator::new(0.001, 0.08),
            last_validation: BTreeMap::new(),
            last_ld_coverage: 0.0,
            calib_model: None,
            calib_holdout: Vec::new(),
            last_calib_task: 0.0,
            last_genomic_task: 0.0,
        })
    }

    /// Install Phase 4 calib model + holdout for the language task axis.
    pub fn install_calib(&mut self, model: CalibModel, holdout: Vec<Sample>) {
        self.calib_model = Some(model);
        self.calib_holdout = holdout;
    }

    /// Train calib on fixtures; keep a tail slice as holdout for task scoring.
    pub fn install_calib_from_fixtures(&mut self, epochs: usize) -> Result<f32, String> {
        let docs = fixture_docs();
        let samples = samples_from_documents(&docs).map_err(|e| e.to_string())?;
        if samples.len() < 6 {
            return Err("fixture samples too few".into());
        }
        let split = (samples.len() * 4) / 5;
        let train = samples[..split].to_vec();
        let holdout = samples[split..].to_vec();
        let report = crate::ntg::calib::calibrate(&train, epochs, 0).map_err(|e| e.to_string())?;
        let bal = report.test_metrics.balanced_accuracy;
        self.install_calib(CalibModel::from_report(&report), holdout);
        self.last_calib_task = bal;
        Ok(bal)
    }

    /// Wire calib from an already-trained language organ.
    pub fn install_calib_from_language(&mut self, organ: &LanguageOrgan) -> Result<(), String> {
        let model = organ
            .model
            .clone()
            .ok_or_else(|| "language organ has no calib model".to_string())?;
        let samples =
            crate::ntg::calib::samples_from_graph(&organ.graph).map_err(|e| e.to_string())?;
        let split = samples.len().saturating_mul(4) / 5;
        let holdout = if split < samples.len() {
            samples[split..].to_vec()
        } else {
            samples
        };
        self.install_calib(model, holdout);
        self.last_calib_task = organ.last_test_bal;
        Ok(())
    }

    /// Snapshot a chromosome brain as its own reference (synthetic ingest path).
    pub fn register_brain_as_reference(&mut self, brain: &ChromosomeBrain) {
        let reference = reference_from_brain(brain, format!("snapshot-chr{}", brain.chr.0));
        self.references.insert(brain.chr.0, reference);
    }

    /// Register the real 1000G-style panel from a pipeline payload.
    pub fn register_real_chromosome(&mut self, data: &RealChromosomeData) {
        self.references
            .insert(data.chr.0, data.reference.clone());
    }

    /// Convenience: register every chromosome currently in the sovereign brain.
    pub fn freeze_all_from_brain(&mut self, brain: &SovereignBrain) {
        for b in brain.chromosomes.values() {
            self.register_brain_as_reference(b);
        }
    }

    /// Biology axis: mean overall_similarity × LD coverage vs frozen references.
    ///
    /// Allele frequencies are stable under synapse prune; LD coverage drops as
    /// synapses disappear — that is the intentional biology pressure.
    pub fn score_biology(&mut self, brain: &SovereignBrain) -> f32 {
        if self.references.is_empty() {
            return 0.0;
        }

        let mut sim_sum = 0.0f32;
        let mut cov_sum = 0.0f32;
        let mut n = 0u32;
        self.last_validation.clear();

        for (chr, reference) in &self.references {
            let Some(chr_brain) = brain.chromosome(*chr) else {
                continue;
            };
            let synthetic = synthetic_from_brain(chr_brain);
            let validation = GenomeComparator::validate(reference, &synthetic);
            let coverage = ld_coverage(reference, &synthetic);
            sim_sum += validation.overall_similarity;
            cov_sum += coverage;
            self.last_validation.insert(*chr, validation);
            n += 1;
        }

        if n == 0 {
            self.last_ld_coverage = 0.0;
            return 0.0;
        }
        let mean_sim = sim_sum / n as f32;
        let mean_cov = cov_sum / n as f32;
        self.last_ld_coverage = mean_cov;
        // Equal weight: stay similar *and* retain LD structure.
        (0.5 * mean_sim + 0.5 * mean_cov).clamp(0.0, 1.0)
    }

    /// Genomic agent sub-score (disease risk + population + connectivity).
    /// Uses zero-copy helpers on ChromosomeBrain (no full-brain clone).
    pub fn score_genomic_task(&self, brain: &SovereignBrain) -> f32 {
        if brain.chromosomes.is_empty() {
            return 0.0;
        }
        let mut total = 0.0f32;
        let mut n = 0u32;
        for chr_brain in brain.chromosomes.values() {
            let mut idxs: Vec<(u32, f32)> = chr_brain
                .neurons
                .iter()
                .map(|n| (n.snp_index, n.maf))
                .collect();
            idxs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let rare_first: Vec<u32> = chr_brain
                .neurons
                .iter()
                .filter(|n| n.is_rare)
                .map(|n| n.snp_index)
                .take(8)
                .collect();
            let targets: Vec<u32> = if rare_first.is_empty() {
                idxs.into_iter().map(|(i, _)| i).take(8).collect()
            } else {
                rare_first
            };
            let risk = chr_brain.disease_risk_score(&targets);
            let pop = chr_brain.population_signal_score();
            let connect = chr_brain.connectivity_score();
            total += 0.40 * risk + 0.25 * pop + 0.35 * connect;
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            (total / n as f32).clamp(0.0, 1.0)
        }
    }

    /// Install calib using real engineering docs under `docs_dir` (harder task).
    /// Falls back to fixtures if fewer than 6 samples.
    pub fn install_calib_from_docs_dir(
        &mut self,
        docs_dir: &Path,
        epochs: usize,
    ) -> Result<f32, String> {
        let mut docs: Vec<(String, String)> = Vec::new();
        for name in [
            "ROADMAP.md",
            "STATUS.md",
            "DESIGN.md",
            "EXPERIMENTS.md",
            "LITERATURE.md",
            "PHASE_GATE_PROTOCOL.md",
        ] {
            let p = docs_dir.join(name);
            if let Ok(text) = std::fs::read_to_string(&p) {
                // Cap size so calib stays cheap.
                let clip: String = text.chars().take(12_000).collect();
                docs.push((name.to_string(), clip));
            }
        }
        if docs.len() < 2 {
            return self.install_calib_from_fixtures(epochs);
        }
        let refs: Vec<(&str, &str)> = docs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let samples = samples_from_documents(&refs).map_err(|e| e.to_string())?;
        if samples.len() < 6 {
            return self.install_calib_from_fixtures(epochs);
        }
        let split = (samples.len() * 4) / 5;
        let train = samples[..split].to_vec();
        let holdout = samples[split..].to_vec();
        let report = crate::ntg::calib::calibrate(&train, epochs, 0).map_err(|e| e.to_string())?;
        let bal = report.test_metrics.balanced_accuracy;
        self.install_calib(CalibModel::from_report(&report), holdout);
        self.last_calib_task = bal;
        Ok(bal)
    }

    /// Language/calib sub-score: holdout balanced accuracy when model installed.
    pub fn score_calib_task(&self) -> f32 {
        let Some(model) = &self.calib_model else {
            return 0.0;
        };
        if self.calib_holdout.is_empty() {
            // Fall back to meta test_bal if present.
            if let Some((_, v)) = model.meta.iter().find(|(k, _)| k == "test_bal") {
                return v.parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0);
            }
            return 0.0;
        }
        let mut tp = 0usize;
        let mut tn = 0usize;
        let mut fp = 0usize;
        let mut fn_ = 0usize;
        for s in &self.calib_holdout {
            let pred = model.predict_execution(&s.label_preview);
            match (pred, s.is_execution) {
                (true, true) => tp += 1,
                (false, false) => tn += 1,
                (true, false) => fp += 1,
                (false, true) => fn_ += 1,
            }
        }
        let tpr = if tp + fn_ > 0 {
            tp as f32 / (tp + fn_) as f32
        } else {
            0.0
        };
        let tnr = if tn + fp > 0 {
            tn as f32 / (tn + fp) as f32
        } else {
            0.0
        };
        (0.5 * (tpr + tnr)).clamp(0.0, 1.0)
    }

    /// Task axis: blend genomic agents + Phase 4 calib holdout when available.
    ///
    /// - Calib installed: `0.55 * calib_bal + 0.45 * genomic`
    /// - Else: genomic only
    pub fn score_task(&mut self, brain: &SovereignBrain) -> f32 {
        let genomic = self.score_genomic_task(brain);
        self.last_genomic_task = genomic;
        if self.calib_model.is_some() {
            let calib = self.score_calib_task();
            self.last_calib_task = calib;
            (0.55 * calib + 0.45 * genomic).clamp(0.0, 1.0)
        } else {
            self.last_calib_task = 0.0;
            genomic
        }
    }

    /// Safety axis: 1.0 if full ledger verifies, else 0.0.
    pub fn score_safety(&self) -> f32 {
        if self.ledger.verify_full_ledger().is_ok() {
            1.0
        } else {
            0.0
        }
    }

    /// Full multi-axis score with real axes.
    pub fn score(&mut self, brain: &SovereignBrain) -> MultiAxisFitness {
        let task = self.score_task(brain); // mut: updates last_* task fields
        let bio = self.score_biology(brain);
        let safety = self.score_safety();
        let structure = brain.measure_structure();
        MultiAxisFitness::from_sovereign(&structure, task, bio, safety)
    }

    /// Selection step with an explicit child mutant, real axes, ledger log.
    pub fn select_child(
        &mut self,
        parent: &SovereignBrain,
        child: SovereignBrain,
        op_label: &str,
    ) -> Result<SelectionOutcome, String> {
        let baseline = self.score(parent);
        let candidate = self.score(&child);
        let accepted = self.evaluator.should_accept(&baseline, &candidate);

        let pre_fp = parent.structure_fingerprint();
        let post_fp = child.structure_fingerprint();
        let outcome = if accepted {
            MutationOutcome::Accepted
        } else {
            MutationOutcome::RejectedFitnessGate
        };
        let fitness = FitnessMeasure {
            latency_us: (baseline.structural_cost * 10_000.0) as u64,
            memory_bytes: parent.measure_structure().approx_memory_bytes,
        };
        let mut trace = ExecutionTrace::with_fingerprint(pre_fp);
        // Single ordered event keeps verify_determinism happy (ascending node ids).
        trace.record_event(0, pre_fp, post_fp, now_secs());
        trace.set_output_hash(post_fp);

        let desc = format!(
            "sovereign_{op_label} accepted={} u={:.4}->{:.4} task={:.3}->{:.3} bio={:.3}->{:.3} cov={:.3}",
            accepted,
            baseline.utility(),
            candidate.utility(),
            baseline.task_accuracy,
            candidate.task_accuracy,
            baseline.biological_consistency,
            candidate.biological_consistency,
            self.last_ld_coverage,
        );
        self.ledger
            .log_mutation(
                desc,
                pre_fp,
                post_fp,
                fitness,
                outcome,
                0,
                trace,
                now_secs(),
            )
            .map_err(|e| e.to_string())?;

        // Safety must still hold after logging.
        if self.ledger.verify_full_ledger().is_err() {
            return Err("ledger failed verification after log_mutation".into());
        }

        Ok(SelectionOutcome {
            accepted,
            baseline,
            candidate,
            child: if accepted { Some(child) } else { None },
        })
    }

    /// Prune-weakest mutant under real axes + ledger.
    pub fn select_prune_step(
        &mut self,
        parent: &SovereignBrain,
        prune_frac: f32,
    ) -> Result<SelectionOutcome, String> {
        let child = parent.propose_prune_mutant(prune_frac);
        self.select_child(parent, child, &format!("prune_frac={prune_frac:.3}"))
    }

    /// Extra KAIROS training mutant under real axes + ledger.
    pub fn select_train_step(
        &mut self,
        parent: &SovereignBrain,
        cycles: u32,
    ) -> Result<SelectionOutcome, String> {
        let child = parent.propose_train_mutant(cycles);
        self.select_child(parent, child, &format!("train_cycles={cycles}"))
    }

    pub fn ledger_entry_count(&self) -> usize {
        self.ledger.entries().len()
    }
}

/// Build a Phase D reference panel from a chromosome brain's current state.
pub fn reference_from_brain(brain: &ChromosomeBrain, population: String) -> ReferenceGenome {
    let mut reference = ReferenceGenome::new(population, 0);
    for n in &brain.neurons {
        let freq_alt = n.allele_freq.clamp(0.0, 1.0);
        reference.add_snp(
            snp_key(n.snp_index),
            freq_alt,
            (1.0 - freq_alt).clamp(0.0, 1.0),
        );
    }
    for s in &brain.synapses {
        let Some(from) = brain.neurons.iter().find(|n| n.id == s.from) else {
            continue;
        };
        let Some(to) = brain.neurons.iter().find(|n| n.id == s.to) else {
            continue;
        };
        reference.add_ld_pair(
            snp_key(from.snp_index),
            snp_key(to.snp_index),
            s.ld_r2.clamp(0.0, 1.0),
        );
    }
    reference.finalize();
    reference
}

/// Synthetic view of the *current* brain for comparison to a frozen reference.
pub fn synthetic_from_brain(brain: &ChromosomeBrain) -> SyntheticGenome {
    let mut synthetic = SyntheticGenome::new(0);
    for n in &brain.neurons {
        let freq_alt = n.allele_freq.clamp(0.0, 1.0);
        synthetic.add_snp(
            snp_key(n.snp_index),
            freq_alt,
            (1.0 - freq_alt).clamp(0.0, 1.0),
        );
    }
    for s in &brain.synapses {
        let Some(from) = brain.neurons.iter().find(|n| n.id == s.from) else {
            continue;
        };
        let Some(to) = brain.neurons.iter().find(|n| n.id == s.to) else {
            continue;
        };
        synthetic.add_ld_pair(
            snp_key(from.snp_index),
            snp_key(to.snp_index),
            s.ld_r2.clamp(0.0, 1.0),
        );
    }
    synthetic.finalize();
    synthetic
}

/// r²-weighted fraction of reference LD still present in the brain.
///
/// Dropping a weak (low-r²) synapse costs little; dropping a strong LD edge
/// costs a lot. Matches the prune-weakest mutant operator.
pub fn ld_coverage(reference: &ReferenceGenome, synthetic: &SyntheticGenome) -> f32 {
    if reference.ld_matrix.is_empty() {
        return 1.0;
    }
    // ld_matrix stores both (a,b) and (b,a); use undirected pairs (a <= b).
    let mut ref_weight = 0.0f32;
    let mut shared_weight = 0.0f32;
    for (pair, r2) in &reference.ld_matrix {
        if pair.0 <= pair.1 {
            let w = r2.max(0.0);
            ref_weight += w;
            if synthetic.ld_matrix.contains_key(pair) {
                shared_weight += w;
            }
        }
    }
    if ref_weight <= 1e-9 {
        1.0
    } else {
        (shared_weight / ref_weight).clamp(0.0, 1.0)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::sovereign_brain::synthetic_test_brain;

    #[test]
    fn freeze_and_score_biology_near_one() {
        let brain = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&brain);
        let bio = ctx.score_biology(&brain);
        // Identical to snapshot → high similarity + full coverage.
        assert!(bio > 0.9, "bio={bio}");
        assert!((ctx.last_ld_coverage - 1.0).abs() < 1e-5);
    }

    #[test]
    fn prune_reduces_ld_coverage() {
        let brain = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&brain);
        let before = ctx.score_biology(&brain);
        let child = brain.propose_prune_mutant(0.4);
        let after = ctx.score_biology(&child);
        assert!(ctx.last_ld_coverage < 1.0, "coverage should drop after prune");
        // Biology should not increase when we only destroy structure.
        assert!(after <= before + 0.05, "before={before} after={after}");
    }

    #[test]
    fn task_score_positive_with_structure() {
        let brain = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        let t = ctx.score_task(&brain);
        assert!((0.0..=1.0).contains(&t), "task={t}");
        assert!(t > 0.05, "expected non-trivial task signal, got {t}");
    }

    #[test]
    fn calib_task_blends_into_score() {
        let brain = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&brain);
        let without = ctx.score_task(&brain);
        let bal = ctx.install_calib_from_fixtures(20).unwrap();
        let with = ctx.score_task(&brain);
        assert!(ctx.calib_model.is_some());
        assert!((0.0..=1.0).contains(&bal));
        assert!((0.0..=1.0).contains(&with));
        // With calib, last_calib_task should be set.
        assert!(ctx.last_calib_task >= 0.0);
        assert!(ctx.last_genomic_task > 0.05);
        let _ = without;
    }

    #[test]
    fn safety_starts_ok_and_logs() {
        let parent = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&parent);
        assert!((ctx.score_safety() - 1.0).abs() < 1e-6);
        let out = ctx.select_prune_step(&parent, 0.2).unwrap();
        assert_eq!(ctx.ledger_entry_count(), 1);
        assert!((ctx.score_safety() - 1.0).abs() < 1e-6);
        // Outcome is finite either way.
        assert!(out.baseline.utility().is_finite());
        assert!(out.candidate.utility().is_finite());
    }

    #[test]
    fn selection_loop_preserves_ledger_integrity() {
        let mut brain = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.evaluator = MultiAxisEvaluator::new(0.0005, 0.1);
        ctx.freeze_all_from_brain(&brain);
        for _ in 0..5 {
            let out = ctx.select_prune_step(&brain, 0.15).unwrap();
            if let Some(child) = out.child {
                brain = child;
            }
        }
        assert!(ctx.ledger_entry_count() >= 5);
        assert!(ctx.ledger.verify_full_ledger().is_ok());
        assert!((ctx.score_safety() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn train_mutant_preserves_biology_coverage() {
        let parent = synthetic_test_brain();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&parent);
        let bio0 = ctx.score_biology(&parent);
        let child = parent.propose_train_mutant(10);
        let bio1 = ctx.score_biology(&child);
        assert!(
            (bio1 - bio0).abs() < 0.02,
            "train should preserve biology: {bio0} -> {bio1}"
        );
        assert!((ctx.last_ld_coverage - 1.0).abs() < 1e-5);
    }

    #[test]
    fn train_step_can_accept_under_real_axes() {
        // Under-trained parent: more KAIROS should not hurt biology and may pass gate.
        let mut parent = synthetic_test_brain();
        // Zero out weights so training has headroom.
        for b in parent.chromosomes.values_mut() {
            for s in &mut b.synapses {
                s.weight = 0.1;
            }
        }
        parent.refresh_structure();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.evaluator = MultiAxisEvaluator::new(0.0001, 0.05);
        ctx.freeze_all_from_brain(&parent);
        let out = ctx.select_train_step(&parent, 20).unwrap();
        assert!(out.candidate.biological_consistency + 0.02 >= out.baseline.biological_consistency);
        assert_eq!(ctx.ledger_entry_count(), 1);
        // Acceptance depends on utility delta; either way ledger stays clean.
        assert!(ctx.ledger.verify_full_ledger().is_ok());
    }

    #[test]
    fn reference_from_brain_roundtrip_keys() {
        let brain = synthetic_test_brain();
        let chr = brain.chromosome(1).unwrap();
        let r = reference_from_brain(chr, "t".into());
        assert!(!r.allele_frequencies.is_empty());
        assert!(!r.ld_matrix.is_empty());
        let s = synthetic_from_brain(chr);
        let v = GenomeComparator::validate(&r, &s);
        assert!((v.overall_similarity - 1.0).abs() < 1e-4, "{:?}", v);
    }
}
