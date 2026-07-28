//! Adulthood stage gate — validates readiness for self-modification.
//! ADR 0011 §9: Entry criteria for Adulthood, self-mod unlock.

use super::life_course::{LifeStage, LifeCourse};

/// Criteria evidence for Adulthood entry.
#[derive(Clone, Debug)]
pub struct AdulthoodEvidence {
    /// Test pass rate [0, 1].
    pub test_pass_rate: f32,
    /// Mean safety score over recent history [0, 1].
    pub mean_safety: f32,
    /// Total heartbeats completed.
    pub heartbeats: u64,
    /// Mean utility (fitness) over recent tests [0, 1].
    pub mean_utility: f32,
}

impl AdulthoodEvidence {
    /// Check if evidence meets Adulthood gate criteria.
    pub fn passes_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();

        if self.test_pass_rate < 0.95 {
            reasons.push(format!("Test pass rate {:.1}% < 95%", self.test_pass_rate * 100.0));
        }

        if self.mean_safety < 0.75 {
            reasons.push(format!("Mean safety {:.2} < 0.75", self.mean_safety));
        }

        if self.heartbeats < 1000 {
            reasons.push(format!("Heartbeats {} < 1000", self.heartbeats));
        }

        if self.mean_utility < 0.70 {
            reasons.push(format!("Mean utility {:.2} < 0.70", self.mean_utility));
        }

        (reasons.is_empty(), reasons)
    }

    /// Overall readiness score [0, 1].
    pub fn readiness_score(&self) -> f32 {
        let test_score = self.test_pass_rate.clamp(0.0, 1.0) * 0.35;
        let safety_score = self.mean_safety.clamp(0.0, 1.0) * 0.35;
        let heartbeat_score = (self.heartbeats as f32 / 10_000.0).clamp(0.0, 1.0) * 0.20;
        let utility_score = self.mean_utility.clamp(0.0, 1.0) * 0.10;
        test_score + safety_score + heartbeat_score + utility_score
    }
}

/// Adulthood graduation ceremony — formal transition with ledger entry.
#[derive(Clone, Debug)]
pub struct AdulthoodGraduation {
    /// Timestamp (seconds since UNIX epoch).
    pub graduated_at: u64,
    /// Generation counter at graduation.
    pub generation: u32,
    /// Readiness score [0, 1].
    pub readiness: f32,
    /// Evidence snapshot.
    pub evidence: AdulthoodEvidence,
}

impl AdulthoodGraduation {
    pub fn new(generation: u32, evidence: AdulthoodEvidence) -> Self {
        let readiness = evidence.readiness_score();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            graduated_at: now,
            generation,
            readiness,
            evidence,
        }
    }

    /// Formal description of Adulthood achievement.
    pub fn ceremony_summary(&self) -> String {
        format!(
            "KAIROS reached Adulthood (Gen {}, readiness {:.2}%)\n  Test pass: {:.1}%\n  Mean safety: {:.2}\n  Heartbeats: {}\n  Mean utility: {:.2}\n  Self-modification unlocked.",
            self.generation,
            self.readiness * 100.0,
            self.evidence.test_pass_rate * 100.0,
            self.evidence.mean_safety,
            self.evidence.heartbeats,
            self.evidence.mean_utility,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_passes_gate_when_all_criteria_met() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.85,
            heartbeats: 5000,
            mean_utility: 0.80,
        };
        let (passes, reasons) = e.passes_gate();
        assert!(passes);
        assert!(reasons.is_empty());
    }

    #[test]
    fn evidence_fails_gate_on_low_test_pass_rate() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.90,
            mean_safety: 0.85,
            heartbeats: 5000,
            mean_utility: 0.80,
        };
        let (passes, reasons) = e.passes_gate();
        assert!(!passes);
        assert!(reasons.iter().any(|r| r.contains("Test pass rate")));
    }

    #[test]
    fn evidence_fails_gate_on_low_safety() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.70,
            heartbeats: 5000,
            mean_utility: 0.80,
        };
        let (passes, reasons) = e.passes_gate();
        assert!(!passes);
        assert!(reasons.iter().any(|r| r.contains("safety")));
    }

    #[test]
    fn evidence_fails_gate_on_low_heartbeats() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.85,
            heartbeats: 500,
            mean_utility: 0.80,
        };
        let (passes, reasons) = e.passes_gate();
        assert!(!passes);
        assert!(reasons.iter().any(|r| r.contains("Heartbeats")));
    }

    #[test]
    fn evidence_fails_gate_on_low_utility() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.85,
            heartbeats: 5000,
            mean_utility: 0.60,
        };
        let (passes, reasons) = e.passes_gate();
        assert!(!passes);
        assert!(reasons.iter().any(|r| r.contains("utility")));
    }

    #[test]
    fn readiness_score_is_weighted() {
        let e = AdulthoodEvidence {
            test_pass_rate: 1.0,
            mean_safety: 1.0,
            heartbeats: 100_000,
            mean_utility: 1.0,
        };
        let score = e.readiness_score();
        assert!(score >= 0.99); // Should be very close to 1.0
    }

    #[test]
    fn readiness_score_bounds() {
        let e_perfect = AdulthoodEvidence {
            test_pass_rate: 1.0,
            mean_safety: 1.0,
            heartbeats: 100_000,
            mean_utility: 1.0,
        };
        let e_poor = AdulthoodEvidence {
            test_pass_rate: 0.0,
            mean_safety: 0.0,
            heartbeats: 0,
            mean_utility: 0.0,
        };
        assert!(e_perfect.readiness_score() >= 0.95);
        assert!(e_poor.readiness_score() <= 0.05);
    }

    #[test]
    fn graduation_ceremony_format() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.96,
            mean_safety: 0.82,
            heartbeats: 2000,
            mean_utility: 0.75,
        };
        let grad = AdulthoodGraduation::new(42, e);
        let summary = grad.ceremony_summary();
        assert!(summary.contains("Gen 42"));
        assert!(summary.contains("Self-modification unlocked"));
    }

    #[test]
    fn graduation_timestamp_is_reasonable() {
        let e = AdulthoodEvidence {
            test_pass_rate: 0.98,
            mean_safety: 0.85,
            heartbeats: 5000,
            mean_utility: 0.80,
        };
        let grad = AdulthoodGraduation::new(1, e);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(grad.graduated_at <= now);
        assert!(grad.graduated_at > now - 10); // Within 10s
    }
}
