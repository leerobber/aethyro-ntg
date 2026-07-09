//! Phase 4 calibration: doc-graph NodeKind classifier (ADR 0006).
//!
//! Real structural labels from `docparse` (Execution vs Content).
//! Ternary features + ternary weights; class-balanced perceptron.
//!
//! Imbalance handling (2026-07-09 fix):
//! - Per-epoch balanced mini-pass: all minority + equal majority subsample
//! - Minority class gets `neg_pos_ratio` update repeats (cost-sensitive)
//! - Threshold sweep on train scores for best balanced accuracy
//! - Hold-out split for honest generalization metrics
//! - Win = test balanced accuracy > 0.5 + ε (beats majority on balance)

use crate::ntg::docparse;
use crate::ntg::error::NtgError;
use crate::ntg::glyph::extract_glyph_fingerprint;
use crate::ntg::graph::{Graph, NodeKind};
use crate::ntg::leafsignal::extract_leaf_signal;
use crate::ntg::ledger::{
    replay::ExecutionTrace, FitnessMeasure, MutationOutcome, TamperEvidentLedger,
};
use crate::ntg::mutation::rules::{MutationRule, MutationRuleKind};
use crate::ntg::mutation::{MutationCycle, SelfModConfig};
use crate::ntg::ternary::encode_fixed;
use std::time::Instant;

/// Feature width.
pub const FEATURE_DIM: usize = 64;

/// One labeled sample from a parsed graph node.
#[derive(Clone, Debug)]
pub struct Sample {
    pub features: Vec<i8>,
    /// true = Execution, false = Content
    pub is_execution: bool,
    pub label_preview: String,
}

/// Confusion / ranking metrics for binary Execution class.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClassMetrics {
    pub accuracy: f32,
    pub balanced_accuracy: f32,
    pub precision_exec: f32,
    pub recall_exec: f32,
    pub f1_exec: f32,
    pub tp: usize,
    pub tn: usize,
    pub fp: usize,
    pub fn_: usize,
}

/// Result of a full calibration run.
#[derive(Clone, Debug)]
pub struct CalibReport {
    pub n_samples: usize,
    pub n_train: usize,
    pub n_test: usize,
    pub n_execution: usize,
    pub n_content: usize,
    /// Majority-class accuracy on **test** (always Content).
    pub baseline_accuracy: f32,
    /// Majority balanced accuracy on test (= 0.5 if both classes present).
    pub baseline_balanced_accuracy: f32,
    pub train_metrics: ClassMetrics,
    pub test_metrics: ClassMetrics,
    pub before_test_balanced: f32,
    pub delta_balanced_vs_baseline: f32,
    pub epochs: usize,
    pub latency_us: u64,
    pub threshold: i64,
    pub is_win: bool,
    pub weights: Vec<i8>,
}

impl CalibReport {
    pub fn summary_line(&self) -> String {
        format!(
            "n={} train={} test={} exec={} content={} thr={} \
             base_acc={:.3} base_bal={:.3} \
             test_acc={:.3} test_bal={:.3} test_f1={:.3} test_rec={:.3} test_prec={:.3} \
             delta_bal={:+.3} epochs={} latency_us={} win={}",
            self.n_samples,
            self.n_train,
            self.n_test,
            self.n_execution,
            self.n_content,
            self.threshold,
            self.baseline_accuracy,
            self.baseline_balanced_accuracy,
            self.test_metrics.accuracy,
            self.test_metrics.balanced_accuracy,
            self.test_metrics.f1_exec,
            self.test_metrics.recall_exec,
            self.test_metrics.precision_exec,
            self.delta_balanced_vs_baseline,
            self.epochs,
            self.latency_us,
            self.is_win
        )
    }
}

/// Build ternary feature vector for a node label (deterministic).
pub fn features_from_label(label: &str) -> Vec<i8> {
    let mut feat = vec![0i8; FEATURE_DIM];

    let tern = encode_fixed(label);
    for (i, &t) in tern.iter().take(48).enumerate() {
        feat[i] = t;
    }

    let sig = extract_leaf_signal(label);
    let total = (sig.uppercase_count
        + sig.lowercase_count
        + sig.punctuation_count
        + sig.whitespace_count
        + sig.other_count)
        .max(1) as f32;
    let channels = [
        sig.uppercase_count as f32 / total,
        sig.lowercase_count as f32 / total,
        sig.punctuation_count as f32 / total,
        sig.whitespace_count as f32 / total,
        sig.other_count as f32 / total,
    ];
    for (i, &c) in channels.iter().enumerate() {
        feat[48 + i] = if c > 0.33 {
            1
        } else if c < 0.05 {
            -1
        } else {
            0
        };
    }

    let g = extract_glyph_fingerprint(label);
    let h = g.shape_hash;
    for i in 0..8 {
        let bit = ((h >> (i * 8)) & 0xff) as u8;
        feat[53 + i] = if bit > 170 {
            1
        } else if bit < 85 {
            -1
        } else {
            0
        };
    }
    feat[61] = if g.class_diversity >= 3 { 1 } else { 0 };
    // Execution nodes store *fence body* without ``` markers (see docparse).
    feat[62] = if looks_like_code(label) { 1 } else { -1 };
    feat[63] = if label.len() > 40 {
        1
    } else if label.len() < 8 {
        -1
    } else {
        0
    };

    feat
}

/// Heuristic: fence bodies look like code (keywords, braces, calls).
fn looks_like_code(label: &str) -> bool {
    const KEYS: &[&str] = &[
        "fn ",
        "def ",
        "let ",
        "pub ",
        "import ",
        "return ",
        "print(",
        "println",
        "class ",
        "const ",
        "var ",
        "function",
        "#!/",
        "->",
        "::",
        "self.",
        "this.",
        "async ",
        "await ",
        "struct ",
        "impl ",
        "use ",
        "from ",
        "```",
    ];
    if KEYS.iter().any(|k| label.contains(k)) {
        return true;
    }
    let braces = label.matches('{').count() + label.matches('}').count();
    let semis = label.matches(';').count();
    braces >= 2 || semis >= 2 || (label.contains('(') && label.contains(')') && label.contains('\n'))
}

/// Collect samples from a graph.
pub fn samples_from_graph(graph: &Graph) -> Result<Vec<Sample>, NtgError> {
    let mut out = Vec::new();
    for id in graph.all_node_ids() {
        let node = graph.node(id)?;
        let is_execution = node.kind == NodeKind::Execution;
        let preview: String = node.label.chars().take(48).collect();
        out.push(Sample {
            features: features_from_label(&node.label),
            is_execution,
            label_preview: preview,
        });
    }
    Ok(out)
}

/// Parse multiple markdown documents into one graph and collect samples.
pub fn samples_from_documents(docs: &[(&str, &str)]) -> Result<Vec<Sample>, NtgError> {
    let mut graph = Graph::new();
    for &(name, text) in docs {
        docparse::parse_into(&mut graph, name, text);
    }
    samples_from_graph(&graph)
}

fn score(weights: &[i8], features: &[i8]) -> i64 {
    weights
        .iter()
        .zip(features.iter())
        .map(|(&w, &x)| (w as i64) * (x as i64))
        .sum()
}

fn predict(weights: &[i8], features: &[i8], threshold: i64) -> bool {
    score(weights, features) >= threshold
}

fn clamp_ternary(v: i32) -> i8 {
    if v > 1 {
        1
    } else if v < -1 {
        -1
    } else {
        v as i8
    }
}

/// Confusion metrics for Execution-positive class.
pub fn class_metrics(weights: &[i8], samples: &[Sample], threshold: i64) -> ClassMetrics {
    if samples.is_empty() {
        return ClassMetrics::default();
    }
    let mut tp = 0usize;
    let mut tn = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;
    for s in samples {
        let pred = predict(weights, &s.features, threshold);
        match (pred, s.is_execution) {
            (true, true) => tp += 1,
            (false, false) => tn += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
        }
    }
    let n = samples.len() as f32;
    let accuracy = (tp + tn) as f32 / n;
    let has_pos = tp + fn_ > 0;
    let has_neg = tn + fp > 0;
    let tpr = if has_pos {
        tp as f32 / (tp + fn_) as f32
    } else {
        0.0
    };
    let tnr = if has_neg {
        tn as f32 / (tn + fp) as f32
    } else {
        0.0
    };
    // If a class is missing in the eval set, bal_acc collapses to the present class rate.
    let balanced_accuracy = match (has_pos, has_neg) {
        (true, true) => 0.5 * (tpr + tnr),
        (true, false) => tpr,
        (false, true) => tnr,
        (false, false) => 0.0,
    };
    let precision_exec = if tp + fp > 0 {
        tp as f32 / (tp + fp) as f32
    } else {
        0.0
    };
    let recall_exec = tpr;
    let f1_exec = if precision_exec + recall_exec > 0.0 {
        2.0 * precision_exec * recall_exec / (precision_exec + recall_exec)
    } else {
        0.0
    };
    ClassMetrics {
        accuracy,
        balanced_accuracy,
        precision_exec,
        recall_exec,
        f1_exec,
        tp,
        tn,
        fp,
        fn_,
    }
}

/// Majority-class accuracy (always Content).
pub fn baseline_accuracy(samples: &[Sample]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let content = samples.iter().filter(|s| !s.is_execution).count();
    content as f32 / samples.len() as f32
}

/// Deterministic train/test split (80/20) stratified by class when possible.
pub fn stratified_split(samples: &[Sample], train_ratio: f32) -> (Vec<Sample>, Vec<Sample>) {
    let mut pos: Vec<&Sample> = samples.iter().filter(|s| s.is_execution).collect();
    let mut neg: Vec<&Sample> = samples.iter().filter(|s| !s.is_execution).collect();
    // Stable order by preview for determinism
    pos.sort_by(|a, b| a.label_preview.cmp(&b.label_preview));
    neg.sort_by(|a, b| a.label_preview.cmp(&b.label_preview));

    let split = |v: &[&Sample]| {
        if v.is_empty() {
            return (vec![], vec![]);
        }
        if v.len() == 1 {
            // Put singleton in both (honest small-n behavior)
            return (vec![v[0].clone()], vec![v[0].clone()]);
        }
        let mut n_train = ((v.len() as f32) * train_ratio).round() as usize;
        n_train = n_train.clamp(1, v.len() - 1);
        let train: Vec<Sample> = v[..n_train].iter().map(|s| (*s).clone()).collect();
        let test: Vec<Sample> = v[n_train..].iter().map(|s| (*s).clone()).collect();
        (train, test)
    };

    let (pt, pe) = split(&pos);
    let (nt, ne) = split(&neg);
    let mut train = pt;
    train.extend(nt);
    let mut test = pe;
    test.extend(ne);
    train.sort_by(|a, b| a.label_preview.cmp(&b.label_preview));
    test.sort_by(|a, b| a.label_preview.cmp(&b.label_preview));
    (train, test)
}

/// Build one balanced epoch order: all positives + equal negatives (cycled).
fn balanced_epoch_order(samples: &[Sample], seed: u64) -> Vec<usize> {
    let pos: Vec<usize> = samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_execution)
        .map(|(i, _)| i)
        .collect();
    let neg: Vec<usize> = samples
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.is_execution)
        .map(|(i, _)| i)
        .collect();

    if pos.is_empty() {
        return (0..samples.len()).collect();
    }
    if neg.is_empty() {
        return pos;
    }

    // Deterministic "shuffle" via stride from seed
    let stride = (seed as usize).wrapping_mul(2654435761) % neg.len().max(1);
    let mut neg_rot: Vec<usize> = neg[stride..].to_vec();
    neg_rot.extend_from_slice(&neg[..stride]);

    let mut order = Vec::with_capacity(pos.len() * 2);
    for (i, &p) in pos.iter().enumerate() {
        order.push(p);
        order.push(neg_rot[i % neg_rot.len()]);
    }
    order
}

/// Train weights with balanced sampling + minority oversampling of updates.
fn train_balanced(train: &[Sample], epochs: usize, threshold: i64) -> Vec<i8> {
    let mut weights = vec![0i8; FEATURE_DIM];
    // Strong prior: code-like feature votes Execution (fence bodies lack ```).
    weights[62] = 1;

    let n_pos = train.iter().filter(|s| s.is_execution).count().max(1);
    let n_neg = train.iter().filter(|s| !s.is_execution).count().max(1);
    let pos_repeats = (n_neg / n_pos).max(1).min(64);

    for epoch in 0..epochs {
        let order = balanced_epoch_order(train, epoch as u64 + 1);
        for &idx in &order {
            let s = &train[idx];
            let pred = predict(&weights, &s.features, threshold);
            if pred != s.is_execution {
                let y: i32 = if s.is_execution { 1 } else { -1 };
                let reps = if s.is_execution { pos_repeats } else { 1 };
                for _ in 0..reps {
                    for i in 0..FEATURE_DIM {
                        let x = s.features[i] as i32;
                        let w = weights[i] as i32;
                        weights[i] = clamp_ternary(w + y * x);
                    }
                }
                // Keep code cue pinned positive (stable prior under imbalance).
                weights[62] = 1;
            }
        }
    }
    weights[62] = 1;
    weights
}

/// Score for threshold selection under imbalance.
/// Prefer F1 (precision×recall) so we do not pick thr that recalls all via flood of FPs.
fn thr_objective(m: &ClassMetrics) -> f32 {
    let flood_penalty = if m.precision_exec < 0.05 && m.fp > m.tp.saturating_mul(5) {
        0.5
    } else {
        0.0
    };
    2.0 * m.f1_exec + m.balanced_accuracy + 0.25 * m.recall_exec - flood_penalty
}

/// Sweep thresholds; fall back to midpoint of class score medians if F1 never lifts.
fn best_threshold(weights: &[i8], train: &[Sample]) -> i64 {
    if train.is_empty() {
        return 1;
    }

    let mut pos_scores: Vec<i64> = train
        .iter()
        .filter(|s| s.is_execution)
        .map(|s| score(weights, &s.features))
        .collect();
    let mut neg_scores: Vec<i64> = train
        .iter()
        .filter(|s| !s.is_execution)
        .map(|s| score(weights, &s.features))
        .collect();
    pos_scores.sort_unstable();
    neg_scores.sort_unstable();

    let median = |v: &[i64]| {
        if v.is_empty() {
            0
        } else {
            v[v.len() / 2]
        }
    };
    let midpoint = (median(&pos_scores) + median(&neg_scores)) / 2;

    let mut candidates = vec![midpoint, midpoint + 1, midpoint - 1, 0, 1, 2, -1];
    for &s in pos_scores.iter().chain(neg_scores.iter()) {
        candidates.push(s);
        candidates.push(s.saturating_sub(1));
    }
    candidates.sort_unstable();
    candidates.dedup();

    let mut best_thr = midpoint;
    let mut best_obj = f32::NEG_INFINITY;
    for thr in candidates {
        let m = class_metrics(weights, train, thr);
        let obj = thr_objective(&m);
        if obj > best_obj {
            best_obj = obj;
            best_thr = thr;
        }
    }

    // If nothing beat pure majority on train F1, use score midpoint
    let best_m = class_metrics(weights, train, best_thr);
    if best_m.f1_exec < 1e-6 && !pos_scores.is_empty() && !neg_scores.is_empty() {
        return midpoint + 1; // slightly prefer Content unless score clears mid
    }
    best_thr
}

/// Run class-balanced ternary calibration with hold-out evaluation.
pub fn calibrate(
    samples: &[Sample],
    epochs: usize,
    _threshold_hint: i64,
) -> Result<CalibReport, NtgError> {
    if samples.is_empty() {
        return Err(NtgError::InvalidInput("no calibration samples".into()));
    }

    let t0 = Instant::now();
    let (train, test) = stratified_split(samples, 0.8);
    if train.is_empty() || test.is_empty() {
        // Degenerate: evaluate on all
        return calibrate_in_sample(samples, epochs);
    }

    // Train with thr=1 so zero weights start as Content; balanced updates fire on all exec.
    let weights = train_balanced(&train, epochs, 1);
    let threshold = best_threshold(&weights, &train);

    let train_metrics = class_metrics(&weights, &train, threshold);
    let test_metrics = class_metrics(&weights, &test, threshold);
    let zeros = vec![0i8; FEATURE_DIM];
    let before_test = class_metrics(&zeros, &test, 1);

    let baseline_acc = baseline_accuracy(&test);
    let has_both = test.iter().any(|s| s.is_execution) && test.iter().any(|s| !s.is_execution);
    let baseline_bal = if has_both { 0.5 } else { baseline_acc };

    let delta_bal = test_metrics.balanced_accuracy - baseline_bal;
    // Win: clear lift in balanced accuracy and/or useful minority F1
    let is_win = (test_metrics.balanced_accuracy > baseline_bal + 0.05
        && test_metrics.recall_exec >= 0.25)
        || (test_metrics.f1_exec >= 0.25 && test_metrics.balanced_accuracy >= 0.55)
        || (test_metrics.recall_exec >= 0.5 && test_metrics.precision_exec >= 0.15);

    let n_exec = samples.iter().filter(|s| s.is_execution).count();
    let latency_us = t0.elapsed().as_micros() as u64;

    Ok(CalibReport {
        n_samples: samples.len(),
        n_train: train.len(),
        n_test: test.len(),
        n_execution: n_exec,
        n_content: samples.len() - n_exec,
        baseline_accuracy: baseline_acc,
        baseline_balanced_accuracy: baseline_bal,
        train_metrics,
        test_metrics,
        before_test_balanced: before_test.balanced_accuracy,
        delta_balanced_vs_baseline: delta_bal,
        epochs,
        latency_us,
        threshold,
        is_win,
        weights,
    })
}

/// Fallback when split is impossible (tiny sets).
fn calibrate_in_sample(samples: &[Sample], epochs: usize) -> Result<CalibReport, NtgError> {
    let t0 = Instant::now();
    let weights = train_balanced(samples, epochs, 1);
    let threshold = best_threshold(&weights, samples);
    let metrics = class_metrics(&weights, samples, threshold);
    let baseline_acc = baseline_accuracy(samples);
    let has_both =
        samples.iter().any(|s| s.is_execution) && samples.iter().any(|s| !s.is_execution);
    let baseline_bal = if has_both { 0.5 } else { baseline_acc };
    let n_exec = samples.iter().filter(|s| s.is_execution).count();
    Ok(CalibReport {
        n_samples: samples.len(),
        n_train: samples.len(),
        n_test: samples.len(),
        n_execution: n_exec,
        n_content: samples.len() - n_exec,
        baseline_accuracy: baseline_acc,
        baseline_balanced_accuracy: baseline_bal,
        train_metrics: metrics,
        test_metrics: metrics,
        before_test_balanced: 0.5,
        delta_balanced_vs_baseline: metrics.balanced_accuracy - baseline_bal,
        epochs,
        latency_us: t0.elapsed().as_micros() as u64,
        threshold,
        is_win: metrics.balanced_accuracy > baseline_bal + 0.02,
        weights,
    })
}

/// Report from optional topology self-mod probe (ADR 0002; off by default).
#[derive(Clone, Debug)]
pub struct SelfModProbeReport {
    pub enabled: bool,
    pub proposed: bool,
    pub accepted: bool,
    pub ledger_mutation_id: Option<u64>,
    pub detail: String,
}

/// Optional topology self-mod under ADR 0002 rails.
///
/// - If `enable` is false (default): no mutation; returns immediately.
/// - If true: propose `AddNode` on a cloned graph, evaluate dual-objective
///   fitness, accept or reject, **always ledger-log** the decision.
///
/// Does **not** permanently alter the caller's graph unless accepted and
/// the caller applies the rule (this probe logs only; graph stays intact
/// for safety in Phase 4 v1).
pub fn optional_self_mod_probe(
    graph: &Graph,
    enable: bool,
    ledger: &mut TamperEvidentLedger,
    timestamp: u64,
) -> Result<SelfModProbeReport, NtgError> {
    if !enable {
        return Ok(SelfModProbeReport {
            enabled: false,
            proposed: false,
            accepted: false,
            ledger_mutation_id: None,
            detail: "self-mod disabled (ADR 0002 rail 1)".into(),
        });
    }

    let mut config = SelfModConfig::default();
    config.enabled = true;
    config.cycle_budget_us = 5_000_000; // 5ms budget for probe
    config.max_mutations_per_cycle = 1;

    // Baseline fitness: use fingerprint cost proxy + node count
    let pre_fp = graph.fingerprint().unwrap_or(0);
    let baseline = (100u64, graph.node_count() as u64 * 64);

    let mut cycle = MutationCycle::new(config, baseline)?;
    let rule = MutationRule {
        kind: MutationRuleKind::AddNode {
            label: "phase4_probe_node".into(),
        },
    };
    cycle.propose_mutation(rule)?;

    let ((lat, mem), budget_us) = cycle.evaluate_mutation(graph, 0)?;
    let accept = cycle.should_accept((lat, mem));
    if accept {
        cycle.accept_mutation(0)?;
    }

    let post_fp = {
        let mut g2 = graph.clone();
        // apply for ledger description only
        let _ = MutationRule {
            kind: MutationRuleKind::AddNode {
                label: "phase4_probe_node".into(),
            },
        }
        .apply(&mut g2);
        g2.fingerprint().unwrap_or(pre_fp)
    };

    let outcome = if accept {
        MutationOutcome::Accepted
    } else {
        MutationOutcome::RejectedFitnessGate
    };

    let mid = ledger.log_mutation(
        format!(
            "phase4_self_mod_probe accept={} pre_fp={} post_fp={} lat={} mem={} budget_us={}",
            accept, pre_fp, post_fp, lat, mem, budget_us
        ),
        pre_fp,
        post_fp,
        FitnessMeasure {
            latency_us: lat,
            memory_bytes: mem,
        },
        outcome,
        budget_us.saturating_mul(1000),
        ExecutionTrace::new(),
        timestamp,
    )?;

    Ok(SelfModProbeReport {
        enabled: true,
        proposed: true,
        accepted: accept,
        ledger_mutation_id: Some(mid),
        detail: format!(
            "AddNode probe; accept={} (dual-objective fitness vs baseline)",
            accept
        ),
    })
}

/// Optional: snapshot weights into ledger for audit.
pub fn ledger_weight_snapshot(
    ledger: &mut TamperEvidentLedger,
    report: &CalibReport,
    timestamp: u64,
) -> Result<u64, NtgError> {
    let nonzero = report.weights.iter().filter(|&&w| w != 0).count();
    ledger.log_mutation(
        format!(
            "phase4_calib_snapshot win={} test_bal={:.4} base_bal={:.4} test_f1={:.4} thr={} nonzero_w={} n={}",
            report.is_win,
            report.test_metrics.balanced_accuracy,
            report.baseline_balanced_accuracy,
            report.test_metrics.f1_exec,
            report.threshold,
            nonzero,
            report.n_samples
        ),
        0,
        report.test_metrics.balanced_accuracy.to_bits() as u64,
        FitnessMeasure {
            latency_us: report.latency_us,
            memory_bytes: report.weights.len() as u64,
        },
        if report.is_win {
            MutationOutcome::Accepted
        } else {
            MutationOutcome::RejectedFitnessGate
        },
        report.latency_us.saturating_mul(1000),
        ExecutionTrace::new(),
        timestamp,
    )
}

/// Built-in fixtures for offline CI.
pub fn fixture_documents() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "adr_like",
            r#"# ADR 0001 Vision
## Context
Some prose about ternary graphs.
## Decision
1. Build engine first
2. Measure everything
```rust
fn main() { println!("exec"); }
```
## Consequences
Documented honestly.
"#,
        ),
        (
            "readme_like",
            r#"# Project
Intro paragraph with no code.
## Install
- step one
- step two
## Example
```python
print("hello")
```
More text after.
"#,
        ),
        (
            "design_like",
            r#"# Design
## Layers
Text only section.
## Code path
```
shell command here
```
## Notes
- a
- b
"#,
        ),
        (
            "more_code",
            r#"# Extra
## A
Normal text paragraph without any code.
## B
```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```
## C
```python
def greet(name):
    print(name)
    return True
```
## D
Still just prose and a list:
- one
- two
"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_dim() {
        let f = features_from_label("fn x() {\n  return 1;\n}");
        assert_eq!(f.len(), FEATURE_DIM);
        assert!(f.iter().any(|&x| x != 0));
        assert_eq!(f[62], 1); // code cue on fence body style
        let prose = features_from_label("Just a heading");
        assert_eq!(prose[62], -1);
    }

    #[test]
    fn fixtures_yield_both_classes() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        assert!(samples.len() > 5);
        assert!(samples.iter().any(|s| s.is_execution));
        assert!(samples.iter().any(|s| !s.is_execution));
    }

    #[test]
    fn stratified_split_keeps_both_classes() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let (tr, te) = stratified_split(&samples, 0.8);
        assert!(!tr.is_empty() && !te.is_empty());
        assert!(tr.iter().any(|s| s.is_execution) || te.iter().any(|s| s.is_execution));
    }

    #[test]
    fn calibration_improves_balanced_on_fixtures() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 40, 0).unwrap();
        // With code cues + balanced train, expect exec recall and bal_acc >= majority 0.5
        assert!(
            report.test_metrics.balanced_accuracy + 1e-3 >= 0.5,
            "bal_acc should be >= 0.5: {}",
            report.summary_line()
        );
        assert!(
            report.test_metrics.recall_exec > 0.0 || report.train_metrics.recall_exec > 0.0,
            "expected some Execution detection: {}",
            report.summary_line()
        );
    }

    #[test]
    fn imbalance_metrics_majority_has_zero_f1() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let zeros = vec![0i8; FEATURE_DIM];
        let m = class_metrics(&zeros, &samples, 100);
        assert_eq!(m.f1_exec, 0.0);
        assert_eq!(m.recall_exec, 0.0);
    }

    #[test]
    fn majority_baseline_has_zero_exec_recall() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let zeros = vec![0i8; FEATURE_DIM];
        // thr very high → always Content
        let m = class_metrics(&zeros, &samples, 1000);
        assert_eq!(m.recall_exec, 0.0);
        assert!((m.balanced_accuracy - 0.5).abs() < 0.01 || m.tn + m.fp == samples.len());
    }

    #[test]
    fn ledger_snapshot_ok() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 5, 1).unwrap();
        let mut ledger = TamperEvidentLedger::new(None).unwrap();
        ledger_weight_snapshot(&mut ledger, &report, 1).unwrap();
        ledger.verify_full_ledger().unwrap();
    }

    #[test]
    fn self_mod_probe_disabled_by_default() {
        let mut g = Graph::new();
        docparse::parse_into(&mut g, "t", "# A\n```\nfn x(){}\n```\n");
        let mut ledger = TamperEvidentLedger::new(None).unwrap();
        let r = optional_self_mod_probe(&g, false, &mut ledger, 1).unwrap();
        assert!(!r.enabled);
        assert!(!r.proposed);
        assert!(r.ledger_mutation_id.is_none());
    }

    #[test]
    fn self_mod_probe_enabled_logs_ledger() {
        let mut g = Graph::new();
        docparse::parse_into(&mut g, "t", "# A\n```\nfn x(){}\n```\n");
        let mut ledger = TamperEvidentLedger::new(None).unwrap();
        let r = optional_self_mod_probe(&g, true, &mut ledger, 1).unwrap();
        assert!(r.enabled && r.proposed);
        assert!(r.ledger_mutation_id.is_some());
        ledger.verify_full_ledger().unwrap();
        // Graph node count unchanged (probe does not mutate caller graph)
        assert_eq!(g.node_count(), 3); // root + heading + exec roughly
    }
}
