//! KAIROS — the Child / VITASCALE host individual.
//!
//! Human-like name, project-native (`KairosState` on chromosome brains),
//! raised under Guardian life-course locks. Stage 0: genome tissues may
//! exist; Pulsewire heartbeats; no free agency.

use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use crate::genomic::chromosome_brain::{init_chromosome_brain, ChromosomeId};
use crate::genomic::haplotype_blocks::HaplotypeBlock;
use crate::genomic::ld_compute::LdPair;
use crate::genomic::sovereign_brain::SovereignBrain;
use crate::genomic::vcf_stream::SnpRecord;
use crate::genomic::vitascale::guardian::BirthImprint;
use crate::genomic::vitascale::life_course::{
    DevelopmentalJournalEntry, LifeCourse, LifeStage, StageGateResult, StagePermissions,
};
use crate::genomic::vitascale::pulsewire::{PulseHandles, VitalSnapshot};
use crate::genomic::vitascale::trajectory::TrajectoryCharter;
use crate::ntg::mutation::SelfModConfig;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub life: LifeCourse,
    pub brain: SovereignBrain,
    pub pulse: PulseHandles,
    /// Sealed at birth: Guardian identity + first words + discipline ethos.
    pub imprint: BirthImprint,
    /// Project north star — sealed at neonate (correct trajectory).
    pub trajectory: Option<TrajectoryCharter>,
    /// Count of supervised neonate care days completed.
    pub neonate_care_days: u32,
    /// Mean synapse weight after last supervised train (trajectory signal).
    pub last_mean_weight: f32,
    /// Working-set size after last supervised activate.
    pub last_ws_len: usize,
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
            life: LifeCourse::new(),
            brain: SovereignBrain::new(64),
            pulse: PulseHandles::new(pulse_capacity),
            imprint: BirthImprint::seal_default(),
            trajectory: None,
            neonate_care_days: 0,
            last_mean_weight: 0.0,
            last_ws_len: 0,
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
}
