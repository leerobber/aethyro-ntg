//! Phase 4 calibration: doc-graph NodeKind classifier (ADR 0006).
//!
//! Real structural labels from `docparse` (Execution vs Content).
//! Ternary features + ternary weights; perceptron-style calibration.
//! Self-modification remains off; optional ledger snapshot of weights.

use crate::ntg::docparse;
use crate::ntg::error::NtgError;
use crate::ntg::glyph::extract_glyph_fingerprint;
use crate::ntg::graph::{Graph, NodeKind};
use crate::ntg::leafsignal::extract_leaf_signal;
use crate::ntg::ledger::{
    replay::ExecutionTrace, FitnessMeasure, MutationOutcome, TamperEvidentLedger,
};
use crate::ntg::ternary::encode_fixed;
use std::time::Instant;

/// Feature width (must match encode_fixed pad + side channels packing).
pub const FEATURE_DIM: usize = 64;

/// One labeled sample from a parsed graph node.
#[derive(Clone, Debug)]
pub struct Sample {
    pub features: Vec<i8>,
    /// true = Execution, false = Content
    pub is_execution: bool,
    pub label_preview: String,
}

/// Result of a full calibration run.
#[derive(Clone, Debug)]
pub struct CalibReport {
    pub n_samples: usize,
    pub n_execution: usize,
    pub n_content: usize,
    pub baseline_accuracy: f32,
    pub before_accuracy: f32,
    pub after_accuracy: f32,
    pub delta_vs_baseline: f32,
    pub epochs: usize,
    pub latency_us: u64,
    pub threshold: i64,
    pub is_win: bool,
    pub weights: Vec<i8>,
}

impl CalibReport {
    pub fn summary_line(&self) -> String {
        format!(
            "n={} exec={} content={} baseline={:.3} before={:.3} after={:.3} delta={:+.3} epochs={} latency_us={} win={}",
            self.n_samples,
            self.n_execution,
            self.n_content,
            self.baseline_accuracy,
            self.before_accuracy,
            self.after_accuracy,
            self.delta_vs_baseline,
            self.epochs,
            self.latency_us,
            self.is_win
        )
    }
}

/// Build ternary feature vector for a node label (deterministic).
pub fn features_from_label(label: &str) -> Vec<i8> {
    let mut feat = vec![0i8; FEATURE_DIM];

    // Primary: fixed-threshold ternary of label bytes (encode_fixed on &str)
    let tern = encode_fixed(label);
    for (i, &t) in tern.iter().take(48).enumerate() {
        feat[i] = t;
    }

    // Side channels: leaf signal ratios → ternary
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

    // Glyph fingerprint bits → ternary
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
    // Remaining slots: diversity / length signal
    feat[61] = if g.class_diversity >= 3 { 1 } else { 0 };
    feat[62] = if label.contains("```") || label.contains("fn ") || label.contains("->") {
        1
    } else {
        0
    };
    feat[63] = if label.len() > 40 { 1 } else if label.len() < 8 { -1 } else { 0 };

    feat
}

/// Collect samples from a graph (all nodes except optional skip of pure roots).
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

fn accuracy(weights: &[i8], samples: &[Sample], threshold: i64) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let correct = samples
        .iter()
        .filter(|s| predict(weights, &s.features, threshold) == s.is_execution)
        .count();
    correct as f32 / samples.len() as f32
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

/// Majority-class baseline accuracy (always Content).
pub fn baseline_accuracy(samples: &[Sample]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let content = samples.iter().filter(|s| !s.is_execution).count();
    content as f32 / samples.len() as f32
}

/// Run ternary perceptron calibration.
pub fn calibrate(
    samples: &[Sample],
    epochs: usize,
    threshold: i64,
) -> Result<CalibReport, NtgError> {
    if samples.is_empty() {
        return Err(NtgError::InvalidInput("no calibration samples".into()));
    }

    let t0 = Instant::now();
    let mut weights = vec![0i8; FEATURE_DIM];
    let baseline = baseline_accuracy(samples);
    let before = accuracy(&weights, samples, threshold);

    for _ in 0..epochs {
        for s in samples {
            let pred = predict(&weights, &s.features, threshold);
            if pred != s.is_execution {
                let y: i32 = if s.is_execution { 1 } else { -1 };
                for i in 0..FEATURE_DIM {
                    let x = s.features[i] as i32;
                    let w = weights[i] as i32;
                    // Move weights toward correct class
                    weights[i] = clamp_ternary(w + y * x);
                }
            }
        }
    }

    let after = accuracy(&weights, samples, threshold);
    let n_exec = samples.iter().filter(|s| s.is_execution).count();
    let latency_us = t0.elapsed().as_micros() as u64;
    let delta = after - baseline;
    // Win: beats majority baseline by any positive margin
    let is_win = after > baseline + 1e-6;

    Ok(CalibReport {
        n_samples: samples.len(),
        n_execution: n_exec,
        n_content: samples.len() - n_exec,
        baseline_accuracy: baseline,
        before_accuracy: before,
        after_accuracy: after,
        delta_vs_baseline: delta,
        epochs,
        latency_us,
        threshold,
        is_win,
        weights,
    })
}

/// Optional: snapshot weights into ledger for audit (not topology self-mod).
pub fn ledger_weight_snapshot(
    ledger: &mut TamperEvidentLedger,
    report: &CalibReport,
    timestamp: u64,
) -> Result<u64, NtgError> {
    let nonzero = report.weights.iter().filter(|&&w| w != 0).count();
    ledger.log_mutation(
        format!(
            "phase4_calib_snapshot win={} after={:.4} baseline={:.4} nonzero_w={} n={}",
            report.is_win,
            report.after_accuracy,
            report.baseline_accuracy,
            nonzero,
            report.n_samples
        ),
        0,
        report.after_accuracy.to_bits() as u64,
        FitnessMeasure {
            latency_us: report.latency_us,
            memory_bytes: (report.weights.len() * 1) as u64,
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

/// Built-in real-structure fixtures (markdown with fences) for offline CI.
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_dim() {
        let f = features_from_label("```rust\nfn x(){}\n```");
        assert_eq!(f.len(), FEATURE_DIM);
        assert!(f.iter().any(|&x| x != 0));
    }

    #[test]
    fn fixtures_yield_both_classes() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        assert!(samples.len() > 5);
        assert!(samples.iter().any(|s| s.is_execution));
        assert!(samples.iter().any(|s| !s.is_execution));
    }

    #[test]
    fn calibration_runs_and_reports() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 20, 1).unwrap();
        assert_eq!(report.n_samples, samples.len());
        assert!(report.after_accuracy >= 0.0 && report.after_accuracy <= 1.0);
        assert_eq!(report.weights.len(), FEATURE_DIM);
        // Zero weights before training ≈ not better than chance on mixed;
        // after training should not crash and should be finite.
        let _ = report.summary_line();
    }

    #[test]
    fn ledger_snapshot_ok() {
        let samples = samples_from_documents(&fixture_documents()).unwrap();
        let report = calibrate(&samples, 5, 1).unwrap();
        let mut ledger = TamperEvidentLedger::new(None).unwrap();
        ledger_weight_snapshot(&mut ledger, &report, 1).unwrap();
        ledger.verify_full_ledger().unwrap();
    }
}
