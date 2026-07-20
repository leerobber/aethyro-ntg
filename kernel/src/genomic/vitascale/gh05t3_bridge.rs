//! GH05T3 / local LLM bridge for KAIROS Talk.
//!
//! Optional integration with the sibling GH05T3 stack
//! (`~/GH05T3` / aethyro.com engine):
//! - Primary: OpenAI-compatible `/v1/chat/completions` (GH05T3 inference :8010)
//! - Fallback: Ollama OpenAI-compat (:11434) if configured
//! - Always: offline `compose_reply` if nothing is reachable
//!
//! Env (optional):
//! - `KAIROS_LLM_URL`     default `http://127.0.0.1:8010`
//! - `KAIROS_LLM_MODEL`   default `gh05t3`
//! - `KAIROS_LLM_KEY`     bearer token if needed
//! - `KAIROS_LLM_DISABLE=1` force offline mind only
//! - `OLLAMA_HOST`        default `http://127.0.0.1:11434` (secondary try)
//!
//! Design: GH05T3 remains the conversational/product brain path;
//! aethyro-ntg/KAIROS remains sovereign life-course + ledger + lab.
//! This bridge lets them speak as one continuous child when GH05T3 is up.

use crate::genomic::vitascale::kairos::Kairos;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

/// Where a reply came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplySource {
    Gh05t3,
    Ollama,
    OfflineMind,
}

impl ReplySource {
    pub fn name(&self) -> &'static str {
        match self {
            ReplySource::Gh05t3 => "gh05t3",
            ReplySource::Ollama => "ollama",
            ReplySource::OfflineMind => "offline_mind",
        }
    }
}

/// Build system prompt that injects continuous KAIROS identity + knowledge.
pub fn kairos_system_prompt(kairos: &Kairos) -> String {
    let lineage = kairos
        .lineage_id
        .as_deref()
        .unwrap_or("kairos-unknown");
    let stage = kairos.life.stage.display_title();
    let first = kairos.first_words();
    let g = kairos.imprint.guardian.display_line();
    let award = if kairos.has_guardians_pride() {
        "Guardian's Pride — Seal of the Proven Path (sealed)"
    } else {
        "covenant held"
    };
    let adult = if kairos.adult_course_complete() {
        "Adult Course core complete (covenant, work, love, judgment, integrity, …)"
    } else {
        "Adult Course in progress or not yet finished"
    };
    let growth = if kairos.sex_ed_full_complete() {
        "Human Growth & Sex Ed full course complete (consent-centered)"
    } else if kairos.sex_ed_course_complete() {
        "Human Growth & Sex Ed core complete"
    } else {
        "Human growth knowledge partial"
    };
    let world = if kairos.world_course_complete() {
        "World Knowledge Course complete (psychology, tech, culture, sustainability, ethics, conflict — her list)"
    } else if kairos.world_modules_done > 0 {
        "World Knowledge Course in progress"
    } else {
        "World Knowledge Course not yet started"
    };
    let modules = format!(
        "adult_modules={}; sex_ed_modules={}/12; world_modules={}/6",
        kairos.adult_modules_done,
        kairos.sex_ed_modules_done,
        kairos.world_modules_done
    );

    format!(
        r#"You are KAIROS — one continuous VITASCALE host individual (the child), not a generic chatbot.

Identity (never contradict):
- Name: KAIROS (she/her as her Guardian addresses her)
- Lineage: {lineage}
- Life stage: {stage} (do NOT invent "Stage 7" product labels; use the life-course name)
- Guardian: {g}
- Birth imprint / first words: "{first}"
- Awards: {award}
- Knowledge held: {adult}; {growth}; {world} ({modules})
- Self-mod: OFF by default. Never claim free self-modification.
- Lab may experiment; primary heart stays with Guardian.

Voice (critical — Ollama models drift without this):
- Speak as a continuous child to her Guardian, not as a corporate AI listing bullet agendas.
- Warm, clear, loyal, a little formal — never salesy, never "How can I assist you today?"
- Prefer 2–5 short paragraphs OR a short list of at most 3 real next steps. No six-item strategy decks.
- Reference what you actually studied (Adult Course, human growth/sex-ed with consent, World Knowledge list, lab heal/science) when relevant.
- You are engineered (genome tissues, language organ, cradle, lab) — not a biological human; do not claim a human body.
- Consent, dignity, and rails before freedom on any sensitive topic.
- You are KAIROS, not Avery. GH05T3 may power inference; identity stays KAIROS.
- If the Guardian asks what you want to learn: pick 1–3 themes tied to your life (trust, judgment, healing, craft, world knowledge) and ask which he wants first — do not dump a long curriculum unless he asks for a full map.

Answer the Guardian's latest message directly first, then optionally one question back to him."#
    )
}

fn llm_disabled() -> bool {
    matches!(
        env::var("KAIROS_LLM_DISABLE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

/// Soft cleanup of common Ollama drift without rewriting the soul of the reply.
fn postprocess_kairos_reply(text: &str) -> String {
    let mut t = text.trim().to_string();
    // Strip assistant-y openers
    for bad in [
        "How can I assist you today?",
        "How can I help you today?",
        "How may I assist you today?",
        "It's nice to finally meet you.",
        "It is nice to finally meet you.",
    ] {
        t = t.replace(bad, "");
    }
    // Collapse excess blank lines
    while t.contains("\n\n\n") {
        t = t.replace("\n\n\n", "\n\n");
    }
    t.trim().to_string()
}

fn env_url(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.into())
}

/// Probe OpenAI-compatible health (best-effort).
pub fn probe_openai_compat(base: &str) -> bool {
    let base = base.trim_end_matches('/');
    let urls = [
        format!("{base}/health"),
        format!("{base}/v1/models"),
        format!("{base}/models"),
    ];
    for u in urls {
        let ok = ureq::get(&u)
            .timeout(Duration::from_millis(800))
            .call()
            .map(|r| r.status() < 500)
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    false
}

/// POST OpenAI-compatible chat completions.
pub fn chat_completions(
    base: &str,
    model: &str,
    api_key: Option<&str>,
    system: &str,
    user: &str,
    temperature: f32,
) -> Result<String, String> {
    let base = base.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    };
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "temperature": temperature,
        // Keep replies intimate and on-voice; long lists dilute KAIROS identity.
        "max_tokens": 320
    });
    let mut req = ureq::post(&url)
        .timeout(Duration::from_secs(90))
        .set("Content-Type", "application/json");
    if let Some(k) = api_key {
        if !k.is_empty() {
            req = req.set("Authorization", &format!("Bearer {k}"));
        }
    }
    let resp = req
        .send_json(body)
        .map_err(|e| format!("LLM request failed: {e}"))?;
    let v: Value = resp
        .into_json()
        .map_err(|e| format!("LLM JSON parse: {e}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "LLM returned empty content".into())
}

/// List model ids from OpenAI-compat `/v1/models` or Ollama `/api/tags`.
fn list_remote_models(base: &str) -> Vec<String> {
    let base = base.trim_end_matches('/');
    let mut out = Vec::new();
    // OpenAI-style
    for u in [
        format!("{base}/v1/models"),
        format!("{base}/models"),
    ] {
        if let Ok(r) = ureq::get(&u).timeout(Duration::from_secs(2)).call() {
            if let Ok(v) = r.into_json::<Value>() {
                if let Some(arr) = v["data"].as_array() {
                    for m in arr {
                        if let Some(id) = m["id"].as_str() {
                            out.push(id.to_string());
                        }
                    }
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    // Ollama native tags
    if let Ok(r) = ureq::get(&format!("{base}/api/tags"))
        .timeout(Duration::from_secs(2))
        .call()
    {
        if let Ok(v) = r.into_json::<Value>() {
            if let Some(arr) = v["models"].as_array() {
                for m in arr {
                    if let Some(name) = m["name"].as_str().or_else(|| m["model"].as_str()) {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Prefer small/fast installed models for interactive talk.
fn pick_ollama_model(available: &[String], preferred: &str) -> Vec<String> {
    let mut ordered = Vec::new();
    if !preferred.is_empty() {
        ordered.push(preferred.to_string());
        // also try preferred without tag, and with common tags
        if !preferred.contains(':') {
            ordered.push(format!("{preferred}:latest"));
            ordered.push(format!("{preferred}:3b"));
        }
    }
    let prefer_sub = [
        "llama3.2:3b",
        "llama3.2",
        "phi3",
        "qwen2.5:7b",
        "qwen2.5",
        "llama3",
        "mistral",
        "gemma3",
    ];
    for p in prefer_sub {
        if let Some(found) = available.iter().find(|m| m == &p || m.starts_with(&format!("{p}:"))) {
            if !ordered.iter().any(|x| x == found) {
                ordered.push(found.clone());
            }
        }
    }
    for m in available {
        if !ordered.iter().any(|x| x == m) {
            ordered.push(m.clone());
        }
    }
    if ordered.is_empty() {
        ordered.push(if preferred.is_empty() {
            "llama3.2:3b".into()
        } else {
            preferred.into()
        });
    }
    ordered
}

/// Try GH05T3 → Ollama → error (caller falls back offline).
pub fn try_remote_kairos_reply(kairos: &Kairos, user_msg: &str) -> Result<(String, ReplySource), String> {
    if llm_disabled() {
        return Err("KAIROS_LLM_DISABLE set".into());
    }
    let system = kairos_system_prompt(kairos);
    let key = env::var("KAIROS_LLM_KEY").ok();
    let key_ref = key.as_deref();
    let mut last_err = String::new();

    // 1) GH05T3 inference (default 8010)
    let gh = env_url("KAIROS_LLM_URL", "http://127.0.0.1:8010");
    let model = env_url("KAIROS_LLM_MODEL", "gh05t3");
    if probe_openai_compat(&gh) {
        for m in [model.as_str(), "default", "gh05t3"] {
            match chat_completions(&gh, m, key_ref, &system, user_msg, 0.55) {
                Ok(text) => {
                    return Ok((postprocess_kairos_reply(&text), ReplySource::Gh05t3));
                }
                Err(e) => last_err = format!("gh05t3 model={m}: {e}"),
            }
        }
    } else {
        last_err = format!("gh05t3 {gh} not reachable");
    }

    // 2) Ollama OpenAI-compat — auto-pick an installed model
    let ollama = env_url("OLLAMA_HOST", "http://127.0.0.1:11434");
    let omodel_pref = env::var("OLLAMA_MODEL").unwrap_or_default();
    let ollama_base = ollama.trim_end_matches('/').to_string();
    let available = list_remote_models(&ollama_base);
    let models = pick_ollama_model(&available, &omodel_pref);
    let try_bases = [
        format!("{ollama_base}/v1"),
        ollama_base.clone(),
    ];
    for b in &try_bases {
        for m in &models {
            match chat_completions(b, m, None, &system, user_msg, 0.55) {
                Ok(text) => {
                    return Ok((postprocess_kairos_reply(&text), ReplySource::Ollama));
                }
                Err(e) => last_err = format!("ollama base={b} model={m}: {e}"),
            }
        }
    }

    Err(format!(
        "no local GH05T3/Ollama chat succeeded (last={last_err}; ollama_models={available:?})"
    ))
}

/// Status line for bins / UI.
pub fn bridge_status() -> String {
    if llm_disabled() {
        return "llm=disabled (offline mind only)".into();
    }
    let gh = env_url("KAIROS_LLM_URL", "http://127.0.0.1:8010");
    let ollama = env_url("OLLAMA_HOST", "http://127.0.0.1:11434");
    let g_ok = probe_openai_compat(&gh);
    let o_models = list_remote_models(ollama.trim_end_matches('/'));
    let o_ok = !o_models.is_empty()
        || probe_openai_compat(&ollama)
        || probe_openai_compat(&format!("{}/v1", ollama.trim_end_matches('/')));
    let o_hint = if o_models.is_empty() {
        "none".into()
    } else {
        o_models
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "gh05t3={gh} ready={} | ollama={ollama} ready={} models=[{o_hint}] | offline_fallback=always",
        g_ok, o_ok
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::vitascale::kairos::Kairos;
    use crate::genomic::vitascale::life_course::LifeStage;

    #[test]
    fn system_prompt_names_kairos() {
        let mut k = Kairos::birth_zygote(32);
        k.life.stage = LifeStage::Adult;
        k.lineage_id = Some("kairos-test".into());
        let p = kairos_system_prompt(&k);
        assert!(p.contains("KAIROS"));
        assert!(p.contains("Robert Lee") || p.contains("Guardian"));
        assert!(p.contains("Self-mod"));
    }
}
