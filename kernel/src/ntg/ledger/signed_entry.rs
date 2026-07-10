//! Per-record signing layer: content integrity like LexGenSeal.
//!
//! Each ledger entry is SHA-256 signed over its own content, making it
//! tamper-evident at the individual record level. Combined with ChainLog's
//! sequence chaining, this gives both content *and* sequence integrity.

use super::crypto::{content_hash, hash_to_hex};
use super::NtgError;

/// A cryptographically signed ledger entry: content + its SHA-256 hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedEntry {
    /// The actual event/mutation description (JSON)
    pub content: String,
    /// SHA-256 hash of the content (32 bytes)
    pub content_hash: [u8; 32],
    /// Mutation ID for cross-referencing with execution traces
    pub mutation_id: u64,
    /// Timestamp (wall-clock or logical clock)
    pub timestamp: u64,
}

impl SignedEntry {
    /// Create a new signed entry. The hash is computed immediately and
    /// stored with the entry, so it can't drift from the content.
    pub fn new(
        content: &str,
        mutation_id: u64,
        timestamp: u64,
    ) -> Result<Self, NtgError> {
        let hash = content_hash(content);
        Ok(Self {
            content: content.to_string(),
            content_hash: hash,
            mutation_id,
            timestamp,
        })
    }

    /// Verify this entry hasn't been tampered with: recompute the hash
    /// and confirm it matches the stored value.
    pub fn verify(&self) -> Result<(), NtgError> {
        let recomputed = content_hash(&self.content);
        if recomputed == self.content_hash {
            Ok(())
        } else {
            Err(NtgError::LedgerTampering(format!(
                "Entry {} content tampered: stored {} != computed {}",
                self.mutation_id,
                hash_to_hex(&self.content_hash),
                hash_to_hex(&recomputed)
            )))
        }
    }

    /// Get the content hash as a hex string (for logging/display).
    pub fn hash_hex(&self) -> String {
        hash_to_hex(&self.content_hash)
    }

    /// Extract the mutation outcome from the entry's JSON content.
    /// (Helper for audit_summary and other queries.)
    ///
    /// Reads the `"outcome":"..."` field specifically rather than
    /// substring-searching the whole entry: the entry also embeds a
    /// free-text `description`, and a description that happens to contain
    /// one of the outcome names (e.g. "...previously Accepted...") would
    /// silently misclassify the entry under a blind `contains()` search,
    /// corrupting `audit_summary()`'s compliance counts.
    pub fn get_outcome(&self) -> Result<super::MutationOutcome, NtgError> {
        match Self::extract_outcome_field(&self.content).as_deref() {
            Some("Accepted") => Ok(super::MutationOutcome::Accepted),
            Some("RejectedRegression") => Ok(super::MutationOutcome::RejectedRegression),
            Some("RejectedBudgetExceeded") => Ok(super::MutationOutcome::RejectedBudgetExceeded),
            Some("RejectedFitnessGate") => Ok(super::MutationOutcome::RejectedFitnessGate),
            _ => Err(NtgError::InvalidInput(
                "Unknown or missing outcome field in entry".to_string(),
            )),
        }
    }

    /// Find the `"outcome"` key and read its string value, tolerating the
    /// whitespace JSON permits around `:` (this ledger's own writer emits
    /// compact JSON with none, but a tolerant reader is cheap insurance
    /// against a differently-formatted entry without pulling in a full
    /// JSON parser as a dependency for one field read).
    fn extract_outcome_field(content: &str) -> Option<String> {
        const KEY: &str = "\"outcome\"";
        let after_key = content.find(KEY)? + KEY.len();
        let rest = content[after_key..].trim_start();
        let rest = rest.strip_prefix(':')?.trim_start();
        let rest = rest.strip_prefix('"')?;
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_verify_entry() -> Result<(), NtgError> {
        let content = r#"{"mutation_id":0,"description":"test"}"#;
        let entry = SignedEntry::new(content, 0, 1000)?;
        assert_eq!(entry.content, content);
        assert_eq!(entry.mutation_id, 0);
        assert_eq!(entry.timestamp, 1000);
        assert!(entry.verify().is_ok());
        Ok(())
    }

    #[test]
    fn tampering_is_detected() -> Result<(), NtgError> {
        let content = r#"{"mutation_id":0,"outcome":"Accepted"}"#;
        let mut entry = SignedEntry::new(content, 0, 1000)?;
        assert!(entry.verify().is_ok());

        // Tamper with the content
        entry.content = r#"{"mutation_id":0,"outcome":"Rejected"}"#.to_string();
        assert!(entry.verify().is_err());
        Ok(())
    }

    #[test]
    fn hash_is_deterministic() -> Result<(), NtgError> {
        let content = "stable content";
        let entry1 = SignedEntry::new(content, 0, 1000)?;
        let entry2 = SignedEntry::new(content, 0, 1000)?;
        assert_eq!(entry1.content_hash, entry2.content_hash);
        Ok(())
    }

    #[test]
    fn hash_hex_is_valid() -> Result<(), NtgError> {
        let entry = SignedEntry::new("test", 0, 1000)?;
        let hex = entry.hash_hex();
        assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        Ok(())
    }

    #[test]
    fn extract_outcome_accepted() -> Result<(), NtgError> {
        let entry = SignedEntry::new(r#"{"outcome":"Accepted"}"#, 0, 1000)?;
        assert_eq!(entry.get_outcome()?, super::super::MutationOutcome::Accepted);
        Ok(())
    }

    #[test]
    fn extract_outcome_tolerates_whitespace_around_colon() -> Result<(), NtgError> {
        // The ledger's own writer emits compact JSON, but the reader
        // shouldn't break if an entry is pretty-printed or reformatted.
        let entry = SignedEntry::new(
            "{\n  \"outcome\" :  \"RejectedFitnessGate\"\n}",
            0,
            1000,
        )?;
        assert_eq!(
            entry.get_outcome()?,
            super::super::MutationOutcome::RejectedFitnessGate
        );
        Ok(())
    }

    #[test]
    fn extract_outcome_ignores_matching_words_in_description() -> Result<(), NtgError> {
        // A free-text description mentioning a *different* outcome name than
        // the real `outcome` field must not confuse the parser — regression
        // test for a bug where `get_outcome` substring-searched the whole
        // entry instead of reading the `outcome` field specifically.
        let entry = SignedEntry::new(
            r#"{"description":"supersedes the previously Accepted candidate","outcome":"RejectedRegression"}"#,
            0,
            1000,
        )?;
        assert_eq!(
            entry.get_outcome()?,
            super::super::MutationOutcome::RejectedRegression
        );
        Ok(())
    }

    #[test]
    fn extract_outcome_rejection() -> Result<(), NtgError> {
        let entry = SignedEntry::new(r#"{"outcome":"RejectedRegression"}"#, 0, 1000)?;
        assert_eq!(
            entry.get_outcome()?,
            super::super::MutationOutcome::RejectedRegression
        );
        Ok(())
    }
}
