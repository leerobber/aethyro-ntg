//! Project trajectory — the correct north star for raising KAIROS.
//!
//! Lean, disciplined, Aethyro-native: ternary/genome host under rails,
//! not wasteful feature abundance or open self-mod.

/// One fixed pillar of the project's intended path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrajectoryPillar {
    pub id: &'static str,
    pub title: &'static str,
    pub intent: &'static str,
}

/// Sealed charter: what "correct trajectory" means for this build.
#[derive(Clone, Debug)]
pub struct TrajectoryCharter {
    pub name: &'static str,
    pub sealed_at_stage: &'static str,
    pub pillars: Vec<TrajectoryPillar>,
}

impl TrajectoryCharter {
    /// Default Aethyro / VITASCALE trajectory for KAIROS.
    pub fn aethyro_default() -> Self {
        Self {
            name: "Aethyro VITASCALE North Star",
            sealed_at_stage: "neonate",
            pillars: vec![
                TrajectoryPillar {
                    id: "ternary_hot",
                    title: "Ternary / bitplane / popcount hot path",
                    intent: "Keep compute discrete and measured; no float/text on hot kernels.",
                },
                TrajectoryPillar {
                    id: "genome_real",
                    title: "Genome & chromosome truth",
                    intent: "Grow from nursery DNA toward real multi-chr data when earned (not dumped).",
                },
                TrajectoryPillar {
                    id: "rails",
                    title: "Guardian rails & ledger",
                    intent: "Self-mod off by default; budgets, rollback, audit before power.",
                },
                TrajectoryPillar {
                    id: "lean",
                    title: "Lean discipline",
                    intent: "No wasteful abundance — stage gates, real work, honest failure.",
                },
                TrajectoryPillar {
                    id: "school_then_world",
                    title: "School then world",
                    intent: "Child stage = ntg_school; adolescent = real VCF campaigns under curfew.",
                },
                TrajectoryPillar {
                    id: "compose",
                    title: "Crown composition",
                    intent: "Neurocytes → Crown → Gestalt; multi-tissue host, not a monolith dump.",
                },
                TrajectoryPillar {
                    id: "vitals",
                    title: "Pulsewire vitals",
                    intent: "Live binary readings; cold JSONL/dashboard only off hot path.",
                },
                TrajectoryPillar {
                    id: "trust",
                    title: "Guardian trust",
                    intent: "Robert Lee present; KAIROS may surface anything honestly in the journal.",
                },
                TrajectoryPillar {
                    id: "sovereign_edge",
                    title: "Sovereign edge posture",
                    intent: "Air-gap friendly; no product overclaim vs aethyro.com until measured.",
                },
                TrajectoryPillar {
                    id: "measure",
                    title: "Measure, don't assume",
                    intent: "EXPERIMENTS.md and stage gates — win and non-win recorded alike.",
                },
            ],
        }
    }

    pub fn pillar_ids(&self) -> Vec<&'static str> {
        self.pillars.iter().map(|p| p.id).collect()
    }

    pub fn journal_block(&self) -> String {
        let lines: Vec<String> = self
            .pillars
            .iter()
            .map(|p| format!("[{}] {} — {}", p.id, p.title, p.intent))
            .collect();
        format!(
            "TRAJECTORY SEAL ({}) @ {} | {}",
            self.name,
            self.sealed_at_stage,
            lines.join(" || ")
        )
    }

    pub fn summary_lines(&self) -> Vec<String> {
        self.pillars
            .iter()
            .map(|p| format!("· {} — {}", p.title, p.intent))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charter_has_core_pillars() {
        let c = TrajectoryCharter::aethyro_default();
        let ids = c.pillar_ids();
        assert!(ids.contains(&"ternary_hot"));
        assert!(ids.contains(&"genome_real"));
        assert!(ids.contains(&"trust"));
        assert!(ids.contains(&"lean"));
        assert!(c.journal_block().contains("TRAJECTORY SEAL"));
    }
}
