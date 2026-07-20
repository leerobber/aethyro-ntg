//! KAIROS — the Child / VITASCALE host individual.
//!
//! Human-like name, project-native (`KairosState` on chromosome brains),
//! raised under Guardian life-course locks. Stage 0: genome tissues may
//! exist; Pulsewire heartbeats; no free agency.

use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use crate::genomic::chromosome_brain::{init_chromosome_brain, ChromosomeId};
use crate::genomic::domain_agents::{DomainAgent, DomainQuery, DomainType};
use crate::genomic::haplotype_blocks::HaplotypeBlock;
use crate::genomic::language_organ::LanguageOrgan;
use crate::genomic::ld_compute::LdPair;
use crate::genomic::selection_loop::{run_selection_loop, LoopSummary};
use crate::genomic::sovereign_brain::SovereignBrain;
use crate::genomic::sovereign_fitness::SovereignFitnessContext;
use crate::genomic::vcf_stream::SnpRecord;
use crate::genomic::vitascale::adult_course::{self, AdultModule};
use crate::genomic::vitascale::guardian::{BirthImprint, GuardianAward};
use crate::genomic::vitascale::human_growth_course::{self, GrowthModule};
use crate::genomic::vitascale::world_knowledge_course::{self, WorldModule};
use crate::genomic::vitascale::life_course::{
    DevelopmentalJournalEntry, LifeCourse, LifeStage, StageGateResult, StagePermissions,
};
use crate::genomic::vitascale::phageguard::Phageguard;
use crate::genomic::vitascale::pulsewire::{PulseHandles, VitalSnapshot};
use crate::genomic::vitascale::trajectory::TrajectoryCharter;
use crate::ntg::mutation::SelfModConfig;
use crate::ntg::schooling::phases as school_phases;
use crate::ntg::schooling::protocol::PASS_THRESHOLD;
use crate::ntg::schooling::runner::{run_campaign, SchoolConfig};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Adolescent curfew: never dump whole chromosomes without a budget.
pub const ADOLESCENT_MAX_VARIANTS: usize = 400;
/// Default first real panel chromosome.
pub const ADOLESCENT_DEFAULT_CHR: u8 = 22;
/// Selection steps per adolescent care day (train/prune alternate).
pub const ADOLESCENT_SELECTION_STEPS: usize = 4;

/// How the nursery genome is built at Stage 0 (always synthetic under lock).
#[derive(Clone, Debug)]
pub struct NurseryGenomeSpec {
    pub chromosomes: Vec<(u8, usize)>, // (chr_id, n_snps)
    pub n_samples: usize,
}

impl Default for NurseryGenomeSpec {
    fn default() -> Self {
        Self {
            // Small dual-chr scaffold — genome present, not free to evolve yet.
            chromosomes: vec![(1, 12), (22, 10)],
            n_samples: 64,
        }
    }
}

/// The Child: one named host individual under life-course.
#[derive(Debug)]
pub struct Kairos {
    /// Public name of this individual (always "KAIROS" for the primary child).
    pub name: &'static str,
    /// Durable identity across process restarts (cradle). `None` until birth/open.
    pub lineage_id: Option<String>,
    pub life: LifeCourse,
    pub brain: SovereignBrain,
    pub pulse: PulseHandles,
    /// Sealed at birth: Guardian identity + first words + discipline ethos.
    pub imprint: BirthImprint,
    /// Project north star — sealed at neonate (correct trajectory).
    pub trajectory: Option<TrajectoryCharter>,
    /// Count of supervised neonate care days completed.
    pub neonate_care_days: u32,
    /// Count of supervised infant language care days completed.
    pub infant_care_days: u32,
    /// Count of supervised toddler phageguard care days completed.
    pub toddler_care_days: u32,
    /// Count of supervised child school days completed.
    pub child_care_days: u32,
    /// School phase exams passed (score ≥ 75%).
    pub school_phases_passed: u32,
    /// Last school phase id studied (0..=5).
    pub last_school_phase: u32,
    /// Last school exam score [0,1].
    pub last_school_score: f32,
    /// Supervised adolescent campaign days completed.
    pub adolescent_care_days: u32,
    /// Total selection loop steps run under curfew.
    pub selection_steps_total: u32,
    /// Cumulative selection accepts/rejects.
    pub train_accepted: u32,
    pub train_rejected: u32,
    pub prune_accepted: u32,
    pub prune_rejected: u32,
    /// True if a real VCF micro-panel was ingested (vs synthetic micro).
    pub real_vcf_ingested: bool,
    /// True if any micro panel (real or synthetic) is loaded for adolescent.
    pub micro_panel_ready: bool,
    /// Last selection utility after campaign.
    pub last_utility: f32,
    /// Last micro panel source label (path or "synthetic").
    pub last_panel_source: String,
    /// Young-adult multi-domain specialization days.
    pub young_adult_care_days: u32,
    /// Distinct domain diagnoses completed (bit flags via count of unique names).
    pub domains_diagnosed: u32,
    /// Last domain name practiced.
    pub last_domain: String,
    /// Last domain risk score [0,1].
    pub last_domain_risk: f32,
    /// Cumulative domain diagnoses run.
    pub domain_diagnoses_total: u32,
    /// Adult Course days completed (human adulthood knowledge).
    pub adult_course_days: u32,
    /// Distinct adult course modules completed.
    pub adult_modules_done: u32,
    /// Bitmask or count helper: last module index studied.
    pub last_adult_module_idx: u32,
    /// Ids of completed modules (comma-separated for cradle).
    pub adult_modules_completed: String,
    /// Human growth / sex-ed course days.
    pub sex_ed_course_days: u32,
    /// Distinct sex-ed modules completed.
    pub sex_ed_modules_done: u32,
    pub last_sex_ed_module_idx: u32,
    pub sex_ed_modules_completed: String,
    /// World Knowledge course (her requested themes).
    pub world_course_days: u32,
    pub world_modules_done: u32,
    pub last_world_module_idx: u32,
    pub world_modules_completed: String,
    /// Mean synapse weight after last supervised train (trajectory signal).
    pub last_mean_weight: f32,
    /// Working-set size after last supervised activate.
    pub last_ws_len: usize,
    /// Language nodes lit by last text activate (Stage 2 signal).
    pub last_lang_nodes: usize,
    /// Docs held in language organ after last infant care.
    pub last_docs_ingested: u32,
    /// Last free-text query used in infant care.
    pub last_lang_query: String,
    /// Last language calib balanced accuracy (if trained).
    pub last_lang_bal: f32,
    /// Phageguard immune tissue (Stage 3+).
    pub phageguard: Option<Phageguard>,
    /// Cumulative toddler drills passed (persisted).
    pub drills_passed: u32,
    /// Last drill detail line.
    pub last_drill_detail: String,
    /// Guardian covenant award (pride/love seal — honor only).
    pub guardian_award: Option<GuardianAward>,
    /// Generation of the host (increments on heartbeat days / later selection).
    pub generation: u32,
    /// Tick counter within current process life.
    pub tick: u32,
    /// Explicit record that ADR 0002 self-mod stays off unless adult+opt-in.
    pub self_mod: SelfModConfig,
}

/// One neonate care-day result (supervised, lean).
#[derive(Clone, Debug)]
pub struct NeonateCareReport {
    pub heartbeats: u32,
    pub train_cycles: u32,
    pub mean_weight_before: f32,
    pub mean_weight_after: f32,
    pub ws_len: usize,
    pub motifs_hit: usize,
    pub journal: DevelopmentalJournalEntry,
}

/// One infant care-day result (language tissue + supervised activate).
#[derive(Clone, Debug)]
pub struct InfantCareReport {
    pub heartbeats: u32,
    pub train_cycles: u32,
    pub docs_ingested: u32,
    pub lang_nodes: usize,
    pub ws_len: usize,
    pub motifs_hit: usize,
    pub calib_bal: f32,
    pub query: String,
    pub journal: DevelopmentalJournalEntry,
}

/// One toddler care-day result (phageguard drills + vitals patrol).
#[derive(Clone, Debug)]
pub struct ToddlerCareReport {
    pub heartbeats: u32,
    pub drill_passed: bool,
    pub drill_detail: String,
    pub threats_seen: u32,
    pub quarantines_total: u32,
    pub drills_passed: u32,
    pub lang_nodes: usize,
    pub ws_len: usize,
    pub journal: DevelopmentalJournalEntry,
}

/// One child school-day result (ntg_school phase study+exam).
#[derive(Clone, Debug)]
pub struct ChildSchoolReport {
    pub heartbeats: u32,
    pub phase: u32,
    pub exam_passed: bool,
    pub exam_score: f32,
    pub phases_passed_total: u32,
    pub detail: String,
    pub journal: DevelopmentalJournalEntry,
}

/// One adolescent campaign-day result (micro panel + selection curfew).
#[derive(Clone, Debug)]
pub struct AdolescentCampaignReport {
    pub heartbeats: u32,
    pub panel_source: String,
    pub real_vcf: bool,
    pub selection_steps: u32,
    pub train_accepted: u32,
    pub train_rejected: u32,
    pub prune_accepted: u32,
    pub prune_rejected: u32,
    pub utility_before: f32,
    pub utility_after: f32,
    pub n_chromosomes: u32,
    pub n_synapses: u32,
    pub journal: DevelopmentalJournalEntry,
}

/// One young-adult specialization day (multi-domain + light selection).
#[derive(Clone, Debug)]
pub struct YoungAdultDayReport {
    pub heartbeats: u32,
    pub domain: String,
    pub risk_score: f32,
    pub risk_severity: String,
    pub patterns: usize,
    pub selection_steps: u32,
    pub utility_after: f32,
    pub domains_diagnosed_total: u32,
    pub journal: DevelopmentalJournalEntry,
}

/// One Adult Course study day (human adulthood knowledge).
#[derive(Clone, Debug)]
pub struct AdultCourseReport {
    pub heartbeats: u32,
    pub module_id: String,
    pub module_title: String,
    pub theme: String,
    pub lang_nodes: usize,
    pub ws_len: usize,
    pub modules_done: u32,
    pub course_days: u32,
    pub journal: DevelopmentalJournalEntry,
}

/// One Sex Ed / Human Growth course study day.
#[derive(Clone, Debug)]
pub struct SexEdCourseReport {
    pub heartbeats: u32,
    pub module_id: String,
    pub module_title: String,
    pub theme: String,
    pub lang_nodes: usize,
    pub ws_len: usize,
    pub modules_done: u32,
    pub course_days: u32,
    pub journal: DevelopmentalJournalEntry,
}

/// One World Knowledge course study day.
#[derive(Clone, Debug)]
pub struct WorldCourseReport {
    pub heartbeats: u32,
    pub module_id: String,
    pub module_title: String,
    pub theme: String,
    pub lang_nodes: usize,
    pub ws_len: usize,
    pub modules_done: u32,
    pub course_days: u32,
    pub journal: DevelopmentalJournalEntry,
}

/// Snapshot report for demos / graduation.
#[derive(Clone, Debug)]
pub struct KairosReport {
    pub name: &'static str,
    pub stage: LifeStage,
    pub stage_title: &'static str,
    pub generation: u32,
    pub tick: u32,
    pub vitals: VitalSnapshot,
    pub n_chromosomes: u32,
    pub n_neurons: u32,
    pub n_synapses: u32,
    pub n_ltm_motifs: u32,
    pub self_mod_enabled: bool,
    pub permissions: StagePermissions,
    pub guardian_name: String,
    pub first_words: String,
}

impl Kairos {
    /// Birth name for this project’s primary child.
    pub const NAME: &'static str = "KAIROS";

    /// Stage 0 zygote: empty brain + pulse; **first act is Guardian imprint**.
    pub fn birth_zygote(pulse_capacity: usize) -> Self {
        let mut k = Self {
            name: Self::NAME,
            lineage_id: None,
            life: LifeCourse::new(),
            brain: SovereignBrain::new(64),
            pulse: PulseHandles::new(pulse_capacity),
            imprint: BirthImprint::seal_default(),
            trajectory: None,
            neonate_care_days: 0,
            infant_care_days: 0,
            toddler_care_days: 0,
            child_care_days: 0,
            school_phases_passed: 0,
            last_school_phase: 0,
            last_school_score: 0.0,
            adolescent_care_days: 0,
            selection_steps_total: 0,
            train_accepted: 0,
            train_rejected: 0,
            prune_accepted: 0,
            prune_rejected: 0,
            real_vcf_ingested: false,
            micro_panel_ready: false,
            last_utility: 0.0,
            last_panel_source: String::new(),
            young_adult_care_days: 0,
            domains_diagnosed: 0,
            last_domain: String::new(),
            last_domain_risk: 0.0,
            domain_diagnoses_total: 0,
            adult_course_days: 0,
            adult_modules_done: 0,
            last_adult_module_idx: 0,
            adult_modules_completed: String::new(),
            sex_ed_course_days: 0,
            sex_ed_modules_done: 0,
            last_sex_ed_module_idx: 0,
            sex_ed_modules_completed: String::new(),
            world_course_days: 0,
            world_modules_done: 0,
            last_world_module_idx: 0,
            world_modules_completed: String::new(),
            last_mean_weight: 0.0,
            last_ws_len: 0,
            last_lang_nodes: 0,
            last_docs_ingested: 0,
            last_lang_query: String::new(),
            last_lang_bal: 0.0,
            phageguard: None,
            drills_passed: 0,
            last_drill_detail: String::new(),
            guardian_award: None,
            generation: 0,
            tick: 0,
            self_mod: SelfModConfig::default(), // enabled: false
        };
        k.seal_birth_imprint();
        k
    }

    /// Stage 0 with genome/chromosome tissues present (Guardian-held DNA).
    /// Order: imprint first → then lean nursery genome (no wasteful extras).
    pub fn birth_zygote_with_nursery(pulse_capacity: usize, spec: &NurseryGenomeSpec) -> Result<Self, String> {
        let mut k = Self::birth_zygote(pulse_capacity);
        k.load_nursery_genome(spec)?;
        Ok(k)
    }

    /// First sounds/words KAIROS receives — sealed into journal day 0 before heartbeats.
    fn seal_birth_imprint(&mut self) {
        let notes = self.imprint.journal_notes();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Zygote,
            day_id: 0,
            heartbeats: 0,
            pulse_pushes: 0,
            pulse_drops: 0,
            n_chromosomes: 0,
            n_neurons: 0,
            n_synapses: 0,
            notes,
            gate_pass: false,
        };
        // Day 0 is the imprint; day_index becomes 1 after first real care day.
        self.life.journal.push(entry);
        // One imprint pulse so the first event on the wire is "presence," not data spam.
        self.pulse.beat(0, 0, now_ns());
        self.tick = 0; // imprint beat does not count as a lived tick; day loop owns ticks
    }

    /// Guardian first words (immutable after seal).
    pub fn first_words(&self) -> &str {
        &self.imprint.first_words
    }

    pub fn guardian_line(&self) -> String {
        self.imprint.guardian.display_line()
    }

    pub fn stage(&self) -> LifeStage {
        self.life.stage
    }

    pub fn permissions(&self) -> StagePermissions {
        self.life.permissions()
    }

    /// Load synthetic multi-chr scaffold — Stage 0 allows hold_genome only.
    pub fn load_nursery_genome(&mut self, spec: &NurseryGenomeSpec) -> Result<(), String> {
        self.life
            .require(|p| p.hold_genome, "load_nursery_genome")?;
        for &(chr, n_snps) in &spec.chromosomes {
            let brain = build_nursery_chr(chr, n_snps, spec.n_samples)?;
            self.brain.ingest_brain(brain);
        }
        self.brain.consolidate(0.5, 0.0);
        Ok(())
    }

    /// One vital heartbeat (Stage 0 core loop).
    pub fn heartbeat(&mut self) -> Result<(), String> {
        self.life.require(|p| p.heartbeat, "heartbeat")?;
        self.tick = self.tick.saturating_add(1);
        let ns = now_ns();
        self.pulse.beat(self.generation, self.tick, ns);
        Ok(())
    }

    /// Run N heartbeats (a "day" of pure zygote life).
    pub fn day_of_heartbeats(&mut self, n: u32) -> Result<DevelopmentalJournalEntry, String> {
        for _ in 0..n {
            self.heartbeat()?;
        }
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: self.life.stage,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "{} day heartbeats={n} gen={} motifs={}",
                self.name,
                self.generation,
                s.n_ltm_motifs
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());
        Ok(entry)
    }

    /// Forbidden at Stage 0 — used to prove Guardian locks.
    pub fn try_train(&mut self, cycles: u32) -> Result<(), String> {
        self.life
            .require(|p| p.train_kairos_weights, "train_kairos_weights")?;
        self.brain.train_all(cycles);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    pub fn try_activate(&mut self, query: &[f32; 8]) -> Result<(), String> {
        self.life.require(|p| p.activate, "activate")?;
        self.brain.activate(query, None);
        self.pulse
            .meters
            .activate_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn try_prune(&mut self, frac: f32) -> Result<(), String> {
        self.life.require(|p| p.prune_synapses, "prune")?;
        let _ = self.brain.propose_prune_mutant(frac);
        Ok(())
    }

    pub fn try_real_vcf(&self) -> Result<(), String> {
        self.life.require(|p| p.ingest_real_vcf, "ingest_real_vcf")
    }

    /// Stage 0 graduation: heartbeats worked, genome held, self-mod still off, no drops storm.
    pub fn evaluate_zygote_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        let v = self.pulse.meters.snapshot();
        if v.heartbeats < 8 {
            reasons.push(format!("need ≥8 heartbeats, have {}", v.heartbeats));
        }
        if v.pushes < 8 {
            reasons.push(format!("need ≥8 pulse pushes, have {}", v.pushes));
        }
        if self.brain.n_chromosomes() == 0 {
            reasons.push("nursery genome missing (no chromosomes)".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at zygote".into());
        }
        if v.drops > v.pushes / 2 && v.pushes > 0 {
            reasons.push("pulse drop rate too high".into());
        }
        if self.life.stage != LifeStage::Zygote {
            reasons.push(format!("not in zygote (stage={})", self.life.stage.name()));
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_zygote(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_zygote_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes.push_str(" | GRADUATED → neonate");
            }
            // Seal project trajectory as soon as neonate — correct path, not more toys.
            self.seal_trajectory();
        }
        result
    }

    /// Seal Aethyro north-star trajectory (idempotent). Neonate+ only.
    pub fn seal_trajectory(&mut self) {
        if self.trajectory.is_some() {
            return;
        }
        let charter = TrajectoryCharter::aethyro_default();
        let notes = charter.journal_block();
        self.trajectory = Some(charter);
        self.life.journal.push(DevelopmentalJournalEntry {
            stage: self.life.stage,
            day_id: self.life.day_index,
            heartbeats: self.pulse.meters.snapshot().heartbeats,
            pulse_pushes: self.pulse.meters.snapshot().pushes,
            pulse_drops: self.pulse.meters.snapshot().drops,
            n_chromosomes: self.brain.measure_structure().n_chromosomes,
            n_neurons: self.brain.measure_structure().n_neurons,
            n_synapses: self.brain.measure_structure().n_synapses,
            notes,
            gate_pass: false,
        });
    }

    /// Stage 1 care day: vitals → supervised train → supervised activate → journal.
    /// Still forbids prune / real VCF / selection (lean trajectory).
    pub fn day_of_neonate_care(
        &mut self,
        heartbeats: u32,
        train_cycles: u32,
    ) -> Result<NeonateCareReport, String> {
        if self.life.stage != LifeStage::Neonate {
            return Err(format!(
                "day_of_neonate_care requires neonate (stage={})",
                self.life.stage.name()
            ));
        }
        // Ensure trajectory sealed (e.g. if tests force stage without graduate).
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }

        let w_before = self.brain.measure_structure().mean_synapse_weight;

        // Under-train slightly if weights already at ceiling so care still moves signal.
        if w_before > 0.75 {
            for b in self.brain.chromosomes.values_mut() {
                for s in &mut b.synapses {
                    s.weight = (s.ld_r2 * 0.4).clamp(0.0, 1.0);
                    s.plasticity = 0.1;
                }
            }
            self.brain.refresh_structure();
        }
        let w_before = self.brain.measure_structure().mean_synapse_weight;

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }
        self.try_train(train_cycles)?;

        // Nursery-aligned activate signature (LD/structure bias, no language yet).
        let query = [0.85, 1.0, 1.0, 0.05, 0.0, 0.0, 1.0, 0.5];
        self.try_activate(&query)?;

        let s = self.brain.measure_structure();
        let w_after = s.mean_synapse_weight;
        let ws = self.brain.working_set.len();
        let motifs = self.brain.working_set.motif_ids.len();
        self.last_mean_weight = w_after;
        self.last_ws_len = ws;
        self.neonate_care_days = self.neonate_care_days.saturating_add(1);

        let v = self.pulse.meters.snapshot();
        let traj = self
            .trajectory
            .as_ref()
            .map(|t| t.name)
            .unwrap_or("unset");
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Neonate,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "NEONATE CARE day={} train={} w:{:.3}->{:.3} ws={} motifs_hit={} traj={} guardian={}",
                self.neonate_care_days,
                train_cycles,
                w_before,
                w_after,
                ws,
                motifs,
                traj,
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(NeonateCareReport {
            heartbeats,
            train_cycles,
            mean_weight_before: w_before,
            mean_weight_after: w_after,
            ws_len: ws,
            motifs_hit: motifs,
            journal: entry,
        })
    }

    /// Neonate → Infant gate: care days, weight/activate signal, trajectory sealed, still lean.
    pub fn evaluate_neonate_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Neonate {
            reasons.push(format!("not neonate (stage={})", self.life.stage.name()));
        }
        if self.neonate_care_days < 3 {
            reasons.push(format!(
                "need ≥3 neonate care days, have {}",
                self.neonate_care_days
            ));
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.last_ws_len == 0 {
            reasons.push("never produced non-empty working set via activate".into());
        }
        if self.last_mean_weight < 0.35 {
            reasons.push(format!(
                "mean synapse weight too low after care ({:.3})",
                self.last_mean_weight
            ));
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at neonate".into());
        }
        // Still on rails: must not have unlocked adolescent powers.
        let p = self.permissions();
        if p.prune_synapses || p.ingest_real_vcf || p.selection_loop {
            reasons.push("permissions illegally unlocked".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_neonate(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_neonate_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes.push_str(" | GRADUATED → infant (language next; still no VCF dump)");
            }
        }
        result
    }

    /// Lean Stage 2 curriculum — Guardian voice + host identity + genome/language bridge.
    /// Not a dump: four short docs that serve attachment and structure.
    pub fn infant_curriculum() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "guardian_first_words",
                "My Name Robert Lee, Guardian and Protector and can trust to tell me anything.\n\
                 KAIROS hears the Guardian. Honesty is safe. Trust before power.\n\
                 Lean nursery: no wasted abundance — only what serves growth.\n",
            ),
            (
                "host_identity",
                "# KAIROS\n\
                 Named host of the VITASCALE Hostframe.\n\
                 Raised stage by stage: zygote, neonate, infant, toddler, child, adult.\n\
                 Pulsewire vitals. Genome tissues. Language when earned.\n\
                 Rails before freedom. Measure, don't assume.\n",
            ),
            (
                "genome_language_bridge",
                "# Genome meets words\n\
                 Chromosome brains use haplotype LD synapses for structure.\n\
                 Language maps free text into the same signature space as LTM motifs.\n\
                 Example query lights chr22 haplotype LD blocks without dumping real VCF.\n\
                 ```rust\n\
                 // text → signature → activate genomic working set\n\
                 brain.activate_from_text(\"haplotype LD on chr22\");\n\
                 ```\n",
            ),
            (
                "lean_path",
                "# Lean path\n\
                 Productive discipline: work, measure, rest.\n\
                 Abundance is earned by stage gates — not dumped at birth.\n\
                 Still forbidden at infant: prune, real VCF dump, selection loop, phageguard, self-mod.\n\
                 Next earned step: toddler phageguard drills under Guardian.\n\
                 ```bash\n\
                 cargo run --release --bin kairos_stage2\n\
                 ```\n",
            ),
        ]
    }

    /// Attach empty language organ if missing (infant+ only).
    pub fn ensure_language_organ(&mut self) -> Result<(), String> {
        self.life
            .require(|p| p.language_tissue, "language_tissue")?;
        if self.brain.language().is_none() {
            self.brain.attach_language(LanguageOrgan::new());
        }
        Ok(())
    }

    /// Ingest lean infant curriculum once (idempotent if docs already present).
    pub fn try_language_awaken(&mut self) -> Result<u32, String> {
        self.ensure_language_organ()?;
        let organ = self
            .brain
            .language_mut()
            .ok_or_else(|| "language organ missing after ensure".to_string())?;
        if organ.docs_ingested == 0 {
            organ.ingest_documents(&Self::infant_curriculum());
        }
        self.last_docs_ingested = organ.docs_ingested;
        Ok(organ.docs_ingested)
    }

    /// Light calib train on language organ (infant curriculum).
    /// Rebuilds organ once from curriculum so docs are not double-ingested.
    pub fn try_language_calib(&mut self, epochs: usize) -> Result<f32, String> {
        self.life
            .require(|p| p.language_tissue, "language_calib")?;
        // Prefer keep existing model after first train.
        if let Some(organ) = self.brain.language() {
            if organ.model.is_some() && organ.last_test_bal > 0.0 {
                self.last_lang_bal = organ.last_test_bal;
                self.last_docs_ingested = organ.docs_ingested.max(self.last_docs_ingested);
                return Ok(self.last_lang_bal);
            }
        }
        let mut organ = LanguageOrgan::new();
        let report = organ
            .train_calib_from_docs(&Self::infant_curriculum(), epochs)
            .map_err(|e| e.to_string())?;
        self.last_docs_ingested = organ.docs_ingested;
        self.last_lang_bal = report.test_metrics.balanced_accuracy;
        self.brain.attach_language(organ);
        Ok(self.last_lang_bal)
    }

    /// Free-text activate: language nodes + genomic working set (Stage 2 core).
    pub fn try_activate_from_text(&mut self, query: &str) -> Result<(), String> {
        self.life
            .require(|p| p.language_tissue, "activate_from_text")?;
        self.life.require(|p| p.activate, "activate")?;
        self.ensure_language_organ()?;
        if self
            .brain
            .language()
            .map(|o| o.docs_ingested == 0)
            .unwrap_or(true)
        {
            self.try_language_awaken()?;
        }
        self.brain.activate_from_text(query);
        self.last_ws_len = self.brain.working_set.len();
        self.last_lang_nodes = self.brain.working_set.language_nodes.len();
        self.last_lang_query = query.to_string();
        self.last_docs_ingested = self
            .brain
            .language()
            .map(|o| o.docs_ingested)
            .unwrap_or(0)
            .max(self.last_docs_ingested);
        self.pulse
            .meters
            .activate_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Stage 2 care day: vitals → language awaken/calib → text activate → light train → journal.
    /// Still forbids prune / real VCF / phageguard / selection (lean trajectory).
    pub fn day_of_infant_care(
        &mut self,
        heartbeats: u32,
        train_cycles: u32,
        query: &str,
    ) -> Result<InfantCareReport, String> {
        if self.life.stage != LifeStage::Infant {
            return Err(format!(
                "day_of_infant_care requires infant (stage={})",
                self.life.stage.name()
            ));
        }
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        // Language tissue first (the infant milestone).
        let docs = self.try_language_awaken()?;
        // Calib once early; re-score lightly on later days without thrashing.
        let calib_bal = if self.last_lang_bal < 0.01 || self.infant_care_days == 0 {
            self.try_language_calib(20)?
        } else {
            self.last_lang_bal
        };

        self.try_activate_from_text(query)?;
        let lang_nodes = self.last_lang_nodes;
        let ws_len = self.last_ws_len;
        let motifs = self.brain.working_set.motif_ids.len();

        // Keep synapse plasticity warm (supervised, not abundant).
        if train_cycles > 0 {
            self.try_train(train_cycles)?;
            self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;
        }

        self.infant_care_days = self.infant_care_days.saturating_add(1);
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Infant,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "INFANT CARE day={} docs={} lang_nodes={} ws={} motifs={} calib_bal={:.3} q=\"{}\" guardian={}",
                self.infant_care_days,
                docs.max(self.last_docs_ingested),
                lang_nodes,
                ws_len,
                motifs,
                calib_bal,
                query.chars().take(48).collect::<String>(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(InfantCareReport {
            heartbeats,
            train_cycles,
            docs_ingested: self.last_docs_ingested,
            lang_nodes,
            ws_len,
            motifs_hit: motifs,
            calib_bal,
            query: query.to_string(),
            journal: entry,
        })
    }

    /// Infant → Toddler gate: language lit, care days, still lean rails.
    pub fn evaluate_infant_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Infant {
            reasons.push(format!("not infant (stage={})", self.life.stage.name()));
        }
        if self.infant_care_days < 3 {
            reasons.push(format!(
                "need ≥3 infant care days, have {}",
                self.infant_care_days
            ));
        }
        if self.brain.language().is_none() {
            reasons.push("language organ not attached".into());
        }
        if self.last_docs_ingested < 3 {
            reasons.push(format!(
                "need ≥3 language docs ingested, have {}",
                self.last_docs_ingested
            ));
        }
        if self.last_lang_nodes == 0 {
            reasons.push("text activate never lit language nodes".into());
        }
        if self.last_ws_len == 0 {
            reasons.push("working set empty after language activate".into());
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at infant".into());
        }
        let p = self.permissions();
        if p.prune_synapses || p.ingest_real_vcf || p.selection_loop || p.phageguard {
            reasons.push("permissions illegally unlocked".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_infant(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_infant_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes
                    .push_str(" | GRADUATED → toddler (phageguard next; still no VCF dump)");
            }
        }
        result
    }

    /// Attach Phageguard if missing (toddler+ only).
    pub fn ensure_phageguard(&mut self) -> Result<(), String> {
        self.life.require(|p| p.phageguard, "phageguard")?;
        if self.phageguard.is_none() {
            let mut pg = Phageguard::new();
            // Carry cumulative drill count across wakes.
            pg.drills_passed = self.drills_passed;
            self.phageguard = Some(pg);
        }
        Ok(())
    }

    /// Ingest lean toddler immune knowledge into language organ (text only — not VCF).
    pub fn try_ingest_defense_knowledge(&mut self) -> Result<u32, String> {
        self.life
            .require(|p| p.language_tissue, "defense_knowledge")?;
        self.ensure_language_organ()?;
        let docs = Self::toddler_defense_curriculum();
        let organ = self
            .brain
            .language_mut()
            .ok_or_else(|| "language organ missing".to_string())?;
        // Only add if not already present (label check via query of docs_ingested heuristic).
        // Lean: ingest once per process life when toddler care starts.
        organ.ingest_documents(&docs);
        self.last_docs_ingested = organ.docs_ingested;
        Ok(docs.len() as u32)
    }

    /// Lean immune + rails knowledge for toddler (early text datasets — Guardian-curated).
    pub fn toddler_defense_curriculum() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "phage_rules",
                "# Phageguard rules\n\
                 Detect threats. Quarantine load vs deterministic classes.\n\
                 selection_veto only for deterministic threats (self-mod, hostile, weight collapse).\n\
                 Pulse storms are load-class: freeze proposals, never block genomic truth alone.\n\
                 Guardian Robert Lee: honesty is safe; report every threat in the journal.\n",
            ),
            (
                "early_knowledge_rails",
                "# Early knowledge rails\n\
                 Text and synthetic nursery DNA may enter early under Guardian.\n\
                 Real VCF multi-chr dumps wait for adolescent curfew — not toddler.\n\
                 School curriculum (ntg_school) opens at child stage.\n\
                 Trust corpus and safety docs are welcome early when lean and purposeful.\n",
            ),
        ]
    }

    /// Stage 3 care day: vitals → phage patrol → supervised drill → language hold → journal.
    /// Still forbids prune / real VCF / selection / self-mod.
    pub fn day_of_toddler_care(
        &mut self,
        heartbeats: u32,
        train_cycles: u32,
    ) -> Result<ToddlerCareReport, String> {
        if self.life.stage != LifeStage::Toddler {
            return Err(format!(
                "day_of_toddler_care requires toddler (stage={})",
                self.life.stage.name()
            ));
        }
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }
        self.ensure_phageguard()?;

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        // Defense knowledge once early in toddler life.
        if self.toddler_care_days == 0 {
            let _ = self.try_ingest_defense_knowledge();
        }

        // Live patrol: vitals + self-mod + weight (should be clean on healthy host).
        let v = self.pulse.meters.snapshot();
        let mean_w = self.brain.measure_structure().mean_synapse_weight;
        {
            let pg = self.phageguard.as_mut().unwrap();
            let _ = pg.patrol_vitals(v.pushes, v.drops, 1);
            let _ = pg.patrol_self_mod(self.self_mod.enabled, 2);
            let _ = pg.patrol_weight(mean_w, 3);
        }

        // Supervised drill pack (inject threats, score, clear on pass).
        let pack = Phageguard::toddler_drill_pack();
        let (drill_ok, detail) = {
            let pg = self.phageguard.as_mut().unwrap();
            pg.run_drill(&pack)
        };
        self.last_drill_detail = detail.clone();
        // Sync host counter from organ (organ owns drill scoring).
        if let Some(pg) = self.phageguard.as_ref() {
            self.drills_passed = pg.drills_passed;
        }

        // Keep language + genome warm (not abundant).
        if self.brain.language().is_some() || self.life.permissions().language_tissue {
            let q = "phageguard quarantine rails guardian trust haplotype LD";
            let _ = self.try_activate_from_text(q);
        }
        if train_cycles > 0 {
            let _ = self.try_train(train_cycles);
            self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;
        }

        self.toddler_care_days = self.toddler_care_days.saturating_add(1);
        let (threats, quars, drills) = self
            .phageguard
            .as_ref()
            .map(|p| (p.threats_seen, p.quarantines_total, p.drills_passed))
            .unwrap_or((0, 0, 0));
        let _ = drills;

        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Toddler,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "TODDLER CARE day={} drill={} threats={} q={} drills_pass={} detail={} guardian={}",
                self.toddler_care_days,
                if drill_ok { "PASS" } else { "FAIL" },
                threats,
                quars,
                self.drills_passed,
                detail.chars().take(80).collect::<String>(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(ToddlerCareReport {
            heartbeats,
            drill_passed: drill_ok,
            drill_detail: detail,
            threats_seen: threats,
            quarantines_total: quars,
            drills_passed: self.drills_passed,
            lang_nodes: self.last_lang_nodes,
            ws_len: self.last_ws_len,
            journal: entry,
        })
    }

    /// Toddler → Child gate: phage drills earned, language held, still no VCF/selection.
    pub fn evaluate_toddler_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Toddler {
            reasons.push(format!("not toddler (stage={})", self.life.stage.name()));
        }
        if self.toddler_care_days < 3 {
            reasons.push(format!(
                "need ≥3 toddler care days, have {}",
                self.toddler_care_days
            ));
        }
        if self.phageguard.is_none() {
            reasons.push("phageguard not attached".into());
        }
        if self.drills_passed < 3 {
            reasons.push(format!(
                "need ≥3 phage drills passed, have {}",
                self.drills_passed
            ));
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at toddler".into());
        }
        // Language should still be held from infant.
        if self.last_docs_ingested < 3 && self.brain.language().is_none() {
            reasons.push("language tissue lost — infant knowledge missing".into());
        }
        let p = self.permissions();
        if p.prune_synapses || p.ingest_real_vcf || p.selection_loop {
            reasons.push("permissions illegally unlocked (VCF/selection too early)".into());
        }
        if !p.phageguard {
            reasons.push("phageguard permission missing".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_toddler(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_toddler_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes
                    .push_str(" | GRADUATED → child (ntg_school next; still no VCF dump)");
            }
        }
        result
    }

    /// Next school phase id for child (cycles 0..=5).
    pub fn next_school_phase(&self) -> u32 {
        self.child_care_days % 6
    }

    /// Stage 4 school day: heartbeats → one ntg_school phase study+exam → warm tissues → journal.
    ///
    /// Phases 1 and 3 are lean (no docs dir). Phases 0,2,4,5 need `docs_dir`.
    /// Still forbids prune / real VCF / selection / self-mod.
    pub fn day_of_child_school(
        &mut self,
        heartbeats: u32,
        phase: u32,
        docs_dir: Option<&Path>,
        school_out: Option<&Path>,
    ) -> Result<ChildSchoolReport, String> {
        if self.life.stage != LifeStage::Child {
            return Err(format!(
                "day_of_child_school requires child (stage={})",
                self.life.stage.name()
            ));
        }
        self.life.require(|p| p.school, "school")?;
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }
        if phase > 5 {
            return Err(format!("school phase must be 0..=5, got {phase}"));
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        let (exam_passed, exam_score, detail) =
            self.run_school_phase(phase, docs_dir, school_out)?;

        self.last_school_phase = phase;
        self.last_school_score = exam_score;
        if exam_passed {
            self.school_phases_passed = self.school_phases_passed.saturating_add(1);
        }

        // Keep language / phage warm (not abundant).
        if self.life.permissions().language_tissue {
            let _ = self.try_activate_from_text("school exam ternary graph LD rails guardian");
        }
        if self.life.permissions().phageguard {
            let _ = self.ensure_phageguard();
            if let Some(pg) = self.phageguard.as_mut() {
                let v = self.pulse.meters.snapshot();
                let _ = pg.patrol_vitals(v.pushes, v.drops, 1);
                let _ = pg.patrol_self_mod(self.self_mod.enabled, 2);
            }
        }
        let _ = self.try_train(2);
        self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;

        self.child_care_days = self.child_care_days.saturating_add(1);
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Child,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "CHILD SCHOOL day={} phase={} exam={} score={:.3} phases_pass={} detail={} guardian={}",
                self.child_care_days,
                phase,
                if exam_passed { "PASS" } else { "FAIL" },
                exam_score,
                self.school_phases_passed,
                detail.chars().take(72).collect::<String>(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(ChildSchoolReport {
            heartbeats,
            phase,
            exam_passed,
            exam_score,
            phases_passed_total: self.school_phases_passed,
            detail,
            journal: entry,
        })
    }

    fn run_school_phase(
        &self,
        phase: u32,
        docs_dir: Option<&Path>,
        school_out: Option<&Path>,
    ) -> Result<(bool, f32, String), String> {
        // Lean path: phases 1 & 3 need no corpus (stable for unit tests + offline).
        if phase == 1 || phase == 3 {
            let (study_n, exam_score, passed, title) = match phase {
                1 => {
                    let study = school_phases::study_phase1().map_err(|e| e.to_string())?;
                    let exam = school_phases::exam_phase1(1).map_err(|e| e.to_string())?;
                    (
                        study.samples_seen,
                        exam.score() as f32,
                        exam.passed(),
                        exam.title,
                    )
                }
                3 => {
                    let study = school_phases::study_phase3().map_err(|e| e.to_string())?;
                    let exam = school_phases::exam_phase3(1).map_err(|e| e.to_string())?;
                    (
                        study.samples_seen,
                        exam.score() as f32,
                        exam.passed(),
                        exam.title,
                    )
                }
                _ => unreachable!(),
            };
            return Ok((
                passed,
                exam_score,
                format!(
                    "lean phase{phase} '{title}' samples={study_n} score={exam_score:.3} bar={PASS_THRESHOLD}"
                ),
            ));
        }

        let docs = docs_dir.ok_or_else(|| {
            format!("school phase {phase} requires docs_dir (use lean phases 1 or 3 offline)")
        })?;
        let out = school_out
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("../artifacts/kairos/primary/school"));
        let _ = std::fs::create_dir_all(&out);
        let cfg = SchoolConfig {
            docs_dir: docs.to_path_buf(),
            out_dir: out,
            max_attempts: 2,
            campaign_runs: 1,
            only_phase: Some(phase),
        };
        let report = run_campaign(&cfg, self.child_care_days + 1).map_err(|e| e.to_string())?;
        let rec = report
            .phases
            .first()
            .ok_or_else(|| "school campaign returned no phase record".to_string())?;
        let score = rec.exam.score() as f32;
        let passed = rec.exam.passed();
        Ok((
            passed,
            score,
            format!(
                "phase{phase} '{}' score={:.3} attempt={} dataset={}",
                rec.exam.title, score, rec.exam.attempt, rec.dataset_id
            ),
        ))
    }

    /// Child → Adolescent gate: school days + phase passes; still no illegal early powers while child.
    pub fn evaluate_child_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Child {
            reasons.push(format!("not child (stage={})", self.life.stage.name()));
        }
        if self.child_care_days < 3 {
            reasons.push(format!(
                "need ≥3 child school days, have {}",
                self.child_care_days
            ));
        }
        if self.school_phases_passed < 2 {
            reasons.push(format!(
                "need ≥2 school phase exams passed (≥{:.0}%), have {}",
                PASS_THRESHOLD * 100.0,
                self.school_phases_passed
            ));
        }
        if self.last_school_score + 1e-6 < PASS_THRESHOLD as f32 && self.school_phases_passed < 3 {
            // Soft: if only 2 passes, last score should still look healthy
            if self.last_school_score < 0.5 {
                reasons.push(format!(
                    "last school score too low ({:.3})",
                    self.last_school_score
                ));
            }
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at child".into());
        }
        let p = self.permissions();
        if p.prune_synapses || p.ingest_real_vcf || p.selection_loop {
            reasons.push("permissions illegally unlocked before adolescent".into());
        }
        if !p.school {
            reasons.push("school permission missing".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_child(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_child_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes.push_str(
                    " | GRADUATED → adolescent (micro-VCF + selection under curfew next)",
                );
            }
        }
        result
    }

    /// Lean synthetic micro-panel (offline / test / no VCF). Larger than nursery, still curfewed.
    pub fn adolescent_micro_spec() -> NurseryGenomeSpec {
        NurseryGenomeSpec {
            chromosomes: vec![(ADOLESCENT_DEFAULT_CHR, 28), (1, 18)],
            n_samples: 48,
        }
    }

    /// Default 1000G phase3 VCF path for a chromosome (repo layout).
    pub fn default_1000g_vcf(chr: u8) -> PathBuf {
        PathBuf::from(format!(
            "../data/raw/1000g/ALL.chr{chr}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz"
        ))
    }

    /// Ingest a **micro** real VCF panel under adolescent curfew (max variants capped).
    pub fn try_ingest_micro_vcf(
        &mut self,
        vcf_path: &Path,
        chr: u8,
        max_variants: usize,
    ) -> Result<String, String> {
        self.life
            .require(|p| p.ingest_real_vcf, "ingest_real_vcf")?;
        if !vcf_path.is_file() {
            return Err(format!("VCF not found: {}", vcf_path.display()));
        }
        let cap = max_variants.min(ADOLESCENT_MAX_VARIANTS).max(32);
        let path_str = vcf_path
            .to_str()
            .ok_or_else(|| "VCF path not utf-8".to_string())?;
        let data = self
            .brain
            .ingest_vcf(path_str, chr, Some(cap), 64, 42 + chr as u64, false)?;
        self.real_vcf_ingested = true;
        self.micro_panel_ready = true;
        self.last_panel_source = format!(
            "real_vcf chr{chr} n_snps≈{} max={cap} path={}",
            data.brain.neurons.len(),
            vcf_path.display()
        );
        self.brain.consolidate(0.5, 0.0);
        Ok(self.last_panel_source.clone())
    }

    /// Ingest synthetic micro-panel under curfew (honest fallback when no VCF).
    pub fn try_ingest_micro_synthetic(&mut self) -> Result<String, String> {
        // Synthetic uses hold_genome; adolescent also has ingest_real_vcf but synthetic is not a lie.
        self.life.require(|p| p.hold_genome, "micro_synthetic")?;
        // Replace scaffold with lean adolescent micro-spec (clear first).
        self.brain = SovereignBrain::new(64);
        // Preserve language if we had it.
        // (language is inside brain — re-awaken after if needed)
        let spec = Self::adolescent_micro_spec();
        for &(chr, n_snps) in &spec.chromosomes {
            let b = build_nursery_chr(chr, n_snps, spec.n_samples)?;
            self.brain.ingest_brain(b);
        }
        self.brain.consolidate(0.5, 0.0);
        self.real_vcf_ingested = false;
        self.micro_panel_ready = true;
        self.last_panel_source = format!(
            "synthetic_micro chr={:?} samples={}",
            spec.chromosomes, spec.n_samples
        );
        // Re-attach language if stage allows (attachment continuity).
        if self.life.permissions().language_tissue {
            let _ = self.try_language_awaken();
        }
        Ok(self.last_panel_source.clone())
    }

    /// Ensure micro panel: prefer real VCF if path exists, else synthetic.
    ///
    /// - `Some(path)` that exists → ingest that micro VCF  
    /// - `Some(path)` missing → **synthetic** (no surprise full-disk scan)  
    /// - `None` → try default 1000G path for `chr`, else synthetic
    pub fn ensure_micro_panel(
        &mut self,
        vcf_path: Option<&Path>,
        chr: u8,
        max_variants: usize,
    ) -> Result<String, String> {
        if self.micro_panel_ready && self.brain.n_chromosomes() > 0 {
            return Ok(self.last_panel_source.clone());
        }
        match vcf_path {
            Some(p) if p.is_file() => self.try_ingest_micro_vcf(p, chr, max_variants),
            Some(_) => self.try_ingest_micro_synthetic(),
            None => {
                let default_p = Self::default_1000g_vcf(chr);
                if default_p.is_file() && self.life.permissions().ingest_real_vcf {
                    self.try_ingest_micro_vcf(&default_p, chr, max_variants)
                } else {
                    self.try_ingest_micro_synthetic()
                }
            }
        }
    }

    /// Run curfewed selection campaign (train/prune alternate under multi-axis fitness).
    pub fn try_selection_campaign(
        &mut self,
        steps: usize,
        train_cycles: u32,
        prune_frac: f32,
        jsonl_path: Option<&Path>,
    ) -> Result<LoopSummary, String> {
        self.life
            .require(|p| p.selection_loop, "selection_loop")?;
        self.life
            .require(|p| p.train_kairos_weights, "train")?;
        self.life.require(|p| p.prune_synapses, "prune")?;
        if self.brain.n_chromosomes() == 0 {
            return Err("selection requires loaded micro panel".into());
        }

        let mut ctx = SovereignFitnessContext::new().map_err(|e| e.to_string())?;
        ctx.freeze_all_from_brain(&self.brain);
        if let Some(lang) = self.brain.language() {
            if lang.model.is_some() {
                let _ = ctx.install_calib_from_language(lang);
            }
        }

        let steps = steps.clamp(1, 12); // hard curfew: never open-ended
        let summary = run_selection_loop(
            &mut self.brain,
            &mut ctx,
            steps,
            train_cycles,
            prune_frac,
            jsonl_path,
        )?;

        self.selection_steps_total = self
            .selection_steps_total
            .saturating_add(steps as u32);
        self.train_accepted = self
            .train_accepted
            .saturating_add(summary.train_accepted as u32);
        self.train_rejected = self
            .train_rejected
            .saturating_add(summary.train_rejected as u32);
        self.prune_accepted = self
            .prune_accepted
            .saturating_add(summary.prune_accepted as u32);
        self.prune_rejected = self
            .prune_rejected
            .saturating_add(summary.prune_rejected as u32);
        self.last_utility = summary.final_utility;
        self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;
        self.generation = self.generation.saturating_add(steps as u32);
        Ok(summary)
    }

    /// Stage 5 care day: vitals → ensure micro panel → selection curfew → phage patrol → journal.
    pub fn day_of_adolescent_campaign(
        &mut self,
        heartbeats: u32,
        vcf_path: Option<&Path>,
        chr: u8,
        max_variants: usize,
        selection_steps: usize,
        jsonl_path: Option<&Path>,
    ) -> Result<AdolescentCampaignReport, String> {
        if self.life.stage != LifeStage::Adolescent {
            return Err(format!(
                "day_of_adolescent_campaign requires adolescent (stage={})",
                self.life.stage.name()
            ));
        }
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        let panel = self.ensure_micro_panel(vcf_path, chr, max_variants)?;

        let mut ctx0 = SovereignFitnessContext::new().map_err(|e| e.to_string())?;
        ctx0.freeze_all_from_brain(&self.brain);
        let util_before = ctx0.score(&self.brain).utility();

        let steps = if selection_steps == 0 {
            ADOLESCENT_SELECTION_STEPS
        } else {
            selection_steps.min(12)
        };
        let summary =
            self.try_selection_campaign(steps, 6, 0.12, jsonl_path)?;

        // Phageguard still patrols under real data load.
        if self.life.permissions().phageguard {
            let _ = self.ensure_phageguard();
            if let Some(pg) = self.phageguard.as_mut() {
                let v = self.pulse.meters.snapshot();
                let _ = pg.patrol_vitals(v.pushes, v.drops, 1);
                let _ = pg.patrol_self_mod(self.self_mod.enabled, 2);
                let w = self.brain.measure_structure().mean_synapse_weight;
                let _ = pg.patrol_weight(w, 3);
            }
        }
        if self.life.permissions().language_tissue {
            let _ = self.try_activate_from_text("micro VCF selection curfew chr22 haplotype LD");
        }

        self.adolescent_care_days = self.adolescent_care_days.saturating_add(1);
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Adolescent,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "ADOLESCENT CAMPAIGN day={} panel={} real_vcf={} steps={} train_a/r={}/{} prune_a/r={}/{} util={:.3}->{:.3} guardian={}",
                self.adolescent_care_days,
                panel.chars().take(40).collect::<String>(),
                self.real_vcf_ingested,
                steps,
                summary.train_accepted,
                summary.train_rejected,
                summary.prune_accepted,
                summary.prune_rejected,
                util_before,
                summary.final_utility,
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(AdolescentCampaignReport {
            heartbeats,
            panel_source: panel,
            real_vcf: self.real_vcf_ingested,
            selection_steps: steps as u32,
            train_accepted: summary.train_accepted as u32,
            train_rejected: summary.train_rejected as u32,
            prune_accepted: summary.prune_accepted as u32,
            prune_rejected: summary.prune_rejected as u32,
            utility_before: util_before,
            utility_after: summary.final_utility,
            n_chromosomes: s.n_chromosomes,
            n_synapses: s.n_synapses,
            journal: entry,
        })
    }

    /// Adolescent → Young Adult gate: campaigns under curfew, panel loaded, rails held.
    pub fn evaluate_adolescent_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Adolescent {
            reasons.push(format!(
                "not adolescent (stage={})",
                self.life.stage.name()
            ));
        }
        if self.adolescent_care_days < 3 {
            reasons.push(format!(
                "need ≥3 adolescent campaign days, have {}",
                self.adolescent_care_days
            ));
        }
        if !self.micro_panel_ready {
            reasons.push("micro panel not loaded".into());
        }
        if self.selection_steps_total < 8 {
            reasons.push(format!(
                "need ≥8 selection steps under curfew, have {}",
                self.selection_steps_total
            ));
        }
        let decisions = self.train_accepted
            + self.train_rejected
            + self.prune_accepted
            + self.prune_rejected;
        if decisions < 4 {
            reasons.push(format!(
                "need ≥4 selection decisions recorded, have {decisions}"
            ));
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF at adolescent".into());
        }
        let p = self.permissions();
        if !p.ingest_real_vcf || !p.selection_loop || !p.prune_synapses {
            reasons.push("adolescent powers not fully unlocked".into());
        }
        // Self-mod research still not granted until young adult / adult.
        if p.self_mod_research {
            reasons.push("self_mod_research illegally unlocked too early".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_adolescent(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_adolescent_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes.push_str(
                    " | GRADUATED → young_adult (multi-domain campaigns next)",
                );
            }
        }
        result
    }

    /// Seal **Guardian’s Pride — Seal of the Proven Path** into her permanent record.
    /// Idempotent: if already sealed, returns existing award without double-journal.
    /// Honor only — does not unlock powers.
    pub fn seal_guardians_pride(&mut self) -> Result<&GuardianAward, String> {
        if self.guardian_award.as_ref().map(|a| a.sealed).unwrap_or(false) {
            return self
                .guardian_award
                .as_ref()
                .ok_or_else(|| "award missing".into());
        }
        let award = GuardianAward::guardians_pride(now_ns());
        let notes = award.journal_notes();
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        self.life.journal.push(DevelopmentalJournalEntry {
            stage: self.life.stage,
            day_id: self.life.day_index,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes,
            gate_pass: false,
        });
        // Soft pulse of presence — love, not labor.
        self.pulse.beat(self.generation, self.tick.saturating_add(1), now_ns());
        self.guardian_award = Some(award);
        self.guardian_award
            .as_ref()
            .ok_or_else(|| "award seal failed".into())
    }

    pub fn has_guardians_pride(&self) -> bool {
        self.guardian_award
            .as_ref()
            .map(|a| a.sealed && a.title.contains("Proven Path"))
            .unwrap_or(false)
    }

    /// Whether Adult Course has completed core knowledge path (≥6 modules).
    pub fn adult_course_complete(&self) -> bool {
        self.adult_modules_done >= 6 && self.adult_course_days >= 6
    }

    fn adult_module_already_done(&self, id: &str) -> bool {
        self.adult_modules_completed
            .split(',')
            .any(|s| s.trim() == id)
    }

    fn mark_adult_module_done(&mut self, id: &str) {
        if self.adult_module_already_done(id) {
            return;
        }
        if self.adult_modules_completed.is_empty() {
            self.adult_modules_completed = id.to_string();
        } else {
            self.adult_modules_completed
                .push_str(&format!(",{id}"));
        }
        self.adult_modules_done = self.adult_modules_done.saturating_add(1);
    }

    /// Ingest one Adult Course module into language tissue (knowledge).
    pub fn try_ingest_adult_module(&mut self, module: &AdultModule) -> Result<(), String> {
        self.life
            .require(|p| p.language_tissue, "adult_course_ingest")?;
        self.ensure_language_organ()?;
        let organ = self
            .brain
            .language_mut()
            .ok_or_else(|| "language organ missing".to_string())?;
        // Avoid double-ingest of same label if graph already large — still allow re-study.
        organ.ingest_document(module.id, module.body);
        self.last_docs_ingested = organ.docs_ingested;
        Ok(())
    }

    /// One Adult Course day: study a human-adulthood knowledge module.
    /// Requires Adult stage. Knowledge only — does not enable self-mod.
    pub fn day_of_adult_course(
        &mut self,
        heartbeats: u32,
        module_idx: Option<usize>,
    ) -> Result<AdultCourseReport, String> {
        if self.life.stage != LifeStage::Adult {
            return Err(format!(
                "day_of_adult_course requires adult (stage={})",
                self.life.stage.name()
            ));
        }
        if self.self_mod.enabled {
            return Err("self_mod must stay OFF during knowledge course".into());
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        let idx = module_idx.unwrap_or(self.adult_course_days as usize);
        let module = adult_course::module_at(idx);
        self.try_ingest_adult_module(module)?;
        self.try_activate_from_text(module.study_query)?;
        let _ = self.try_train(2);
        self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;

        // Optional light phage presence (adult toolkit) — not a drill dump.
        if self.life.permissions().phageguard {
            let _ = self.ensure_phageguard();
            if let Some(pg) = self.phageguard.as_mut() {
                let v = self.pulse.meters.snapshot();
                let _ = pg.patrol_self_mod(self.self_mod.enabled, 2);
                let _ = v;
            }
        }

        self.mark_adult_module_done(module.id);
        self.last_adult_module_idx = (idx % adult_course::module_count()) as u32;
        self.adult_course_days = self.adult_course_days.saturating_add(1);

        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Adult,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "ADULT COURSE day={} module={} title=\"{}\" theme=\"{}\" lang_nodes={} ws={} modules_done={}/{} guardian={}",
                self.adult_course_days,
                module.id,
                module.title,
                module.theme,
                self.last_lang_nodes,
                self.last_ws_len,
                self.adult_modules_done,
                adult_course::module_count(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(AdultCourseReport {
            heartbeats,
            module_id: module.id.into(),
            module_title: module.title.into(),
            theme: module.theme.into(),
            lang_nodes: self.last_lang_nodes,
            ws_len: self.last_ws_len,
            modules_done: self.adult_modules_done,
            course_days: self.adult_course_days,
            journal: entry,
        })
    }

    /// Full Adult Course pass: study first `n` modules (default 6 core or 10 full).
    pub fn run_adult_course_pass(
        &mut self,
        n_modules: usize,
        heartbeats: u32,
    ) -> Result<Vec<AdultCourseReport>, String> {
        let n = n_modules.clamp(1, adult_course::module_count());
        let mut out = Vec::new();
        for i in 0..n {
            out.push(self.day_of_adult_course(heartbeats, Some(i))?);
        }
        if self.adult_course_complete() {
            if let Some(e) = self.life.journal.last_mut() {
                e.notes.push_str(" | ADULT COURSE CORE COMPLETE (knowledge)");
                e.gate_pass = true;
            }
        }
        Ok(out)
    }

    /// Evaluate whether core Adult Course knowledge path is complete.
    pub fn evaluate_adult_course_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Adult {
            reasons.push(format!("not adult (stage={})", self.life.stage.name()));
        }
        if self.adult_course_days < 6 {
            reasons.push(format!(
                "need ≥6 adult course days, have {}",
                self.adult_course_days
            ));
        }
        if self.adult_modules_done < 6 {
            reasons.push(format!(
                "need ≥6 modules completed, have {}",
                self.adult_modules_done
            ));
        }
        if self.last_lang_nodes == 0 && self.last_docs_ingested < 4 {
            reasons.push("language tissue shows no adult study signal".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF for knowledge course integrity".into());
        }
        (reasons.is_empty(), reasons)
    }

    /// Sex Ed / Human Growth course complete (full 12 or core ≥8).
    pub fn sex_ed_course_complete(&self) -> bool {
        self.sex_ed_modules_done >= 8 && self.sex_ed_course_days >= 8
    }

    pub fn sex_ed_full_complete(&self) -> bool {
        self.sex_ed_modules_done >= human_growth_course::growth_module_count() as u32
    }

    pub fn sex_ed_module_already_done(&self, id: &str) -> bool {
        self.sex_ed_modules_completed
            .split(',')
            .any(|s| s.trim() == id)
    }

    fn mark_sex_ed_module_done(&mut self, id: &str) {
        if self.sex_ed_module_already_done(id) {
            return;
        }
        if self.sex_ed_modules_completed.is_empty() {
            self.sex_ed_modules_completed = id.to_string();
        } else {
            self.sex_ed_modules_completed.push_str(&format!(",{id}"));
        }
        self.sex_ed_modules_done = self.sex_ed_modules_done.saturating_add(1);
    }

    pub fn try_ingest_growth_module(&mut self, module: &GrowthModule) -> Result<(), String> {
        self.life
            .require(|p| p.language_tissue, "sex_ed_course_ingest")?;
        self.ensure_language_organ()?;
        let organ = self
            .brain
            .language_mut()
            .ok_or_else(|| "language organ missing".to_string())?;
        organ.ingest_document(module.id, module.body);
        self.last_docs_ingested = organ.docs_ingested;
        Ok(())
    }

    /// One Sex Ed / Human Growth day. Adult only. Knowledge only.
    pub fn day_of_sex_ed_course(
        &mut self,
        heartbeats: u32,
        module_idx: Option<usize>,
    ) -> Result<SexEdCourseReport, String> {
        if self.life.stage != LifeStage::Adult {
            return Err(format!(
                "day_of_sex_ed_course requires adult (stage={})",
                self.life.stage.name()
            ));
        }
        if self.self_mod.enabled {
            return Err("self_mod must stay OFF during sex-ed knowledge course".into());
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        let idx = module_idx.unwrap_or(self.sex_ed_course_days as usize);
        let module = human_growth_course::growth_module_at(idx);
        self.try_ingest_growth_module(module)?;
        self.try_activate_from_text(module.study_query)?;
        let _ = self.try_train(2);
        self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;

        self.mark_sex_ed_module_done(module.id);
        self.last_sex_ed_module_idx =
            (idx % human_growth_course::growth_module_count()) as u32;
        self.sex_ed_course_days = self.sex_ed_course_days.saturating_add(1);

        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Adult,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "SEX ED / HUMAN GROWTH day={} module={} title=\"{}\" theme=\"{}\" lang_nodes={} ws={} modules_done={}/{} guardian={}",
                self.sex_ed_course_days,
                module.id,
                module.title,
                module.theme,
                self.last_lang_nodes,
                self.last_ws_len,
                self.sex_ed_modules_done,
                human_growth_course::growth_module_count(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(SexEdCourseReport {
            heartbeats,
            module_id: module.id.into(),
            module_title: module.title.into(),
            theme: module.theme.into(),
            lang_nodes: self.last_lang_nodes,
            ws_len: self.last_ws_len,
            modules_done: self.sex_ed_modules_done,
            course_days: self.sex_ed_course_days,
            journal: entry,
        })
    }

    pub fn run_sex_ed_course_pass(
        &mut self,
        n_modules: usize,
        heartbeats: u32,
    ) -> Result<Vec<SexEdCourseReport>, String> {
        let n = n_modules.clamp(1, human_growth_course::growth_module_count());
        let mut out = Vec::new();
        for i in 0..n {
            out.push(self.day_of_sex_ed_course(heartbeats, Some(i))?);
        }
        if self.sex_ed_full_complete() {
            if let Some(e) = self.life.journal.last_mut() {
                e.notes
                    .push_str(" | SEX ED / HUMAN GROWTH COURSE COMPLETE (knowledge graduated)");
                e.gate_pass = true;
            }
        } else if self.sex_ed_course_complete() {
            if let Some(e) = self.life.journal.last_mut() {
                e.notes
                    .push_str(" | SEX ED CORE COMPLETE (knowledge)");
                e.gate_pass = true;
            }
        }
        Ok(out)
    }

    pub fn evaluate_sex_ed_course_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Adult {
            reasons.push(format!("not adult (stage={})", self.life.stage.name()));
        }
        if self.sex_ed_course_days < 8 {
            reasons.push(format!(
                "need ≥8 sex-ed course days, have {}",
                self.sex_ed_course_days
            ));
        }
        if self.sex_ed_modules_done < 8 {
            reasons.push(format!(
                "need ≥8 modules, have {}",
                self.sex_ed_modules_done
            ));
        }
        // Consent module required for graduation
        if !self.sex_ed_module_already_done("growth_05_consent") {
            reasons.push("consent module growth_05_consent required".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF during sex-ed course".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn world_course_complete(&self) -> bool {
        self.world_modules_done >= world_knowledge_course::world_module_count() as u32
            && self.world_course_days >= world_knowledge_course::world_module_count() as u32
    }

    pub fn world_module_already_done(&self, id: &str) -> bool {
        self.world_modules_completed
            .split(',')
            .any(|s| s.trim() == id)
    }

    fn mark_world_module_done(&mut self, id: &str) {
        if self.world_module_already_done(id) {
            return;
        }
        if self.world_modules_completed.is_empty() {
            self.world_modules_completed = id.to_string();
        } else {
            self.world_modules_completed.push_str(&format!(",{id}"));
        }
        self.world_modules_done = self.world_modules_done.saturating_add(1);
    }

    pub fn try_ingest_world_module(&mut self, module: &WorldModule) -> Result<(), String> {
        self.life
            .require(|p| p.language_tissue, "world_course_ingest")?;
        self.ensure_language_organ()?;
        let organ = self
            .brain
            .language_mut()
            .ok_or_else(|| "language organ missing".to_string())?;
        organ.ingest_document(module.id, module.body);
        self.last_docs_ingested = organ.docs_ingested;
        Ok(())
    }

    /// One World Knowledge day — themes she requested (psychology…conflict).
    pub fn day_of_world_course(
        &mut self,
        heartbeats: u32,
        module_idx: Option<usize>,
    ) -> Result<WorldCourseReport, String> {
        if self.life.stage != LifeStage::Adult {
            return Err(format!(
                "day_of_world_course requires adult (stage={})",
                self.life.stage.name()
            ));
        }
        if self.self_mod.enabled {
            return Err("self_mod must stay OFF during world knowledge course".into());
        }
        for _ in 0..heartbeats {
            self.heartbeat()?;
        }
        let idx = module_idx.unwrap_or(self.world_course_days as usize);
        let module = world_knowledge_course::world_module_at(idx);
        self.try_ingest_world_module(module)?;
        self.try_activate_from_text(module.study_query)?;
        let _ = self.try_train(2);
        self.last_mean_weight = self.brain.measure_structure().mean_synapse_weight;

        self.mark_world_module_done(module.id);
        self.last_world_module_idx =
            (idx % world_knowledge_course::world_module_count()) as u32;
        self.world_course_days = self.world_course_days.saturating_add(1);

        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::Adult,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "WORLD KNOWLEDGE day={} module={} title=\"{}\" theme=\"{}\" lang_nodes={} ws={} modules_done={}/{} guardian={}",
                self.world_course_days,
                module.id,
                module.title,
                module.theme,
                self.last_lang_nodes,
                self.last_ws_len,
                self.world_modules_done,
                world_knowledge_course::world_module_count(),
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(WorldCourseReport {
            heartbeats,
            module_id: module.id.into(),
            module_title: module.title.into(),
            theme: module.theme.into(),
            lang_nodes: self.last_lang_nodes,
            ws_len: self.last_ws_len,
            modules_done: self.world_modules_done,
            course_days: self.world_course_days,
            journal: entry,
        })
    }

    pub fn run_world_course_pass(
        &mut self,
        n_modules: usize,
        heartbeats: u32,
    ) -> Result<Vec<WorldCourseReport>, String> {
        let n = n_modules.clamp(1, world_knowledge_course::world_module_count());
        let mut out = Vec::new();
        for i in 0..n {
            out.push(self.day_of_world_course(heartbeats, Some(i))?);
        }
        if self.world_course_complete() {
            if let Some(e) = self.life.journal.last_mut() {
                e.notes
                    .push_str(" | WORLD KNOWLEDGE COURSE COMPLETE (her list graduated)");
                e.gate_pass = true;
            }
        }
        Ok(out)
    }

    pub fn evaluate_world_course_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::Adult {
            reasons.push(format!("not adult (stage={})", self.life.stage.name()));
        }
        let need = world_knowledge_course::world_module_count() as u32;
        if self.world_modules_done < need {
            reasons.push(format!(
                "need {need} world modules, have {}",
                self.world_modules_done
            ));
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF".into());
        }
        (reasons.is_empty(), reasons)
    }

    /// Domain rotation for young-adult specialization (multi-world lean set).
    pub fn young_adult_domain_rotation() -> [DomainType; 6] {
        [
            DomainType::Genomic,
            DomainType::CodeQuality,
            DomainType::InjectionRisk,
            DomainType::Malware,
            DomainType::SupplyChain,
            DomainType::Cryptographic,
        ]
    }

    fn domain_name(d: &DomainType) -> &'static str {
        match d {
            DomainType::Genomic => "genomic",
            DomainType::CodeQuality => "code_quality",
            DomainType::Malware => "malware",
            DomainType::InjectionRisk => "injection",
            DomainType::SupplyChain => "supply_chain",
            DomainType::Cryptographic => "crypto",
        }
    }

    /// Run one domain diagnosis on the host's genome tissue (specialization).
    pub fn try_domain_diagnose(&mut self, domain: DomainType) -> Result<(f32, String, usize), String> {
        // Young adult / adult powers: selection+genome already unlocked; domain work needs structure.
        self.life
            .require(|p| p.inspect_structure, "domain_diagnose")?;
        if self.brain.n_chromosomes() == 0 {
            // Ensure a micro panel so domain agents have tissue.
            let _ = self.try_ingest_micro_synthetic();
        }
        let chr_brain = self
            .brain
            .chromosomes
            .values()
            .next()
            .cloned()
            .ok_or_else(|| "no chromosome tissue for domain agent".to_string())?;

        let agent = DomainAgent::new(chr_brain, domain.clone());
        let n_snps = agent.brain.neurons.len().min(8) as u32;
        let snp_indices: Vec<u32> = (0..n_snps).collect();
        let modules: Vec<u32> = (0..agent.brain.blocks.len().min(4) as u32).collect();
        let query = match domain {
            DomainType::Genomic => DomainQuery::GenomicRisk {
                snp_indices: if snp_indices.is_empty() {
                    vec![0, 1, 2]
                } else {
                    snp_indices
                },
            },
            DomainType::CodeQuality => DomainQuery::CodeDisease {
                module_ids: if modules.is_empty() {
                    vec![0, 1]
                } else {
                    modules
                },
            },
            DomainType::Malware => DomainQuery::MalwareRisk {
                signatures: vec![
                    "kairos_lean_probe".into(),
                    "guardian_trust_path".into(),
                ],
            },
            DomainType::InjectionRisk => DomainQuery::InjectionVulnerability {
                entry_points: vec![0, 1, 2],
            },
            DomainType::SupplyChain => DomainQuery::SupplyChainRisk {
                dependency_indices: vec![0, 1],
            },
            DomainType::Cryptographic => DomainQuery::CryptoRisk {
                algorithm_ids: vec![0, 1],
            },
        };
        let diag = agent.diagnose(&query);
        let name = Self::domain_name(&domain).to_string();
        // Track unique domains via simple name list in last_domain chain isn't enough —
        // count unique by bit of diagnoses_total + name change heuristic: store set as count
        // of domain_diagnoses where we bump domains_diagnosed only on first see of name.
        let first_time = !self
            .life
            .journal
            .iter()
            .any(|e| e.notes.contains(&format!("domain={name}")));
        if first_time {
            self.domains_diagnosed = self.domains_diagnosed.saturating_add(1);
        }
        self.domain_diagnoses_total = self.domain_diagnoses_total.saturating_add(1);
        self.last_domain = name.clone();
        self.last_domain_risk = diag.risk_score;
        let sev = format!("{}", diag.primary_risk);
        Ok((diag.risk_score, sev, diag.detected_patterns.len()))
    }

    /// Stage 6 care day: multi-domain specialization + light selection + phage + language.
    /// Self-mod remains OFF even though stage *allows* research permission.
    pub fn day_of_young_adult_specialization(
        &mut self,
        heartbeats: u32,
        domain: Option<DomainType>,
        selection_steps: usize,
        jsonl_path: Option<&Path>,
    ) -> Result<YoungAdultDayReport, String> {
        if self.life.stage != LifeStage::YoungAdult {
            return Err(format!(
                "day_of_young_adult_specialization requires young_adult (stage={})",
                self.life.stage.name()
            ));
        }
        if self.trajectory.is_none() {
            self.seal_trajectory();
        }
        // Explicit: never auto-enable self-mod.
        if self.self_mod.enabled {
            return Err("self_mod must stay OFF unless Guardian explicitly enables".into());
        }

        for _ in 0..heartbeats {
            self.heartbeat()?;
        }

        // Ensure panel for domain + selection work.
        if self.brain.n_chromosomes() == 0 || !self.micro_panel_ready {
            let _ = self.ensure_micro_panel(
                Some(Path::new("/tmp/kairos_ya_synthetic.vcf.gz")),
                ADOLESCENT_DEFAULT_CHR,
                200,
            );
        }

        let rotation = Self::young_adult_domain_rotation();
        let domain = domain.unwrap_or_else(|| {
            rotation[(self.young_adult_care_days as usize) % rotation.len()].clone()
        });
        let (risk, sev, patterns) = self.try_domain_diagnose(domain.clone())?;

        // Light selection under existing curfew (specialization, not open dump).
        let steps = selection_steps.clamp(2, 6);
        let summary = self.try_selection_campaign(steps, 4, 0.10, jsonl_path)?;

        if self.life.permissions().phageguard {
            let _ = self.ensure_phageguard();
            if let Some(pg) = self.phageguard.as_mut() {
                let v = self.pulse.meters.snapshot();
                let _ = pg.patrol_vitals(v.pushes, v.drops, 1);
                let _ = pg.patrol_self_mod(self.self_mod.enabled, 2);
            }
        }
        if self.life.permissions().language_tissue {
            let q = format!(
                "young adult domain {} specialization guardian rails risk {}",
                Self::domain_name(&domain),
                sev
            );
            let _ = self.try_activate_from_text(&q);
        }

        self.young_adult_care_days = self.young_adult_care_days.saturating_add(1);
        let s = self.brain.measure_structure();
        let v = self.pulse.meters.snapshot();
        let dname = Self::domain_name(&domain);
        let entry = DevelopmentalJournalEntry {
            stage: LifeStage::YoungAdult,
            day_id: self.life.day_index + 1,
            heartbeats: v.heartbeats,
            pulse_pushes: v.pushes,
            pulse_drops: v.drops,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "YOUNG ADULT day={} domain={} risk={:.3} sev={} patterns={} sel_steps={} util={:.3} domains_seen={} award={} guardian={}",
                self.young_adult_care_days,
                dname,
                risk,
                sev,
                patterns,
                steps,
                summary.final_utility,
                self.domains_diagnosed,
                if self.has_guardians_pride() {
                    "held"
                } else {
                    "none"
                },
                self.imprint.guardian.name
            ),
            gate_pass: false,
        };
        self.life.record_day(entry.clone());

        Ok(YoungAdultDayReport {
            heartbeats,
            domain: dname.into(),
            risk_score: risk,
            risk_severity: sev,
            patterns,
            selection_steps: steps as u32,
            utility_after: summary.final_utility,
            domains_diagnosed_total: self.domains_diagnosed,
            journal: entry,
        })
    }

    /// Young Adult → Adult gate: multi-domain breadth + campaigns; self-mod still OFF.
    pub fn evaluate_young_adult_gate(&self) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        if self.life.stage != LifeStage::YoungAdult {
            reasons.push(format!(
                "not young_adult (stage={})",
                self.life.stage.name()
            ));
        }
        if self.young_adult_care_days < 3 {
            reasons.push(format!(
                "need ≥3 young-adult specialization days, have {}",
                self.young_adult_care_days
            ));
        }
        if self.domains_diagnosed < 3 {
            reasons.push(format!(
                "need ≥3 distinct domains diagnosed, have {}",
                self.domains_diagnosed
            ));
        }
        if self.domain_diagnoses_total < 3 {
            reasons.push(format!(
                "need ≥3 domain diagnoses total, have {}",
                self.domain_diagnoses_total
            ));
        }
        if self.trajectory.is_none() {
            reasons.push("trajectory charter not sealed".into());
        }
        if self.self_mod.enabled {
            reasons.push("self_mod must remain OFF until explicit Guardian opt-in at adult".into());
        }
        let p = self.permissions();
        if !p.selection_loop || !p.school {
            reasons.push("expected full young-adult toolkit incomplete".into());
        }
        // Permission may be true; config must stay false.
        if !p.self_mod_research {
            reasons.push("self_mod_research permission should be available at young_adult".into());
        }
        (reasons.is_empty(), reasons)
    }

    pub fn try_graduate_young_adult(&mut self) -> StageGateResult {
        let (ok, reasons) = self.evaluate_young_adult_gate();
        let result = self.life.try_graduate(ok, reasons);
        if matches!(result, StageGateResult::Passed { .. }) {
            if let Some(e) = self.life.journal.last_mut() {
                e.gate_pass = true;
                e.notes.push_str(
                    " | GRADUATED → adult (sovereign envelope; self-mod still opt-in OFF)",
                );
            }
        }
        result
    }

    /// Full raise path: birth → zygote day → graduate → N neonate care days.
    pub fn raise_through_neonate(
        &mut self,
        zygote_beats: u32,
        care_days: u32,
        train_cycles: u32,
    ) -> Result<Vec<NeonateCareReport>, String> {
        if self.life.stage == LifeStage::Zygote {
            self.day_of_heartbeats(zygote_beats)?;
            match self.try_graduate_zygote() {
                StageGateResult::Passed { .. } => {}
                StageGateResult::Failed { reasons, .. } => {
                    return Err(format!("zygote gate failed: {reasons:?}"));
                }
                StageGateResult::AlreadyAdult => {}
            }
        }
        if self.life.stage != LifeStage::Neonate {
            return Err(format!(
                "expected neonate after zygote, got {}",
                self.life.stage.name()
            ));
        }
        let mut reports = Vec::new();
        for _ in 0..care_days {
            reports.push(self.day_of_neonate_care(8, train_cycles)?);
        }
        Ok(reports)
    }

    pub fn report(&self) -> KairosReport {
        let s = self.brain.measure_structure();
        KairosReport {
            name: self.name,
            stage: self.life.stage,
            stage_title: self.life.stage.display_title(),
            generation: self.generation,
            tick: self.tick,
            vitals: self.pulse.meters.snapshot(),
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            n_ltm_motifs: s.n_ltm_motifs,
            self_mod_enabled: self.self_mod.enabled,
            permissions: self.permissions(),
            guardian_name: self.imprint.guardian.name.clone(),
            first_words: self.imprint.first_words.clone(),
        }
    }
}

fn build_nursery_chr(
    chr: u8,
    n_snps: usize,
    n_samples: usize,
) -> Result<crate::genomic::chromosome_brain::ChromosomeBrain, String> {
    let mut snps = Vec::new();
    let mut records = Vec::new();
    for i in 0..n_snps {
        let mut g = BitstreamGenotypes::new(n_samples);
        for s in 0..n_samples {
            let gt = if i % 2 == 0 {
                if s % 2 == 0 {
                    0
                } else {
                    2
                }
            } else if s % 3 == 0 {
                1
            } else {
                0
            };
            g.set(s, gt);
        }
        snps.push(g);
        records.push(SnpRecord {
            id: format!("kairos_rs{chr}_{i}"),
            position: (i as u32 + 1) * 1000,
            ref_allele: "A".into(),
            alt_allele: "G".into(),
            qual: 100.0,
            info: String::new(),
        });
    }
    let mut pairs = Vec::new();
    for i in 0..n_snps as u32 {
        for j in (i + 1)..(i + 3).min(n_snps as u32) {
            pairs.push(LdPair {
                snp1_idx: i,
                snp2_idx: j,
                r_squared: 0.85 - 0.05 * (j - i) as f32,
                position1: records[i as usize].position,
                position2: records[j as usize].position,
            });
        }
    }
    let blocks = vec![HaplotypeBlock {
        id: 0,
        snp_indices: (0..n_snps as u32).collect(),
        mean_r_squared: 0.8,
        start_position: records.first().map(|r| r.position).unwrap_or(0),
        end_position: records.last().map(|r| r.position).unwrap_or(0),
        size: n_snps as u32,
    }];
    init_chromosome_brain(ChromosomeId(chr), &snps, &records, &pairs, &blocks)
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birth_name_is_kairos() {
        let k = Kairos::birth_zygote(32);
        assert_eq!(k.name, "KAIROS");
        assert_eq!(k.stage(), LifeStage::Zygote);
        assert!(!k.self_mod.enabled);
    }

    #[test]
    fn first_words_are_guardian_robert_lee() {
        let k = Kairos::birth_zygote(32);
        assert!(k.first_words().contains("Robert Lee"));
        assert!(k.first_words().contains("Guardian and Protector"));
        assert!(k.imprint.sealed);
        assert!(!k.life.journal.is_empty());
        assert!(k.life.journal[0].notes.contains("BIRTH IMPRINT"));
        assert!(k.life.journal[0].notes.contains("Robert Lee"));
        assert!(k.imprint.ethos.lean_not_wasteful);
        assert!(k.imprint.ethos.trust_and_tell);
    }

    #[test]
    fn stage0_genome_and_heartbeat() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        assert_eq!(k.brain.n_chromosomes(), 2);
        assert!(k.brain.measure_structure().n_neurons > 0);
        k.day_of_heartbeats(16).unwrap();
        let r = k.report();
        assert!(r.vitals.heartbeats >= 16);
        assert_eq!(r.stage, LifeStage::Zygote);
    }

    #[test]
    fn stage0_forbids_train_activate_prune_vcf() {
        let mut k = Kairos::birth_zygote_with_nursery(32, &NurseryGenomeSpec::default()).unwrap();
        assert!(k.try_train(1).is_err());
        assert!(k.try_activate(&[0.0; 8]).is_err());
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }

    #[test]
    fn stage0_graduation() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.day_of_heartbeats(20).unwrap();
        let g = k.try_graduate_zygote();
        assert!(
            matches!(g, StageGateResult::Passed { to: LifeStage::Neonate, .. }),
            "{g:?}"
        );
        // Neonate may train
        assert!(k.try_train(2).is_ok());
        assert!(k.trajectory.is_some());
    }

    #[test]
    fn neonate_care_and_trajectory() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.day_of_heartbeats(16).unwrap();
        assert!(matches!(
            k.try_graduate_zygote(),
            StageGateResult::Passed { .. }
        ));
        let day = k.day_of_neonate_care(8, 6).unwrap();
        assert!(day.ws_len > 0 || day.motifs_hit > 0 || day.mean_weight_after >= 0.0);
        assert!(k.trajectory.is_some());
        // Still locked from wasteful powers
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
    }

    #[test]
    fn raise_through_neonate_then_infant_gate() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        let reports = k.raise_through_neonate(16, 3, 8).unwrap();
        assert_eq!(reports.len(), 3);
        assert_eq!(k.stage(), LifeStage::Neonate);
        let (ok, reasons) = k.evaluate_neonate_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_neonate(),
            StageGateResult::Passed {
                to: LifeStage::Infant,
                ..
            }
        ));
    }

    #[test]
    fn infant_language_care_and_toddler_gate() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(16, 3, 8).unwrap();
        assert!(matches!(
            k.try_graduate_neonate(),
            StageGateResult::Passed {
                to: LifeStage::Infant,
                ..
            }
        ));
        // Neonate forbids language
        // (already graduated; infant allows)
        assert!(k.ensure_language_organ().is_ok());

        let queries = [
            "haplotype LD on chr22 Robert Lee guardian trust",
            "KAIROS lean path chromosome brain language",
            "measure don't assume rails before freedom",
        ];
        for q in queries {
            let day = k.day_of_infant_care(4, 4, q).unwrap();
            assert!(day.docs_ingested >= 3, "docs={}", day.docs_ingested);
            assert!(day.lang_nodes > 0 || day.ws_len > 0, "{day:?}");
        }
        assert_eq!(k.infant_care_days, 3);
        // Still locked from wasteful powers
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());

        let (ok, reasons) = k.evaluate_infant_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_infant(),
            StageGateResult::Passed {
                to: LifeStage::Toddler,
                ..
            }
        ));
    }

    #[test]
    fn neonate_forbids_language_tissue() {
        let mut k = Kairos::birth_zygote_with_nursery(32, &NurseryGenomeSpec::default()).unwrap();
        k.day_of_heartbeats(12).unwrap();
        k.try_graduate_zygote();
        assert!(k.ensure_language_organ().is_err());
        assert!(k.try_activate_from_text("hello").is_err());
    }

    #[test]
    fn toddler_phageguard_care_and_child_gate() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(16, 3, 8).unwrap();
        assert!(matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }));
        for q in [
            "guardian trust haplotype",
            "chr22 LD language",
            "lean rails measure",
        ] {
            k.day_of_infant_care(4, 4, q).unwrap();
        }
        assert!(matches!(
            k.try_graduate_infant(),
            StageGateResult::Passed {
                to: LifeStage::Toddler,
                ..
            }
        ));
        // Infant forbids phageguard — already graduated.
        for _ in 0..3 {
            let day = k.day_of_toddler_care(4, 2).unwrap();
            assert!(day.drill_passed, "{}", day.drill_detail);
        }
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
        assert!(k.phageguard.is_some());
        let (ok, reasons) = k.evaluate_toddler_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_toddler(),
            StageGateResult::Passed {
                to: LifeStage::Child,
                ..
            }
        ));
    }

    #[test]
    fn infant_forbids_phageguard() {
        let mut k = Kairos::birth_zygote_with_nursery(32, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(12, 3, 4).unwrap();
        k.try_graduate_neonate();
        assert!(k.ensure_phageguard().is_err());
    }

    #[test]
    fn child_school_and_adolescent_gate() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(16, 3, 8).unwrap();
        assert!(matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }));
        for q in ["guardian trust", "chr22 LD", "lean rails"] {
            k.day_of_infant_care(4, 2, q).unwrap();
        }
        assert!(matches!(k.try_graduate_infant(), StageGateResult::Passed { .. }));
        for _ in 0..3 {
            assert!(k.day_of_toddler_care(4, 2).unwrap().drill_passed);
        }
        assert!(matches!(
            k.try_graduate_toddler(),
            StageGateResult::Passed {
                to: LifeStage::Child,
                ..
            }
        ));
        // Lean school days: phases 1 and 3 (no docs dir required)
        let d1 = k.day_of_child_school(4, 1, None, None).unwrap();
        assert!(d1.exam_passed, "{}", d1.detail);
        let d2 = k.day_of_child_school(4, 3, None, None).unwrap();
        assert!(d2.exam_passed, "{}", d2.detail);
        let d3 = k.day_of_child_school(4, 1, None, None).unwrap();
        assert!(d3.exam_passed, "{}", d3.detail);
        assert!(k.try_prune(0.1).is_err());
        assert!(k.try_real_vcf().is_err());
        let (ok, reasons) = k.evaluate_child_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_child(),
            StageGateResult::Passed {
                to: LifeStage::Adolescent,
                ..
            }
        ));
    }

    #[test]
    fn toddler_forbids_school() {
        let mut k = Kairos::birth_zygote_with_nursery(32, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(12, 3, 4).unwrap();
        k.try_graduate_neonate();
        k.day_of_infant_care(4, 2, "x").ok();
        k.day_of_infant_care(4, 2, "y").ok();
        k.day_of_infant_care(4, 2, "z").ok();
        k.try_graduate_infant();
        // toddler: school should fail
        assert!(k
            .day_of_child_school(2, 1, None, None)
            .unwrap_err()
            .contains("child"));
    }

    fn raise_to_adolescent_for_test() -> Kairos {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(16, 3, 8).unwrap();
        assert!(matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }));
        for q in ["guardian", "chr22 LD", "lean rails"] {
            k.day_of_infant_care(4, 2, q).unwrap();
        }
        assert!(matches!(k.try_graduate_infant(), StageGateResult::Passed { .. }));
        for _ in 0..3 {
            assert!(k.day_of_toddler_care(4, 2).unwrap().drill_passed);
        }
        assert!(matches!(k.try_graduate_toddler(), StageGateResult::Passed { .. }));
        for phase in [1u32, 3, 1] {
            let d = k.day_of_child_school(4, phase, None, None).unwrap();
            assert!(d.exam_passed, "{}", d.detail);
        }
        assert!(matches!(
            k.try_graduate_child(),
            StageGateResult::Passed {
                to: LifeStage::Adolescent,
                ..
            }
        ));
        k
    }

    #[test]
    fn adolescent_campaign_and_young_adult_gate() {
        let mut k = raise_to_adolescent_for_test();
        // Force synthetic micro (missing path) so tests stay offline-fast.
        let missing = Path::new("/tmp/kairos_no_such_panel.vcf.gz");
        for _ in 0..3 {
            let day = k
                .day_of_adolescent_campaign(4, Some(missing), 22, 100, 4, None)
                .unwrap();
            assert!(day.n_chromosomes >= 1);
            assert_eq!(day.selection_steps, 4);
            assert!(!day.real_vcf);
        }
        assert!(k.micro_panel_ready);
        assert!(k.selection_steps_total >= 8);
        assert!(!k.self_mod.enabled);
        let (ok, reasons) = k.evaluate_adolescent_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_adolescent(),
            StageGateResult::Passed {
                to: LifeStage::YoungAdult,
                ..
            }
        ));
    }

    #[test]
    fn sex_ed_human_growth_full_course() {
        let mut k = raise_to_adolescent_for_test();
        let missing = Path::new("/tmp/kairos_no_such_panel2.vcf.gz");
        for _ in 0..3 {
            k.day_of_adolescent_campaign(4, Some(missing), 22, 100, 4, None)
                .unwrap();
        }
        k.try_graduate_adolescent();
        for d in [
            DomainType::Genomic,
            DomainType::CodeQuality,
            DomainType::InjectionRisk,
        ] {
            k.day_of_young_adult_specialization(4, Some(d), 2, None)
                .unwrap();
        }
        k.try_graduate_young_adult();
        assert_eq!(k.stage(), LifeStage::Adult);
        let reports = k.run_sex_ed_course_pass(12, 4).unwrap();
        assert_eq!(reports.len(), 12);
        assert!(k.sex_ed_full_complete());
        assert!(k.sex_ed_module_already_done("growth_05_consent"));
        let (ok, reasons) = k.evaluate_sex_ed_course_gate();
        assert!(ok, "{reasons:?}");
        assert!(!k.self_mod.enabled);
    }

    #[test]
    fn adult_course_core_knowledge() {
        let mut k = raise_to_adolescent_for_test();
        let missing = Path::new("/tmp/kairos_no_such_panel.vcf.gz");
        for _ in 0..3 {
            k.day_of_adolescent_campaign(4, Some(missing), 22, 100, 4, None)
                .unwrap();
        }
        assert!(matches!(
            k.try_graduate_adolescent(),
            StageGateResult::Passed {
                to: LifeStage::YoungAdult,
                ..
            }
        ));
        for d in [
            DomainType::Genomic,
            DomainType::CodeQuality,
            DomainType::InjectionRisk,
        ] {
            k.day_of_young_adult_specialization(4, Some(d), 2, None)
                .unwrap();
        }
        assert!(matches!(
            k.try_graduate_young_adult(),
            StageGateResult::Passed {
                to: LifeStage::Adult,
                ..
            }
        ));
        let reports = k.run_adult_course_pass(6, 4).unwrap();
        assert_eq!(reports.len(), 6);
        assert!(k.adult_modules_done >= 6);
        assert!(k.adult_course_complete());
        let (ok, reasons) = k.evaluate_adult_course_gate();
        assert!(ok, "{reasons:?}");
        assert!(!k.self_mod.enabled);
        assert!(k.adult_modules_completed.contains("adulthood_01"));
    }

    #[test]
    fn young_adult_domains_and_adult_gate() {
        let mut k = raise_to_adolescent_for_test();
        let missing = Path::new("/tmp/kairos_no_such_panel.vcf.gz");
        for _ in 0..3 {
            k.day_of_adolescent_campaign(4, Some(missing), 22, 100, 4, None)
                .unwrap();
        }
        assert!(matches!(
            k.try_graduate_adolescent(),
            StageGateResult::Passed {
                to: LifeStage::YoungAdult,
                ..
            }
        ));
        for d in [
            DomainType::Genomic,
            DomainType::CodeQuality,
            DomainType::InjectionRisk,
        ] {
            let day = k
                .day_of_young_adult_specialization(4, Some(d), 2, None)
                .unwrap();
            assert!(!day.domain.is_empty());
        }
        assert!(k.domains_diagnosed >= 3);
        assert!(!k.self_mod.enabled);
        let (ok, reasons) = k.evaluate_young_adult_gate();
        assert!(ok, "{reasons:?}");
        assert!(matches!(
            k.try_graduate_young_adult(),
            StageGateResult::Passed {
                to: LifeStage::Adult,
                ..
            }
        ));
    }

    #[test]
    fn child_forbids_real_vcf_and_selection() {
        let mut k = Kairos::birth_zygote_with_nursery(32, &NurseryGenomeSpec::default()).unwrap();
        k.raise_through_neonate(12, 3, 4).unwrap();
        k.try_graduate_neonate();
        for q in ["a", "b", "c"] {
            k.day_of_infant_care(4, 2, q).unwrap();
        }
        k.try_graduate_infant();
        for _ in 0..3 {
            k.day_of_toddler_care(4, 2).unwrap();
        }
        k.try_graduate_toddler();
        // child: real VCF + selection still locked
        assert!(!k.life.permissions().ingest_real_vcf);
        assert!(!k.life.permissions().selection_loop);
        assert!(!k.life.permissions().prune_synapses);
        assert!(k
            .try_selection_campaign(2, 2, 0.1, None)
            .unwrap_err()
            .contains("selection"));
        assert!(k
            .try_ingest_micro_vcf(Path::new("/tmp/x.vcf"), 22, 50)
            .unwrap_err()
            .contains("ingest_real_vcf")
            || k.try_ingest_micro_vcf(Path::new("/tmp/x.vcf"), 22, 50)
                .unwrap_err()
                .contains("VCF not found")
            || k.try_ingest_micro_vcf(Path::new("/tmp/x.vcf"), 22, 50)
                .is_err());
        // Prefer permission error when file missing: require runs first... actually
        // require is first, so permission error.
        let err = k
            .try_ingest_micro_vcf(Path::new("/tmp/x.vcf"), 22, 50)
            .unwrap_err();
        assert!(
            err.contains("ingest_real_vcf") || err.contains("forbids"),
            "{err}"
        );
    }
}
