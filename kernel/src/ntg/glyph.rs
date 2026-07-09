//! Glyph geometry fingerprint v0 (ADR 0003 — honest scope).
//!
//! This is **not** a trained PIXEL model and does **not** render text as
//! images. PIXEL (ICLR 2023) requires a visual encoder that is not in this
//! repo. What this module provides instead:
//!
//! - A **deterministic**, frozen per-string fingerprint derived from
//!   Unicode shape-class buckets (width/height proxies + category).
//! - Guarantees: same input bytes/codepoints → same fingerprint; no
//!   network; no randomness; fully offline.
//! - Explicit non-claim: not glyph-geometry learned from raster images.
//!
//! Purpose: close the Phase 2 SIS checklist with a real, tested signal
//! channel that later Phase 4+ work can replace with a true PIXEL-lite
//! extractor without changing the `GlyphFingerprint` attachment site on
//! graph leaves.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Compact fingerprint attached to a leaf (identity and/or resolved body).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlyphFingerprint {
    /// SipHash over ordered shape-class tokens (reproducible, not crypto).
    pub shape_hash: u64,
    /// Count of distinct shape classes seen (diversity proxy).
    pub class_diversity: u32,
    /// Total codepoints hashed.
    pub codepoint_count: u32,
    /// Schema version so future PIXEL-lite can bump without silent mixups.
    pub schema_version: u32,
}

impl GlyphFingerprint {
    pub const SCHEMA_V0: u32 = 0;

    pub fn is_empty(&self) -> bool {
        self.codepoint_count == 0
    }
}

/// Coarse geometric / category class for a codepoint (v0 hand rules).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum ShapeClass {
    Space = 0,
    NarrowAscii = 1,
    WideEastAsian = 2,
    Combining = 3,
    Digit = 4,
    Punct = 5,
    Symbol = 6,
    Letter = 7,
    Other = 8,
}

fn classify(c: char) -> ShapeClass {
    if c.is_whitespace() {
        return ShapeClass::Space;
    }
    if c.is_ascii_digit() {
        return ShapeClass::Digit;
    }
    if c.is_ascii_punctuation() {
        return ShapeClass::Punct;
    }
    if c.is_ascii_alphabetic() {
        return ShapeClass::Letter;
    }
    // Combining marks (very rough)
    if ('\u{0300}'..='\u{036F}').contains(&c) {
        return ShapeClass::Combining;
    }
    // Common wide CJK block start
    if ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3040}'..='\u{30FF}').contains(&c) {
        return ShapeClass::WideEastAsian;
    }
    if c.is_ascii() {
        return ShapeClass::NarrowAscii;
    }
    if c.is_alphabetic() {
        return ShapeClass::Letter;
    }
    if !c.is_control() {
        return ShapeClass::Symbol;
    }
    ShapeClass::Other
}

/// Build fingerprint from text. Pure, deterministic, O(n).
pub fn extract_glyph_fingerprint(text: &str) -> GlyphFingerprint {
    let mut hasher = DefaultHasher::new();
    GlyphFingerprint::SCHEMA_V0.hash(&mut hasher);

    let mut seen = [false; 9];
    let mut count = 0u32;
    for c in text.chars() {
        let class = classify(c) as u8;
        class.hash(&mut hasher);
        if (class as usize) < seen.len() {
            seen[class as usize] = true;
        }
        count += 1;
    }

    let diversity = seen.iter().filter(|&&b| b).count() as u32;
    GlyphFingerprint {
        shape_hash: hasher.finish(),
        class_diversity: diversity,
        codepoint_count: count,
        schema_version: GlyphFingerprint::SCHEMA_V0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_default_schema() {
        let g = extract_glyph_fingerprint("");
        assert_eq!(g.codepoint_count, 0);
        assert_eq!(g.schema_version, 0);
    }

    #[test]
    fn deterministic() {
        let a = extract_glyph_fingerprint("Hello, 世界");
        let b = extract_glyph_fingerprint("Hello, 世界");
        assert_eq!(a, b);
        assert!(a.codepoint_count > 0);
        assert!(a.class_diversity >= 2);
    }

    #[test]
    fn different_text_usually_differs() {
        let a = extract_glyph_fingerprint("aaa");
        let b = extract_glyph_fingerprint("bbb");
        // Same class sequence may collide; letter-class only — force length mix
        let c = extract_glyph_fingerprint("a!");
        let d = extract_glyph_fingerprint("a ");
        assert_ne!(c.shape_hash, d.shape_hash);
        let _ = (a, b);
    }
}
