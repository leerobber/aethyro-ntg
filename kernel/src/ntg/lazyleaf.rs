//! Lazy leaf body resolution (ADR 0003 structural-first, content-lazy).
//!
//! A leaf always has an **identity** (label / path segment) and may hold a
//! deferred **body** (full document bytes or file contents). Body is not
//! required at graph-build time; call [`LazyLeaf::resolve_body`] when the
//! leaf is actually read or executed.
//!
//! Glyph fingerprints update when identity is set and again when body is
//! resolved (fingerprint of body if present, else identity).

use super::glyph::{extract_glyph_fingerprint, GlyphFingerprint};
use super::leafsignal::{extract_leaf_signal, LeafSignal};

/// Lazy leaf payload attached to graph nodes that represent content leaves.
#[derive(Clone, Debug)]
pub struct LazyLeaf {
    /// Always present identity (heading text, path segment, etc.).
    pub identity: String,
    /// Full body; `None` until resolved.
    body: Option<String>,
    /// Case/punct signal of identity (always), refreshed from body if resolved.
    pub signal: LeafSignal,
    /// Glyph fingerprint v0 (see `glyph.rs`); not trained PIXEL.
    pub glyph: GlyphFingerprint,
}

impl LazyLeaf {
    pub fn from_identity(identity: impl Into<String>) -> Self {
        let identity = identity.into();
        let signal = extract_leaf_signal(&identity);
        let glyph = extract_glyph_fingerprint(&identity);
        Self {
            identity,
            body: None,
            signal,
            glyph,
        }
    }

    /// Whether full body bytes have been materialized.
    pub fn is_resolved(&self) -> bool {
        self.body.is_some()
    }

    /// Materialize full body. Updates signal + glyph from body text.
    pub fn resolve_body(&mut self, body: impl Into<String>) {
        let body = body.into();
        self.signal = extract_leaf_signal(&body);
        self.glyph = extract_glyph_fingerprint(&body);
        self.body = Some(body);
    }

    /// Borrow body if resolved.
    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// Content used for execution/read: body if resolved, else identity.
    pub fn effective_text(&self) -> &str {
        self.body.as_deref().unwrap_or(self.identity.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_unresolved() {
        let leaf = LazyLeaf::from_identity("README");
        assert!(!leaf.is_resolved());
        assert_eq!(leaf.effective_text(), "README");
    }

    #[test]
    fn resolve_updates_signal_and_glyph() {
        let mut leaf = LazyLeaf::from_identity("short");
        let g0 = leaf.glyph;
        leaf.resolve_body("HELLO WORLD!!!");
        assert!(leaf.is_resolved());
        assert_eq!(leaf.body(), Some("HELLO WORLD!!!"));
        assert!(leaf.signal.uppercase_count >= 10);
        // Body glyph should reflect longer content
        assert_ne!(leaf.glyph.codepoint_count, g0.codepoint_count);
    }
}
