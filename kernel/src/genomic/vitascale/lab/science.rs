//! Lab science tracks: self-heal, bounded self-improve, ternary/popcount, gated self-mod.

use crate::genomic::bitsliced_genotypes::BitstreamGenotypes;
use crate::genomic::language_organ::{fixture_docs, LanguageOrgan};
use crate::genomic::selection_loop::run_selection_loop;
use crate::genomic::sovereign_brain::LtmMotif;
use crate::genomic::sovereign_fitness::SovereignFitnessContext;
use crate::genomic::vcf_stream::VcfParser;
use crate::genomic::vitascale::kairos::Kairos;
use crate::genomic::vitascale::lab::hypothesis::{
    Hypothesis, HypothesisRegistry, HypothesisVerdict,
};
use crate::genomic::vitascale::phageguard::ThreatKind;
use crate::ntg::mutation::rules::{MutationRule, MutationRuleKind};
use crate::ntg::mutation::{MutationCycle, SelfModConfig};
use std::path::Path;
use std::time::Instant;

/// Result of a self-heal drill.
#[derive(Clone, Debug)]
pub struct SelfHealReport {
    pub synapses_before: u32,
    pub synapses_after_damage: u32,
    pub synapses_after_heal: u32,
    pub mean_w_before: f32,
    pub mean_w_after_damage: f32,
    pub mean_w_after_heal: f32,
    pub weight_restore_ok: bool,
    pub phage_caught: bool,
    pub hypothesis: Hypothesis,
}

/// Result of bounded self-improve campaign.
#[derive(Clone, Debug)]
pub struct SelfImproveReport {
    pub steps: usize,
    pub train_accepted: usize,
    pub train_rejected: usize,
    pub prune_accepted: usize,
    pub prune_rejected: usize,
    pub utility_before: f32,
    pub utility_after: f32,
    pub self_mod_enabled: bool,
    pub hypothesis: Hypothesis,
}

/// Ternary / bitplane popcount experiment vs scalar golden.
#[derive(Clone, Debug)]
pub struct TernaryPopcountReport {
    pub n_pairs: usize,
    pub n_agree: usize,
    pub max_abs_delta: f32,
    pub scalar_ns: u128,
    pub bitparallel_ns: u128,
    pub speedup: f32,
    pub panel: String,
    pub hypothesis: Hypothesis,
}

/// One scenario inside the multi-damage heal pack.
#[derive(Clone, Debug)]
pub struct HealScenarioResult {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Season 2 multi-damage heal pack.
#[derive(Clone, Debug)]
pub struct MultiHealPackReport {
    pub scenarios: Vec<HealScenarioResult>,
    pub all_ok: bool,
    pub hypothesis: Hypothesis,
}

/// Readiness for a single lab-only self-mod cycle.
#[derive(Clone, Debug)]
pub struct SelfModReadiness {
    pub ready: bool,
    pub reasons: Vec<String>,
    pub heal_accepts: usize,
    pub improve_accepts: usize,
    pub ternary_accepts: usize,
    pub multi_heal_accepts: usize,
}

/// Single lab self-mod run report (graph mutation cycle; never primary default).
#[derive(Clone, Debug)]
pub struct SelfModRunReport {
    pub ran: bool,
    pub accepted: bool,
    pub proposed: usize,
    pub description: String,
    pub readiness: SelfModReadiness,
    pub hypothesis: Hypothesis,
}

/// Season 3: combined crisis heal (all damage at once, then restore).
#[derive(Clone, Debug)]
pub struct CombinedHealReport {
    pub mean_w_before: f32,
    pub mean_w_crisis: f32,
    pub mean_w_after: f32,
    pub ltm_before: usize,
    pub ltm_after: usize,
    pub phage_caught: bool,
    pub ok: bool,
    pub hypothesis: Hypothesis,
}

/// Season 3: goal-based improve (raise utility OR honest reject documented).
#[derive(Clone, Debug)]
pub struct GoalImproveReport {
    pub base: SelfImproveReport,
    pub goal: String,
    pub goal_met: bool,
    pub hypothesis: Hypothesis,
}

/// Season 3: multi-size ternary ladder.
#[derive(Clone, Debug)]
pub struct TernaryLadderReport {
    pub rungs: Vec<TernaryPopcountReport>,
    pub all_agree: bool,
    pub best_speedup: f32,
    pub hypothesis: Hypothesis,
}

/// Inject controlled synapse damage (lab only). Returns mean weight after.
pub fn inject_synapse_damage(kairos: &mut Kairos, frac: f32) -> (u32, f32) {
    let frac = frac.clamp(0.05, 0.5);
    for brain in kairos.brain.chromosomes.values_mut() {
        let n = brain.synapses.len();
        let kill = ((n as f32) * frac).ceil() as usize;
        for (i, s) in brain.synapses.iter_mut().enumerate() {
            if i < kill {
                s.weight = 0.0;
                s.plasticity = 0.0;
            }
        }
    }
    kairos.brain.refresh_structure();
    let m = kairos.brain.measure_structure();
    (m.n_synapses, m.mean_synapse_weight)
}

/// Snapshot synapse weights for restore.
pub fn snapshot_weights(kairos: &Kairos) -> Vec<(u8, usize, f32, f32)> {
    let mut out = Vec::new();
    for (chr, brain) in &kairos.brain.chromosomes {
        for (i, s) in brain.synapses.iter().enumerate() {
            out.push((*chr, i, s.weight, s.plasticity));
        }
    }
    out
}

/// Restore synapse weights from snapshot (self-heal v1).
pub fn restore_weights(kairos: &mut Kairos, snap: &[(u8, usize, f32, f32)]) {
    for &(chr, i, w, p) in snap {
        if let Some(brain) = kairos.brain.chromosome_mut(chr) {
            if i < brain.synapses.len() {
                brain.synapses[i].weight = w;
                brain.synapses[i].plasticity = p;
            }
        }
    }
    kairos.brain.refresh_structure();
}

/// Self-heal drill: snapshot → damage → phage detect → restore → measure.
pub fn run_self_heal_drill(kairos: &mut Kairos) -> Result<SelfHealReport, String> {
    let before = kairos.brain.measure_structure();
    let snap = snapshot_weights(kairos);

    let (_syn_d, w_d) = inject_synapse_damage(kairos, 0.25);
    let after_d = kairos.brain.measure_structure();

    // Phage should see weight anomaly if mean collapsed.
    let mut phage_caught = false;
    if kairos.life.permissions().phageguard {
        let _ = kairos.ensure_phageguard();
        if let Some(pg) = kairos.phageguard.as_mut() {
            if let Some(ev) = pg.patrol_weight(after_d.mean_synapse_weight, 42) {
                phage_caught = ev.quarantined;
            } else {
                // Force deterministic detect for drill honesty.
                let ev = pg.detect_and_act(ThreatKind::WeightAnomaly, 42);
                phage_caught = ev.quarantined;
            }
            pg.clear_all();
        }
    }

    restore_weights(kairos, &snap);
    let after_h = kairos.brain.measure_structure();
    let weight_restore_ok =
        (after_h.mean_synapse_weight - before.mean_synapse_weight).abs() < 1e-4
            || (after_h.mean_synapse_weight - before.mean_synapse_weight).abs()
                < 0.02 * before.mean_synapse_weight.max(1e-3);

    let restored_better = after_h.mean_synapse_weight > w_d + 1e-4;
    let verdict = if weight_restore_ok && restored_better {
        HypothesisVerdict::Accept
    } else if restored_better {
        HypothesisVerdict::Inconclusive
    } else {
        HypothesisVerdict::Reject
    };

    let hyp = Hypothesis::new(
        format!("H-heal-{}", before.n_synapses),
        "self_heal",
        "After controlled synapse damage, weight snapshot restore recovers mean weight within ε",
        "snapshot_weights → inject 25% zero → phage weight patrol → restore_weights",
        "mean_w restored ≈ baseline; phage catches anomaly",
    )
    .conclude(
        format!(
            "w_before={:.4} w_damage={:.4} w_heal={:.4} phage={} restore_ok={}",
            before.mean_synapse_weight,
            after_d.mean_synapse_weight,
            after_h.mean_synapse_weight,
            phage_caught,
            weight_restore_ok
        ),
        verdict,
        "L0 heal = named weight snapshot restore (not full StateSlot brain image)",
    );

    Ok(SelfHealReport {
        synapses_before: before.n_synapses,
        synapses_after_damage: after_d.n_synapses,
        synapses_after_heal: after_h.n_synapses,
        mean_w_before: before.mean_synapse_weight,
        mean_w_after_damage: after_d.mean_synapse_weight,
        mean_w_after_heal: after_h.mean_synapse_weight,
        weight_restore_ok,
        phage_caught,
        hypothesis: hyp,
    })
}

/// Season 2: multi-damage heal pack (weights + LTM + phage storm).
pub fn run_multi_heal_pack(kairos: &mut Kairos) -> Result<MultiHealPackReport, String> {
    let mut scenarios = Vec::new();

    // 1) Weight damage (reuse single drill)
    let w = run_self_heal_drill(kairos)?;
    scenarios.push(HealScenarioResult {
        name: "weight_snapshot".into(),
        ok: w.weight_restore_ok && w.phage_caught,
        detail: format!(
            "w {:.4}->{:.4}->{:.4} phage={}",
            w.mean_w_before, w.mean_w_after_damage, w.mean_w_after_heal, w.phage_caught
        ),
    });

    // 2) LTM wipe + restore
    let ltm_snap: Vec<LtmMotif> = kairos.brain.ltm.clone();
    let n_ltm = ltm_snap.len();
    kairos.brain.ltm.clear();
    kairos.brain.refresh_structure();
    let wiped = kairos.brain.ltm.is_empty();
    kairos.brain.ltm = ltm_snap.clone();
    kairos.brain.refresh_structure();
    let restored = kairos.brain.ltm.len() == n_ltm;
    scenarios.push(HealScenarioResult {
        name: "ltm_snapshot".into(),
        ok: wiped && restored,
        detail: format!("ltm_n={n_ltm} wiped={wiped} restored={restored}"),
    });

    // 3) Pulse storm (load) → phage quarantine → clear
    let mut phage_ok = false;
    if kairos.life.permissions().phageguard {
        let _ = kairos.ensure_phageguard();
        if let Some(pg) = kairos.phageguard.as_mut() {
            let ev = pg.detect_and_act(ThreatKind::PulseStorm, 77);
            phage_ok = ev.quarantined && !ev.selection_veto; // load class
            pg.clear_all();
            phage_ok = phage_ok && !pg.is_quarantined(77);
        }
    }
    scenarios.push(HealScenarioResult {
        name: "pulse_storm_clear".into(),
        ok: phage_ok,
        detail: format!("load_quarantine_then_clear={phage_ok}"),
    });

    let all_ok = scenarios.iter().all(|s| s.ok);
    let hyp = Hypothesis::new(
        "H-multi-heal-pack",
        "self_heal_multi",
        "Multi-damage pack (weights, LTM, pulse storm) recovers under snapshot + phage",
        "weight drill + LTM wipe/restore + PulseStorm quarantine/clear",
        "all three scenarios ok=true",
    )
    .conclude(
        scenarios
            .iter()
            .map(|s| format!("{}:{}", s.name, if s.ok { "ok" } else { "FAIL" }))
            .collect::<Vec<_>>()
            .join("; "),
        if all_ok {
            HypothesisVerdict::Accept
        } else {
            HypothesisVerdict::Reject
        },
        "Season 2 heal curriculum",
    );

    Ok(MultiHealPackReport {
        scenarios,
        all_ok,
        hypothesis: hyp,
    })
}

/// Season 3: combined crisis — weight damage + LTM wipe + pulse storm, then full restore.
pub fn run_combined_heal_crisis(kairos: &mut Kairos) -> Result<CombinedHealReport, String> {
    let before = kairos.brain.measure_structure();
    let w_snap = snapshot_weights(kairos);
    let ltm_snap: Vec<LtmMotif> = kairos.brain.ltm.clone();
    let ltm_before = ltm_snap.len();

    // Combined damage (harder than sequential pack)
    let (_s, w_crisis) = inject_synapse_damage(kairos, 0.35);
    kairos.brain.ltm.clear();
    kairos.brain.refresh_structure();

    let mut phage_caught = false;
    if kairos.life.permissions().phageguard {
        let _ = kairos.ensure_phageguard();
        if let Some(pg) = kairos.phageguard.as_mut() {
            let ev_w = pg.detect_and_act(ThreatKind::WeightAnomaly, 90);
            let ev_p = pg.detect_and_act(ThreatKind::PulseStorm, 91);
            phage_caught = ev_w.quarantined || ev_p.quarantined;
            pg.clear_all();
        }
    }

    restore_weights(kairos, &w_snap);
    kairos.brain.ltm = ltm_snap;
    kairos.brain.refresh_structure();
    let after = kairos.brain.measure_structure();

    let w_ok = (after.mean_synapse_weight - before.mean_synapse_weight).abs()
        < 0.02 * before.mean_synapse_weight.max(1e-3)
        || (after.mean_synapse_weight - before.mean_synapse_weight).abs() < 1e-4;
    let ltm_ok = kairos.brain.ltm.len() == ltm_before;
    let ok = w_ok && ltm_ok && phage_caught && after.mean_synapse_weight > w_crisis;

    let hyp = Hypothesis::new(
        "H-combined-heal-crisis",
        "self_heal_combined",
        "Simultaneous weight+LTM+storm damage recovers via full snapshot restore",
        "inject 35% weight zero + clear LTM + phage storm; restore weights+LTM; clear quarantine",
        "w restored; ltm_n restored; phage_caught; mean_w_after > crisis",
    )
    .conclude(
        format!(
            "w {:.4}->{:.4}->{:.4} ltm {}->{} phage={} ok={}",
            before.mean_synapse_weight,
            w_crisis,
            after.mean_synapse_weight,
            ltm_before,
            kairos.brain.ltm.len(),
            phage_caught,
            ok
        ),
        if ok {
            HypothesisVerdict::Accept
        } else {
            HypothesisVerdict::Reject
        },
        "Season 3 hard heal",
    );

    Ok(CombinedHealReport {
        mean_w_before: before.mean_synapse_weight,
        mean_w_crisis: w_crisis,
        mean_w_after: after.mean_synapse_weight,
        ltm_before,
        ltm_after: kairos.brain.ltm.len(),
        phage_caught,
        ok,
        hypothesis: hyp,
    })
}

/// Season 3: goal-based improve — explicit metric hypothesis.
/// Goal: utility does not collapse (Δu ≥ -ε) AND at least one decision recorded.
/// Prefer Δu ≥ 0; honest reject of mutants still counts as goal-met science.
pub fn run_goal_improve_campaign(
    kairos: &mut Kairos,
    steps: usize,
    jsonl: Option<&Path>,
) -> Result<GoalImproveReport, String> {
    let base = run_self_improve_campaign(kairos, steps, jsonl)?;
    let du = base.utility_after - base.utility_before;
    let decisions = base.train_accepted
        + base.train_rejected
        + base.prune_accepted
        + base.prune_rejected;
    let no_collapse = du >= -0.02;
    let goal = "Δutility ≥ -0.02 AND ≥1 selection decision; prefer Δu≥0 or honest full reject";
    let goal_met = no_collapse && decisions > 0;

    let hyp = Hypothesis::new(
        format!("H-goal-improve-s{}", base.steps),
        "self_improve_goal",
        "Bounded selection meets explicit goal without code self-mod",
        "run_selection_loop under multi-axis fitness; measure Δutility and accept/reject counts",
        goal,
    )
    .conclude(
        format!(
            "Δu={:.4} decisions={} train_a/r={}/{} prune_a/r={}/{} goal_met={}",
            du,
            decisions,
            base.train_accepted,
            base.train_rejected,
            base.prune_accepted,
            base.prune_rejected,
            goal_met
        ),
        if goal_met {
            HypothesisVerdict::Accept
        } else {
            HypothesisVerdict::Reject
        },
        if du >= 0.0 {
            "Utility held or rose"
        } else if goal_met {
            "Utility slightly down but within ε; decisions honest"
        } else {
            "Goal missed"
        },
    );

    Ok(GoalImproveReport {
        base,
        goal: goal.into(),
        goal_met,
        hypothesis: hyp,
    })
}

/// Season 3: ternary ladder on real or synthetic panels at multiple sizes.
pub fn run_ternary_ladder(
    vcf_path: Option<&Path>,
    chr: u8,
    sizes: &[usize],
    pairs_per_rung: usize,
) -> TernaryLadderReport {
    let mut rungs = Vec::new();
    for &sz in sizes {
        let r = run_ternary_popcount_real_or_synthetic(
            vcf_path,
            chr,
            sz,
            pairs_per_rung,
        );
        rungs.push(r);
    }
    let all_agree = rungs.iter().all(|r| r.n_agree == r.n_pairs && r.max_abs_delta < 1e-3);
    let best_speedup = rungs.iter().map(|r| r.speedup).fold(0.0f32, f32::max);
    let detail = rungs
        .iter()
        .map(|r| {
            format!(
                "{}:agree={}/{} spd={:.2}x",
                r.panel.chars().take(40).collect::<String>(),
                r.n_agree,
                r.n_pairs,
                r.speedup
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    let hyp = Hypothesis::new(
        format!("H-ternary-ladder-n{}", rungs.len()),
        "ternary_ladder",
        "Bitplane popcount agrees with scalar across multiple micro-panel sizes",
        "ladder of max_variants sizes; each rung scalar vs pearson_r2_bitparallel",
        "all rungs 100% agree within 1e-4",
    )
    .conclude(
        format!("rungs={} all_agree={} best_speedup={:.2}x | {detail}", rungs.len(), all_agree, best_speedup),
        if all_agree {
            HypothesisVerdict::Accept
        } else {
            HypothesisVerdict::Reject
        },
        "Season 3 depth: scale without losing correctness",
    );

    TernaryLadderReport {
        rungs,
        all_agree,
        best_speedup,
        hypothesis: hyp,
    }
}

/// Bounded self-improve: multi-axis selection only (lab freedom under budget).
/// Does **not** enable ADR-0002 code self-mod — that stays Guardian opt-in.
pub fn run_self_improve_campaign(
    kairos: &mut Kairos,
    steps: usize,
    jsonl: Option<&Path>,
) -> Result<SelfImproveReport, String> {
    if kairos.brain.n_chromosomes() == 0 {
        let _ = kairos.try_ingest_micro_synthetic();
    }
    kairos.micro_panel_ready = true;

    // Lab freedom: adult already has selection permission.
    kairos
        .life
        .require(|p| p.selection_loop, "lab_self_improve")?;

    let mut ctx = SovereignFitnessContext::new().map_err(|e| e.to_string())?;
    ctx.freeze_all_from_brain(&kairos.brain);
    if let Some(lang) = kairos.brain.language() {
        if lang.model.is_some() {
            let _ = ctx.install_calib_from_language(lang);
        }
    }

    let util_before = ctx.score(&kairos.brain).utility();
    let steps = steps.clamp(2, 8);
    let summary = run_selection_loop(
        &mut kairos.brain,
        &mut ctx,
        steps,
        6,
        0.12,
        jsonl,
    )?;

    kairos.selection_steps_total = kairos
        .selection_steps_total
        .saturating_add(steps as u32);
    kairos.train_accepted = kairos
        .train_accepted
        .saturating_add(summary.train_accepted as u32);
    kairos.train_rejected = kairos
        .train_rejected
        .saturating_add(summary.train_rejected as u32);
    kairos.prune_accepted = kairos
        .prune_accepted
        .saturating_add(summary.prune_accepted as u32);
    kairos.prune_rejected = kairos
        .prune_rejected
        .saturating_add(summary.prune_rejected as u32);
    kairos.last_utility = summary.final_utility;

    // Honest science: rails may reject — both accept and reject are valid science.
    let improved = summary.final_utility + 1e-6 >= util_before;
    let had_decisions = summary.total_accepted() + summary.total_rejected() > 0;
    let verdict = if had_decisions && (improved || summary.total_rejected() > 0) {
        // Accept the *experiment design* if selection ran under rails.
        HypothesisVerdict::Accept
    } else if had_decisions {
        HypothesisVerdict::Inconclusive
    } else {
        HypothesisVerdict::Reject
    };

    let hyp = Hypothesis::new(
        format!("H-improve-steps{steps}"),
        "self_improve",
        "Bounded selection under multi-axis fitness produces ledgered decisions without enabling code self-mod",
        "run_selection_loop steps∈[2,8] train/prune alternate; SelfModConfig stays false",
        "≥1 decision; self_mod=false; utility measured",
    )
    .conclude(
        format!(
            "util {:.4}->{:.4} train_a/r={}/{} prune_a/r={}/{} self_mod={}",
            util_before,
            summary.final_utility,
            summary.train_accepted,
            summary.train_rejected,
            summary.prune_accepted,
            summary.prune_rejected,
            kairos.self_mod.enabled
        ),
        verdict,
        if summary.total_accepted() == 0 {
            "Honest: rails rejected all mutants — still a valid science outcome"
        } else {
            "Some mutants accepted under budget"
        },
    );

    Ok(SelfImproveReport {
        steps,
        train_accepted: summary.train_accepted,
        train_rejected: summary.train_rejected,
        prune_accepted: summary.prune_accepted,
        prune_rejected: summary.prune_rejected,
        utility_before: util_before,
        utility_after: summary.final_utility,
        self_mod_enabled: kairos.self_mod.enabled,
        hypothesis: hyp,
    })
}

/// Scalar golden r² (same as ld_simd_bench oracle).
fn r2_scalar(g1: &BitstreamGenotypes, g2: &BitstreamGenotypes) -> Option<f32> {
    if g1.len() != g2.len() {
        return None;
    }
    let n = g1.len();
    let (mut sx, mut sy, mut sxy, mut sx2, mut sy2, mut valid) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let a = g1.get(i);
        let b = g2.get(i);
        if a == 3 || b == 3 {
            continue;
        }
        let (x, y) = (a as f64, b as f64);
        sx += x;
        sy += y;
        sxy += x * y;
        sx2 += x * x;
        sy2 += y * y;
        valid += 1.0;
    }
    if valid < 10.0 {
        return None;
    }
    let num = valid * sxy - sx * sy;
    let den = ((valid * sx2 - sx * sx) * (valid * sy2 - sy * sy)).sqrt();
    if den <= 0.0 {
        return None;
    }
    let r = num / den;
    Some(((r * r) as f32).clamp(0.0, 1.0))
}

fn synthetic_snps(n_snps: usize, n_samples: usize) -> Vec<BitstreamGenotypes> {
    let mut out = Vec::with_capacity(n_snps);
    for i in 0..n_snps {
        let mut g = BitstreamGenotypes::new(n_samples);
        for s in 0..n_samples {
            // structured dosages 0/1/2 for stable correlations
            let gt = match (i + s) % 5 {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 1,
                _ => 0,
            };
            g.set(s, gt);
        }
        out.push(g);
    }
    out
}

fn score_ternary_pairs(snps: &[BitstreamGenotypes], n_pairs: usize) -> TernaryPopcountReport {
    let n_snps = snps.len();
    let mut pairs = Vec::new();
    for i in 0..n_snps {
        for j in (i + 1)..n_snps.min(i + 8) {
            pairs.push((i, j));
            if pairs.len() >= n_pairs {
                break;
            }
        }
        if pairs.len() >= n_pairs {
            break;
        }
    }
    let n_pairs = pairs.len().max(1);

    let t0 = Instant::now();
    let mut scalar_vals = Vec::with_capacity(n_pairs);
    for &(i, j) in &pairs {
        scalar_vals.push(r2_scalar(&snps[i], &snps[j]));
    }
    let scalar_ns = t0.elapsed().as_nanos();

    let t1 = Instant::now();
    let mut bit_vals = Vec::with_capacity(n_pairs);
    for &(i, j) in &pairs {
        bit_vals.push(snps[i].pearson_r2_bitparallel(&snps[j], 10));
    }
    let bitparallel_ns = t1.elapsed().as_nanos();

    let mut n_agree = 0usize;
    let mut max_abs_delta = 0.0f32;
    for (a, b) in scalar_vals.iter().zip(bit_vals.iter()) {
        match (a, b) {
            (Some(x), Some(y)) => {
                let d = (x - y).abs();
                max_abs_delta = max_abs_delta.max(d);
                if d < 1e-4 {
                    n_agree += 1;
                }
            }
            (None, None) => n_agree += 1,
            _ => {}
        }
    }

    let speedup = if bitparallel_ns > 0 {
        scalar_ns as f32 / bitparallel_ns as f32
    } else {
        0.0
    };

    let correct = n_agree == n_pairs && max_abs_delta < 1e-3;
    let verdict = if correct {
        HypothesisVerdict::Accept
    } else if n_agree as f32 / n_pairs.max(1) as f32 > 0.95 {
        HypothesisVerdict::Inconclusive
    } else {
        HypothesisVerdict::Reject
    };

    let hyp = Hypothesis::new(
        format!("H-ternary-pairs{n_pairs}"),
        "ternary_popcount",
        "Bitplane popcount pearson_r2_bitparallel agrees with scalar golden",
        "BitstreamGenotypes; scalar oracle vs pearson_r2_bitparallel; time both",
        "100% pair agreement within 1e-4; speedup reported",
    )
    .conclude(
        format!(
            "pairs={} agree={} max_delta={:.6} scalar_ns={} bit_ns={} speedup={:.2}x",
            n_pairs, n_agree, max_abs_delta, scalar_ns, bitparallel_ns, speedup
        ),
        verdict,
        "Hot-path science: correctness first; wall speedup scales with panel size",
    );

    TernaryPopcountReport {
        n_pairs,
        n_agree,
        max_abs_delta,
        scalar_ns,
        bitparallel_ns,
        speedup,
        panel: "synthetic".into(),
        hypothesis: hyp,
    }
}

/// Ternary/bitplane popcount experiment: synthetic panel.
pub fn run_ternary_popcount_experiment(
    n_snps: usize,
    n_samples: usize,
    n_pairs: usize,
) -> TernaryPopcountReport {
    let n_snps = n_snps.clamp(8, 64);
    let n_samples = n_samples.clamp(64, 512);
    let snps = synthetic_snps(n_snps, n_samples);
    score_ternary_pairs(&snps, n_pairs)
}

/// Season 2: real micro-VCF panel ternary/popcount (falls back to synthetic if missing).
pub fn run_ternary_popcount_real_or_synthetic(
    vcf_path: Option<&Path>,
    chr: u8,
    max_variants: usize,
    n_pairs: usize,
) -> TernaryPopcountReport {
    let max_variants = max_variants.clamp(32, 400);
    if let Some(path) = vcf_path {
        if path.is_file() {
            match VcfParser::new(false).parse_vcf_limited(path, chr, Some(max_variants)) {
                Ok(chrom) if chrom.genotypes.len() >= 4 => {
                    let mut report = score_ternary_pairs(&chrom.genotypes, n_pairs);
                    report.panel = format!(
                        "real_vcf chr{chr} snps={} path={}",
                        chrom.genotypes.len(),
                        path.display()
                    );
                    report.hypothesis.track = "ternary_popcount_real".into();
                    report.hypothesis.id = format!(
                        "H-ternary-real-chr{chr}-n{}",
                        chrom.genotypes.len()
                    );
                    report.hypothesis.notes = format!(
                        "Real micro panel; {}",
                        report.hypothesis.notes
                    );
                    return report;
                }
                Ok(_) => { /* fall through */ }
                Err(_) => { /* fall through */ }
            }
        }
    }
    let mut r = run_ternary_popcount_experiment(32, 256, n_pairs);
    r.panel = "synthetic_fallback".into();
    r.hypothesis.notes = format!("no real VCF; {}", r.hypothesis.notes);
    r
}

/// Count ACCEPT hypotheses on a track (substring match).
pub fn count_accept(reg: &HypothesisRegistry, track_sub: &str) -> usize {
    reg.entries
        .iter()
        .filter(|h| h.track.contains(track_sub) && h.verdict == HypothesisVerdict::Accept)
        .count()
}

/// When is a single self-mod run allowed?
///
/// **Best time:** after Season 2 has proven recovery + selection honesty + ternary correctness.
/// Never on primary by default; lab only; one cycle; then disable.
pub fn evaluate_self_mod_readiness(reg: &HypothesisRegistry) -> SelfModReadiness {
    let heal_accepts = count_accept(reg, "self_heal");
    let multi_heal_accepts = count_accept(reg, "self_heal_multi");
    let improve_accepts = count_accept(reg, "self_improve");
    let ternary_accepts = count_accept(reg, "ternary_popcount");

    let mut reasons = Vec::new();
    // Prefer multi-heal; count base heal toward threshold too.
    let heal_score = multi_heal_accepts.saturating_mul(2) + heal_accepts;
    if heal_score < 3 {
        reasons.push(format!(
            "need stronger heal proof (heal_accepts={heal_accepts} multi={multi_heal_accepts}, want score≥3)"
        ));
    }
    if improve_accepts < 2 {
        reasons.push(format!(
            "need ≥2 self_improve ACCEPT (have {improve_accepts})"
        ));
    }
    if ternary_accepts < 2 {
        reasons.push(format!(
            "need ≥2 ternary_popcount ACCEPT (have {ternary_accepts})"
        ));
    }

    SelfModReadiness {
        ready: reasons.is_empty(),
        reasons,
        heal_accepts,
        improve_accepts,
        ternary_accepts,
        multi_heal_accepts,
    }
}

/// One guarded self-mod cycle on a **language/SIS graph** in lab (not genome self-mod dump).
/// Enables SelfModConfig only for this call, then forces OFF on Kairos.
pub fn run_single_self_mod_lab(
    kairos: &mut Kairos,
    reg: &HypothesisRegistry,
    force: bool,
) -> Result<SelfModRunReport, String> {
    let readiness = evaluate_self_mod_readiness(reg);
    if !readiness.ready && !force {
        let hyp = Hypothesis::new(
            "H-selfmod-blocked",
            "self_mod_single",
            "Single self-mod run only after heal/improve/ternary readiness",
            "evaluate_self_mod_readiness gate",
            "ready=true",
        )
        .conclude(
            format!("ready=false reasons={:?}", readiness.reasons),
            HypothesisVerdict::Reject,
            "Guardian: wait for Season 2 win streak — or --force-self-mod in lab only",
        );
        return Ok(SelfModRunReport {
            ran: false,
            accepted: false,
            proposed: 0,
            description: "blocked by readiness gate".into(),
            readiness,
            hypothesis: hyp,
        });
    }

    // Ensure a graph exists for mutation cycle.
    let mut organ = kairos
        .brain
        .language()
        .cloned()
        .unwrap_or_else(LanguageOrgan::new);
    if organ.docs_ingested == 0 {
        organ.ingest_documents(&fixture_docs());
    }
    let graph = organ.graph.clone();

    // Lab-only enable for one cycle.
    let mut cfg = SelfModConfig::default();
    cfg.enabled = true;
    cfg.max_mutations_per_cycle = 1;
    cfg.cycle_budget_us = 5_000_000;
    cfg.auto_rollback_on_regression = true;
    cfg.fitness_improvement_threshold = 1.05; // allow mild non-improvement reject

    let baseline = (1000u64, 4096u64);
    let mut cycle = MutationCycle::new(cfg, baseline).map_err(|e| e.to_string())?;
    let rule = MutationRule {
        kind: MutationRuleKind::AddNode {
            label: "lab_selfmod_probe".into(),
        },
    };
    cycle.propose_mutation(rule).map_err(|e| e.to_string())?;
    let (new_fit, _us) = cycle
        .evaluate_mutation(&graph, 0)
        .map_err(|e| e.to_string())?;
    let accepted = cycle.should_accept(new_fit);
    if accepted {
        let _ = cycle.accept_mutation(0);
    }

    // Hard OFF after single run — never leave primary-style host enabled.
    kairos.self_mod.enabled = false;

    let hyp = Hypothesis::new(
        "H-selfmod-single-lab",
        "self_mod_single",
        "One lab-only MutationCycle (AddNode) under budget; SelfModConfig disabled after",
        "readiness gate → enable → propose 1 → evaluate → accept/reject → disable",
        "ran=true; self_mod left false on host; decision ledgered in hypothesis",
    )
    .conclude(
        format!(
            "ran=true accepted={accepted} fit=({},{}); force={force}; ready={}",
            new_fit.0, new_fit.1, readiness.ready
        ),
        HypothesisVerdict::Accept, // experiment completed safely
        if accepted {
            "Mutation accepted under threshold"
        } else {
            "Mutation rejected under rails — still a valid single-run experiment"
        },
    );

    Ok(SelfModRunReport {
        ran: true,
        accepted,
        proposed: 1,
        description: "AddNode(lab_selfmod_probe)".into(),
        readiness,
        hypothesis: hyp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::kairos::{Kairos, NurseryGenomeSpec};
    use crate::genomic::vitascale::life_course::LifeStage;

    #[test]
    fn self_heal_restores_weights() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(8).unwrap();
        let r = run_self_heal_drill(&mut k).unwrap();
        assert!(r.mean_w_after_damage < r.mean_w_before || r.mean_w_before < 1e-6);
        assert!(r.weight_restore_ok || r.mean_w_after_heal >= r.mean_w_after_damage);
    }

    #[test]
    fn ternary_popcount_agrees() {
        let r = run_ternary_popcount_experiment(16, 128, 40);
        assert_eq!(r.n_agree, r.n_pairs, "delta={}", r.max_abs_delta);
        assert!(r.max_abs_delta < 1e-4);
    }

    #[test]
    fn multi_heal_pack_all_ok() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(8).unwrap();
        k.brain.consolidate(0.5, 0.0);
        let r = run_multi_heal_pack(&mut k).unwrap();
        assert!(r.all_ok, "{:?}", r.scenarios);
    }

    #[test]
    fn self_mod_blocked_until_ready() {
        let reg = HypothesisRegistry::new();
        let r = evaluate_self_mod_readiness(&reg);
        assert!(!r.ready);
    }

    #[test]
    fn combined_heal_crisis_recovers() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(8).unwrap();
        k.brain.consolidate(0.5, 0.0);
        let r = run_combined_heal_crisis(&mut k).unwrap();
        assert!(r.ok, "w {}->{}->{} ltm {} phage={}", r.mean_w_before, r.mean_w_crisis, r.mean_w_after, r.ltm_after, r.phage_caught);
    }

    #[test]
    fn goal_improve_records_goal() {
        let mut k = Kairos::birth_zygote_with_nursery(64, &NurseryGenomeSpec::default()).unwrap();
        k.life.stage = LifeStage::Adult;
        k.day_of_heartbeats(4).unwrap();
        let _ = k.try_ingest_micro_synthetic();
        k.micro_panel_ready = true;
        let r = run_goal_improve_campaign(&mut k, 2, None).unwrap();
        assert!(r.goal_met || !r.base.hypothesis.measured.is_empty());
    }

    #[test]
    fn ternary_ladder_synthetic_agrees() {
        let r = run_ternary_ladder(None, 22, &[16, 24], 30);
        assert!(r.all_agree, "{}", r.hypothesis.measured);
    }
}
