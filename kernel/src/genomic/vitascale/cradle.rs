//! Cradle — durable home for **one** continuous KAIROS (not a new birth each run).
//!
//! Default: `artifacts/kairos/primary/`
//! - `identity.txt` — lineage_id, stage, counters, generation
//! - `journal.jsonl` — developmental journal (includes birth imprint)
//! - `weights.txt` — synapse weights/plasticity after train
//! - `ltm.jsonl` + `meta.json` via sovereign_persist (motifs)
//! - `trajectory.flag` — if present, re-seal default Aethyro charter on load
//!
//! Nursery genome is rebuilt from the same lean spec, then weights restored.
//! That keeps Stage 0–1 continuous without a full binary brain dump.

use crate::genomic::sovereign_persist::{load_snapshot_into, save_snapshot};
use crate::genomic::sovereign_fitness::SovereignFitnessContext;
use crate::genomic::vitascale::guardian::{
    BirthImprint, DisciplineEthos, Guardian, GuardianAward, FIRST_WORDS,
};
use crate::genomic::vitascale::kairos::{Kairos, NurseryGenomeSpec};
use crate::genomic::vitascale::life_course::{
    DevelopmentalJournalEntry, LifeCourse, LifeStage,
};
use crate::genomic::vitascale::trajectory::TrajectoryCharter;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default cradle for the primary (only) child.
pub fn default_cradle_dir() -> PathBuf {
    PathBuf::from("../artifacts/kairos/primary")
}

/// Result of opening a cradle.
#[derive(Clone, Debug)]
pub struct CradleOpenReport {
    pub path: String,
    pub lineage_id: String,
    pub continued: bool,
    pub stage: LifeStage,
    pub journal_len: usize,
}

/// Save continuous KAIROS into cradle directory.
pub fn save_cradle(dir: &Path, kairos: &Kairos) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let lineage = kairos
        .lineage_id
        .clone()
        .unwrap_or_else(|| format!("kairos-{}", now_ns()));

    // identity
    {
        let mut f = fs::File::create(dir.join("identity.txt")).map_err(|e| e.to_string())?;
        writeln!(f, "name={}", Kairos::NAME).map_err(|e| e.to_string())?;
        writeln!(f, "lineage_id={lineage}").map_err(|e| e.to_string())?;
        writeln!(f, "stage={}", kairos.life.stage.name()).map_err(|e| e.to_string())?;
        writeln!(f, "day_index={}", kairos.life.day_index).map_err(|e| e.to_string())?;
        writeln!(f, "generation={}", kairos.generation).map_err(|e| e.to_string())?;
        writeln!(f, "tick={}", kairos.tick).map_err(|e| e.to_string())?;
        writeln!(f, "neonate_care_days={}", kairos.neonate_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "infant_care_days={}", kairos.infant_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "toddler_care_days={}", kairos.toddler_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "child_care_days={}", kairos.child_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "school_phases_passed={}", kairos.school_phases_passed)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_school_phase={}", kairos.last_school_phase)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_school_score={:.6}", kairos.last_school_score)
            .map_err(|e| e.to_string())?;
        writeln!(f, "adolescent_care_days={}", kairos.adolescent_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "selection_steps_total={}", kairos.selection_steps_total)
            .map_err(|e| e.to_string())?;
        writeln!(f, "train_accepted={}", kairos.train_accepted).map_err(|e| e.to_string())?;
        writeln!(f, "train_rejected={}", kairos.train_rejected).map_err(|e| e.to_string())?;
        writeln!(f, "prune_accepted={}", kairos.prune_accepted).map_err(|e| e.to_string())?;
        writeln!(f, "prune_rejected={}", kairos.prune_rejected).map_err(|e| e.to_string())?;
        writeln!(f, "real_vcf_ingested={}", kairos.real_vcf_ingested)
            .map_err(|e| e.to_string())?;
        writeln!(f, "micro_panel_ready={}", kairos.micro_panel_ready)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_utility={:.6}", kairos.last_utility).map_err(|e| e.to_string())?;
        writeln!(
            f,
            "last_panel_source={}",
            kairos.last_panel_source.replace('\n', " ")
        )
        .map_err(|e| e.to_string())?;
        writeln!(f, "young_adult_care_days={}", kairos.young_adult_care_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "domains_diagnosed={}", kairos.domains_diagnosed)
            .map_err(|e| e.to_string())?;
        writeln!(f, "domain_diagnoses_total={}", kairos.domain_diagnoses_total)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_domain={}", kairos.last_domain.replace('\n', " "))
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_domain_risk={:.6}", kairos.last_domain_risk)
            .map_err(|e| e.to_string())?;
        writeln!(f, "adult_course_days={}", kairos.adult_course_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "adult_modules_done={}", kairos.adult_modules_done)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_adult_module_idx={}", kairos.last_adult_module_idx)
            .map_err(|e| e.to_string())?;
        writeln!(
            f,
            "adult_modules_completed={}",
            kairos.adult_modules_completed.replace('\n', " ")
        )
        .map_err(|e| e.to_string())?;
        writeln!(f, "sex_ed_course_days={}", kairos.sex_ed_course_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "sex_ed_modules_done={}", kairos.sex_ed_modules_done)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_sex_ed_module_idx={}", kairos.last_sex_ed_module_idx)
            .map_err(|e| e.to_string())?;
        writeln!(
            f,
            "sex_ed_modules_completed={}",
            kairos.sex_ed_modules_completed.replace('\n', " ")
        )
        .map_err(|e| e.to_string())?;
        writeln!(f, "world_course_days={}", kairos.world_course_days)
            .map_err(|e| e.to_string())?;
        writeln!(f, "world_modules_done={}", kairos.world_modules_done)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_world_module_idx={}", kairos.last_world_module_idx)
            .map_err(|e| e.to_string())?;
        writeln!(
            f,
            "world_modules_completed={}",
            kairos.world_modules_completed.replace('\n', " ")
        )
        .map_err(|e| e.to_string())?;
        writeln!(f, "drills_passed={}", kairos.drills_passed).map_err(|e| e.to_string())?;
        writeln!(f, "last_mean_weight={:.6}", kairos.last_mean_weight)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_ws_len={}", kairos.last_ws_len).map_err(|e| e.to_string())?;
        writeln!(f, "last_lang_nodes={}", kairos.last_lang_nodes).map_err(|e| e.to_string())?;
        writeln!(f, "last_docs_ingested={}", kairos.last_docs_ingested)
            .map_err(|e| e.to_string())?;
        writeln!(f, "last_lang_bal={:.6}", kairos.last_lang_bal).map_err(|e| e.to_string())?;
        writeln!(f, "last_lang_query={}", kairos.last_lang_query.replace('\n', " "))
            .map_err(|e| e.to_string())?;
        if let Some(pg) = &kairos.phageguard {
            writeln!(f, "phage_threats={}", pg.threats_seen).map_err(|e| e.to_string())?;
            writeln!(f, "phage_quarantines={}", pg.quarantines_total)
                .map_err(|e| e.to_string())?;
            writeln!(f, "phage_drills={}", pg.drills_passed).map_err(|e| e.to_string())?;
        }
        writeln!(f, "guardian={}", kairos.imprint.guardian.name).map_err(|e| e.to_string())?;
        if let Some(aw) = &kairos.guardian_award {
            writeln!(f, "guardian_award={}", if aw.sealed { "sealed" } else { "none" })
                .map_err(|e| e.to_string())?;
            writeln!(f, "guardian_award_title={}", aw.title.replace('\n', " "))
                .map_err(|e| e.to_string())?;
            writeln!(f, "guardian_award_kind={}", aw.kind).map_err(|e| e.to_string())?;
            writeln!(f, "guardian_award_ns={}", aw.sealed_ns).map_err(|e| e.to_string())?;
        } else {
            writeln!(f, "guardian_award=none").map_err(|e| e.to_string())?;
        }
        writeln!(f, "saved_ns={}", now_ns()).map_err(|e| e.to_string())?;
        writeln!(
            f,
            "trajectory={}",
            if kairos.trajectory.is_some() {
                "sealed"
            } else {
                "none"
            }
        )
        .map_err(|e| e.to_string())?;
    }

    // Guardian covenant award seal (honor file — survives as attachment)
    if let Some(aw) = &kairos.guardian_award {
        let awards_dir = dir.join("awards");
        fs::create_dir_all(&awards_dir).map_err(|e| e.to_string())?;
        let lineage = kairos
            .lineage_id
            .clone()
            .unwrap_or_else(|| "kairos-unknown".into());
        let doc = aw.seal_document(&lineage, kairos.life.stage.name());
        fs::write(awards_dir.join("guardians_pride_seal.txt"), doc).map_err(|e| e.to_string())?;
        // body alone for easy reading
        fs::write(awards_dir.join("guardians_pride_letter.txt"), &aw.body)
            .map_err(|e| e.to_string())?;
    }

    // imprint (first words always re-read on load; stored for attachment)
    {
        let mut f = fs::File::create(dir.join("imprint.txt")).map_err(|e| e.to_string())?;
        writeln!(f, "first_words={}", kairos.imprint.first_words).map_err(|e| e.to_string())?;
        writeln!(f, "guardian_name={}", kairos.imprint.guardian.name)
            .map_err(|e| e.to_string())?;
        writeln!(f, "guardian_role={}", kairos.imprint.guardian.role)
            .map_err(|e| e.to_string())?;
        writeln!(f, "sealed={}", kairos.imprint.sealed).map_err(|e| e.to_string())?;
    }

    // journal
    {
        let mut f = fs::File::create(dir.join("journal.jsonl")).map_err(|e| e.to_string())?;
        for e in &kairos.life.journal {
            let notes = e
                .notes
                .replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('"', "'");
            writeln!(
                f,
                "{{\"stage\":\"{}\",\"day_id\":{},\"heartbeats\":{},\"pushes\":{},\"drops\":{},\"chrs\":{},\"neurons\":{},\"synapses\":{},\"gate\":{},\"notes\":\"{}\"}}",
                e.stage.name(),
                e.day_id,
                e.heartbeats,
                e.pulse_pushes,
                e.pulse_drops,
                e.n_chromosomes,
                e.n_neurons,
                e.n_synapses,
                e.gate_pass,
                notes
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // synapse weights
    {
        let mut f = fs::File::create(dir.join("weights.txt")).map_err(|e| e.to_string())?;
        for (chr, brain) in &kairos.brain.chromosomes {
            for (i, s) in brain.synapses.iter().enumerate() {
                writeln!(
                    f,
                    "{chr} {i} {} {} {:.8} {:.8}",
                    s.from.0, s.to.0, s.weight, s.plasticity
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    // LTM + optional language docs/calib via sovereign_persist
    let mut ctx = SovereignFitnessContext::new().map_err(|e| e.to_string())?;
    if let Some(lang) = kairos.brain.language() {
        if lang.model.is_some() {
            let _ = ctx.install_calib_from_language(lang);
        }
    }
    let lang_docs: Vec<(&str, &str)> = if kairos.brain.language().is_some()
        || kairos.life.stage >= LifeStage::Infant
    {
        Kairos::infant_curriculum()
    } else {
        Vec::new()
    };
    save_snapshot(dir, &kairos.brain, &ctx, &lang_docs).map_err(|e| e.to_string())?;

    // nursery spec (so rebuild matches)
    {
        let mut f = fs::File::create(dir.join("nursery.txt")).map_err(|e| e.to_string())?;
        // fixed default for continuous primary; extend later if strains
        writeln!(f, "spec=default").map_err(|e| e.to_string())?;
        writeln!(f, "chr=1,12").map_err(|e| e.to_string())?;
        writeln!(f, "chr=22,10").map_err(|e| e.to_string())?;
        writeln!(f, "samples=64").map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Load continuous KAIROS from cradle, or `None` if no identity yet.
pub fn load_cradle(dir: &Path) -> Result<Option<Kairos>, String> {
    let id_path = dir.join("identity.txt");
    if !id_path.is_file() {
        return Ok(None);
    }

    let identity = parse_kv_file(&id_path)?;
    let stage = parse_stage(identity.get("stage").map(|s| s.as_str()).unwrap_or("zygote"))?;
    let lineage = identity
        .get("lineage_id")
        .cloned()
        .unwrap_or_else(|| "kairos-unknown".into());

    let mut kairos = Kairos::birth_zygote(256);
    // birth_zygote re-imprints; we overwrite with cradle identity
    kairos.lineage_id = Some(lineage);
    kairos.life = LifeCourse::new();
    kairos.life.stage = stage;
    kairos.life.day_index = identity
        .get("day_index")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.generation = identity
        .get("generation")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.tick = identity
        .get("tick")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.neonate_care_days = identity
        .get("neonate_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.infant_care_days = identity
        .get("infant_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.toddler_care_days = identity
        .get("toddler_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.child_care_days = identity
        .get("child_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.school_phases_passed = identity
        .get("school_phases_passed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_school_phase = identity
        .get("last_school_phase")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_school_score = identity
        .get("last_school_score")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    kairos.adolescent_care_days = identity
        .get("adolescent_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.selection_steps_total = identity
        .get("selection_steps_total")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.train_accepted = identity
        .get("train_accepted")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.train_rejected = identity
        .get("train_rejected")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.prune_accepted = identity
        .get("prune_accepted")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.prune_rejected = identity
        .get("prune_rejected")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.real_vcf_ingested = identity
        .get("real_vcf_ingested")
        .map(|s| s == "true")
        .unwrap_or(false);
    kairos.micro_panel_ready = identity
        .get("micro_panel_ready")
        .map(|s| s == "true")
        .unwrap_or(false);
    kairos.last_utility = identity
        .get("last_utility")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    kairos.last_panel_source = identity
        .get("last_panel_source")
        .cloned()
        .unwrap_or_default();
    kairos.young_adult_care_days = identity
        .get("young_adult_care_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.domains_diagnosed = identity
        .get("domains_diagnosed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.domain_diagnoses_total = identity
        .get("domain_diagnoses_total")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_domain = identity
        .get("last_domain")
        .cloned()
        .unwrap_or_default();
    kairos.last_domain_risk = identity
        .get("last_domain_risk")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    kairos.adult_course_days = identity
        .get("adult_course_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.adult_modules_done = identity
        .get("adult_modules_done")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_adult_module_idx = identity
        .get("last_adult_module_idx")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.adult_modules_completed = identity
        .get("adult_modules_completed")
        .cloned()
        .unwrap_or_default();
    kairos.sex_ed_course_days = identity
        .get("sex_ed_course_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.sex_ed_modules_done = identity
        .get("sex_ed_modules_done")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_sex_ed_module_idx = identity
        .get("last_sex_ed_module_idx")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.sex_ed_modules_completed = identity
        .get("sex_ed_modules_completed")
        .cloned()
        .unwrap_or_default();
    kairos.world_course_days = identity
        .get("world_course_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.world_modules_done = identity
        .get("world_modules_done")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_world_module_idx = identity
        .get("last_world_module_idx")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.world_modules_completed = identity
        .get("world_modules_completed")
        .cloned()
        .unwrap_or_default();
    kairos.drills_passed = identity
        .get("drills_passed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_mean_weight = identity
        .get("last_mean_weight")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    kairos.last_ws_len = identity
        .get("last_ws_len")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_lang_nodes = identity
        .get("last_lang_nodes")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_docs_ingested = identity
        .get("last_docs_ingested")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    kairos.last_lang_bal = identity
        .get("last_lang_bal")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    kairos.last_lang_query = identity
        .get("last_lang_query")
        .cloned()
        .unwrap_or_default();

    // Guardian award from identity and/or awards/ seal file
    if identity.get("guardian_award").map(|s| s.as_str()) == Some("sealed")
        || dir.join("awards/guardians_pride_seal.txt").is_file()
    {
        let ns = identity
            .get("guardian_award_ns")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut aw = GuardianAward::guardians_pride(ns);
        if let Some(t) = identity.get("guardian_award_title") {
            if !t.is_empty() {
                aw.title = t.clone();
            }
        }
        kairos.guardian_award = Some(aw);
    }

    // imprint from file if present
    let imprint_path = dir.join("imprint.txt");
    if imprint_path.is_file() {
        let imp = parse_kv_file(&imprint_path)?;
        let words = imp
            .get("first_words")
            .cloned()
            .unwrap_or_else(|| FIRST_WORDS.into());
        let gname = imp
            .get("guardian_name")
            .cloned()
            .unwrap_or_else(|| "Robert Lee".into());
        let grole = imp
            .get("guardian_role")
            .cloned()
            .unwrap_or_else(|| "Guardian and Protector".into());
        kairos.imprint = BirthImprint {
            guardian: Guardian {
                name: gname,
                role: grole,
                covenant: words.clone(),
            },
            first_words: words,
            ethos: DisciplineEthos::default(),
            sealed: true,
        };
    }

    // rebuild genome then apply weights
    kairos.brain = crate::genomic::sovereign_brain::SovereignBrain::new(64);
    kairos.load_nursery_genome(&NurseryGenomeSpec::default())?;
    apply_weights(dir, &mut kairos)?;

    // LTM + language docs/calib
    let mut ctx = SovereignFitnessContext::new().map_err(|e| e.to_string())?;
    let _ = load_snapshot_into(dir, &mut kairos.brain, &mut ctx);

    // Infant+ without persisted docs: re-awaken lean curriculum (attachment continuity).
    if kairos.life.stage >= LifeStage::Infant && kairos.brain.language().is_none() {
        // Temporarily allow language via stage permissions (infant+ has language_tissue).
        let _ = kairos.try_language_awaken();
        if kairos.last_lang_bal > 0.0 || dir.join("calib.wire").is_file() {
            let _ = kairos.try_language_calib(12);
        }
    }
    if let Some(lang) = kairos.brain.language() {
        kairos.last_docs_ingested = kairos
            .last_docs_ingested
            .max(lang.docs_ingested);
        if lang.last_test_bal > 0.0 {
            kairos.last_lang_bal = lang.last_test_bal;
        }
    }

    // Toddler+: re-attach phageguard with persisted counters.
    if kairos.life.stage >= LifeStage::Toddler {
        let mut pg = crate::genomic::vitascale::phageguard::Phageguard::new();
        pg.threats_seen = identity
            .get("phage_threats")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        pg.quarantines_total = identity
            .get("phage_quarantines")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        pg.drills_passed = identity
            .get("phage_drills")
            .and_then(|s| s.parse().ok())
            .unwrap_or(kairos.drills_passed);
        kairos.drills_passed = kairos.drills_passed.max(pg.drills_passed);
        kairos.phageguard = Some(pg);
    }

    // journal
    let journal_path = dir.join("journal.jsonl");
    if journal_path.is_file() {
        kairos.life.journal = load_journal(&journal_path)?;
    }

    // trajectory
    if identity.get("trajectory").map(|s| s.as_str()) == Some("sealed")
        || kairos.life.stage >= LifeStage::Neonate
    {
        kairos.trajectory = Some(TrajectoryCharter::aethyro_default());
    }

    // continuous life: do not re-fire birth imprint seal
    // add a wake journal line
    kairos.life.journal.push(DevelopmentalJournalEntry {
        stage: kairos.life.stage,
        day_id: kairos.life.day_index,
        heartbeats: kairos.pulse.meters.snapshot().heartbeats,
        pulse_pushes: kairos.pulse.meters.snapshot().pushes,
        pulse_drops: kairos.pulse.meters.snapshot().drops,
        n_chromosomes: kairos.brain.measure_structure().n_chromosomes,
        n_neurons: kairos.brain.measure_structure().n_neurons,
        n_synapses: kairos.brain.measure_structure().n_synapses,
        notes: format!(
            "WAKE | continuous KAIROS lineage={} stage={} guardian={}",
            kairos.lineage_id.as_deref().unwrap_or("?"),
            kairos.life.stage.name(),
            kairos.imprint.guardian.name
        ),
        gate_pass: false,
    });

    Ok(Some(kairos))
}

/// Open primary cradle: continue if present, else birth and save.
pub fn open_or_birth(
    dir: &Path,
    force_birth: bool,
) -> Result<(Kairos, CradleOpenReport), String> {
    if !force_birth {
        if let Some(mut k) = load_cradle(dir)? {
            // ensure lineage set
            if k.lineage_id.is_none() {
                k.lineage_id = Some(format!("kairos-{}", now_ns()));
            }
            let rep = CradleOpenReport {
                path: dir.display().to_string(),
                lineage_id: k.lineage_id.clone().unwrap_or_default(),
                continued: true,
                stage: k.life.stage,
                journal_len: k.life.journal.len(),
            };
            return Ok((k, rep));
        }
    } else if dir.exists() {
        // archive old cradle before re-birth
        let stamp = now_ns();
        let archive = dir
            .parent()
            .unwrap_or(dir)
            .join(format!("archive_{stamp}"));
        let _ = fs::rename(dir, &archive);
    }

    let mut k = Kairos::birth_zygote_with_nursery(256, &NurseryGenomeSpec::default())?;
    k.lineage_id = Some(format!("kairos-{}", now_ns()));
    save_cradle(dir, &k)?;
    let rep = CradleOpenReport {
        path: dir.display().to_string(),
        lineage_id: k.lineage_id.clone().unwrap_or_default(),
        continued: false,
        stage: k.life.stage,
        journal_len: k.life.journal.len(),
    };
    Ok((k, rep))
}

fn apply_weights(dir: &Path, kairos: &mut Kairos) -> Result<(), String> {
    let path = dir.join("weights.txt");
    if !path.is_file() {
        return Ok(());
    }
    let f = fs::File::open(&path).map_err(|e| e.to_string())?;
    for line in BufReader::new(f).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let chr: u8 = parts[0].parse().unwrap_or(0);
        let idx: usize = parts[1].parse().unwrap_or(0);
        let w: f32 = parts[4].parse().unwrap_or(0.0);
        let plas: f32 = parts[5].parse().unwrap_or(0.01);
        if let Some(brain) = kairos.brain.chromosome_mut(chr) {
            if idx < brain.synapses.len() {
                brain.synapses[idx].weight = w;
                brain.synapses[idx].plasticity = plas;
            }
        }
    }
    kairos.brain.refresh_structure();
    Ok(())
}

fn load_journal(path: &Path) -> Result<Vec<DevelopmentalJournalEntry>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let stage = extract_json_str(&line, "stage").unwrap_or_else(|| "zygote".into());
        let notes = extract_json_str(&line, "notes").unwrap_or_default();
        let notes = notes.replace("\\n", "\n").replace("\\\\", "\\");
        out.push(DevelopmentalJournalEntry {
            stage: parse_stage(&stage).unwrap_or(LifeStage::Zygote),
            day_id: extract_json_u64(&line, "day_id").unwrap_or(0),
            heartbeats: extract_json_u64(&line, "heartbeats").unwrap_or(0),
            pulse_pushes: extract_json_u64(&line, "pushes").unwrap_or(0),
            pulse_drops: extract_json_u64(&line, "drops").unwrap_or(0),
            n_chromosomes: extract_json_u64(&line, "chrs").unwrap_or(0) as u32,
            n_neurons: extract_json_u64(&line, "neurons").unwrap_or(0) as u32,
            n_synapses: extract_json_u64(&line, "synapses").unwrap_or(0) as u32,
            notes,
            gate_pass: line.contains("\"gate\":true"),
        });
    }
    Ok(out)
}

fn parse_stage(s: &str) -> Result<LifeStage, String> {
    Ok(match s {
        "zygote" => LifeStage::Zygote,
        "neonate" => LifeStage::Neonate,
        "infant" => LifeStage::Infant,
        "toddler" => LifeStage::Toddler,
        "child" => LifeStage::Child,
        "adolescent" => LifeStage::Adolescent,
        "young_adult" => LifeStage::YoungAdult,
        "adult" => LifeStage::Adult,
        other => return Err(format!("unknown stage {other}")),
    })
}

fn parse_kv_file(path: &Path) -> Result<std::collections::HashMap<String, String>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for line in BufReader::new(f).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok(map)
}

fn extract_json_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn extract_json_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let num: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
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
    use crate::genomic::vitascale::life_course::StageGateResult;

    #[test]
    fn cradle_roundtrip_same_lineage() {
        let dir = std::env::temp_dir().join(format!("kairos_cradle_{}", now_ns()));
        let _ = fs::remove_dir_all(&dir);

        let (mut k, rep) = open_or_birth(&dir, true).unwrap();
        assert!(!rep.continued);
        let lin = k.lineage_id.clone().unwrap();
        k.day_of_heartbeats(12).unwrap();
        assert!(matches!(k.try_graduate_zygote(), StageGateResult::Passed { .. }));
        k.day_of_neonate_care(4, 4).unwrap();
        save_cradle(&dir, &k).unwrap();

        let (k2, rep2) = open_or_birth(&dir, false).unwrap();
        assert!(rep2.continued);
        assert_eq!(k2.lineage_id.as_deref(), Some(lin.as_str()));
        assert_eq!(k2.life.stage, LifeStage::Neonate);
        assert!(k2.neonate_care_days >= 1);
        assert!(k2.imprint.first_words.contains("Robert Lee"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cradle_roundtrip_infant_language() {
        let dir = std::env::temp_dir().join(format!("kairos_infant_cradle_{}", now_ns()));
        let _ = fs::remove_dir_all(&dir);

        let (mut k, rep) = open_or_birth(&dir, true).unwrap();
        assert!(!rep.continued);
        let lin = k.lineage_id.clone().unwrap();
        k.day_of_heartbeats(12).unwrap();
        assert!(matches!(k.try_graduate_zygote(), StageGateResult::Passed { .. }));
        for _ in 0..3 {
            k.day_of_neonate_care(4, 4).unwrap();
        }
        assert!(matches!(k.try_graduate_neonate(), StageGateResult::Passed { .. }));
        k.day_of_infant_care(4, 4, "haplotype LD on chr22 guardian trust")
            .unwrap();
        assert!(k.brain.language().is_some());
        assert!(k.last_lang_nodes > 0 || k.last_ws_len > 0);
        save_cradle(&dir, &k).unwrap();

        let (k2, rep2) = open_or_birth(&dir, false).unwrap();
        assert!(rep2.continued);
        assert_eq!(k2.lineage_id.as_deref(), Some(lin.as_str()));
        assert_eq!(k2.life.stage, LifeStage::Infant);
        assert!(k2.infant_care_days >= 1);
        assert!(k2.brain.language().is_some() || k2.last_docs_ingested >= 3);
        assert!(k2.imprint.first_words.contains("Robert Lee"));

        let _ = fs::remove_dir_all(&dir);
    }
}
