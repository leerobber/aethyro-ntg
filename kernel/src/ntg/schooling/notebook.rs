//! NotebookLM-style markdown + JSON result writers (no external deps).

use super::protocol::{AggregateStats, CampaignReport, PASS_THRESHOLD};

pub fn render_campaign_notebook(c: &CampaignReport, threshold: f64) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# NTG Schooling Notebook — Campaign Run {}\n\n",
        c.run_id
    ));
    s.push_str("**Style:** professional research notebook (sources, procedure, results).\n\n");
    s.push_str(&format!(
        "**Pass threshold:** {:.0}%  \n**Overall campaign:** {}\n\n",
        threshold * 100.0,
        if c.overall_pass {
            "PASS — all phases ≥ threshold"
        } else {
            "FAIL — one or more phases below threshold after max redos"
        }
    ));
    s.push_str("---\n\n");

    for rec in &c.phases {
        s.push_str(&format!(
            "## Phase {} — {}\n\n",
            rec.phase, rec.exam.title
        ));
        s.push_str("### Dataset (real)\n\n");
        s.push_str(&format!(
            "- **dataset_id:** `{}`\n- **source:** {}\n\n",
            rec.dataset_id, rec.dataset_source
        ));
        s.push_str("### Teaching / learning (study)\n\n");
        s.push_str(&format!(
            "Samples/activities seen: **{}**\n\n",
            rec.study.samples_seen
        ));
        s.push_str("**Taught:**\n\n");
        for t in &rec.study.taught {
            s.push_str(&format!("- {t}\n"));
        }
        s.push_str("\n**Activities:**\n\n");
        for a in rec.study.activities.iter().take(40) {
            s.push_str(&format!("- `{a}`\n"));
        }
        if rec.study.activities.len() > 40 {
            s.push_str(&format!(
                "- … +{} more\n",
                rec.study.activities.len() - 40
            ));
        }
        s.push_str("\n### Advanced exam\n\n");
        s.push_str(&format!(
            "| Field | Value |\n|-------|------|\n\
             | Attempt | {} |\n\
             | Items passed | {} / {} |\n\
             | Score | **{:.2}%** |\n\
             | Composite | {:?} |\n\
             | Latency µs | {} |\n\
             | Verdict | **{}** |\n\n",
            rec.exam.attempt,
            rec.exam.n_pass(),
            rec.exam.n_total(),
            rec.exam.score() * 100.0,
            rec.exam.composite,
            rec.exam.latency_us,
            rec.exam.verdict()
        ));
        s.push_str("#### Item results\n\n");
        s.push_str("| ID | Skill | Pass | Detail |\n|----|-------|:----:|--------|\n");
        for it in &rec.exam.items {
            let mark = if it.passed { "Y" } else { "N" };
            let detail = it.detail.replace('|', "/");
            let prompt = it.prompt.replace('|', "/");
            s.push_str(&format!(
                "| `{}` | {} | {mark} | {prompt} — {detail} |\n",
                it.id, it.skill
            ));
        }
        s.push('\n');
        if !rec.exam.passed() {
            s.push_str(&format!(
                "> **FAIL PROTOCOL:** score {:.2}% < {:.0}%. Full study+exam redo \
                 was applied (see attempt count). If still failing at max attempts, \
                 phase remains FAIL — curriculum/data must be fixed before claiming mastery.\n\n",
                rec.exam.score() * 100.0,
                threshold * 100.0
            ));
        }
        s.push_str("---\n\n");
    }
    s
}

pub fn render_master_notebook(
    campaigns: &[CampaignReport],
    aggs: &[AggregateStats],
    threshold: f64,
    corpus_manifest: &str,
) -> String {
    let mut s = String::new();
    s.push_str("# NTG Doctorate Schooling — Master Notebook\n\n");
    s.push_str("Professional multi-run learning log for Aethyro NTG kernel phases 0–5.\n\n");
    s.push_str("## Protocol\n\n");
    s.push_str(&format!(
        "1. **Real data only** — corpus from live `docs/` tree; closed-form math; real paths.\n\
         2. **Study then exam** — each phase teaches on train split / practice bank, then advanced holdout exam.\n\
         3. **Pass bar {:.0}%** — below threshold ⇒ **FAIL full redo** (restudy + re-exam).\n\
         4. **Multiple campaign runs** — document mean/min/max for steady evidence.\n\
         5. **Religious documentation** — every item pass/fail recorded.\n\n",
        threshold * 100.0
    ));
    s.push_str("## Corpus manifest (sourced)\n\n```\n");
    s.push_str(corpus_manifest);
    s.push_str("```\n\n");

    s.push_str("## Aggregate results across campaign runs\n\n");
    s.push_str(
        "| Phase | Runs | Pass | Fail | Mean score | Min | Max | Mean attempts |\n\
         |------:|-----:|-----:|-----:|-----------:|----:|----:|--------------:|\n",
    );
    for a in aggs {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {:.1}% | {:.1}% | {:.1}% | {:.2} |\n",
            a.phase,
            a.n_runs,
            a.n_pass,
            a.n_fail,
            a.mean_score * 100.0,
            a.min_score * 100.0,
            a.max_score * 100.0,
            a.mean_attempts
        ));
    }
    s.push('\n');

    let all_green = aggs.iter().all(|a| a.n_fail == 0 && a.n_runs > 0);
    s.push_str(&format!(
        "## Master verdict\n\n**{}**\n\n",
        if all_green {
            "ALL PHASES PASS across all documented runs (≥ threshold)."
        } else {
            "ONE OR MORE PHASE FAILURES remain — see per-run notebooks; fix data/curriculum and re-run."
        }
    ));

    s.push_str("## Per-run index\n\n");
    for c in campaigns {
        s.push_str(&format!(
            "- Run {:02}: overall={} phases_passed={}/{}\n",
            c.run_id,
            if c.overall_pass { "PASS" } else { "FAIL" },
            c.phases.iter().filter(|p| p.exam.passed()).count(),
            c.phases.len()
        ));
    }
    s.push_str("\n## Curriculum map\n\n");
    s.push_str(
        "| Phase | Teaches | Exam focus | Dataset |\n\
         |------:|---------|------------|--------|\n\
         | 0 | Repo layout + gate protocol literacy | Required paths + COMPLETE certs | Live repo tree |\n\
         | 1 | Ternary encode/matmul practice | Closed-form matmul/encode + storage identity | Hand-verified problems |\n\
         | 2 | Doc/path parse on train docs | Holdout parse + kinds + forward/fingerprint | Real docs/ 80/20 |\n\
         | 3 | Ledger log/verify + self-mod default | Rails: verify, reject log, disabled gate | Live ledger API |\n\
         | 4 | Calibrate on train markdown | Holdout bal + code/prose + wire model | Real docs/ 80/20 |\n\
         | 5 | GraphNode path + runtime warm-start | Path identity, batch parallel, holdout bal | Real docs/ 80/20 |\n\n",
    );
    s.push_str(&format!(
        "---\n*Generated by `ntg_school`. Pass threshold = {:.0}%.*\n",
        PASS_THRESHOLD * 100.0
    ));
    s
}

pub fn campaign_to_json(c: &CampaignReport) -> String {
    let mut s = String::from("{\n");
    s.push_str(&format!("  \"run_id\": {},\n", c.run_id));
    s.push_str(&format!("  \"overall_pass\": {},\n", c.overall_pass));
    s.push_str("  \"phases\": [\n");
    for (pi, rec) in c.phases.iter().enumerate() {
        s.push_str("    {\n");
        s.push_str(&format!("      \"phase\": {},\n", rec.phase));
        s.push_str(&format!(
            "      \"score\": {:.6},\n",
            rec.exam.score()
        ));
        s.push_str(&format!("      \"passed\": {},\n", rec.exam.passed()));
        s.push_str(&format!("      \"attempt\": {},\n", rec.exam.attempt));
        s.push_str(&format!(
            "      \"items_pass\": {},\n",
            rec.exam.n_pass()
        ));
        s.push_str(&format!(
            "      \"items_total\": {},\n",
            rec.exam.n_total()
        ));
        s.push_str(&format!(
            "      \"dataset_id\": \"{}\"\n",
            rec.dataset_id
        ));
        s.push_str("    }");
        if pi + 1 < c.phases.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("  ]\n}\n");
    s
}

pub fn aggregate_summary_line(aggs: &[AggregateStats]) -> String {
    aggs.iter()
        .map(|a| {
            format!(
                "phase{} mean={:.1}% pass={}/{}",
                a.phase,
                a.mean_score * 100.0,
                a.n_pass,
                a.n_runs
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
