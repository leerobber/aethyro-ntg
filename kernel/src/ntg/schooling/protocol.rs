//! Schooling protocol: doctorate-style pass bar and run records.
//!
//! **Hard rule:** any phase exam score **below 75.0%** is a **FAIL**.
//! Protocol requires full redo of study + exam for that phase (up to
//! `max_attempts`). Scores and evidence are written to the notebook.

use std::fmt;

/// Minimum fraction correct (or composite skill score) to pass a phase exam.
pub const PASS_THRESHOLD: f64 = 0.75;

/// Maximum full redo attempts per phase per campaign.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Number of independent campaign runs for statistical documentation.
pub const DEFAULT_CAMPAIGN_RUNS: u32 = 5;

#[derive(Clone, Debug)]
pub struct ExamItem {
    pub id: String,
    pub skill: String,
    pub prompt: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct PhaseExamResult {
    pub phase: u32,
    pub title: String,
    pub items: Vec<ExamItem>,
    pub attempt: u32,
    /// Wall-clock study + exam for this attempt (µs).
    pub latency_us: u64,
    /// Optional continuous score in [0,1] when not pure item-count (e.g. calib F1).
    pub composite: Option<f64>,
}

impl PhaseExamResult {
    pub fn n_pass(&self) -> usize {
        self.items.iter().filter(|i| i.passed).count()
    }

    pub fn n_total(&self) -> usize {
        self.items.len()
    }

    /// Primary score: composite if set, else item pass rate.
    pub fn score(&self) -> f64 {
        if let Some(c) = self.composite {
            return c.clamp(0.0, 1.0);
        }
        if self.items.is_empty() {
            return 0.0;
        }
        self.n_pass() as f64 / self.n_total() as f64
    }

    pub fn passed(&self) -> bool {
        self.score() + 1e-12 >= PASS_THRESHOLD
    }

    pub fn verdict(&self) -> &'static str {
        if self.passed() {
            "PASS"
        } else {
            "FAIL — full redo required"
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhaseStudyReport {
    pub phase: u32,
    pub activities: Vec<String>,
    /// What was trained / practiced (human-readable, sourced).
    pub taught: Vec<String>,
    pub samples_seen: usize,
}

#[derive(Clone, Debug)]
pub struct PhaseRunRecord {
    pub phase: u32,
    pub study: PhaseStudyReport,
    pub exam: PhaseExamResult,
    pub dataset_id: String,
    pub dataset_source: String,
}

#[derive(Clone, Debug)]
pub struct CampaignReport {
    pub run_id: u32,
    pub phases: Vec<PhaseRunRecord>,
    pub overall_pass: bool,
}

impl CampaignReport {
    pub fn all_passed(&self) -> bool {
        self.phases.iter().all(|p| p.exam.passed())
    }
}

#[derive(Clone, Debug, Default)]
pub struct AggregateStats {
    pub phase: u32,
    pub n_runs: u32,
    pub n_pass: u32,
    pub n_fail: u32,
    pub mean_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub mean_attempts: f64,
}

impl fmt::Display for PhaseExamResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "phase{} {} items={}/{} score={:.1}% verdict={}",
            self.phase,
            self.title,
            self.n_pass(),
            self.n_total(),
            self.score() * 100.0,
            self.verdict()
        )
    }
}

pub fn item(id: &str, skill: &str, prompt: &str, passed: bool, detail: impl Into<String>) -> ExamItem {
    ExamItem {
        id: id.into(),
        skill: skill.into(),
        prompt: prompt.into(),
        passed,
        detail: detail.into(),
    }
}
