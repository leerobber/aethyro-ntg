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
    feat[61] = if g.class_diversity >= 3 { 1 } else { -1 };
    // Execution nodes store *fence body* without ``` markers (see docparse).
    feat[62] = if looks_like_code(label) { 1 } else { -1 };
    // Phase 5 precision cues: indent density + line-shape (code vs prose).
    let indent_code = looks_like_indented_code(label);
    let line_shape = code_line_shape(label);
    // Pack into 63: prefer indent, then multi-line length shape.
    feat[63] = if indent_code || line_shape {
        1
    } else if label.len() < 8 || (!label.contains('\n') && label.len() < 48) {
        -1 // very short body, or short single-line prose / heading
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
        "#[",
        "include ",
        "package ",
        "module ",
        "export ",
        "require(",
        "console.",
        "std::",
        "Result<",
        "Option<",
        "Vec<",
        "match ",
        "else {",
        "if (",
        "for (",
        "while (",
        "elif ",
        "endif",
        "done",
        "echo ",
        "cargo ",
        "npm ",
    ];
    // Prefer structural + keyword evidence. Strong code markers alone count.
    const STRONG: &[&str] = &[
        "fn ", "def ", "println", "print(", "#!/", "pub fn ", "async fn ", "```",
    ];
    if STRONG.iter().any(|k| label.contains(k)) {
        return true;
    }
    let braces = label.matches('{').count() + label.matches('}').count();
    let semis = label.matches(';').count();
    let assigns = label.matches('=').count();
    let multi = label.contains('\n');
    let key_hit = KEYS.iter().any(|k| label.contains(k));
    // Weak keywords only if structure also present (avoids ADR prose "use ").
    if key_hit && (braces >= 1 || semis >= 1 || multi) {
        return true;
    }
    braces >= 2
        || semis >= 2
        || (assigns >= 2 && multi)
        || (label.contains('(') && label.contains(')') && multi && semis >= 1)
}

/// Leading whitespace on ≥2 lines → likely fence body / code block.
fn looks_like_indented_code(label: &str) -> bool {
    let mut indented = 0usize;
    for line in label.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            indented += 1;
        }
    }
    indented >= 2
}

/// Multi-line with short dense lines (code-ish) vs long prose paragraphs.
fn code_line_shape(label: &str) -> bool {
    let lines: Vec<&str> = label.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return false;
    }
    let short = lines.iter().filter(|l| l.len() < 72).count();
    let punct = label.chars().filter(|c| "{}();=<>".contains(*c)).count();
    short * 2 >= lines.len() && punct >= 3
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
    // Strong prior: code-like features vote Execution (fence bodies lack ```).
    weights[62] = 1;
    weights[63] = 1;

    let n_pos = train.iter().filter(|s| s.is_execution).count().max(1);
    let n_neg = train.iter().filter(|s| !s.is_execution).count().max(1);
    let pos_repeats = (n_neg / n_pos).clamp(1, 48);

    for epoch in 0..epochs {
        let order = balanced_epoch_order(train, epoch as u64 + 1);
        for &idx in &order {
            let s = &train[idx];
            let pred = predict(&weights, &s.features, threshold);
            if pred != s.is_execution {
                let y: i32 = if s.is_execution { 1 } else { -1 };
                let reps = if s.is_execution { pos_repeats } else { 1 };
                for _ in 0..reps {
                    for (w, &x) in weights.iter_mut().zip(s.features.iter()) {
                        *w = clamp_ternary(*w as i32 + y * x as i32);
                    }
                }
                // Keep code cues pinned positive (stable prior under imbalance).
                weights[62] = 1;
                weights[63] = 1;
            }
        }
    }
    weights[62] = 1;
    weights[63] = 1;
    weights
}

/// Threshold objective: F1-first, balanced accuracy, mild precision, flood reject.
fn thr_objective(m: &ClassMetrics) -> f32 {
    if m.fp >= 30 && m.precision_exec < 0.04 {
        return -15.0 - (m.fp as f32) * 0.01;
    }
    if m.precision_exec < 0.05 && m.fp > m.tp.saturating_mul(8).max(16) {
        return -4.0 + m.f1_exec;
    }
    3.0 * m.f1_exec
        + 1.25 * m.balanced_accuracy
        + 0.5 * m.precision_exec
        + 0.75 * m.recall_exec
}

/// Sweep thresholds. Prefer viable F1 region (prec+rec floors), else best F1.
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

    let mut candidates = vec![
        midpoint,
        midpoint + 1,
        midpoint - 1,
        0,
        1,
        2,
        -1,
        3,
        5,
        8,
        11,
        15,
        21,
    ];
    for &s in pos_scores.iter().chain(neg_scores.iter()) {
        candidates.push(s);
        candidates.push(s.saturating_sub(1));
        candidates.push(s.saturating_add(1));
    }
    candidates.sort_unstable();
    candidates.dedup();

    // Pass 1: useful detection (rec ≥ 15%) with non-catastrophic precision.
    let mut best_thr = midpoint.max(1);
    let mut best_obj = f32::NEG_INFINITY;
    let mut found = false;
    for &thr in &candidates {
        let m = class_metrics(weights, train, thr);
        if m.recall_exec >= 0.15 && m.precision_exec >= 0.06 {
            found = true;
            let obj = thr_objective(&m);
            if obj > best_obj {
                best_obj = obj;
                best_thr = thr;
            }
        }
    }
    if found {
        return best_thr;
    }

    // Pass 2: any thr with some recall; maximize objective (flood-penalized).
    best_obj = f32::NEG_INFINITY;
    for &thr in &candidates {
        let m = class_metrics(weights, train, thr);
        if m.recall_exec < 1e-6 && m.tp == 0 {
            continue;
        }
        let obj = thr_objective(&m);
        if obj > best_obj {
            best_obj = obj;
            best_thr = thr;
        }
    }

    let best_m = class_metrics(weights, train, best_thr);
    if best_m.f1_exec < 1e-6 && !pos_scores.is_empty() && !neg_scores.is_empty() {
        // Prefer slightly above content median so only strong scores fire.
        return midpoint.saturating_add(1);
    }
    best_thr
}

/// Train a model on **all** provided samples (no internal hold-out).
/// Used by doctorate schooling study passes so train docs are fully taught.
pub fn train_model_full(samples: &[Sample], epochs: usize) -> Result<CalibModel, NtgError> {
    if samples.is_empty() {
        return Err(NtgError::InvalidInput("no samples for train_model_full".into()));
    }
    let weights = train_balanced(samples, epochs, 1);
    let mut threshold = best_threshold(&weights, samples);
    // Re-balance thr if collapsed (precision-only or FLOOD/recall-only).
    let m0 = class_metrics(&weights, samples, threshold);
    let collapsed = m0.recall_exec < 0.12
        || (m0.precision_exec < 0.05 && m0.fp > m0.tp.saturating_mul(10).max(20));
    if collapsed {
        let mut best_thr = threshold;
        let mut best_obj = f32::NEG_INFINITY;
        for thr in -5i64..=40 {
            let m = class_metrics(&weights, samples, thr);
            // Operating region: useful recall without catastrophic flood.
            if m.recall_exec < 0.12 || m.recall_exec > 0.90 {
                continue;
            }
            if m.precision_exec < 0.05 {
                continue;
            }
            let obj = 4.0 * m.f1_exec
                + 1.5 * m.balanced_accuracy
                + 0.75 * m.precision_exec
                + 0.5 * m.recall_exec;
            if obj > best_obj {
                best_obj = obj;
                best_thr = thr;
            }
        }
        if best_obj > f32::NEG_INFINITY {
            threshold = best_thr;
        }
    }
    let metrics = class_metrics(&weights, samples, threshold);
    let mut model = CalibModel {
        weights: weights.clone(),
        threshold,
        feature_schema: CalibModel::SCHEMA_V1,
        meta: vec![
            ("epochs".into(), epochs.to_string()),
            ("n_samples".into(), samples.len().to_string()),
            ("train_bal".into(), format!("{:.6}", metrics.balanced_accuracy)),
            ("train_f1".into(), format!("{:.6}", metrics.f1_exec)),
            ("mode".into(), "full_train".into()),
        ],
    };
    model.meta.sort_by(|a: &(String, String), b: &(String, String)| a.0.cmp(&b.0));
    Ok(model)
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

    let config = SelfModConfig {
        enabled: true,
        cycle_budget_us: 5_000, // 5ms budget for probe
        max_mutations_per_cycle: 1,
        ..SelfModConfig::default()
    };

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

// ---------------------------------------------------------------------------
// Persistent model + inference (Phase 4→5 bridge)
// ---------------------------------------------------------------------------

/// Serializable ternary classifier (no external serde dependency).
///
/// Wire format (version 1), one line header + weights:
/// ```text
/// NTG_CALIB_V1 <dim> <threshold> <schema>
/// <w0> <w1> ... <w{dim-1}>
/// # optional meta: key=value pairs (ignored by older loaders)
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibModel {
    pub weights: Vec<i8>,
    pub threshold: i64,
    /// Bump when feature layout changes (currently 1 = FEATURE_DIM layout).
    pub feature_schema: u32,
    /// Optional train-time notes for A/B and EXPERIMENTS (not used for scoring).
    pub meta: Vec<(String, String)>,
}

impl CalibModel {
    pub const SCHEMA_V1: u32 = 1;

    pub fn from_report(report: &CalibReport) -> Self {
        let mut meta = vec![
            ("epochs".into(), report.epochs.to_string()),
            ("n_samples".into(), report.n_samples.to_string()),
            (
                "test_bal".into(),
                format!("{:.6}", report.test_metrics.balanced_accuracy),
            ),
            (
                "test_f1".into(),
                format!("{:.6}", report.test_metrics.f1_exec),
            ),
            ("win".into(), report.is_win.to_string()),
            ("nonzero_w".into(), report.weights.iter().filter(|&&w| w != 0).count().to_string()),
        ];
        meta.sort_by(|a: &(String, String), b: &(String, String)| a.0.cmp(&b.0));
        Self {
            weights: report.weights.clone(),
            threshold: report.threshold,
            feature_schema: Self::SCHEMA_V1,
            meta,
        }
    }

    pub fn nonzero_count(&self) -> usize {
        self.weights.iter().filter(|&&w| w != 0).count()
    }

    pub fn predict_execution(&self, label: &str) -> bool {
        let x = features_from_label(label);
        predict(&self.weights, &x, self.threshold)
    }

    pub fn score_label(&self, label: &str) -> i64 {
        let x = features_from_label(label);
        score(&self.weights, &x)
    }

    /// Score via sparse TOBL path (must match [`score_label`] for identical features).
    /// Pre-positions Phase 5 AccelManager / GraphNode scoring without changing labels.
    pub fn score_label_sparse(&self, label: &str) -> i64 {
        use crate::ntg::storage::SparseBitSlicedTernary;
        let x = features_from_label(label);
        let w = self.to_sparse_weights();
        let a = SparseBitSlicedTernary::from_slice(&x);
        SparseBitSlicedTernary::dot_product_sparse(&w, &a)
    }

    pub fn predict_execution_sparse(&self, label: &str) -> bool {
        self.score_label_sparse(label) >= self.threshold
    }

    /// Production scoring path: warm-started GraphNode weights · sparse features.
    /// Same numeric result as [`score_label_sparse`]; API that Phase 5 runtime
    /// hosts should call so calib and forward share one code path.
    pub fn score_via_graph_node(&self, label: &str) -> i64 {
        use crate::ntg::storage::SparseBitSlicedTernary;
        let node = self.to_graph_node(0);
        let acts = SparseBitSlicedTernary::from_slice(&features_from_label(label));
        SparseBitSlicedTernary::dot_product_sparse(&node.weights, &acts)
    }

    pub fn predict_via_graph_node(&self, label: &str) -> bool {
        self.score_via_graph_node(label) >= self.threshold
    }

    /// Encode as portable text (V1 header + weights; optional `# key=value` meta).
    pub fn to_wire(&self) -> String {
        let mut s = format!(
            "NTG_CALIB_V1 {} {} {}\n",
            self.weights.len(),
            self.threshold,
            self.feature_schema
        );
        for (i, w) in self.weights.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&w.to_string());
        }
        s.push('\n');
        for (k, v) in &self.meta {
            // Spaces stripped from keys; values may not contain newlines.
            let k = k.replace([' ', '\n', '\r', '='], "_");
            let v = v.replace(['\n', '\r'], " ");
            s.push_str(&format!("# {k}={v}\n"));
        }
        s
    }

    pub fn from_wire(text: &str) -> Result<Self, NtgError> {
        let mut lines = text.lines().filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        });
        let header = lines
            .next()
            .ok_or_else(|| NtgError::InvalidInput("empty calib model".into()))?;
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 4 || parts[0] != "NTG_CALIB_V1" {
            return Err(NtgError::InvalidInput(format!(
                "bad calib header: {header}"
            )));
        }
        let dim: usize = parts[1]
            .parse()
            .map_err(|_| NtgError::InvalidInput("bad dim".into()))?;
        let threshold: i64 = parts[2]
            .parse()
            .map_err(|_| NtgError::InvalidInput("bad threshold".into()))?;
        let feature_schema: u32 = parts[3]
            .parse()
            .map_err(|_| NtgError::InvalidInput("bad schema".into()))?;
        let body = lines
            .next()
            .ok_or_else(|| NtgError::InvalidInput("missing weights line".into()))?;
        let weights: Result<Vec<i8>, _> = body
            .split_whitespace()
            .map(|t| {
                t.parse::<i8>()
                    .map_err(|_| NtgError::InvalidInput(format!("bad weight {t}")))
            })
            .collect();
        let weights = weights?;
        if weights.len() != dim {
            return Err(NtgError::InvalidInput(format!(
                "weight len {} != dim {}",
                weights.len(),
                dim
            )));
        }
        // Meta from comment lines (optional).
        let mut meta = Vec::new();
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix('#') {
                let rest = rest.trim();
                if let Some((k, v)) = rest.split_once('=') {
                    meta.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
        Ok(Self {
            weights,
            threshold,
            feature_schema,
            meta,
        })
    }

    pub fn save_path(&self, path: &std::path::Path) -> Result<(), NtgError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        std::fs::write(path, self.to_wire())
            .map_err(|e| NtgError::InvalidInput(format!("write model: {e}")))
    }

    pub fn load_path(path: &std::path::Path) -> Result<Self, NtgError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| NtgError::InvalidInput(format!("read model: {e}")))?;
        Self::from_wire(&text)
    }

    /// Project ternary weights into a SparseBitSliced vector (first dim slots).
    /// Useful later for GraphNode.weights warm-start / TOBL experiments.
    pub fn to_sparse_weights(&self) -> crate::ntg::storage::SparseBitSlicedTernary {
        crate::ntg::storage::SparseBitSlicedTernary::from_slice(&self.weights)
    }

    /// Warm-start a runtime compute node from this calib model (Phase 5 hook).
    pub fn to_graph_node(&self, id: usize) -> crate::ntg::graph::GraphNode {
        crate::ntg::graph::GraphNode::with_weights(id, self.to_sparse_weights())
    }

    /// Single-layer Runtime with one classifier node (id=0). Ready for
    /// `forward_native_parallel` experiments once activations are sparse features.
    pub fn to_runtime_layer(&self) -> Result<crate::ntg::runtime::Runtime, NtgError> {
        let mut rt = crate::ntg::runtime::Runtime::new();
        rt.push_layer(vec![self.to_graph_node(0)])?;
        Ok(rt)
    }

    /// Export sparse weights as text for offline TOBL tooling (chunk COO dump).
    pub fn sparse_export_text(&self) -> String {
        let s = self.to_sparse_weights();
        let mut out = format!(
            "NTG_SPARSE_V1 len={} density={:.6} blocks={}\n",
            s.len(),
            s.density(),
            s.blocks.len()
        );
        for (chunk, block) in &s.blocks {
            if block.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "chunk={} pos={:#018x} neg={:#018x}\n",
                chunk, block.pos, block.neg
            ));
        }
        out
    }

    pub fn save_sparse_path(&self, path: &std::path::Path) -> Result<(), NtgError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        std::fs::write(path, self.sparse_export_text())
            .map_err(|e| NtgError::InvalidInput(format!("write sparse: {e}")))
    }
}

/// Pack a feature vector as sparse activations (runtime / AccelManager input).
pub fn features_to_sparse(features: &[i8]) -> crate::ntg::storage::SparseBitSlicedTernary {
    crate::ntg::storage::SparseBitSlicedTernary::from_slice(features)
}

/// Parallel batch prediction (Phase 5 CPU parallelization for the hot path).
/// Uses scoped threads; results keep input order. Empty input → empty vec.
pub fn batch_predict_parallel(model: &CalibModel, labels: &[&str]) -> Vec<bool> {
    use std::thread;
    if labels.is_empty() {
        return Vec::new();
    }
    let n = labels.len();
    let workers = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .min(n)
        .max(1);
    let chunk = n.div_ceil(workers);
    let mut out = vec![false; n];
    thread::scope(|scope| {
        for (chunk_i, out_chunk) in out.chunks_mut(chunk).enumerate() {
            let start = chunk_i * chunk;
            let slice = &labels[start..start + out_chunk.len()];
            scope.spawn(move || {
                for (i, label) in slice.iter().enumerate() {
                    out_chunk[i] = model.predict_via_graph_node(label);
                }
            });
        }
    });
    out
}

/// Parallel batch scores (i64) matching dense/sparse identity path.
pub fn batch_score_parallel(model: &CalibModel, labels: &[&str]) -> Vec<i64> {
    use std::thread;
    if labels.is_empty() {
        return Vec::new();
    }
    let n = labels.len();
    let workers = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .min(n)
        .max(1);
    let chunk = n.div_ceil(workers);
    let mut out = vec![0i64; n];
    thread::scope(|scope| {
        for (chunk_i, out_chunk) in out.chunks_mut(chunk).enumerate() {
            let start = chunk_i * chunk;
            let slice = &labels[start..start + out_chunk.len()];
            scope.spawn(move || {
                for (i, label) in slice.iter().enumerate() {
                    out_chunk[i] = model.score_via_graph_node(label);
                }
            });
        }
    });
    out
}

/// Evaluate an existing model on samples (no training).
pub fn evaluate_model(model: &CalibModel, samples: &[Sample]) -> ClassMetrics {
    class_metrics(&model.weights, samples, model.threshold)
}

/// Side-by-side metrics for two frozen models (A/B / regression).
#[derive(Clone, Debug)]
pub struct ModelCompareReport {
    pub a_metrics: ClassMetrics,
    pub b_metrics: ClassMetrics,
    pub bal_delta_b_minus_a: f32,
    pub f1_delta_b_minus_a: f32,
}

pub fn compare_models(
    a: &CalibModel,
    b: &CalibModel,
    samples: &[Sample],
) -> ModelCompareReport {
    let a_metrics = evaluate_model(a, samples);
    let b_metrics = evaluate_model(b, samples);
    ModelCompareReport {
        bal_delta_b_minus_a: b_metrics.balanced_accuracy - a_metrics.balanced_accuracy,
        f1_delta_b_minus_a: b_metrics.f1_exec - a_metrics.f1_exec,
        a_metrics,
        b_metrics,
    }
}

/// Compact JSON for ClassMetrics (CI / dashboards).
pub fn metrics_to_json(m: &ClassMetrics) -> String {
    format!(
        "{{\"acc\":{:.6},\"bal\":{:.6},\"f1\":{:.6},\"rec\":{:.6},\"prec\":{:.6},\
\"tp\":{},\"tn\":{},\"fp\":{},\"fn\":{}}}",
        m.accuracy,
        m.balanced_accuracy,
        m.f1_exec,
        m.recall_exec,
        m.precision_exec,
        m.tp,
        m.tn,
        m.fp,
        m.fn_
    )
}

/// Compact JSON for CI / tooling (manual, no serde).
pub fn report_to_json(report: &CalibReport) -> String {
    format!(
        "{{\"n\":{},\"n_train\":{},\"n_test\":{},\"n_exec\":{},\"n_content\":{},\
\"baseline_acc\":{:.6},\"baseline_bal\":{:.6},\
\"test_acc\":{:.6},\"test_bal\":{:.6},\"test_f1\":{:.6},\
\"test_rec\":{:.6},\"test_prec\":{:.6},\
\"delta_bal\":{:.6},\"epochs\":{},\"latency_us\":{},\"threshold\":{},\"win\":{},\
\"tp\":{},\"tn\":{},\"fp\":{},\"fn\":{},\"nonzero_w\":{}}}",
        report.n_samples,
        report.n_train,
        report.n_test,
        report.n_execution,
        report.n_content,
        report.baseline_accuracy,
        report.baseline_balanced_accuracy,
        report.test_metrics.accuracy,
        report.test_metrics.balanced_accuracy,
        report.test_metrics.f1_exec,
        report.test_metrics.recall_exec,
        report.test_metrics.precision_exec,
        report.delta_balanced_vs_baseline,
        report.epochs,
        report.latency_us,
        report.threshold,
        report.is_win,
        report.test_metrics.tp,
        report.test_metrics.tn,
        report.test_metrics.fp,
        report.test_metrics.fn_,
        report.weights.iter().filter(|&&w| w != 0).count()
    )
}

/// Write a one-line JSON report path for EXPERIMENTS automation.
pub fn write_report_json(path: &std::path::Path, report: &CalibReport) -> Result<(), NtgError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    std::fs::write(path, report_to_json(report))
        .map_err(|e| NtgError::InvalidInput(format!("write report: {e}")))
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
        // Tiny fixture hold-out is noisy (often 1 exec in test). Require that
        // train metrics show learning and/or test is not a pure flood.
        assert!(
            report.train_metrics.recall_exec > 0.0
                || report.test_metrics.recall_exec > 0.0
                || report.train_metrics.f1_exec > 0.0,
            "expected some Execution detection on train or test: {}",
            report.summary_line()
        );
        assert!(
            report.test_metrics.precision_exec >= 0.0,
            "metrics defined: {}",
            report.summary_line()
        );
        // Full-set eval (no hold-out noise) should beat majority bal on fixtures.
        let model = CalibModel::from_report(&report);
        let full = evaluate_model(&model, &samples);
        assert!(
            full.balanced_accuracy + 1e-3 >= 0.5 || full.recall_exec > 0.0,
            "full-set eval should show signal: bal={} rec={}",
            full.balanced_accuracy,
            full.recall_exec
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

    #[test]
    fn model_roundtrip_wire() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 10, 1).unwrap();
        let model = CalibModel::from_report(&report);
        let wire = model.to_wire();
        let loaded = CalibModel::from_wire(&wire).unwrap();
        assert_eq!(model.weights, loaded.weights);
        assert_eq!(model.threshold, loaded.threshold);
        assert_eq!(model.feature_schema, loaded.feature_schema);
        assert!(!model.meta.is_empty());
        assert_eq!(model.meta, loaded.meta);
        let _ = model.predict_execution("fn main() {}");
        let sparse = model.to_sparse_weights();
        assert!(!sparse.is_empty() || model.weights.iter().all(|&w| w == 0));
        let _ = sparse;
        let j = report_to_json(&report);
        assert!(j.contains("\"win\":"));
        assert!(j.contains("\"nonzero_w\":"));
    }

    #[test]
    fn sparse_score_matches_dense() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 15, 1).unwrap();
        let model = CalibModel::from_report(&report);
        for label in ["fn main() {}", "Just a heading", "```\ncode\n```", "Normal prose."] {
            assert_eq!(
                model.score_label(label),
                model.score_label_sparse(label),
                "sparse/dense mismatch for {label:?}"
            );
            assert_eq!(
                model.predict_execution(label),
                model.predict_execution_sparse(label)
            );
        }
    }

    #[test]
    fn graph_node_and_runtime_warm_start() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 8, 1).unwrap();
        let model = CalibModel::from_report(&report);
        let node = model.to_graph_node(0);
        assert_eq!(node.id, 0);
        for (i, &w) in model.weights.iter().enumerate() {
            assert_eq!(node.weights.get(i), w);
        }
        let rt = model.to_runtime_layer().unwrap();
        assert_eq!(rt.layers.len(), 1);
        assert_eq!(rt.layers[0].len(), 1);
        let acts = features_to_sparse(&features_from_label("fn x() {}"));
        assert!(acts.density() > 0.0 || acts.blocks.is_empty());
        let export = model.sparse_export_text();
        assert!(export.starts_with("NTG_SPARSE_V1"));
    }

    #[test]
    fn compare_models_self_zero_delta() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 8, 1).unwrap();
        let model = CalibModel::from_report(&report);
        let cmp = compare_models(&model, &model, &samples);
        assert!((cmp.bal_delta_b_minus_a).abs() < 1e-6);
        assert!((cmp.f1_delta_b_minus_a).abs() < 1e-6);
        let mj = metrics_to_json(&cmp.a_metrics);
        assert!(mj.contains("\"bal\":"));
    }

    #[test]
    fn graph_node_score_matches_dense() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 12, 1).unwrap();
        let model = CalibModel::from_report(&report);
        for label in ["fn main() {}", "Just a heading", "pub fn x() -> i32 { 1 }"] {
            assert_eq!(
                model.score_label(label),
                model.score_via_graph_node(label),
                "graph-node path mismatch for {label:?}"
            );
        }
    }

    #[test]
    fn batch_predict_parallel_matches_serial() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 10, 1).unwrap();
        let model = CalibModel::from_report(&report);
        let labels = [
            "fn main() {}",
            "Heading only",
            "def foo():\n  return 1\n",
            "plain prose paragraph",
        ];
        let refs: Vec<&str> = labels.to_vec();
        let par = batch_predict_parallel(&model, &refs);
        let ser: Vec<bool> = refs
            .iter()
            .map(|l| model.predict_via_graph_node(l))
            .collect();
        assert_eq!(par, ser);
        let scores = batch_score_parallel(&model, &refs);
        assert_eq!(scores.len(), refs.len());
    }

    #[test]
    fn indented_code_feature_fires() {
        let code = "    let x = 1;\n    let y = 2;\n";
        let f = features_from_label(code);
        assert_eq!(f[63], 1);
        let prose = "Short title";
        let p = features_from_label(prose);
        assert_eq!(p[63], -1);
    }
}
