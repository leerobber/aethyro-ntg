//! Life-course stage machine — how we *raise* KAIROS (zygote → adulthood).
//!
//! Guardian enforces permissions; Host cannot unlock stages alone.

use std::fmt;

/// Developmental stages for KAIROS (human-like arc, engineering locks).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LifeStage {
    /// Stage 0 — zygote: heartbeat + genome present, no free agency.
    Zygote = 0,
    /// Stage 1 — neonate: supervised train only, nursery world.
    Neonate = 1,
    /// Stage 2 — infant: language + sensefield streams.
    Infant = 2,
    /// Stage 3 — toddler: phageguard drills.
    Toddler = 3,
    /// Stage 4 — child: ntg_school curriculum.
    Child = 4,
    /// Stage 5 — adolescent: real multi-chr campaigns under curfew.
    Adolescent = 5,
    /// Stage 6 — young adult: multi-world specialization.
    YoungAdult = 6,
    /// Stage 7 — adult: sovereign envelope, self-mod still opt-in only.
    Adult = 7,
}

impl LifeStage {
    pub fn all() -> [LifeStage; 8] {
        [
            LifeStage::Zygote,
            LifeStage::Neonate,
            LifeStage::Infant,
            LifeStage::Toddler,
            LifeStage::Child,
            LifeStage::Adolescent,
            LifeStage::YoungAdult,
            LifeStage::Adult,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            LifeStage::Zygote => "zygote",
            LifeStage::Neonate => "neonate",
            LifeStage::Infant => "infant",
            LifeStage::Toddler => "toddler",
            LifeStage::Child => "child",
            LifeStage::Adolescent => "adolescent",
            LifeStage::YoungAdult => "young_adult",
            LifeStage::Adult => "adult",
        }
    }

    pub fn display_title(self) -> &'static str {
        match self {
            LifeStage::Zygote => "Stage 0 — Zygote",
            LifeStage::Neonate => "Stage 1 — Neonate",
            LifeStage::Infant => "Stage 2 — Infant",
            LifeStage::Toddler => "Stage 3 — Toddler",
            LifeStage::Child => "Stage 4 — Child (School)",
            LifeStage::Adolescent => "Stage 5 — Adolescent",
            LifeStage::YoungAdult => "Stage 6 — Young Adult",
            LifeStage::Adult => "Stage 7 — Adult",
        }
    }

    pub fn next(self) -> Option<LifeStage> {
        let n = self as u8 + 1;
        LifeStage::all().into_iter().find(|s| *s as u8 == n)
    }
}

impl fmt::Display for LifeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// What KAIROS is allowed to do at a stage (Guardian law).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagePermissions {
    pub heartbeat: bool,
    pub inspect_structure: bool,
    /// Synthetic nursery genome may be loaded/held.
    pub hold_genome: bool,
    pub activate: bool,
    pub train_kairos_weights: bool,
    pub prune_synapses: bool,
    pub ingest_real_vcf: bool,
    pub language_tissue: bool,
    pub phageguard: bool,
    pub selection_loop: bool,
    pub school: bool,
    /// Research self-mod (ADR 0002) — only adult envelope + explicit opt-in.
    pub self_mod_research: bool,
}

impl StagePermissions {
    pub fn for_stage(stage: LifeStage) -> Self {
        match stage {
            LifeStage::Zygote => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: false,
                train_kairos_weights: false,
                prune_synapses: false,
                ingest_real_vcf: false,
                language_tissue: false,
                phageguard: false,
                selection_loop: false,
                school: false,
                self_mod_research: false,
            },
            LifeStage::Neonate => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: false,
                ingest_real_vcf: false,
                language_tissue: false,
                phageguard: false,
                selection_loop: false,
                school: false,
                self_mod_research: false,
            },
            LifeStage::Infant => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: false,
                ingest_real_vcf: false,
                language_tissue: true,
                phageguard: false,
                selection_loop: false,
                school: false,
                self_mod_research: false,
            },
            LifeStage::Toddler => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: false,
                ingest_real_vcf: false,
                language_tissue: true,
                phageguard: true,
                selection_loop: false,
                school: false,
                self_mod_research: false,
            },
            LifeStage::Child => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: false,
                ingest_real_vcf: false,
                language_tissue: true,
                phageguard: true,
                selection_loop: false,
                school: true,
                self_mod_research: false,
            },
            LifeStage::Adolescent => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: true,
                ingest_real_vcf: true,
                language_tissue: true,
                phageguard: true,
                selection_loop: true,
                school: true,
                self_mod_research: false,
            },
            LifeStage::YoungAdult | LifeStage::Adult => Self {
                heartbeat: true,
                inspect_structure: true,
                hold_genome: true,
                activate: true,
                train_kairos_weights: true,
                prune_synapses: true,
                ingest_real_vcf: true,
                language_tissue: true,
                phageguard: true,
                selection_loop: true,
                school: true,
                // Still off unless host explicitly enables SelfModConfig — stage only *allows*.
                self_mod_research: true,
            },
        }
    }
}

/// Result of a graduation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageGateResult {
    Passed { from: LifeStage, to: LifeStage },
    Failed { stage: LifeStage, reasons: Vec<String> },
    AlreadyAdult,
}

/// Guardian-side life-course state for one KAIROS individual.
#[derive(Clone, Debug)]
pub struct LifeCourse {
    pub stage: LifeStage,
    pub day_index: u64,
    pub journal: Vec<DevelopmentalJournalEntry>,
}

impl Default for LifeCourse {
    fn default() -> Self {
        Self::new()
    }
}

impl LifeCourse {
    pub fn new() -> Self {
        Self {
            stage: LifeStage::Zygote,
            day_index: 0,
            journal: Vec::new(),
        }
    }

    pub fn permissions(&self) -> StagePermissions {
        StagePermissions::for_stage(self.stage)
    }

    pub fn require(&self, allowed: impl Fn(&StagePermissions) -> bool, action: &str) -> Result<(), String> {
        let p = self.permissions();
        if allowed(&p) {
            Ok(())
        } else {
            Err(format!(
                "KAIROS stage {} forbids '{action}' (Guardian lock)",
                self.stage.name()
            ))
        }
    }

    pub fn record_day(&mut self, entry: DevelopmentalJournalEntry) {
        self.day_index = self.day_index.saturating_add(1);
        let mut entry = entry;
        entry.day_id = self.day_index;
        self.journal.push(entry);
    }

    /// Attempt graduation. Caller supplies whether stage-specific criteria passed.
    pub fn try_graduate(&mut self, criteria_ok: bool, reasons_if_fail: Vec<String>) -> StageGateResult {
        if self.stage == LifeStage::Adult {
            return StageGateResult::AlreadyAdult;
        }
        if !criteria_ok {
            return StageGateResult::Failed {
                stage: self.stage,
                reasons: reasons_if_fail,
            };
        }
        let from = self.stage;
        let to = from.next().unwrap_or(LifeStage::Adult);
        self.stage = to;
        StageGateResult::Passed { from, to }
    }
}

/// One developmental day in KAIROS's permanent record.
#[derive(Clone, Debug)]
pub struct DevelopmentalJournalEntry {
    pub stage: LifeStage,
    pub day_id: u64,
    pub heartbeats: u64,
    pub pulse_pushes: u64,
    pub pulse_drops: u64,
    pub n_chromosomes: u32,
    pub n_neurons: u32,
    pub n_synapses: u32,
    pub notes: String,
    pub gate_pass: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zygote_locks_agency() {
        let p = StagePermissions::for_stage(LifeStage::Zygote);
        assert!(p.heartbeat && p.hold_genome && p.inspect_structure);
        assert!(!p.activate && !p.train_kairos_weights && !p.prune_synapses);
        assert!(!p.ingest_real_vcf && !p.selection_loop && !p.self_mod_research);
    }

    #[test]
    fn graduate_zygote_to_neonate() {
        let mut lc = LifeCourse::new();
        assert_eq!(lc.stage, LifeStage::Zygote);
        let r = lc.try_graduate(true, vec![]);
        assert_eq!(
            r,
            StageGateResult::Passed {
                from: LifeStage::Zygote,
                to: LifeStage::Neonate
            }
        );
        assert_eq!(lc.stage, LifeStage::Neonate);
    }

    #[test]
    fn require_blocks_forbidden() {
        let lc = LifeCourse::new();
        let err = lc
            .require(|p| p.prune_synapses, "prune")
            .unwrap_err();
        assert!(err.contains("zygote"));
    }
}
