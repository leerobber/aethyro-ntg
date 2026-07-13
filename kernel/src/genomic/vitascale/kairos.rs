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
use crate::genomic::vitascale::life_course::{
    DevelopmentalJournalEntry, LifeCourse, LifeStage, StageGateResult, StagePermissions,
};
use crate::genomic::vitascale::pulsewire::{PulseHandles, VitalSnapshot};
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
    /// Generation of the host (increments on heartbeat days / later selection).
    pub generation: u32,
    /// Tick counter within current process life.
    pub tick: u32,
    /// Explicit record that ADR 0002 self-mod stays off unless adult+opt-in.
    pub self_mod: SelfModConfig,
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
}

impl Kairos {
    /// Birth name for this project’s primary child.
    pub const NAME: &'static str = "KAIROS";

    /// Stage 0 zygote: empty brain + pulse, then optional nursery genome load.
    pub fn birth_zygote(pulse_capacity: usize) -> Self {
        Self {
            name: Self::NAME,
            life: LifeCourse::new(),
            brain: SovereignBrain::new(64),
            pulse: PulseHandles::new(pulse_capacity),
            generation: 0,
            tick: 0,
            self_mod: SelfModConfig::default(), // enabled: false
        }
    }

    /// Stage 0 with genome/chromosome tissues present (Guardian-held DNA).
    pub fn birth_zygote_with_nursery(pulse_capacity: usize, spec: &NurseryGenomeSpec) -> Result<Self, String> {
        let mut k = Self::birth_zygote(pulse_capacity);
        k.load_nursery_genome(spec)?;
        Ok(k)
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
        }
        result
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
    }
}
