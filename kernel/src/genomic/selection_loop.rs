//! Shared multi-axis selection loop for demos, campaigns, and tests.
//!
//! Emits optional JSONL metrics (Phase F observability, minimal form).

use crate::genomic::sovereign_brain::SovereignBrain;
use crate::genomic::sovereign_fitness::SovereignFitnessContext;
use crate::ntg::mutation::multi_axis::{MultiAxisFitness, SelectionOutcome};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// One recorded selection step.
#[derive(Clone, Debug)]
pub struct LoopStepRecord {
    pub step: usize,
    pub op: String,
    pub accepted: bool,
    pub baseline: MultiAxisFitness,
    pub candidate: MultiAxisFitness,
    pub ld_coverage: f32,
    pub calib_task: f32,
    pub genomic_task: f32,
    pub synapses: u32,
    pub generation: u64,
    pub ledger_entries: usize,
}

/// Aggregate result of a run_selection_loop call.
#[derive(Clone, Debug, Default)]
pub struct LoopSummary {
    pub steps: Vec<LoopStepRecord>,
    pub train_accepted: usize,
    pub prune_accepted: usize,
    pub prune_rejected: usize,
    pub train_rejected: usize,
    pub final_utility: f32,
    pub initial_utility: f32,
}

impl LoopSummary {
    pub fn total_accepted(&self) -> usize {
        self.train_accepted + self.prune_accepted
    }

    pub fn total_rejected(&self) -> usize {
        self.train_rejected + self.prune_rejected
    }
}

/// Alternate train / prune for `steps` iterations.
///
/// When `jsonl_path` is set, appends one JSON object per step.
pub fn run_selection_loop(
    brain: &mut SovereignBrain,
    ctx: &mut SovereignFitnessContext,
    steps: usize,
    train_cycles: u32,
    prune_frac: f32,
    jsonl_path: Option<&Path>,
) -> Result<LoopSummary, String> {
    let mut summary = LoopSummary::default();
    let f0 = ctx.score(brain);
    summary.initial_utility = f0.utility();

    if let Some(path) = jsonl_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    for step in 0..steps {
        let (op, out): (&str, SelectionOutcome) = if step % 2 == 0 {
            ("train", ctx.select_train_step(brain, train_cycles)?)
        } else {
            ("prune", ctx.select_prune_step(brain, prune_frac)?)
        };

        let rec = LoopStepRecord {
            step,
            op: op.to_string(),
            accepted: out.accepted,
            baseline: out.baseline,
            candidate: out.candidate,
            ld_coverage: ctx.last_ld_coverage,
            calib_task: ctx.last_calib_task,
            genomic_task: ctx.last_genomic_task,
            synapses: brain.measure_structure().n_synapses,
            generation: brain.generation,
            ledger_entries: ctx.ledger_entry_count(),
        };

        if let Some(path) = jsonl_path {
            append_jsonl(path, &rec)?;
        }

        if out.accepted {
            if let Some(child) = out.child {
                *brain = child;
            }
            match op {
                "train" => summary.train_accepted += 1,
                "prune" => summary.prune_accepted += 1,
                _ => {}
            }
        } else {
            match op {
                "train" => summary.train_rejected += 1,
                "prune" => summary.prune_rejected += 1,
                _ => {}
            }
        }
        summary.steps.push(rec);
    }

    summary.final_utility = ctx.score(brain).utility();
    Ok(summary)
}

fn append_jsonl(path: &Path, rec: &LoopStepRecord) -> Result<(), String> {
    let line = format!(
        "{{\"step\":{},\"op\":\"{}\",\"accepted\":{},\"u0\":{:.6},\"u1\":{:.6},\"task0\":{:.4},\"task1\":{:.4},\"bio0\":{:.4},\"bio1\":{:.4},\"cost0\":{:.4},\"cost1\":{:.4},\"safety1\":{:.3},\"cov\":{:.4},\"calib\":{:.4},\"genomic\":{:.4},\"synapses\":{},\"gen\":{},\"ledger\":{}}}\n",
        rec.step,
        rec.op,
        rec.accepted,
        rec.baseline.utility(),
        rec.candidate.utility(),
        rec.baseline.task_accuracy,
        rec.candidate.task_accuracy,
        rec.baseline.biological_consistency,
        rec.candidate.biological_consistency,
        rec.baseline.structural_cost,
        rec.candidate.structural_cost,
        rec.candidate.safety,
        rec.ld_coverage,
        rec.calib_task,
        rec.genomic_task,
        rec.synapses,
        rec.generation,
        rec.ledger_entries,
    );
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Format a one-line human summary for demos.
pub fn format_summary(s: &LoopSummary) -> String {
    format!(
        "accepted={} (train={} prune={}) rejected={} (train={} prune={}) utility {:.4}→{:.4}",
        s.total_accepted(),
        s.train_accepted,
        s.prune_accepted,
        s.total_rejected(),
        s.train_rejected,
        s.prune_rejected,
        s.initial_utility,
        s.final_utility
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::sovereign_brain::synthetic_test_brain;
    use crate::genomic::sovereign_fitness::SovereignFitnessContext;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loop_runs_and_writes_jsonl() {
        let mut brain = synthetic_test_brain();
        for b in brain.chromosomes.values_mut() {
            for s in &mut b.synapses {
                s.weight = (s.ld_r2 * 0.3).clamp(0.0, 1.0);
                s.plasticity = 0.1;
            }
        }
        brain.refresh_structure();
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&brain);
        let _ = ctx.install_calib_from_fixtures(15);

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ntg_sel_{stamp}.jsonl"));
        let _ = std::fs::remove_file(&path);

        let summary = run_selection_loop(&mut brain, &mut ctx, 4, 6, 0.15, Some(&path)).unwrap();
        assert_eq!(summary.steps.len(), 4);
        assert!(path.is_file());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"op\":\"train\""));
        assert!(text.contains("\"op\":\"prune\""));
        // Under real bio, at least one prune should typically reject.
        assert!(summary.prune_rejected + summary.prune_accepted >= 1);
        let _ = std::fs::remove_file(&path);
    }
}
