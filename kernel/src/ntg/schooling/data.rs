//! Real datasets for schooling — sourced from the live repository tree.
//!
//! Nothing mythical: every document is read from disk; every math problem
//! has a closed-form expected answer; every path is a real path shape used
//! by the kernel or docs layout.

use crate::ntg::error::NtgError;
use std::fs;
use std::path::{Path, PathBuf};

/// One real markdown document loaded from the repo.
#[derive(Clone, Debug)]
pub struct RealDoc {
    pub rel_path: String,
    pub text: String,
    pub bytes: usize,
}

/// Campaign-wide data root (repo `docs/` + optional extra corpus).
#[derive(Clone, Debug)]
pub struct SchoolDataRoot {
    pub docs_dir: PathBuf,
    pub repo_root: PathBuf,
}

impl SchoolDataRoot {
    pub fn from_docs_dir(docs: impl AsRef<Path>) -> Result<Self, NtgError> {
        let docs_dir = docs.as_ref().to_path_buf();
        if !docs_dir.is_dir() {
            return Err(NtgError::InvalidInput(format!(
                "docs dir missing: {}",
                docs_dir.display()
            )));
        }
        let repo_root = docs_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| docs_dir.clone());
        Ok(Self {
            docs_dir,
            repo_root,
        })
    }

    /// Load real engineering markdown under docs/ (depth 2).
    ///
    /// Excludes `schooling/**` (curriculum + generated runs) so calib/school
    /// learn from design/ADR/phase certificates — not self-referential school text.
    pub fn load_markdown_corpus(&self) -> Result<Vec<RealDoc>, NtgError> {
        let mut out = Vec::new();
        load_md_recursive(&self.docs_dir, &self.docs_dir, &mut out, 0)?;
        out.retain(|d| {
            !d.rel_path.starts_with("schooling/") && !d.rel_path.contains("schooling/")
        });
        out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        if out.is_empty() {
            return Err(NtgError::InvalidInput(
                "no markdown files found under docs/ (after excluding schooling/)".into(),
            ));
        }
        Ok(out)
    }

    /// Deterministic train/holdout split.
    /// Stratifies by presence of fenced code (```) so both splits can contain
    /// Execution nodes — plain path sort was putting all fences in train.
    pub fn split_docs(docs: &[RealDoc], train_ratio: f32) -> (Vec<RealDoc>, Vec<RealDoc>) {
        if docs.is_empty() {
            return (vec![], vec![]);
        }
        if docs.len() == 1 {
            return (docs.to_vec(), docs.to_vec());
        }
        let mut with_fence: Vec<RealDoc> = docs
            .iter()
            .filter(|d| d.text.contains("```"))
            .cloned()
            .collect();
        let mut no_fence: Vec<RealDoc> = docs
            .iter()
            .filter(|d| !d.text.contains("```"))
            .cloned()
            .collect();
        with_fence.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        no_fence.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        let split_group = |v: &[RealDoc]| -> (Vec<RealDoc>, Vec<RealDoc>) {
            if v.is_empty() {
                return (vec![], vec![]);
            }
            if v.len() == 1 {
                // Put the only fenced/non-fenced doc in both so holdout can see it.
                return (v.to_vec(), v.to_vec());
            }
            let mut n_train = ((v.len() as f32) * train_ratio).round() as usize;
            n_train = n_train.clamp(1, v.len() - 1);
            (v[..n_train].to_vec(), v[n_train..].to_vec())
        };

        let (tw, hw) = split_group(&with_fence);
        let (tn, hn) = split_group(&no_fence);
        let mut train = tw;
        train.extend(tn);
        let mut hold = hw;
        hold.extend(hn);
        train.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        hold.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        if hold.is_empty() && !train.is_empty() {
            hold.push(train[0].clone());
        }
        if train.is_empty() && !hold.is_empty() {
            train.push(hold[0].clone());
        }
        (train, hold)
    }
}

fn load_md_recursive(
    root: &Path,
    dir: &Path,
    out: &mut Vec<RealDoc>,
    depth: usize,
) -> Result<(), NtgError> {
    if depth > 3 {
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| NtgError::InvalidInput(e.to_string()))?;
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() {
            load_md_recursive(root, &p, out, depth + 1)?;
        } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            // Exclude generated run notebooks (would pollute train/holdout).
            if rel.contains("schooling/runs/") {
                continue;
            }
            let text = fs::read_to_string(&p)
                .map_err(|e| NtgError::InvalidInput(format!("read {}: {e}", p.display())))?;
            let bytes = text.len();
            out.push(RealDoc {
                rel_path: rel,
                text,
                bytes,
            });
        }
    }
    Ok(())
}

/// Closed-form ternary matmul problem with precomputed expected output.
/// All matrices use only {-1,0,1}; expected C is exact integer in f32.
#[derive(Clone, Debug)]
pub struct MatmulProblem {
    pub id: String,
    pub m: usize,
    pub k: usize,
    pub n: usize,
    pub a: Vec<i8>,
    pub b: Vec<i8>,
    pub expected: Vec<f32>,
}

/// Real Phase-1 study/exam math corpus (hand-derived, verifiable by pen).
pub fn phase1_matmul_problems() -> Vec<MatmulProblem> {
    // (1x2) @ (2x1) = [1]
    // a=[1,0] b=[1, -1]^T -> 1*1 + 0*(-1) = 1
    let p1 = MatmulProblem {
        id: "mm_1x2x1_simple".into(),
        m: 1,
        k: 2,
        n: 1,
        a: vec![1, 0],
        b: vec![1, -1],
        expected: vec![1.0],
    };
    // (2x2) @ (2x2)
    // A = [[1,-1],[0,1]] B = [[1,0],[-1,1]]
    // C00 = 1*1 + (-1)*(-1) = 2
    // C01 = 1*0 + (-1)*1 = -1
    // C10 = 0*1 + 1*(-1) = -1
    // C11 = 0*0 + 1*1 = 1
    let p2 = MatmulProblem {
        id: "mm_2x2x2_hand".into(),
        m: 2,
        k: 2,
        n: 2,
        a: vec![1, -1, 0, 1],
        b: vec![1, 0, -1, 1],
        expected: vec![2.0, -1.0, -1.0, 1.0],
    };
    // (3x3) @ (3x1) — identity-ish
    let p3 = MatmulProblem {
        id: "mm_3x3x1_sparse".into(),
        m: 3,
        k: 3,
        n: 1,
        a: vec![1, 0, 0, 0, 1, 0, 0, 0, 1],
        b: vec![1, -1, 0],
        expected: vec![1.0, -1.0, 0.0],
    };
    // (2x4) @ (4x2)
    let p4 = MatmulProblem {
        id: "mm_2x4x2".into(),
        m: 2,
        k: 4,
        n: 2,
        a: vec![1, 1, -1, 0, 0, -1, 1, 1],
        b: vec![1, 0, 0, 1, -1, 0, 0, -1],
        // row0 · col0 = 1+0+1+0 = 2
        // row0 · col1 = 0+1+0+0 = 1
        // row1 · col0 = 0+0-1+0 = -1
        // row1 · col1 = 0-1+0-1 = -2
        expected: vec![2.0, 1.0, -1.0, -2.0],
    };
    // Zero product
    let p5 = MatmulProblem {
        id: "mm_zero".into(),
        m: 2,
        k: 2,
        n: 2,
        a: vec![0, 0, 0, 0],
        b: vec![1, -1, 1, -1],
        expected: vec![0.0, 0.0, 0.0, 0.0],
    };
    // Outer-product style
    let p6 = MatmulProblem {
        id: "mm_1x3x1".into(),
        m: 1,
        k: 3,
        n: 1,
        a: vec![1, -1, 1],
        b: vec![1, 1, -1],
        expected: vec![-1.0], // 1 -1 -1 = -1
    };
    // Holdout-style larger
    let p7 = MatmulProblem {
        id: "mm_4x4x1_holdout".into(),
        m: 4,
        k: 4,
        n: 1,
        a: vec![
            1, 0, -1, 0, //
            0, 1, 0, -1, //
            -1, 0, 1, 0, //
            0, -1, 0, 1,
        ],
        b: vec![1, 1, 1, 1],
        expected: vec![0.0, 0.0, 0.0, 0.0],
    };
    let p8 = MatmulProblem {
        id: "mm_2x3x2_holdout".into(),
        m: 2,
        k: 3,
        n: 2,
        a: vec![1, -1, 0, 0, 1, -1],
        b: vec![1, -1, 1, 0, 0, 1],
        // r0c0=1-1+0=0 r0c1=-1+0+0=-1 r1c0=0+1+0=1 r1c1=0+0-1=-1
        expected: vec![0.0, -1.0, 1.0, -1.0],
    };
    vec![p1, p2, p3, p4, p5, p6, p7, p8]
}

/// f32 encode problems: real finite weights with known absmean encoding.
#[derive(Clone, Debug)]
pub struct EncodeProblem {
    pub id: String,
    pub weights: Vec<f32>,
    /// Expected ternary from `encode` (computed offline / verified in study).
    pub expected: Vec<i8>,
}

pub fn phase1_encode_problems() -> Vec<EncodeProblem> {
    // mean_abs([2,0,-2])=4/3, thr=2/3 → [1,0,-1]
    let e1 = EncodeProblem {
        id: "enc_symmetric".into(),
        weights: vec![2.0, 0.0, -2.0],
        expected: vec![1, 0, -1],
    };
    // all small → zeros if below thr
    let e2 = EncodeProblem {
        id: "enc_near_zero".into(),
        weights: vec![0.01, -0.01, 0.0],
        expected: {
            // mean_abs ≈ 0.00667, thr ≈ 0.00333 → [1,-1,0]
            vec![1, -1, 0]
        },
    };
    let e3 = EncodeProblem {
        id: "enc_unit".into(),
        weights: vec![1.0, -1.0, 0.5, -0.5],
        expected: {
            // mean_abs = 0.75, thr = 0.375 → [1,-1,1,-1]
            vec![1, -1, 1, -1]
        },
    };
    let e4 = EncodeProblem {
        id: "enc_empty".into(),
        weights: vec![],
        expected: vec![],
    };
    let e5 = EncodeProblem {
        id: "enc_holdout_ramp".into(),
        weights: vec![-3.0, -1.0, 0.0, 1.0, 3.0],
        expected: {
            // mean_abs = 8/5=1.6, thr=0.8 → [-1,-1,0,1,1]
            vec![-1, -1, 0, 1, 1]
        },
    };
    vec![e1, e2, e3, e4, e5]
}

/// Real filesystem-ish paths for pathparse schooling (not invented FS events).
pub fn phase2_path_corpus() -> Vec<&'static str> {
    vec![
        "kernel/src/ntg/ternary.rs",
        "kernel/src/ntg/graph/mod.rs",
        "kernel/src/bin/phase4_calib.rs",
        "docs/DESIGN.md",
        "docs/architecture/0001-vision-and-pivot.md",
        "tools/dev.sh",
        "README.md",
        "kernel/Cargo.toml",
        "docs/phases/PHASE_1_COMPLETE.md",
        "kernel/src/ntg/calib/mod.rs",
    ]
}

/// Known required repo artifacts for Phase 0 (existence checks).
pub fn phase0_required_paths() -> Vec<&'static str> {
    vec![
        "docs/ROADMAP.md",
        "docs/STATUS.md",
        "docs/DESIGN.md",
        "docs/PHASE_GATE_PROTOCOL.md",
        "docs/architecture/0001-vision-and-pivot.md",
        "docs/architecture/0002-safety-rails-for-self-modification.md",
        "docs/phases/PHASE_0_COMPLETE.md",
        "docs/phases/PHASE_1_COMPLETE.md",
        "kernel/Cargo.toml",
        "kernel/src/lib.rs",
        "kernel/src/ntg/ternary.rs",
        ".github/workflows/ci.yml",
        "LICENSE",
        "README.md",
        "CONTRIBUTING.md",
    ]
}

/// Dataset manifest line for notebooks.
pub fn corpus_manifest(docs: &[RealDoc]) -> String {
    let total_bytes: usize = docs.iter().map(|d| d.bytes).sum();
    let mut s = format!(
        "n_docs={} total_bytes={} (real files under docs/)\n",
        docs.len(),
        total_bytes
    );
    for d in docs {
        s.push_str(&format!("- {} ({} B)\n", d.rel_path, d.bytes));
    }
    s
}
