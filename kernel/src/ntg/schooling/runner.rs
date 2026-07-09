//! Campaign runner: study → exam → 75% gate → full redo on fail.

use super::data::{corpus_manifest, SchoolDataRoot};
use super::notebook;
use super::phases;
use super::protocol::{
    CampaignReport, PhaseRunRecord, AggregateStats, DEFAULT_CAMPAIGN_RUNS, DEFAULT_MAX_ATTEMPTS,
    PASS_THRESHOLD,
};
use crate::ntg::error::NtgError;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct SchoolConfig {
    pub docs_dir: std::path::PathBuf,
    pub out_dir: std::path::PathBuf,
    pub max_attempts: u32,
    pub campaign_runs: u32,
    /// If set, only this phase (0..=5); else all.
    pub only_phase: Option<u32>,
}

impl Default for SchoolConfig {
    fn default() -> Self {
        Self {
            docs_dir: std::path::PathBuf::from("../docs"),
            out_dir: std::path::PathBuf::from("../docs/schooling/runs"),
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            campaign_runs: DEFAULT_CAMPAIGN_RUNS,
            only_phase: None,
        }
    }
}

fn run_phase_with_redo(
    root: &SchoolDataRoot,
    phase: u32,
    max_attempts: u32,
) -> Result<PhaseRunRecord, NtgError> {
    let all = root.load_markdown_corpus()?;
    let (train, holdout) = SchoolDataRoot::split_docs(&all, 0.8);
    let dataset_id = format!(
        "docs_corpus_v1_n{}_train{}_hold{}",
        all.len(),
        train.len(),
        holdout.len()
    );
    let dataset_source = format!(
        "real filesystem: {} ({} docs)",
        root.docs_dir.display(),
        all.len()
    );

    let mut last_exam = None;
    let mut last_study = None;
    for attempt in 1..=max_attempts {
        let (study, exam) = match phase {
            0 => {
                let s = phases::study_phase0(root)?;
                let e = phases::exam_phase0(root, attempt)?;
                (s, e)
            }
            1 => {
                let s = phases::study_phase1()?;
                let e = phases::exam_phase1(attempt)?;
                (s, e)
            }
            2 => {
                let s = phases::study_phase2(root, &train)?;
                let e = phases::exam_phase2(root, &holdout, attempt)?;
                (s, e)
            }
            3 => {
                let s = phases::study_phase3()?;
                let e = phases::exam_phase3(attempt)?;
                (s, e)
            }
            4 => {
                let (s, model) = phases::study_phase4(&train)?;
                let e = phases::exam_phase4(&holdout, &model, attempt)?;
                (s, e)
            }
            5 => {
                let (s, model) = phases::study_phase5(&train)?;
                let e = phases::exam_phase5(&holdout, &model, attempt)?;
                (s, e)
            }
            _ => {
                return Err(NtgError::InvalidInput(format!("unknown phase {phase}")));
            }
        };
        last_study = Some(study);
        if exam.passed() {
            return Ok(PhaseRunRecord {
                phase,
                study: last_study.unwrap(),
                exam,
                dataset_id,
                dataset_source,
            });
        }
        last_exam = Some(exam);
        // Full redo: loop continues study+exam
    }
    Ok(PhaseRunRecord {
        phase,
        study: last_study.expect("at least one attempt"),
        exam: last_exam.expect("at least one attempt"),
        dataset_id,
        dataset_source,
    })
}

/// Run one full campaign (phases 0–5 or a single phase).
pub fn run_campaign(cfg: &SchoolConfig, run_id: u32) -> Result<CampaignReport, NtgError> {
    let root = SchoolDataRoot::from_docs_dir(&cfg.docs_dir)?;
    let phases_to_run: Vec<u32> = match cfg.only_phase {
        Some(p) => vec![p],
        None => vec![0, 1, 2, 3, 4, 5],
    };
    let mut records = Vec::new();
    for p in phases_to_run {
        let rec = run_phase_with_redo(&root, p, cfg.max_attempts)?;
        records.push(rec);
    }
    let overall = records.iter().all(|r| r.exam.passed());
    Ok(CampaignReport {
        run_id,
        phases: records,
        overall_pass: overall,
    })
}

/// Multi-run campaign with notebook + JSON + aggregate stats.
pub fn run_schooling_program(cfg: &SchoolConfig) -> Result<String, NtgError> {
    std::fs::create_dir_all(&cfg.out_dir)
        .map_err(|e| NtgError::InvalidInput(format!("mkdir out: {e}")))?;
    let root = SchoolDataRoot::from_docs_dir(&cfg.docs_dir)?;
    let corpus = root.load_markdown_corpus()?;
    let manifest = corpus_manifest(&corpus);

    // Persist dataset manifest (real sources)
    let ds_path = cfg.out_dir.join("DATASET_MANIFEST.md");
    std::fs::write(
        &ds_path,
        format!(
            "# Dataset manifest (real repo docs)\n\n\
             Generated for NTG doctorate schooling.\n\n\
             Source root: `{}`\n\n{}",
            root.docs_dir.display(),
            manifest
        ),
    )
    .map_err(|e| NtgError::InvalidInput(e.to_string()))?;

    let mut campaigns = Vec::new();
    for run in 1..=cfg.campaign_runs {
        let c = run_campaign(cfg, run)?;
        // Per-run notebook
        let nb = notebook::render_campaign_notebook(&c, PASS_THRESHOLD);
        let path = cfg.out_dir.join(format!("RUN_{run:02}_NOTEBOOK.md"));
        std::fs::write(&path, &nb).map_err(|e| NtgError::InvalidInput(e.to_string()))?;
        let json = notebook::campaign_to_json(&c);
        let jpath = cfg.out_dir.join(format!("RUN_{run:02}_results.json"));
        std::fs::write(&jpath, json).map_err(|e| NtgError::InvalidInput(e.to_string()))?;
        campaigns.push(c);
    }

    let aggs = aggregate(&campaigns);
    let master = notebook::render_master_notebook(&campaigns, &aggs, PASS_THRESHOLD, &manifest);
    let master_path = cfg.out_dir.join("MASTER_NOTEBOOK.md");
    std::fs::write(&master_path, &master).map_err(|e| NtgError::InvalidInput(e.to_string()))?;

    // Also copy/update stable path under docs/schooling/
    let stable = Path::new("../docs/schooling/runs");
    if let Ok(true) = std::fs::canonicalize(&cfg.out_dir).map(|_| true) {
        let _ = std::fs::write(stable.join("MASTER_NOTEBOOK.md"), &master);
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let summary = format!(
        "schooling complete ts={ts} runs={} pass_threshold={:.0}% master={}\n{}",
        cfg.campaign_runs,
        PASS_THRESHOLD * 100.0,
        master_path.display(),
        notebook::aggregate_summary_line(&aggs)
    );
    Ok(summary)
}

fn aggregate(campaigns: &[CampaignReport]) -> Vec<AggregateStats> {
    let mut out = Vec::new();
    for phase in 0..=5u32 {
        let mut scores = Vec::new();
        let mut attempts = Vec::new();
        let mut n_pass = 0u32;
        let mut n_fail = 0u32;
        for c in campaigns {
            if let Some(rec) = c.phases.iter().find(|p| p.phase == phase) {
                scores.push(rec.exam.score());
                attempts.push(rec.exam.attempt as f64);
                if rec.exam.passed() {
                    n_pass += 1;
                } else {
                    n_fail += 1;
                }
            }
        }
        if scores.is_empty() {
            continue;
        }
        let n = scores.len() as f64;
        let mean = scores.iter().sum::<f64>() / n;
        let min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mean_att = attempts.iter().sum::<f64>() / attempts.len() as f64;
        out.push(AggregateStats {
            phase,
            n_runs: scores.len() as u32,
            n_pass,
            n_fail,
            mean_score: mean,
            min_score: min,
            max_score: max,
            mean_attempts: mean_att,
        });
    }
    out
}
