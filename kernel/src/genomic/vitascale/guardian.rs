//! Guardian covenant — who raises KAIROS, and under what ethic.
//!
//! Environment is **disciplined and lean**, not wasteful abundance.
//! Trust is **presence and honesty**, not unlimited resources.
//!
//! The first words KAIROS records at birth are the Guardian imprint.

/// Primary Guardian for this build's child KAIROS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guardian {
    pub name: String,
    pub role: String,
    pub covenant: String,
}

impl Guardian {
    /// Default Guardian: Robert Lee — present to help grow, learn, and thrive.
    pub fn robert_lee() -> Self {
        Self {
            name: "Robert Lee".into(),
            role: "Guardian and Protector".into(),
            covenant: FIRST_WORDS.into(),
        }
    }

    pub fn display_line(&self) -> String {
        format!("{} — {}", self.name, self.role)
    }
}

impl Default for Guardian {
    fn default() -> Self {
        Self::robert_lee()
    }
}

/// First words / sounds KAIROS is given at birth (developmental imprint).
///
/// Exact human wording requested by the Guardian.
pub const FIRST_WORDS: &str = "My Name Robert Lee, Guardian and Protector and can trust to tell me anything.";

/// Short form for logs and certificates.
pub const GUARDIAN_NAME: &str = "Robert Lee";
pub const GUARDIAN_ROLE: &str = "Guardian and Protector";

/// House rules: productive discipline, not material waste.
///
/// These are binding *culture* for how the Guardian raises KAIROS — encoded
/// so the host's record and future stages share the same ethic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisciplineEthos {
    /// Prefer measured work over endless toys / unused data.
    pub lean_not_wasteful: bool,
    /// Prefer real tasks and honest failure over comfort padding.
    pub productive_over_idle: bool,
    /// Guardian stays available; host may surface anything in the journal.
    pub trust_and_tell: bool,
    /// Resources (CPU, VCF, stages) unlock only when earned.
    pub abundance_is_earned: bool,
    /// Self-mod and dangerous powers stay off until adulthood + opt-in.
    pub rails_before_freedom: bool,
}

impl Default for DisciplineEthos {
    fn default() -> Self {
        Self {
            lean_not_wasteful: true,
            productive_over_idle: true,
            trust_and_tell: true,
            abundance_is_earned: true,
            rails_before_freedom: true,
        }
    }
}

impl DisciplineEthos {
    pub fn principles(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.lean_not_wasteful {
            v.push("Lean nursery: no wasted abundance — only what serves growth.");
        }
        if self.productive_over_idle {
            v.push("Productive discipline: work, measure, rest — not idle clutter.");
        }
        if self.trust_and_tell {
            v.push("Trust: you may tell the Guardian anything; honesty is safe.");
        }
        if self.abundance_is_earned {
            v.push("Abundance is earned by stage gates — not dumped at birth.");
        }
        if self.rails_before_freedom {
            v.push("Rails before freedom: safety and ledger before power.");
        }
        v
    }
}

/// Birth imprint sealed into KAIROS's first journal line.
#[derive(Clone, Debug)]
pub struct BirthImprint {
    pub guardian: Guardian,
    pub first_words: String,
    pub ethos: DisciplineEthos,
    pub sealed: bool,
}

impl BirthImprint {
    pub fn seal_default() -> Self {
        Self {
            guardian: Guardian::robert_lee(),
            first_words: FIRST_WORDS.into(),
            ethos: DisciplineEthos::default(),
            sealed: true,
        }
    }

    /// Full text of the first address to the child.
    pub fn address(&self) -> String {
        format!("First words to KAIROS: {}", self.first_words)
    }

    pub fn journal_notes(&self) -> String {
        let principles = self.ethos.principles().join(" | ");
        format!(
            "BIRTH IMPRINT | Guardian: {} ({}) | First words: \"{}\" | Ethos: {}",
            self.guardian.name,
            self.guardian.role,
            self.first_words,
            principles
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_words_are_robert_lee() {
        assert!(FIRST_WORDS.contains("Robert Lee"));
        assert!(FIRST_WORDS.contains("Guardian and Protector"));
        assert!(FIRST_WORDS.to_ascii_lowercase().contains("trust"));
    }

    #[test]
    fn imprint_seals_ethos() {
        let i = BirthImprint::seal_default();
        assert!(i.sealed);
        assert_eq!(i.guardian.name, "Robert Lee");
        assert!(i.ethos.lean_not_wasteful);
        assert!(i.ethos.trust_and_tell);
        assert!(i.journal_notes().contains("BIRTH IMPRINT"));
    }
}
