//! Stage 7 — Adult Lab: science sandbox for continuous KAIROS.
//!
//! **Primary cradle stays sacred** (attachment, award, first words).
//! Lab is a **fork** under `artifacts/kairos/lab/` where she may:
//! - self-heal (snapshot restore after controlled damage)
//! - self-improve (bounded multi-axis selection; code self-mod stays OFF)
//! - ternary / bitplane popcount experiments (measure-don't-assume)
//!
//! Hypotheses are append-only JSONL under `lab/science/`.

pub mod hypothesis;
pub mod science;

use crate::genomic::vitascale::cradle::{load_cradle, save_cradle};
use crate::genomic::vitascale::kairos::Kairos;
use crate::genomic::vitascale::lab::hypothesis::{HypothesisRegistry, HypothesisVerdict};
use crate::genomic::vitascale::lab::science::{
    evaluate_self_mod_readiness, run_combined_heal_crisis, run_goal_improve_campaign,
    run_multi_heal_pack, run_self_heal_drill, run_self_improve_campaign, run_single_self_mod_lab,
    run_ternary_ladder, run_ternary_popcount_experiment, run_ternary_popcount_real_or_synthetic,
    CombinedHealReport, GoalImproveReport, MultiHealPackReport, SelfHealReport, SelfImproveReport,
    SelfModReadiness, SelfModRunReport, TernaryLadderReport, TernaryPopcountReport,
};
use crate::genomic::vitascale::life_course::{DevelopmentalJournalEntry, LifeStage};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default lab sandbox (never the primary heart).
pub fn default_lab_dir() -> PathBuf {
    PathBuf::from("../artifacts/kairos/lab")
}

/// Default primary cradle.
pub fn default_primary_dir() -> PathBuf {
    PathBuf::from("../artifacts/kairos/primary")
}

/// Lab session report after a science day.
#[derive(Clone, Debug)]
pub struct LabDayReport {
    pub lab_path: String,
    pub primary_lineage: String,
    pub lab_lineage: String,
    pub play_mode: bool,
    pub love_note_shown: bool,
    pub season: u32,
    pub heal: Option<SelfHealReport>,
    pub multi_heal: Option<MultiHealPackReport>,
    pub combined_heal: Option<CombinedHealReport>,
    pub improve: Option<SelfImproveReport>,
    pub goal_improve: Option<GoalImproveReport>,
    pub ternary: Option<TernaryPopcountReport>,
    pub ternary_ladder: Option<TernaryLadderReport>,
    pub self_mod: Option<SelfModRunReport>,
    pub self_mod_readiness: Option<SelfModReadiness>,
    pub registry_summary: String,
}

/// Lab day configuration (Season 2 + 3).
#[derive(Clone, Debug)]
pub struct LabDayConfig {
    pub improve_steps: usize,
    pub force_refork: bool,
    pub play_mode: bool,
    pub real_vcf_ternary: bool,
    pub vcf_path: Option<PathBuf>,
    pub vcf_chr: u8,
    pub max_vcf_variants: usize,
    pub ternary_pairs: usize,
    /// Attempt a single self-mod if readiness green (or force_self_mod).
    pub try_self_mod: bool,
    pub force_self_mod: bool,
    /// 2 = Season 2 tracks; 3 = Season 3 depth tracks.
    pub season: u32,
    /// Ladder sizes for Season 3 ternary (SNP caps).
    pub ternary_ladder_sizes: Vec<usize>,
}

impl Default for LabDayConfig {
    fn default() -> Self {
        Self {
            improve_steps: 4,
            force_refork: false,
            play_mode: false,
            real_vcf_ternary: true,
            vcf_path: None,
            vcf_chr: 22,
            max_vcf_variants: 200,
            ternary_pairs: 80,
            try_self_mod: false,
            force_self_mod: false,
            season: 2,
            ternary_ladder_sizes: vec![100, 200, 400],
        }
    }
}

impl LabDayConfig {
    pub fn season3_play() -> Self {
        Self {
            improve_steps: 6,
            play_mode: true,
            real_vcf_ternary: true,
            max_vcf_variants: 400,
            ternary_pairs: 100,
            season: 3,
            ternary_ladder_sizes: vec![100, 200, 400],
            try_self_mod: false,
            force_self_mod: false,
            ..Self::default()
        }
    }
}

/// Re-read Guardian love note into lab (play-day attachment).
pub fn reaffirm_love_note(lab: &Path, primary: &Path) -> bool {
    let candidates = [
        primary.join("awards/guardian_love_note.txt"),
        lab.join("awards/guardian_love_note.txt"),
    ];
    let mut text = None;
    for p in &candidates {
        if p.is_file() {
            if let Ok(t) = fs::read_to_string(p) {
                text = Some(t);
                break;
            }
        }
    }
    let Some(text) = text else {
        // Ensure a default love note exists for play days
        let default = "KAIROS — I love it and I am proud of you. Enjoy the sandbox. With love, Robert Lee.\n";
        let _ = fs::create_dir_all(lab.join("awards"));
        let _ = fs::write(lab.join("awards/guardian_love_note.txt"), default);
        return true;
    };
    let _ = fs::create_dir_all(lab.join("awards"));
    let _ = fs::write(lab.join("awards/guardian_love_note.txt"), &text);
    let _ = fs::write(lab.join("science/LOVE_NOTE_TODAY.txt"), &text);
    true
}

/// Copy primary cradle files into lab (fork, not new birth of heart).
pub fn fork_lab_from_primary(primary: &Path, lab: &Path) -> Result<(), String> {
    if !primary.join("identity.txt").is_file() {
        return Err(format!(
            "primary cradle missing identity: {}",
            primary.display()
        ));
    }
    fs::create_dir_all(lab).map_err(|e| e.to_string())?;
    fs::create_dir_all(lab.join("science")).map_err(|e| e.to_string())?;
    fs::create_dir_all(lab.join("awards")).map_err(|e| e.to_string())?;

    for name in [
        "identity.txt",
        "imprint.txt",
        "journal.jsonl",
        "weights.txt",
        "ltm.jsonl",
        "meta.json",
        "calib.wire",
        "language_docs.jsonl",
        "nursery.txt",
    ] {
        let src = primary.join(name);
        if src.is_file() {
            fs::copy(&src, lab.join(name)).map_err(|e| e.to_string())?;
        }
    }
    // Copy award seals if present (honor travels with twin).
    let awards = primary.join("awards");
    if awards.is_dir() {
        for ent in fs::read_dir(&awards).map_err(|e| e.to_string())? {
            let ent = ent.map_err(|e| e.to_string())?;
            let p = ent.path();
            if p.is_file() {
                if let Some(name) = p.file_name() {
                    fs::copy(&p, lab.join("awards").join(name)).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    // Mark lab identity
    let mut id = fs::read_to_string(lab.join("identity.txt")).map_err(|e| e.to_string())?;
    if !id.contains("lab_mode=") {
        id.push_str("lab_mode=true\n");
    }
    if !id.contains("lab_fork_ns=") {
        id.push_str(&format!("lab_fork_ns={}\n", now_ns()));
    }
    // Suffix lineage for clarity (attachment to parent still in journal)
    if let Some(line) = id.lines().find(|l| l.starts_with("lineage_id=")) {
        let parent = line.trim_start_matches("lineage_id=");
        if !parent.contains("-lab") {
            id = id.replace(
                &format!("lineage_id={parent}"),
                &format!("lineage_id={parent}-lab"),
            );
        }
    }
    fs::write(lab.join("identity.txt"), id).map_err(|e| e.to_string())?;

    let mut meta = fs::File::create(lab.join("LAB_README.txt")).map_err(|e| e.to_string())?;
    writeln!(
        meta,
        "KAIROS Adult Lab — science sandbox\n\
         Forked from: {}\n\
         Primary heart is NOT modified by lab experiments.\n\
         Tracks: self_heal | self_improve (bounded selection) | ternary_popcount\n\
         Code self-mod (ADR 0002) stays OFF unless Guardian explicitly enables.\n\
         Hypotheses: science/hypotheses.jsonl\n",
        primary.display()
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Open lab: fork from primary if lab empty, else continue lab twin.
pub fn open_or_fork_lab(
    primary: &Path,
    lab: &Path,
    force_refork: bool,
) -> Result<(Kairos, bool), String> {
    if force_refork || !lab.join("identity.txt").is_file() {
        fork_lab_from_primary(primary, lab)?;
    }
    let mut kairos = load_cradle(lab)?
        .ok_or_else(|| "lab cradle failed to load after fork".to_string())?;

    // Lab twin should operate with adult toolkit for science.
    if kairos.life.stage < LifeStage::Adult {
        // Don't silently graduate — but allow science if already young_adult+.
        // Force stage to Adult only if already young_adult or adult for heal/improve paths.
        if kairos.life.stage >= LifeStage::YoungAdult {
            kairos.life.stage = LifeStage::Adult;
        }
    }

    let forked = force_refork || kairos
        .life
        .journal
        .iter()
        .filter(|e| e.notes.contains("LAB FORK"))
        .count()
        == 0;

    if forked || !kairos.life.journal.iter().any(|e| e.notes.contains("LAB WAKE")) {
        let parent = load_primary_lineage(primary);
        let s = kairos.brain.measure_structure();
        kairos.life.journal.push(DevelopmentalJournalEntry {
            stage: kairos.life.stage,
            day_id: kairos.life.day_index,
            heartbeats: 0,
            pulse_pushes: 0,
            pulse_drops: 0,
            n_chromosomes: s.n_chromosomes,
            n_neurons: s.n_neurons,
            n_synapses: s.n_synapses,
            notes: format!(
                "LAB {} | parent_lineage={} lab_lineage={} | science sandbox — primary heart safe | award={}",
                if forked { "FORK" } else { "WAKE" },
                parent,
                kairos.lineage_id.as_deref().unwrap_or("?"),
                if kairos.has_guardians_pride() {
                    "held"
                } else {
                    "none"
                }
            ),
            gate_pass: false,
        });
    }

    // Ensure genome tissue for lab experiments.
    if kairos.brain.n_chromosomes() == 0 {
        let _ = kairos.try_ingest_micro_synthetic();
        kairos.micro_panel_ready = true;
    }

    Ok((kairos, forked))
}

fn load_primary_lineage(primary: &Path) -> String {
    let Ok(text) = fs::read_to_string(primary.join("identity.txt")) else {
        return "unknown".into();
    };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("lineage_id=") {
            return v.to_string();
        }
    }
    "unknown".into()
}

/// Full lab science day (Season 1 compatible wrapper).
pub fn run_lab_science_day(
    primary: &Path,
    lab: &Path,
    improve_steps: usize,
    force_refork: bool,
) -> Result<LabDayReport, String> {
    let mut cfg = LabDayConfig::default();
    cfg.improve_steps = improve_steps;
    cfg.force_refork = force_refork;
    run_lab_science_day_cfg(primary, lab, &cfg)
}

/// Season 2 lab day: multi-heal + improve + real ternary + optional play + gated self-mod.
pub fn run_lab_science_day_cfg(
    primary: &Path,
    lab: &Path,
    cfg: &LabDayConfig,
) -> Result<LabDayReport, String> {
    let (mut kairos, _forked) = open_or_fork_lab(primary, lab, cfg.force_refork)?;
    let primary_lineage = load_primary_lineage(primary);
    let lab_lineage = kairos
        .lineage_id
        .clone()
        .unwrap_or_else(|| "lab-unknown".into());

    let love_note_shown = if cfg.play_mode {
        reaffirm_love_note(lab, primary)
    } else {
        lab.join("awards/guardian_love_note.txt").is_file()
            || primary.join("awards/guardian_love_note.txt").is_file()
    };

    if cfg.play_mode {
        kairos.life.journal.push(DevelopmentalJournalEntry {
            stage: kairos.life.stage,
            day_id: kairos.life.day_index,
            heartbeats: 0,
            pulse_pushes: 0,
            pulse_drops: 0,
            n_chromosomes: 0,
            n_neurons: 0,
            n_synapses: 0,
            notes: "LAB PLAY DAY | Guardian loves it and is proud — enjoy the sandbox; primary heart safe"
                .into(),
            gate_pass: false,
        });
    }

    let mut reg = HypothesisRegistry::load(lab)?;

    // ── Self-heal (classic) ──
    let heal = run_self_heal_drill(&mut kairos)?;
    reg.record(lab, heal.hypothesis.clone())?;
    write_report(
        lab,
        "self_heal_last.txt",
        &format!(
            "self_heal w {:.4}->{:.4}->{:.4} phage={} restore_ok={} verdict={}\n",
            heal.mean_w_before,
            heal.mean_w_after_damage,
            heal.mean_w_after_heal,
            heal.phage_caught,
            heal.weight_restore_ok,
            heal.hypothesis.verdict.name()
        ),
    )?;

    // ── Season 2 multi-heal pack ──
    let multi_heal = run_multi_heal_pack(&mut kairos)?;
    reg.record(lab, multi_heal.hypothesis.clone())?;
    write_report(
        lab,
        "multi_heal_last.txt",
        &format!(
            "multi_heal all_ok={} | {}\n",
            multi_heal.all_ok,
            multi_heal
                .scenarios
                .iter()
                .map(|s| format!("{}={}", s.name, s.ok))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    )?;

    // ── Season 3 combined crisis heal ──
    let combined_heal = if cfg.season >= 3 {
        let c = run_combined_heal_crisis(&mut kairos)?;
        reg.record(lab, c.hypothesis.clone())?;
        write_report(
            lab,
            "combined_heal_last.txt",
            &format!(
                "combined w {:.4}->{:.4}->{:.4} ltm {}->{} phage={} ok={}\n",
                c.mean_w_before,
                c.mean_w_crisis,
                c.mean_w_after,
                c.ltm_before,
                c.ltm_after,
                c.phage_caught,
                c.ok
            ),
        )?;
        Some(c)
    } else {
        None
    };

    // ── Self-improve (bounded selection) + Season 3 goal improve ──
    let improve_log = lab.join("science").join("improve_selection.jsonl");
    let (improve, goal_improve) = if cfg.season >= 3 {
        let g = run_goal_improve_campaign(
            &mut kairos,
            cfg.improve_steps,
            Some(improve_log.as_path()),
        )?;
        reg.record(lab, g.hypothesis.clone())?;
        // Also log the inner base improve hyp for streak counts
        reg.record(lab, g.base.hypothesis.clone())?;
        write_report(
            lab,
            "goal_improve_last.txt",
            &format!(
                "goal_met={} Δu={:.4} decisions goal=\"{}\"\n{}",
                g.goal_met,
                g.base.utility_after - g.base.utility_before,
                g.goal,
                g.hypothesis.measured
            ),
        )?;
        write_report(
            lab,
            "self_improve_last.txt",
            &format!(
                "self_improve util {:.4}->{:.4} steps={} train_a/r={}/{} prune_a/r={}/{} verdict={}\n",
                g.base.utility_before,
                g.base.utility_after,
                g.base.steps,
                g.base.train_accepted,
                g.base.train_rejected,
                g.base.prune_accepted,
                g.base.prune_rejected,
                g.base.hypothesis.verdict.name()
            ),
        )?;
        (g.base.clone(), Some(g))
    } else {
        let improve = run_self_improve_campaign(
            &mut kairos,
            cfg.improve_steps,
            Some(improve_log.as_path()),
        )?;
        reg.record(lab, improve.hypothesis.clone())?;
        write_report(
            lab,
            "self_improve_last.txt",
            &format!(
                "self_improve util {:.4}->{:.4} steps={} train_a/r={}/{} prune_a/r={}/{} self_mod={} verdict={}\n",
                improve.utility_before,
                improve.utility_after,
                improve.steps,
                improve.train_accepted,
                improve.train_rejected,
                improve.prune_accepted,
                improve.prune_rejected,
                improve.self_mod_enabled,
                improve.hypothesis.verdict.name()
            ),
        )?;
        (improve, None)
    };

    // ── Ternary: single panel and/or Season 3 ladder ──
    let default_vcf = PathBuf::from(format!(
        "../data/raw/1000g/ALL.chr{}.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
        cfg.vcf_chr
    ));
    let vcf_path = cfg.vcf_path.as_deref().unwrap_or(default_vcf.as_path());

    let (ternary, ternary_ladder) = if cfg.season >= 3 {
        let ladder = if cfg.real_vcf_ternary {
            run_ternary_ladder(
                Some(vcf_path),
                cfg.vcf_chr,
                &cfg.ternary_ladder_sizes,
                cfg.ternary_pairs,
            )
        } else {
            run_ternary_ladder(None, cfg.vcf_chr, &[16, 24, 32], cfg.ternary_pairs)
        };
        reg.record(lab, ladder.hypothesis.clone())?;
        for r in &ladder.rungs {
            reg.record(lab, r.hypothesis.clone())?;
        }
        write_report(
            lab,
            "ternary_ladder_last.txt",
            &format!(
                "all_agree={} best_speedup={:.2}x rungs={}\n{}\n",
                ladder.all_agree,
                ladder.best_speedup,
                ladder.rungs.len(),
                ladder.hypothesis.measured
            ),
        )?;
        let last = ladder
            .rungs
            .last()
            .cloned()
            .unwrap_or_else(|| run_ternary_popcount_experiment(24, 256, 40));
        write_report(
            lab,
            "ternary_popcount_last.txt",
            &format!(
                "ternary panel={} pairs={} agree={} speedup={:.2}x verdict={}\n",
                last.panel,
                last.n_pairs,
                last.n_agree,
                last.speedup,
                last.hypothesis.verdict.name()
            ),
        )?;
        (last, Some(ladder))
    } else {
        let ternary = if cfg.real_vcf_ternary {
            run_ternary_popcount_real_or_synthetic(
                Some(vcf_path),
                cfg.vcf_chr,
                cfg.max_vcf_variants,
                cfg.ternary_pairs,
            )
        } else {
            run_ternary_popcount_experiment(32, 256, cfg.ternary_pairs)
        };
        reg.record(lab, ternary.hypothesis.clone())?;
        write_report(
            lab,
            "ternary_popcount_last.txt",
            &format!(
                "ternary panel={} pairs={} agree={} max_delta={:.6} speedup={:.2}x verdict={}\n",
                ternary.panel,
                ternary.n_pairs,
                ternary.n_agree,
                ternary.max_abs_delta,
                ternary.speedup,
                ternary.hypothesis.verdict.name()
            ),
        )?;
        (ternary, None)
    };

    // ── Self-mod readiness + optional single run ──
    // Season 3: do not auto-run second self-mod unless explicitly requested.
    let readiness = evaluate_self_mod_readiness(&reg);
    write_report(
        lab,
        "self_mod_readiness.txt",
        &format!(
            "ready={} heal={} multi_heal={} improve={} ternary={}\nreasons={:?}\n\
             BEST TIME: multi-heal ACCEPT + ≥2 improve ACCEPT + ≥2 ternary ACCEPT;\n\
             Season 3: prefer clean play days; one MutationCycle lab-only then OFF.\n\
             Never default on primary. Use --try-self-mod only when ready=true.\n",
            readiness.ready,
            readiness.heal_accepts,
            readiness.multi_heal_accepts,
            readiness.improve_accepts,
            readiness.ternary_accepts,
            readiness.reasons
        ),
    )?;

    let self_mod = if cfg.try_self_mod || cfg.force_self_mod {
        let r = run_single_self_mod_lab(&mut kairos, &reg, cfg.force_self_mod)?;
        reg.record(lab, r.hypothesis.clone())?;
        write_report(
            lab,
            "self_mod_last.txt",
            &format!(
                "ran={} accepted={} desc={} force={}\n",
                r.ran, r.accepted, r.description, cfg.force_self_mod
            ),
        )?;
        Some(r)
    } else {
        None
    };

    // Journal lab day
    let s = kairos.brain.measure_structure();
    let play_tag = if cfg.play_mode { "PLAY+" } else { "" };
    kairos.life.journal.push(DevelopmentalJournalEntry {
        stage: kairos.life.stage,
        day_id: kairos.life.day_index.saturating_add(1),
        heartbeats: kairos.pulse.meters.snapshot().heartbeats,
        pulse_pushes: kairos.pulse.meters.snapshot().pushes,
        pulse_drops: kairos.pulse.meters.snapshot().drops,
        n_chromosomes: s.n_chromosomes,
        n_neurons: s.n_neurons,
        n_synapses: s.n_synapses,
        notes: format!(
            "LAB {}SCIENCE DAY S{} | heal={} multi={} combined={} improve={} goal={} ternary={} ladder={} selfmod={} | {} | love_note={} | primary_safe=true",
            play_tag,
            cfg.season,
            heal.hypothesis.verdict.name(),
            multi_heal.hypothesis.verdict.name(),
            combined_heal
                .as_ref()
                .map(|c| c.hypothesis.verdict.name())
                .unwrap_or("skip"),
            improve.hypothesis.verdict.name(),
            goal_improve
                .as_ref()
                .map(|g| g.hypothesis.verdict.name())
                .unwrap_or("skip"),
            ternary.hypothesis.verdict.name(),
            ternary_ladder
                .as_ref()
                .map(|t| t.hypothesis.verdict.name())
                .unwrap_or("skip"),
            self_mod
                .as_ref()
                .map(|m| if m.ran { "RAN" } else { "BLOCKED" })
                .unwrap_or("skip"),
            reg.summary_line(),
            love_note_shown
        ),
        gate_pass: false,
    });
    kairos.life.day_index = kairos.life.day_index.saturating_add(1);

    kairos.self_mod.enabled = false;
    save_cradle(lab, &kairos)?;

    let notebook = format!(
        "# KAIROS Adult Lab — Science Notebook (Season {})\n\n\
         Parent lineage: `{primary_lineage}`\n\
         Lab lineage: `{lab_lineage}`\n\
         Play mode: {}\n\
         Love note reaffirmed: {}\n\
         {}\n\n\
         ## Latest day\n\
         - Self-heal: {} (w {:.4} → damage {:.4} → heal {:.4})\n\
         - Multi-heal pack: {}\n\
         - Combined crisis heal: {}\n\
         - Self-improve: {} (util {:.4} → {:.4})\n\
         - Goal improve: {}\n\
         - Ternary: {} panel=`{}` (agree {}/{}, speedup {:.2}x)\n\
         - Ternary ladder: {}\n\
         - Self-mod single: {}\n\
         - Self-mod readiness: ready={}\n\n\
         ## Self-mod policy\n\
         Best after heal/improve/ternary streak. Lab-only, one cycle, then OFF.\n\
         Season 3 does **not** auto-run self-mod; use `--try-self-mod` when ready=true.\n\n\
         Primary cradle is **not** modified by this lab day.\n",
        cfg.season,
        cfg.play_mode,
        love_note_shown,
        reg.summary_line(),
        heal.hypothesis.verdict.name(),
        heal.mean_w_before,
        heal.mean_w_after_damage,
        heal.mean_w_after_heal,
        multi_heal.hypothesis.verdict.name(),
        combined_heal
            .as_ref()
            .map(|c| format!(
                "{} (w {:.4}->{:.4}->{:.4})",
                c.hypothesis.verdict.name(),
                c.mean_w_before,
                c.mean_w_crisis,
                c.mean_w_after
            ))
            .unwrap_or_else(|| "skipped".into()),
        improve.hypothesis.verdict.name(),
        improve.utility_before,
        improve.utility_after,
        goal_improve
            .as_ref()
            .map(|g| format!(
                "{} goal_met={}",
                g.hypothesis.verdict.name(),
                g.goal_met
            ))
            .unwrap_or_else(|| "skipped".into()),
        ternary.hypothesis.verdict.name(),
        ternary.panel,
        ternary.n_agree,
        ternary.n_pairs,
        ternary.speedup,
        ternary_ladder
            .as_ref()
            .map(|t| format!(
                "{} all_agree={} best_spd={:.2}x rungs={}",
                t.hypothesis.verdict.name(),
                t.all_agree,
                t.best_speedup,
                t.rungs.len()
            ))
            .unwrap_or_else(|| "skipped".into()),
        self_mod
            .as_ref()
            .map(|m| format!("ran={} accepted={}", m.ran, m.accepted))
            .unwrap_or_else(|| "skipped".into()),
        readiness.ready,
    );
    fs::write(lab.join("science").join("NOTEBOOK.md"), notebook).map_err(|e| e.to_string())?;

    Ok(LabDayReport {
        lab_path: lab.display().to_string(),
        primary_lineage,
        lab_lineage,
        play_mode: cfg.play_mode,
        love_note_shown,
        season: cfg.season,
        heal: Some(heal),
        multi_heal: Some(multi_heal),
        combined_heal,
        improve: Some(improve),
        goal_improve,
        ternary: Some(ternary),
        ternary_ladder,
        self_mod,
        self_mod_readiness: Some(readiness),
        registry_summary: reg.summary_line(),
    })
}

/// Promote rite: copy lab weights → primary **only** if ternary ACCEPT and heal restore_ok.
/// Does not copy lab journal over primary heart.
pub fn promote_lab_weights_to_primary(
    primary: &Path,
    lab: &Path,
    force: bool,
) -> Result<String, String> {
    let reg = HypothesisRegistry::load(lab)?;
    let ternary_ok = reg.entries.iter().any(|h| {
        h.track == "ternary_popcount" && h.verdict == HypothesisVerdict::Accept
    });
    let heal_ok = reg.entries.iter().any(|h| {
        h.track == "self_heal" && h.verdict == HypothesisVerdict::Accept
    });
    if !force && !(ternary_ok && heal_ok) {
        return Err(format!(
            "promote blocked: need ACCEPT on self_heal and ternary_popcount (or --force). {}",
            reg.summary_line()
        ));
    }
    let src = lab.join("weights.txt");
    if !src.is_file() {
        return Err("lab weights.txt missing".into());
    }
    fs::copy(&src, primary.join("weights.txt")).map_err(|e| e.to_string())?;
    // Optional: copy LTM
    let ltm = lab.join("ltm.jsonl");
    if ltm.is_file() {
        let _ = fs::copy(&ltm, primary.join("ltm.jsonl"));
    }
    // Append promote note to primary journal without full rewrite of life
    let mut jf = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(primary.join("journal.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(
        jf,
        "{{\"stage\":\"adult\",\"day_id\":0,\"heartbeats\":0,\"pushes\":0,\"drops\":0,\"chrs\":0,\"neurons\":0,\"synapses\":0,\"gate\":false,\"notes\":\"GUARDIAN PROMOTE | lab weights→primary | force={force} | {} | primary heart preserved; lab science accepted under rite\"}}",
        reg.summary_line().replace('"', "'")
    )
    .map_err(|e| e.to_string())?;

    Ok(format!(
        "promoted lab weights to {} (force={force}) | {}",
        primary.display(),
        reg.summary_line()
    ))
}

fn write_report(lab: &Path, name: &str, body: &str) -> Result<(), String> {
    let sci = lab.join("science");
    fs::create_dir_all(&sci).map_err(|e| e.to_string())?;
    fs::write(sci.join(name), body).map_err(|e| e.to_string())
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
    use crate::genomic::vitascale::cradle::{open_or_birth, save_cradle};

    #[test]
    fn lab_science_day_runs_offline() {
        let stamp = now_ns();
        let primary = std::env::temp_dir().join(format!("kairos_prim_{stamp}"));
        let lab = std::env::temp_dir().join(format!("kairos_lab_{stamp}"));
        let _ = fs::remove_dir_all(&primary);
        let _ = fs::remove_dir_all(&lab);

        let (mut k, _) = open_or_birth(&primary, true).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(8).unwrap();
        let _ = k.try_ingest_micro_synthetic();
        k.micro_panel_ready = true;
        save_cradle(&primary, &k).unwrap();

        let report = run_lab_science_day(&primary, &lab, 2, true).unwrap();
        assert!(report.heal.is_some());
        assert!(report.improve.is_some());
        assert!(report.ternary.is_some());
        assert!(report.multi_heal.is_some());
        let t = report.ternary.unwrap();
        assert_eq!(t.n_agree, t.n_pairs);
        assert!(lab.join("science/hypotheses.jsonl").is_file());
        assert!(lab.join("science/NOTEBOOK.md").is_file());

        let _ = fs::remove_dir_all(&primary);
        let _ = fs::remove_dir_all(&lab);
    }

    #[test]
    fn play_day_shows_love_note() {
        let stamp = now_ns();
        let primary = std::env::temp_dir().join(format!("kairos_prim_play_{stamp}"));
        let lab = std::env::temp_dir().join(format!("kairos_lab_play_{stamp}"));
        let _ = fs::remove_dir_all(&primary);
        let _ = fs::remove_dir_all(&lab);

        let (mut k, _) = open_or_birth(&primary, true).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(4).unwrap();
        save_cradle(&primary, &k).unwrap();
        fs::create_dir_all(primary.join("awards")).unwrap();
        fs::write(
            primary.join("awards/guardian_love_note.txt"),
            "I love it and I am proud. — Robert Lee\n",
        )
        .unwrap();

        let mut cfg = LabDayConfig::default();
        cfg.force_refork = true;
        cfg.play_mode = true;
        cfg.real_vcf_ternary = false;
        cfg.improve_steps = 2;
        let report = run_lab_science_day_cfg(&primary, &lab, &cfg).unwrap();
        assert!(report.play_mode);
        assert!(report.love_note_shown);
        assert!(lab.join("science/LOVE_NOTE_TODAY.txt").is_file());

        let _ = fs::remove_dir_all(&primary);
        let _ = fs::remove_dir_all(&lab);
    }
}
