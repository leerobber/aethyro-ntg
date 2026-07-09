//! Minimal, honest placeholder for "preserve case/punctuation as
//! signal" (ADR 0003).
//!
//! This computes simple, real, deterministic per-character counts. It
//! is **not** the PIXEL-lite visual glyph fingerprint ADR 0003
//! describes -- that needs an actual trained visual feature extractor,
//! which does not exist in this repo. What this guarantees: the raw
//! signal is preserved and classified, every character accounted for,
//! nothing silently dropped -- a foundation a later learned component
//! (Phase 3/4) can consume. Nothing here hard-codes what a symbol
//! "means"; it only counts what's there.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LeafSignal {
    pub uppercase_count: usize,
    pub lowercase_count: usize,
    pub punctuation_count: usize,
    pub whitespace_count: usize,
    pub other_count: usize,
}

impl LeafSignal {
    /// Combine two signals by summing corresponding counts -- used to
    /// aggregate a whole graph's signal in `Graph::forward_pass`.
    pub fn combine(&self, other: &LeafSignal) -> LeafSignal {
        LeafSignal {
            uppercase_count: self.uppercase_count + other.uppercase_count,
            lowercase_count: self.lowercase_count + other.lowercase_count,
            punctuation_count: self.punctuation_count + other.punctuation_count,
            whitespace_count: self.whitespace_count + other.whitespace_count,
            other_count: self.other_count + other.other_count,
        }
    }
}

pub fn extract_leaf_signal(text: &str) -> LeafSignal {
    let mut signal = LeafSignal::default();
    for c in text.chars() {
        if c.is_uppercase() {
            signal.uppercase_count += 1;
        } else if c.is_lowercase() {
            signal.lowercase_count += 1;
        } else if c.is_whitespace() {
            signal.whitespace_count += 1;
        } else if c.is_ascii_punctuation() {
            signal.punctuation_count += 1;
        } else {
            signal.other_count += 1;
        }
    }
    signal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_case_and_punctuation_separately() {
        // "Hi, World!" -> H,i,',',' ',W,o,r,l,d,'!' (10 chars)
        let signal = extract_leaf_signal("Hi, World!");
        assert_eq!(signal.uppercase_count, 2); // H, W
        assert_eq!(signal.lowercase_count, 5); // i, o, r, l, d
        assert_eq!(signal.punctuation_count, 2); // , !
        assert_eq!(signal.whitespace_count, 1); // the space
        assert_eq!(signal.other_count, 0);
    }

    #[test]
    fn empty_string_is_all_zero() {
        assert_eq!(extract_leaf_signal(""), LeafSignal::default());
    }

    #[test]
    fn combine_sums_corresponding_counts() {
        let a = extract_leaf_signal("Hi!");
        let b = extract_leaf_signal("Bye.");
        let combined = a.combine(&b);
        assert_eq!(combined.uppercase_count, a.uppercase_count + b.uppercase_count);
        assert_eq!(combined.punctuation_count, a.punctuation_count + b.punctuation_count);
    }

    #[test]
    fn nothing_is_discarded() {
        let text = "Hi, World! 123";
        let signal = extract_leaf_signal(text);
        let total = signal.uppercase_count
            + signal.lowercase_count
            + signal.punctuation_count
            + signal.whitespace_count
            + signal.other_count;
        assert_eq!(
            total,
            text.chars().count(),
            "every character must land in exactly one bucket -- none dropped"
        );
    }
}
