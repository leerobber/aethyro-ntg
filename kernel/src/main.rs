//! NanoKeymaster — sovereign routing agent that IS the API key.
//!
//! Reads newline-delimited JSON from stdin, one object per line:
//!   {"intent": "classify", "payload": {"text": "fn main() {}"}}
//!   {"intent": "score",    "payload": {"text": "some content"}}
//!   {"intent": "_status"}          → memory + ledger summary
//!   {"intent": "_verify_ledger"}   → tamper-evidence check
//!   {"intent": "_quit"}            → clean exit
//!
//! Writes one JSON object per line to stdout.
//!
//! Environment variables:
//!   KEYMASTER_MODEL=<path>        load a frozen CalibModel (.calib) instead of training from fixtures
//!   KEYMASTER_BACKEND_URL=<url>   external fallback backend (routing decision is made; HTTP not yet wired)
//!
//! Run:
//!   cargo run --release --bin kernel_host
//!   KEYMASTER_MODEL=/tmp/ntg.calib cargo run --release --bin kernel_host
//!   echo '{"intent":"classify","payload":{"text":"fn main(){}"}}' | cargo run --release --bin kernel_host

use ntg_kernel::ntg::calib::{calibrate, fixture_documents, samples_from_documents, CalibModel};
use ntg_kernel::ntg::ledger::{FitnessMeasure, MutationOutcome, TamperEvidentLedger};
use ntg_kernel::ntg::ledger::replay::ExecutionTrace;
use ntg_kernel::ntg::graph::IntentMemory;
use ntg_kernel::ntg::mutation::{LoopController, SelfModConfig, DegradationSignal};
use ntg_kernel::{HealthMonitor, TrendSignal};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::time::Instant;

// ── routing decision ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Backend {
    /// Local kernel, confidence above threshold.
    Local,
    /// Local kernel, confidence below threshold (external unavailable or unset).
    LocalFallback,
    /// External API selected — HTTP client not yet wired; routing decision logged.
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

// ── memory (per-intent call stats) ───────────────────────────────────────────

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

// ── the nano-agent ────────────────────────────────────────────────────────────

struct NanoKeymaster {
    model: CalibModel,
    ledger: TamperEvidentLedger,
    /// Per-intent call statistics (parametric memory layer).
    memory: HashMap<String, IntentStats>,
    /// Ternary Memory Graph: intent hypervectors + Hebbian edge learning.
    /// Enables semantic similarity routing and structural forgetting.
    tmg_memory: IntentMemory,
    /// Health monitor: detects degradation and triggers self-healing.
    health_monitor: HealthMonitor,
    /// Autonomous self-improvement loop controller: proposes mutations when degradation detected.
    improvement_loop: LoopController,
    /// Path to persistent mutation ledger (for cross-session learning).
    mutation_ledger_path: String,
    /// External backend URL, if configured. Credential vault.
    external_url: Option<String>,
    call_counter: u64,
}

impl NanoKeymaster {
    fn new(model: CalibModel, external_url: Option<String>) -> Self {
        let improvement_config = SelfModConfig {
            enabled: true,
            cycle_budget_us: 500_000, // 500ms per cycle
            max_mutations_per_cycle: 3,
            fitness_improvement_threshold: 1.01, // 1% improvement required
            auto_rollback_on_regression: true,
        };
        let mut improvement_loop = LoopController::new(improvement_config);

        // Load mutation ledger from persistent storage (if available)
        let ledger_path = "mutation_ledger.json".to_string();
        if let Err(e) = improvement_loop.load_ledger(&ledger_path) {
            eprintln!("[startup] warning: failed to load mutation ledger: {}", e);
        }

        Self {
            ledger: TamperEvidentLedger::new(None).expect("ledger init"),
            memory: HashMap::new(),
            tmg_memory: IntentMemory::new(),
            health_monitor: HealthMonitor::new(100, 1_000_000), // 100 µs baseline, 1MB baseline memory
            improvement_loop,
            mutation_ledger_path: ledger_path,
            external_url,
            call_counter: 0,
            model,
        }
    }

    /// The outer interface. Every call goes through:
    /// policy → route → act → log → update memory (parametric + structural) → respond.
    fn call(&mut self, intent: &str, payload: &Value, _context: &Value) -> Value {
        self.call_counter += 1;
        let call_id = self.call_counter;
        let t0 = Instant::now();

        // 1. Policy brain: score the intent+payload text with the ternary classifier.
        let text = extract_text(payload);
        let raw_score = self.model.score_label(&text);
        let confidence = normalize_confidence(raw_score, self.model.threshold);

        // 2. Routing brain: decide where this call goes (with TMG similarity suggestions).
        let backend = self.route(intent, raw_score);
        let similar_intents = self.tmg_memory.find_similar(intent, 3);
        let similar_names: Vec<&str> = similar_intents
            .iter()
            .take(2)
            .map(|(name, _)| name.as_str())
            .collect();

        // 3. Act: execute against chosen backend.
        let result = self.execute(intent, payload, &text, raw_score, &backend);

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

        // 5. Update memory: both parametric (stats) and structural (TMG).
        let stats = self.memory.entry(intent.to_string()).or_default();
        stats.calls += 1;
        stats.sum_raw_score += raw_score;
        match backend {
            Backend::Local => stats.local_count += 1,
            Backend::LocalFallback => stats.fallback_count += 1,
            Backend::External => {}
        }

        // TMG: fire-together-wire-together on similarity edges (Hebbian learning).
        self.tmg_memory.observe_call(intent, &similar_names);

        // 6. Health monitoring: track latency, detect degradation, trigger healing.
        self.health_monitor.sample(elapsed_us, 1_000_000); // Mock memory for now

        if self.health_monitor.should_trigger_healing() {
            self.heal_topology();
        }

        // 7. Periodically prune stale edges and emit learning updates.
        if call_id.is_multiple_of(10) {
            let pruned = self.tmg_memory.prune_stale_edges();
            if !pruned.is_empty() {
                eprintln!("[tmg:pruning] stale edges removed: {:?}", pruned);
            }
            self.emit_learning_update();

            // Also check health trend
            if self.health_monitor.efficiency_trend() == TrendSignal::Degrading {
                eprintln!("[health] system degrading: efficiency trend negative");
            }
        }

        let (tmg_intents, tmg_edges) = self.tmg_memory.stats();

        json!({
            "call_id": call_id,
            "backend": backend.label(),
            "confidence": confidence,
            "score_raw": raw_score,
            "latency_us": elapsed_us,
            "result": result,
            "tmg_similar": similar_intents,
            "tmg_stats": {
                "intents": tmg_intents,
                "edges": tmg_edges,
            },
            "health": {
                "efficiency": self.health_monitor.current_efficiency(),
                "conservative_mode": self.health_monitor.in_conservative_mode,
            }
        })
    }

    /// Routing brain: maps intent + confidence score to a backend.
    /// In conservative mode, all requests route locally (graceful degradation).
    fn route(&self, intent: &str, score: i64) -> Backend {
        // Conservative mode: all local (system under stress)
        if self.health_monitor.in_conservative_mode {
            return Backend::Local;
        }

        match intent {
            // Core kernel intents: handle locally if confident.
            "classify" | "score" | "predict" => {
                if score >= self.model.threshold {
                    Backend::Local
                } else if self.external_url.is_some() {
                    Backend::External
                } else {
                    Backend::LocalFallback
                }
            }
            // Unknown intent: escalate externally if available.
            _ => {
                if self.external_url.is_some() {
                    Backend::External
                } else {
                    Backend::LocalFallback
                }
            }
        }
    }

    fn execute(
        &self,
        intent: &str,
        _payload: &Value,
        text: &str,
        score: i64,
        backend: &Backend,
    ) -> Value {
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
                // HTTP client not yet wired. Routing decision is made + logged.
                json!({
                    "note": "external backend selected — HTTP client not yet implemented",
                    "would_call": self.external_url,
                    "preview": &text[..text.len().min(80)],
                })
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
        let (tmg_intents, tmg_edges) = self.tmg_memory.stats();
        json!({
            "total_calls": self.call_counter,
            "ledger_entries": self.ledger.len(),
            "model_nonzero_weights": self.model.nonzero_count(),
            "model_threshold": self.model.threshold,
            "external_backend": self.external_url,
            "intent_memory": intent_memory,
            "tmg_memory": {
                "intents": tmg_intents,
                "edges": tmg_edges,
            },
            "health": {
                "efficiency": format!("{:.2}%", self.health_monitor.current_efficiency() * 100.0),
                "trend": format!("{:?}", self.health_monitor.efficiency_trend()),
                "conservative_mode": self.health_monitor.in_conservative_mode,
                "should_heal": self.health_monitor.should_trigger_healing(),
            }
        })
    }

    /// Evolution rail: after N calls, report which intents are confidently local.
    /// These observations are where future routing policy mutations would be proposed.
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

    /// Self-healing: detect degradation and take corrective actions.
    /// Triggers autonomous mutation cycle when efficiency drops.
    fn heal_topology(&mut self) {
        let trend = self.health_monitor.efficiency_trend();
        let current_efficiency = self.health_monitor.current_efficiency();

        eprintln!(
            "[health:healing] triggered — trend: {:?}, efficiency: {:.2}%",
            trend,
            current_efficiency * 100.0
        );

        // Action 1: Prune weak edges in TMG (fast, low-cost recovery).
        eprintln!("[health:healing] pruning weak edges (threshold < 0.15)");
        let pruned = self.tmg_memory.prune_stale_edges();
        if !pruned.is_empty() {
            eprintln!("[health:healing] pruned {} stale edges", pruned.len());
        }

        // Action 2: Determine degradation signal from real HealthMonitor metrics.
        let degradation_signal = Some(DegradationSignal::from_metrics(
            self.health_monitor.baseline_latency_us(),
            self.health_monitor.baseline_memory_bytes(),
            self.health_monitor.current_latency_us(),
            self.health_monitor.current_memory_bytes(),
        ));

        // Action 3: Run autonomous improvement loop if degradation is significant.
        if current_efficiency < 0.9 && self.tmg_memory.stats().0 > 0 {
            eprintln!("[improvement:loop] starting autonomous mutation cycle");
            // Create a graph representing the intent topology for mutation proposals.
            let intent_graph = self.tmg_memory.current_structure();
            match self.improvement_loop.step(
                &intent_graph,
                current_efficiency,
                degradation_signal,
            ) {
                Ok((improved_graph, stats)) => {
                    eprintln!(
                        "[improvement:loop] cycle complete  outcome={:?}  proposed={}  accepted={}  efficiency={:.2}%→{:.2}%",
                        stats.outcome,
                        stats.mutations_proposed,
                        stats.mutations_accepted,
                        stats.baseline_efficiency * 100.0,
                        stats.final_efficiency * 100.0
                    );

                    if stats.mutations_accepted > 0 {
                        eprintln!("[improvement:loop] mutations applied — topology optimized");
                        // Note: In a full implementation, mutations would be replayed against IntentMemory
                        // For now, the improved_graph snapshot represents the optimized topology.
                        // A future phase would implement durability by persisting these mutations.
                    }

                    // Display learned knowledge from mutation ledger (periodic summary)
                    if self.improvement_loop.cycle_count % 5 == 0 {
                        eprintln!("[improvement:loop] === LEARNING REPORT (Cycle {}) ===", self.improvement_loop.cycle_count);
                        eprintln!("{}", self.improvement_loop.learning_report());
                    }

                    // Save ledger for persistence across sessions (every 10 cycles)
                    if self.improvement_loop.cycle_count % 10 == 0 {
                        if let Err(e) = self.improvement_loop.save_ledger(&self.mutation_ledger_path) {
                            eprintln!("[improvement:loop] warning: failed to save mutation ledger: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[improvement:loop] cycle failed: {}", e);
                }
            }
        }

        // Action 4: Check if we should enter conservative mode (both latency & memory high).
        if self.health_monitor.should_enter_conservative_mode() {
            self.health_monitor.set_conservative_mode(true);
            eprintln!(
                "[health:healing] CONSERVATIVE MODE enabled — all requests route local"
            );
        }

        eprintln!(
            "[health:healing] system health after intervention: efficiency={:.2}%",
            self.health_monitor.current_efficiency() * 100.0
        );
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn extract_text(payload: &Value) -> String {
    payload
        .get("text")
        .or_else(|| payload.get("prompt"))
        .or_else(|| payload.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Map raw score relative to threshold to [0.0, 1.0].
fn normalize_confidence(score: i64, threshold: i64) -> f64 {
    if threshold == 0 {
        return 0.5;
    }
    let ratio = score as f64 / threshold.abs() as f64;
    (ratio * 0.5 + 0.5).clamp(0.0, 1.0)
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

// ── main loop ─────────────────────────────────────────────────────────────────

fn main() {
    let external_url = std::env::var("KEYMASTER_BACKEND_URL").ok();

    eprintln!("[keymaster] NanoKeymaster booting");
    let model = load_or_train_model();
    eprintln!(
        "[keymaster] ready  nonzero_weights={}  threshold={}",
        model.nonzero_count(),
        model.threshold
    );
    match &external_url {
        Some(url) => eprintln!("[keymaster] external backend configured: {}", url),
        None => eprintln!("[keymaster] external backend: none (local-only mode)"),
    }
    eprintln!("[keymaster] accepting JSON on stdin — one object per line");
    eprintln!(
        r#"[keymaster] example: {{"intent":"classify","payload":{{"text":"fn main() {{}}"}}}})"#
    );

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
