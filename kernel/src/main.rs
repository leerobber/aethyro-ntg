//! NanoKeymaster — sovereign routing agent that IS the API key.
//!
//! Reads newline-delimited JSON from stdin, one object per line:
//!   {"intent": "classify", "payload": {"text": "fn main() {}"}}
//!   {"intent": "score",    "payload": {"text": "some content"}}
//!   {"intent": "_status"}          → memory + ledger summary
//!   {"intent": "_verify_ledger"}   → tamper-evidence check
//!   {"intent": "_quad_brain"}      → Quad-Brain (α/β/γ/δ) state; it observes every routed call
//!   {"intent": "_hierarchy", "payload": {"nano": 10000}}
//!                                  → build + tick an agent hierarchy (nano capped at 500000)
//!   {"intent": "_quit"}            → clean exit
//!
//! Writes one JSON object per line to stdout.
//!
//! Environment variables:
//!   KEYMASTER_MODEL=<path>        load a frozen CalibModel (.calib) instead of training from fixtures
//!   KEYMASTER_BACKEND_URL=<url>   external backend for intents the local brain does not answer: any HTTP
//!                                 service accepting POST {"intent","payload"} and returning JSON, called at
//!                                 exactly this URL, 10 s timeout. Errors come back as a result payload, with no
//!                                 local fallback. NOT backend/keymaster (docs/architecture/0012-keymaster-contract.md)
//!
//! Run:
//!   cargo run --release --bin kernel_host
//!   KEYMASTER_MODEL=/tmp/ntg.calib cargo run --release --bin kernel_host
//!   echo '{"intent":"classify","payload":{"text":"fn main(){}"}}' | cargo run --release --bin kernel_host

use ntg_kernel::ntg::calib::{calibrate, fixture_documents, samples_from_documents, CalibModel};
use ntg_kernel::ntg::ledger::{FitnessMeasure, MutationOutcome, TamperEvidentLedger};
use ntg_kernel::genomic::vitascale::agent_hierarchy::AgentHierarchy;
use ntg_kernel::ntg::ledger::replay::ExecutionTrace;
use ntg_kernel::ntg::mutation::{AgentId, AgentLevel, BehavioralSignature, QuadBrainAgent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::time::{Duration, Instant};

/// Maximum number of distinct intent keys tracked in the memory map.
/// Overflow intents are aggregated into the "_other" bucket to bound memory.
const MAX_TRACKED_INTENTS: usize = 256;
/// Intent strings longer than this are treated as "_other" to prevent key-bloat.
const MAX_INTENT_KEY_LEN: usize = 64;
/// HTTP timeout for external backend calls.
const EXTERNAL_TIMEOUT: Duration = Duration::from_secs(10);

// ── routing decision ────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Backend {
    /// Local kernel, confidence above threshold.
    Local,
    /// Local kernel, confidence below threshold (external unavailable or unset).
    LocalFallback,
    /// External API selected — POSTs intent+payload as JSON.
    External,
}

impl Backend {
    fn label(&self) -> &'static str {
        match self {
            Backend::Local => "local",
            Backend::LocalFallback => "local_fallback",
            Backend::External => "external",
        }
    }
}

// ── memory (per-intent call stats) ────────────────────────────────────────────────────

#[derive(Default)]
struct IntentStats {
    calls: u64,
    local_count: u64,
    fallback_count: u64,
    sum_raw_score: i64,
}

impl IntentStats {
    fn avg_score(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.sum_raw_score as f64 / self.calls as f64
        }
    }

    fn local_rate(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.local_count as f64 / self.calls as f64
        }
    }
}

// ── the nano-agent ────────────────────────────────────────────────────────────────────────────────

struct NanoKeymaster {
    model: CalibModel,
    ledger: TamperEvidentLedger,
    /// Per-intent call statistics (the memory layer).
    memory: HashMap<String, IntentStats>,
    /// Credential vault: URL never echoed in any response or log line.
    external_url: Option<String>,
    call_counter: u64,
    /// Observes every routed call (policy-brain confidence, backend choice) and
    /// runs a full δ→α→β→γ cycle per call.
    quad: QuadBrainAgent,
}

impl NanoKeymaster {
    fn new(model: CalibModel, external_url: Option<String>) -> Self {
        Self {
            ledger: TamperEvidentLedger::new(None).expect("ledger init"),
            memory: HashMap::new(),
            external_url,
            call_counter: 0,
            model,
            quad: QuadBrainAgent::new(AgentId::new(1), AgentLevel::Super),
        }
    }

    /// Feed the Quad-Brain one observation of this call's real routing behaviour.
    fn observe_quad(&mut self, mem_key: &str, backend: &Backend, confidence: f64, call_id: u64) {
        let (calls, local): (u64, u64) = self
            .memory
            .values()
            .fold((0, 0), |(c, l), s| (c + s.calls, l + s.local_count));
        let strategy = format!("{}:{}", mem_key, backend.label());
        let bucket = (confidence * 8.0) as u64;
        let sig = BehavioralSignature {
            cycle: call_id,
            efficiency: confidence as f32,
            mutation_acceptance_rate: if calls == 0 { 0.0 } else { local as f32 / calls as f32 },
            behavior_hash: fnv1a(&strategy) ^ bucket.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            active_strategies: vec![strategy],
        };
        self.quad.advance_cycle(sig);
    }

    /// Current Quad-Brain state, all values read from the brains themselves.
    fn quad_report(&self) -> Value {
        let q = &self.quad;
        let r = q.report();
        let (stress, growth, repair, coordination) = q.brain_delta.get_hormone_levels();
        let strategies: Vec<Value> = q
            .channel
            .beta_to_gamma
            .strategy_scores
            .iter()
            .map(|(n, sharpe)| json!({"strategy": n, "sharpe": sharpe}))
            .collect();
        json!({
            "cycles": r.cycle,
            "observes": "one cycle per routed call (confidence = efficiency, backend choice = strategy)",
            "synchronized": r.synchronized,
            "conflicts": q.channel.conflict_count,
            "alpha": {
                "drifts_detected": r.alpha_drifts,
                "repairs_successful": r.alpha_repairs,
                "rollbacks": q.brain_alpha.rollbacks_triggered,
                "baseline_efficiency": q.brain_alpha.drift_detector.baseline_efficiency,
                "drift_score": r.drift_score,
                "consensus_threshold": r.consensus_threshold,
            },
            "beta": {
                "patterns_learned": q.brain_beta.pattern_discoveries,
                "strategies": strategies,
            },
            "gamma": {
                "policy_directives_issued": r.quad_metrics.policy_directives_issued,
                "policies_active": r.gamma_policies_active,
                "alignment": r.gamma_alignment,
            },
            "delta": {
                "regime": r.delta_regime,
                "hormones": {"stress": stress, "growth": growth, "repair": repair, "coordination": coordination},
                "forecasts": r.quad_metrics.forecasts_generated,
                "forecast_quality": r.forecast_quality,
            },
        })
    }

    /// Build and tick an agent hierarchy of the requested size, then drop it.
    /// Layout: 1 Super, 10 Sub, 100 Micro (10 per Sub), `nano` Nano spread over the Micros.
    fn hierarchy_report(nano: u64) -> Value {
        const MAX_NANO: u64 = 500_000;
        let nano = nano.clamp(1, MAX_NANO);
        let t0 = Instant::now();
        let mut h = AgentHierarchy::new();
        h.spawn_super();
        let built: Result<(), String> = (|| {
            h.spawn_sub(10)?;
            for sub in 0..10u32 {
                h.spawn_micro(10, sub)?;
            }
            let per_micro = (nano / 100) as u32;
            let remainder = (nano % 100) as u32;
            for micro in 0..100u32 {
                let count = per_micro + if micro == 0 { remainder } else { 0 };
                if count > 0 {
                    h.spawn_nano(count, micro)?;
                }
            }
            Ok(())
        })();
        if let Err(e) = built {
            return json!({"error": e});
        }
        let spawn_ms = t0.elapsed().as_secs_f64() * 1e3;
        let t1 = Instant::now();
        h.broadcast_tick(1);
        let tick_ms = t1.elapsed().as_secs_f64() * 1e3;
        let summary = h.summary();
        json!({
            "requested_nano": nano,
            "total_agents": h.total_agents(),
            "tiers": {
                "super": summary.get("Super").copied().unwrap_or(0),
                "sub": summary.get("Sub").copied().unwrap_or(0),
                "micro": summary.get("Micro").copied().unwrap_or(0),
                "nano": summary.get("Nano").copied().unwrap_or(0),
            },
            "spawn_ms": spawn_ms,
            "broadcast_tick_ms": tick_ms,
            "note": "built, ticked and dropped per request; not retained",
        })
    }

    fn external_configured(&self) -> bool {
        self.external_url.is_some()
    }

    /// The outer interface. Every call goes through:
    /// policy → route → act → log → update memory → respond.
    fn call(&mut self, intent: &str, payload: &Value, _context: &Value) -> Value {
        self.call_counter += 1;
        let call_id = self.call_counter;
        let t0 = Instant::now();

        // 1. Policy brain: score the intent+payload text with the ternary classifier.
        let text = extract_text(payload);
        let raw_score = self.model.score_label(&text);
        let confidence = normalize_confidence(raw_score, self.model.threshold);

        // 2. Routing brain: decide where this call goes.
        let backend = self.route(intent, raw_score);

        // 3. Act: execute against chosen backend.
        let result = self.execute(intent, payload, raw_score, &backend);

        let elapsed_us = t0.elapsed().as_micros() as u64;

        // 4. Log to tamper-evident ledger: every routing decision is hash-chained.
        let description = format!(
            "call_id={} intent={} backend={} score={} confidence={:.3}",
            call_id,
            intent,
            backend.label(),
            raw_score,
            confidence
        );
        let outcome = match backend {
            Backend::Local => MutationOutcome::Accepted,
            Backend::LocalFallback => MutationOutcome::RejectedFitnessGate,
            Backend::External => MutationOutcome::RejectedRegression,
        };
        let _ = self.ledger.log_mutation(
            description,
            fnv1a(intent),
            fnv1a(&result.to_string()),
            FitnessMeasure { latency_us: elapsed_us, memory_bytes: 0 },
            outcome,
            elapsed_us * 1000,
            ExecutionTrace::new(),
            call_id,
        );

        // 5. Update memory — capped to prevent unbounded growth from adversarial intent strings.
        let mem_key: &str =
            if intent.len() > MAX_INTENT_KEY_LEN
                || (self.memory.len() >= MAX_TRACKED_INTENTS && !self.memory.contains_key(intent))
            {
                "_other"
            } else {
                intent
            };
        let stats = self.memory.entry(mem_key.to_string()).or_default();
        stats.calls += 1;
        stats.sum_raw_score += raw_score;
        match backend {
            Backend::Local => stats.local_count += 1,
            Backend::LocalFallback => stats.fallback_count += 1,
            Backend::External => {}
        }

        // 5b. Quad-Brain observes this call (same key-capping as the memory layer).
        self.observe_quad(mem_key, &backend, confidence, call_id);

        // 6. Evolution hint: after every 10 calls, emit routing policy observations.
        if call_id.is_multiple_of(10) {
            self.emit_learning_update();
        }

        json!({
            "call_id": call_id,
            "backend": backend.label(),
            "confidence": confidence,
            "score_raw": raw_score,
            "latency_us": elapsed_us,
            "result": result,
        })
    }

    /// Routing brain: maps intent + confidence score to a backend.
    fn route(&self, intent: &str, score: i64) -> Backend {
        match intent {
            // Core kernel intents: handle locally if confident.
            "classify" | "score" | "predict" => {
                if score >= self.model.threshold {
                    Backend::Local
                } else if self.external_configured() {
                    Backend::External
                } else {
                    Backend::LocalFallback
                }
            }
            // Unknown intent: escalate externally if available.
            _ => {
                if self.external_configured() {
                    Backend::External
                } else {
                    Backend::LocalFallback
                }
            }
        }
    }

    fn execute(&self, intent: &str, payload: &Value, score: i64, backend: &Backend) -> Value {
        match (intent, backend) {
            ("classify" | "predict", Backend::Local) => {
                let accepted = score >= self.model.threshold;
                json!({
                    "prediction": accepted,
                    "label": if accepted { "exec" } else { "skip" },
                })
            }
            ("score", Backend::Local) => {
                json!({ "score": score, "threshold": self.model.threshold })
            }
            (_, Backend::External) => {
                // POST {"intent": ..., "payload": ...} to the external backend.
                // URL is never echoed — credential vault.
                let url = self
                    .external_url
                    .as_deref()
                    .expect("external_url set when Backend::External is chosen");
                let body = json!({ "intent": intent, "payload": payload });
                match ureq::post(url).timeout(EXTERNAL_TIMEOUT).send_json(&body) {
                    Ok(resp) => resp.into_json::<Value>().unwrap_or_else(|e| {
                        json!({
                            "error": format!("external backend non-JSON response: {e}"),
                            "external": true,
                        })
                    }),
                    Err(e) => json!({
                        "error": format!("external backend error: {e}"),
                        "external": true,
                    }),
                }
            }
            _ => {
                // LocalFallback: best-effort local response below confidence threshold.
                json!({
                    "score": score,
                    "note": "local_fallback — below confidence threshold",
                })
            }
        }
    }

    fn status(&self) -> Value {
        let mut intent_memory = serde_json::Map::new();
        for (intent, stats) in &self.memory {
            intent_memory.insert(
                intent.clone(),
                json!({
                    "calls": stats.calls,
                    "avg_score": stats.avg_score(),
                    "local_rate": stats.local_rate(),
                    "local_count": stats.local_count,
                    "fallback_count": stats.fallback_count,
                }),
            );
        }
        json!({
            "total_calls": self.call_counter,
            "ledger_entries": self.ledger.len(),
            "model_nonzero_weights": self.model.nonzero_count(),
            "model_threshold": self.model.threshold,
            "external_backend_configured": self.external_configured(),
            "intent_memory": intent_memory,
            "quad_brain_cycles": self.quad.cycle_count,
        })
    }

    /// Evolution rail: after N calls, report which intents are confidently local.
    fn emit_learning_update(&self) {
        let mut observations = Vec::new();
        for (intent, stats) in &self.memory {
            if stats.calls >= 3 && stats.local_rate() > 0.8 {
                observations.push(format!(
                    "  {} → {:.0}% local over {} calls (avg score {:.1})",
                    intent,
                    stats.local_rate() * 100.0,
                    stats.calls,
                    stats.avg_score()
                ));
            }
        }
        if !observations.is_empty() {
            eprintln!(
                "[keymaster:evolution] routing observations after {} calls:",
                self.call_counter
            );
            for obs in observations {
                eprintln!("{}", obs);
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────────────────────

fn extract_text(payload: &Value) -> String {
    payload
        .get("text")
        .or_else(|| payload.get("prompt"))
        .or_else(|| payload.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Map raw score relative to the decision boundary to [0.0, 1.0].
///
/// Centers on the threshold: score == threshold → 0.5; higher → toward 1.0.
/// Works correctly for both positive and negative thresholds.
fn normalize_confidence(score: i64, threshold: i64) -> f64 {
    let denom = threshold.abs().max(1) as f64;
    let ratio = (score - threshold) as f64 / denom;
    (0.5 + 0.5 * ratio).clamp(0.0, 1.0)
}

/// FNV-1a 64-bit hash — used to create stable fingerprints for ledger entries.
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1_099_511_628_211);
    }
    h
}

fn load_or_train_model() -> CalibModel {
    if let Ok(path_str) = std::env::var("KEYMASTER_MODEL") {
        let p = std::path::Path::new(&path_str);
        if p.exists() {
            match CalibModel::load_path(p) {
                Ok(m) => {
                    eprintln!("[keymaster] loaded model from {}", path_str);
                    return m;
                }
                Err(e) => eprintln!("[keymaster] model load error: {e} — training from fixtures"),
            }
        } else {
            eprintln!("[keymaster] KEYMASTER_MODEL path not found: {path_str} — training from fixtures");
        }
    }
    eprintln!("[keymaster] training model from fixture docs...");
    let docs = fixture_documents();
    let samples = samples_from_documents(&docs).expect("fixture samples");
    let report = calibrate(&samples, 50, 0).expect("calibrate");
    let model = CalibModel::from_report(&report);
    eprintln!(
        "[keymaster] model trained  nonzero={} threshold={}",
        model.nonzero_count(),
        model.threshold
    );
    model
}

// ── main loop ─────────────────────────────────────────────────────────────────────────────────

fn main() {
    let external_url = std::env::var("KEYMASTER_BACKEND_URL").ok();

    eprintln!("[keymaster] NanoKeymaster booting");
    let model = load_or_train_model();
    eprintln!(
        "[keymaster] ready  nonzero_weights={}  threshold={}",
        model.nonzero_count(),
        model.threshold
    );
    if external_url.is_some() {
        eprintln!("[keymaster] external backend configured (URL is credential-vaulted)");
    } else {
        eprintln!("[keymaster] external backend: none (local-only mode)");
    }
    eprintln!("[keymaster] accepting JSON on stdin — one object per line");
    eprintln!("[keymaster] example: {{\"intent\":\"classify\",\"payload\":{{\"text\":\"fn main() {{}}\"}}}}");

    let mut km = NanoKeymaster::new(model, external_url);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(out, "{}", json!({"error": format!("invalid JSON: {e}")}));
                let _ = out.flush();
                continue;
            }
        };

        let intent = req
            .get("intent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let response = match intent {
            "_quit" => {
                let goodbye = json!({
                    "status": "goodbye",
                    "total_calls": km.call_counter,
                    "ledger_entries": km.ledger.len(),
                });
                let _ = writeln!(out, "{}", goodbye);
                let _ = out.flush();
                break;
            }
            "_status" => km.status(),
            "_quad_brain" => km.quad_report(),
            "_hierarchy" => {
                let nano = req
                    .get("payload")
                    .and_then(|p| p.get("nano"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(10_000);
                NanoKeymaster::hierarchy_report(nano)
            }
            "_verify_ledger" => match km.ledger.verify_full_ledger() {
                Ok(()) => json!({"ledger_ok": true, "entries": km.ledger.len()}),
                Err(e) => json!({"ledger_ok": false, "error": e.to_string()}),
            },
            _ => {
                let payload = req
                    .get("payload")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));
                let context = req
                    .get("context")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));
                km.call(intent, &payload, &context)
            }
        };

        let _ = writeln!(out, "{}", response);
        let _ = out.flush();
    }

    // Final ledger verification on exit.
    if km.call_counter > 0 {
        match km.ledger.verify_full_ledger() {
            Ok(()) => eprintln!(
                "[keymaster] shutdown — ledger verified OK  entries={}",
                km.ledger.len()
            ),
            Err(e) => eprintln!("[keymaster] shutdown — ledger TAMPERED: {e}"),
        }
    }
}
