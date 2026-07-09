//! Dynamic byte/codepoint merge (Phase 2 cost mitigation — MrT5-inspired).
//!
//! MrT5 learns merges; this module implements a **deterministic, untrained**
//! merge pass so we can *measure* sequence-length reduction on real text
//! without claiming learned quality.
//!
//! Rules (v0):
//! 1. Collapse runs of whitespace to a single space.
//! 2. Merge consecutive identical letters (e.g. "bookkeeper" keeps doubles
//!    only up to 2 — actually: collapse runs of length > 2 of same char).
//! 3. Drop zero-width / control characters except newline → space.
//!
//! Output is still lossless enough for structure signals (case/punct kept);
//! it is **not** a reversible compression codec.

/// Result of a merge pass.
#[derive(Clone, Debug, PartialEq)]
pub struct ByteMergeReport {
    pub input_codepoints: usize,
    pub output_codepoints: usize,
    pub merged: String,
}

impl ByteMergeReport {
    /// Fraction of codepoints removed in `[0, 1]`.
    pub fn reduction_ratio(&self) -> f32 {
        if self.input_codepoints == 0 {
            return 0.0;
        }
        1.0 - (self.output_codepoints as f32 / self.input_codepoints as f32)
    }
}

/// Apply deterministic merge rules. Pure function.
pub fn merge_codepoints(input: &str) -> ByteMergeReport {
    let input_codepoints = input.chars().count();
    let mut out = String::with_capacity(input.len());
    let mut prev: Option<char> = None;
    let mut run = 0usize;
    let mut in_space_run = false;

    for c in input.chars() {
        if c.is_control() && c != '\n' && c != '\t' {
            continue; // drop most controls
        }
        let c = if c == '\n' || c == '\t' { ' ' } else { c };

        if c.is_whitespace() {
            if !in_space_run {
                out.push(' ');
                in_space_run = true;
                prev = Some(' ');
                run = 1;
            }
            continue;
        }
        in_space_run = false;

        if prev == Some(c) && c.is_alphabetic() {
            run += 1;
            if run <= 2 {
                out.push(c);
            }
            // run > 2: skip further duplicates
        } else {
            out.push(c);
            prev = Some(c);
            run = 1;
        }
    }

    let output_codepoints = out.chars().count();
    ByteMergeReport {
        input_codepoints,
        output_codepoints,
        merged: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace() {
        let r = merge_codepoints("a   \t\n  b");
        assert_eq!(r.merged, "a b");
        assert!(r.reduction_ratio() > 0.0);
    }

    #[test]
    fn collapses_long_letter_runs() {
        let r = merge_codepoints("waaaay");
        // a run of 4 → keep 2
        assert_eq!(r.merged, "waay");
    }

    #[test]
    fn empty() {
        let r = merge_codepoints("");
        assert_eq!(r.input_codepoints, 0);
        assert_eq!(r.output_codepoints, 0);
    }
}
