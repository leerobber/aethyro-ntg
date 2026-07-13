//! Rung 3: Language / SIS organ linked into SovereignBrain working-set activate.
//!
//! Ingests markdown docs via `docparse` into an NTG [`Graph`], optionally
//! trains a Phase 4 [`CalibModel`], and maps free-text queries into the same
//! 8-dim signature space used by genomic LTM activation — so a language
//! query can light up haplotype motifs / neurons without a separate RAG stack.

use crate::ntg::calib::{
    calibrate, features_from_label, samples_from_documents, samples_from_graph, CalibModel,
    CalibReport, Sample, FEATURE_DIM,
};
use crate::ntg::docparse;
use crate::ntg::error::NtgError;
use crate::ntg::graph::{Graph, NodeId, NodeKind};

/// Language/SIS organ: document graph + optional calib scorer.
#[derive(Clone, Debug)]
pub struct LanguageOrgan {
    pub graph: Graph,
    pub model: Option<CalibModel>,
    /// Last calib report summary metrics (if trained).
    pub last_test_bal: f32,
    pub last_test_f1: f32,
    pub docs_ingested: u32,
    pub last_query: String,
    pub last_signature: [f32; 8],
    /// Node ids activated by the last text query (content/exec leaves).
    pub last_active_nodes: Vec<NodeId>,
}

impl Default for LanguageOrgan {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageOrgan {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            model: None,
            last_test_bal: 0.0,
            last_test_f1: 0.0,
            docs_ingested: 0,
            last_query: String::new(),
            last_signature: [0.0; 8],
            last_active_nodes: Vec::new(),
        }
    }

    /// Parse a document into the organ graph.
    pub fn ingest_document(&mut self, label: &str, text: &str) -> NodeId {
        let root = docparse::parse_into(&mut self.graph, label, text);
        self.docs_ingested = self.docs_ingested.saturating_add(1);
        root
    }

    /// Ingest several documents.
    pub fn ingest_documents(&mut self, docs: &[(&str, &str)]) {
        for &(label, text) in docs {
            self.ingest_document(label, text);
        }
    }

    /// Train calib on the organ's current graph; installs model on success.
    pub fn train_calib_from_graph(&mut self, epochs: usize) -> Result<CalibReport, NtgError> {
        let samples = samples_from_graph(&self.graph)?;
        if samples.len() < 4 {
            return Err(NtgError::InvalidInput(
                "language organ graph too small for calib".into(),
            ));
        }
        let report = calibrate(&samples, epochs, 0)?;
        self.last_test_bal = report.test_metrics.balanced_accuracy;
        self.last_test_f1 = report.test_metrics.f1_exec;
        self.model = Some(CalibModel::from_report(&report));
        Ok(report)
    }

    /// Train calib from explicit doc list (does not require prior ingest).
    pub fn train_calib_from_docs(
        &mut self,
        docs: &[(&str, &str)],
        epochs: usize,
    ) -> Result<CalibReport, NtgError> {
        self.ingest_documents(docs);
        let samples = samples_from_documents(docs)?;
        let report = calibrate(&samples, epochs, 0)?;
        self.last_test_bal = report.test_metrics.balanced_accuracy;
        self.last_test_f1 = report.test_metrics.f1_exec;
        self.model = Some(CalibModel::from_report(&report));
        Ok(report)
    }

    /// Train on built-in fixtures (always available, no filesystem).
    pub fn train_calib_fixtures(&mut self, epochs: usize) -> Result<CalibReport, NtgError> {
        let docs = fixture_docs();
        self.train_calib_from_docs(&docs, epochs)
    }

    /// Map free text → 8-dim genomic-compatible activation signature.
    pub fn text_to_signature(text: &str) -> [f32; 8] {
        let feats = features_from_label(text);
        let mut sig = [0.0f32; 8];
        for (i, &v) in feats.iter().enumerate().take(FEATURE_DIM) {
            sig[i % 8] += v as f32;
        }
        // Cheap content priors into unused headroom.
        let lower = text.to_ascii_lowercase();
        if lower.contains("snp") || lower.contains("ld") || lower.contains("haplotype") {
            sig[0] += 2.0;
            sig[6] += 1.5;
        }
        if lower.contains("chr") || lower.contains("chromosome") {
            // crude chr digit pull
            for c in 1..=22u8 {
                let token = format!("chr{c}");
                if lower.contains(&token) {
                    sig[3] = c as f32 / 22.0;
                }
            }
        }
        if lower.contains("fn ") || lower.contains("```") || lower.contains("pub ") {
            sig[7] += 2.0; // code-like
        }
        // L2 normalize for cosine with LTM motifs.
        let mut n2 = 0.0f32;
        for v in &sig {
            n2 += *v * *v;
        }
        let n = n2.sqrt().max(1e-6);
        for v in &mut sig {
            *v /= n;
        }
        sig
    }

    /// Score a label with the installed calib model (if any).
    pub fn score_text(&self, text: &str) -> Option<(i64, bool)> {
        let model = self.model.as_ref()?;
        let s = model.score_label(text);
        Some((s, model.predict_execution(text)))
    }

    /// Holdout accuracy of installed model on samples (task axis input).
    pub fn holdout_accuracy(&self, samples: &[Sample]) -> f32 {
        let Some(model) = self.model.as_ref() else {
            return 0.0;
        };
        if samples.is_empty() {
            return 0.0;
        }
        let mut ok = 0usize;
        for s in samples {
            let pred = model.predict_execution(&s.label_preview);
            if pred == s.is_execution {
                ok += 1;
            }
        }
        ok as f32 / samples.len() as f32
    }

    /// Balanced accuracy on holdout samples (preferred task metric).
    pub fn holdout_balanced_accuracy(&self, samples: &[Sample]) -> f32 {
        let Some(model) = self.model.as_ref() else {
            return 0.0;
        };
        if samples.is_empty() {
            return 0.0;
        }
        let mut tp = 0usize;
        let mut tn = 0usize;
        let mut fp = 0usize;
        let mut fn_ = 0usize;
        for s in samples {
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

    /// Activate language leaves related to query; returns node ids (by simple score).
    pub fn activate_nodes(&mut self, query: &str, budget: usize) -> &[NodeId] {
        self.last_query = query.to_string();
        self.last_signature = Self::text_to_signature(query);
        let mut scored: Vec<(NodeId, i64)> = Vec::new();
        for id in self.graph.all_node_ids() {
            let Ok(node) = self.graph.node(id) else {
                continue;
            };
            let label = &node.label;
            if label.is_empty() {
                continue;
            }
            let score = if let Some(model) = &self.model {
                model.score_label(label).abs()
            } else {
                // overlap score with query tokens
                let q = query.to_ascii_lowercase();
                let l = label.to_ascii_lowercase();
                let mut s = 0i64;
                for tok in q.split(|c: char| !c.is_alphanumeric()) {
                    if tok.len() > 2 && l.contains(tok) {
                        s += 1;
                    }
                }
                s
            };
            if score > 0 {
                scored.push((id, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        self.last_active_nodes = scored
            .into_iter()
            .take(budget.max(1))
            .map(|(id, _)| id)
            .collect();
        &self.last_active_nodes
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn execution_node_count(&self) -> usize {
        self.graph
            .all_node_ids()
            .into_iter()
            .filter(|&id| {
                self.graph
                    .node(id)
                    .map(|n| n.kind == NodeKind::Execution)
                    .unwrap_or(false)
            })
            .count()
    }
}

/// Small real-shaped fixture corpus for offline calib.
pub fn fixture_docs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "readme",
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
            "design",
            r#"# Design
## Layers
Text only section about architecture.
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
            "kernel",
            r#"# Kernel notes
## API
Normal prose about ternary graphs and LD blocks.
## Snippet
```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```
## Biology
Chromosome brains use haplotype LD synapses for structure.
"#,
        ),
        (
            "ops",
            r#"# Ops
## Run
```bash
cargo test
cargo run --release --bin ntg_school
```
## Docs
See ROADMAP and EXPERIMENTS for measured claims.
"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_builds_graph() {
        let mut organ = LanguageOrgan::new();
        organ.ingest_document("t", "# A\n```\nfn x(){}\n```\n");
        assert!(organ.node_count() >= 2);
        assert!(organ.execution_node_count() >= 1);
    }

    #[test]
    fn text_signature_is_normalized() {
        let s = LanguageOrgan::text_to_signature("fn main() { haplotype LD on chr22 }");
        let n: f32 = s.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-3, "norm={n}");
        assert!(s[0] > 0.0 || s[6] > 0.0); // genomic tokens
    }

    #[test]
    fn train_fixtures_installs_model() {
        let mut organ = LanguageOrgan::new();
        let report = organ.train_calib_fixtures(25).unwrap();
        assert!(organ.model.is_some());
        assert!(report.n_samples > 5);
        let (score, _pred) = organ.score_text("fn main() {}").unwrap();
        let _ = score;
    }

    #[test]
    fn activate_nodes_returns_budget() {
        let mut organ = LanguageOrgan::new();
        organ.ingest_documents(&fixture_docs());
        let nodes = organ.activate_nodes("cargo test release", 4);
        assert!(!nodes.is_empty());
        assert!(nodes.len() <= 4);
    }
}
