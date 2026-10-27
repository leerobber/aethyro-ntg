# aethyro-ntg — Project Handoff
# For Claude CLI and new sessions

Session origin: https://claude.ai/code/session_0163sRLcjECJkSUJSrtfFGSN
Last updated: 2026-10-26

---

## What This Repo Is

**aethyro-ntg** is the core Rust NTG (Neural Ternary Graph) engine for the Aethyro sovereign AI stack.
It is the **single active repo** — MiniMax-M2.7 (Python testbed) has been archived; all work lives here.

Target hardware: **tatortot** — local GPU machine (Windows + WSL2, RTX 5050 Laptop GPU 8 GB VRAM,
AMD Ryzen 7 250, 8 cores / 16 threads).

---

## Project Lineage

```
aetherflux-zero  →  Firmament (abandoned)  →  aethyro-ntg (active)
```

- `leerobber/aetherflux-zero` — LM arch research, BPE tokenizer, depth experiments
- `leerobber/MiniMax-M2.7` — **ARCHIVED 2026-07-27** — Python self-evolution testbed
  (ScaffoldAgent / EvalHarness / EvolutionMemory). Superseded by Rust Quad-Brain.
- `leerobber/aethyro-ntg` — **ACTIVE** — everything below

---

## Architecture

### NTG Engine
**Neural Ternary Graph** — unique combination (no prior art as of 2026-07-07):
- Ternary weights (BitNet b1.58 absmean quantization)
- Bounded self-evolving graph topology (5 safety rails, off by default)
- Tamper-evident deterministic-replay SHA-256 audit ledger
- Purpose-built for air-gapped edge deployment

### KAIROS
The AI agent being raised within aethyro-ntg. Staged lifecycle:
`Zygote → Neonate → Infant → Toddler → Child → Adolescent → Young Adult → Adult`
Self-modification locked until Adulthood + explicit opt-in.

### NanoKeymaster
The agent IS the API key. Sovereign routing agent (`kernel_host` binary):
- Routes via ternary policy brain (local / local-fallback / external)
- Logs every routing decision to tamper-evident ledger
- External backend: HTTP POST via ureq (`KEYMASTER_BACKEND_URL` env var) to any service that accepts `{"intent","payload"}` and returns JSON; fail-closed (an error payload, no local fallback). `backend/keymaster` is a separate policy service that `kernel_host` does not call (ADR 0012, decided 2026-10-27)

### SovereignBrain (ADR 0009)
1. Working Set (bounded active addresses)
2. LTM Motifs (long-term memory, consolidated + activated)
3. LanguageOrgan (SIS/docparse graph + calib)
4. Multi-axis fitness (task, structural cost, biological consistency, safety)

### Ternary Memory Graph (Phase 6.11)
8192-dimensional ternary hypervector extension on GraphNode:
- HDC operations: bind (XOR), bundle (majority vote), similarity (Hamming)
- 16× memory compression vs float32
- Bit-sliced encoding (pos/neg u64 slices, 128 words each)

### Safety Governance Engine (Phase 6.12)
- SafetyScore: constraint (0.4) + alignment (0.3) + confidence (0.3)
- Behavioral drift detection: 10-cycle rolling window, 40% threshold
- Rollback checkpoint: triggers at 2× max_mutations consecutive rejections

### Quad-Brain Architecture (Phases 6.14–6.18) — on `main` since 2026-10-26

| Brain | Role |
|-------|------|
| α (Alpha) | Synchronization & Self-Healing: drift detection, rollback, consensus |
| β (Beta)  | Learning & Intelligent Routing: pattern learning, strategy optimization, load prediction |
| γ (Gamma) | Meta-Governance & Evolution: policy synthesis, mutation evolution plans |
| δ (Delta) | Perception & Forecasting: hormone levels, regime detection, 16-dim embeddings |

**Execution cycle:** `δ (perceive) → α (sync) → β (learn) → γ (govern) → loop`

**Scale:** 10,000–500,000 agent hierarchies (4-tier: Super/Sub/Micro/Nano).

**Status, stated plainly (2026-10-26):**
- Before this date the brain modules existed only on the unmerged branch `claude/session-0163srlcjeckjksujsrtfgfsn-y9hf3y`, not on `main`, although this file called them COMPLETE. They were ported to `kernel/src/ntg/mutation/`.
- Even there, `QuadBrainAgent` only owned α and β; γ and δ signals were hard-coded constants. It now owns real `BrainGamma` and `BrainDelta`, runs δ→α→β→γ, and γ's policy changes α's consensus threshold on the next cycle (tests: `quad_brain_closed_loop_drift_tightens_consensus`).
- `kernel_host` feeds the Quad-Brain one observation per routed call (policy confidence as efficiency, backend choice as strategy); read it with `{"intent":"_quad_brain"}` / `GET /api/quad_brain`.
- Limits: single-agent view (cluster/swarm aggregates equal the local observation); γ's perception-focus signal to δ is published but δ does not act on it; no mutation routing happens in `kernel_host`, so β reports routing efficiency 0.0 ("none observed").
- The 10K–500K "agent hierarchy" is a data structure: `{"intent":"_hierarchy","payload":{"nano":N}}` builds 1 Super / 10 Sub / 100 Micro / N Nano records and ticks them (measured: 500,111 agents spawned in about 150 ms, tick about 6 ms, in-process, single thread). No agent runs any behaviour; it demonstrates bookkeeping scale only.

---

## Phase Status

| Phase | Status | What |
|-------|--------|------|
| 0 | COMPLETE | Foundation, ADRs, repo structure |
| 1 | COMPLETE | Genomic pipeline (VCF, bitsliced, LD) |
| 2 | COMPLETE | NTG ternary kernel + SIMD matmul |
| 3 | COMPLETE | Self-modification safety rails + audit ledger |
| 4 | COMPLETE | Calibration (phase4_calib binary, model roundtrip) |
| 5 | COMPLETE | Storage integration |
| 6.0 | RE-MEASURED 2026-10-26 | Ternary vs f32 head-to-head. The July "143.6×" was against a triple-loop scalar f32 baseline. Against SIMD f32 the ternary×ternary popcount kernel is slower (0.15–1.05×); the new ternary-weight × int8-activation kernel (`ntg::storage::ternary_i8_gemm`) is 3–24× faster than the best f32 on one core, with a 2–7 point accuracy cost on a small trained model. On the real pretrained BitNet b1.58 2B-4T, accuracy is preserved (perplexity within 0.15%, ARC-Easy unchanged within noise) and the decode matmuls run 7.7–8.5× faster than bf16 on one core; prefill was 2.7–3.3× slower than torch's on an AMX host and, after the panel prefill kernel (`ternary_i8_prefill`), is 6× faster than torch bf16 on a non-AMX host (not retested with AMX), and `bitnet.cpp` decodes the whole model 1.3×–3.5× faster than it on the same VM (kernel-level parity unknown, `docs/results/bitnet_cpp/`). Caveats in `docs/EXPERIMENTS.md` |
| 6.1 | COMPLETE | NanoKeymaster routing agent |
| 6.2 | COMPLETE | External HTTP backend (ureq POST) |
| 6.11 | COMPLETE | Ternary Memory Graph / HyperVector (8192-dim HDC) |
| 6.12 | COMPLETE | Autonomous Safety & Governance Engine |
| 6.13 | COMPLETE | Domain Coordination (multi-agent) |
| 6.14 | COMPLETE | Brain α — Synchronization & Self-Healing |
| 6.15 | COMPLETE | Brain β — Learning & Intelligent Routing |
| 6.16 | COMPLETE | Twin-Brain + Quad-Brain Integration |
| 6.17 | COMPLETE | Brain γ — Meta-Governance & Evolution |
| 6.18 | COMPLETE | Brain δ — Perception & Forecasting |
| **F (L0)** | **IN PROGRESS** | Self-awareness instrumentation — VITASCALE L0 scaffold complete |

**Test count:** <!--tests-->683<!--/tests--> tests, all passing, 0 ignored (measured 2026-10-26, see
`docs/BASELINE_2026-10-26.md`; CI fails if this number drifts).

---

## Running Tests

```bash
# Everything: build + test + clippy for all four crates
scripts/build-all.sh

# Whole-stack smoke (dashboard + kernel_host + keymaster)
scripts/smoke-whole.sh

# Full Rust test suite (<!--tests-->683<!--/tests--> tests, ~1-2 min)
cd kernel && cargo test --release

# Lint (must be clean)
cd kernel && cargo clippy --all-targets -- -D warnings

# CPU/GPU benchmark (tatortot)
python3 scripts/bench_cpu_gpu.py --reps 20 --warmup 5

# First-time Blackwell setup (auto-upgrades torch to cu128)
chmod +x scripts/setup_bench.sh && ./scripts/setup_bench.sh --reps 20 --warmup 5
```

---

## GPU Benchmark Results — tatortot (RTX 5050 Blackwell, 2026-07-27)

**Hardware:** NVIDIA GeForce RTX 5050 Laptop GPU — 8.0 GB VRAM, 20 SMs, sm_120 (Blackwell)
**Software:** PyTorch 2.11.0+cu128, CPU: AMD Ryzen 7 250, 8 threads

### BENCHMARK 1 — Matrix Multiply (SGEMM, float32)

| Size | CPU GFLOP/s | GPU GFLOP/s | Speedup |
|------|-------------|-------------|---------|
| 512×512 | 295.93 | 1,033.36 | 3.5× |
| 1024×1024 | 304.52 | 4,317.17 | 14.2× |
| 2048×2048 | 295.15 | 6,805.57 | 23.1× |
| 4096×4096 | 323.80 | 7,366.92 | 22.8× |

### BENCHMARK 2 — Batch Matrix Multiply (attention-shaped)

| Config | CPU GFLOP/s | GPU GFLOP/s | Speedup |
|--------|-------------|-------------|---------|
| B=4 H=32 S=512 D=64 | 69.06 | 3,144.81 | 45.5× |
| B=4 H=32 S=2048 D=64 | 67.09 | 3,812.96 | 56.8× |
| B=1 H=32 S=4096 D=128 | 121.49 | 3,995.55 | 32.9× |

### BENCHMARK 3 — Element-wise GELU (fused, memory-bound)

| Size | CPU GB/s | GPU GB/s | Speedup |
|------|----------|----------|---------|
| 1M elems | 1.28 | 14.77 | 11.5× |
| 16M elems | 0.62 | 27.22 | 43.9× |
| 256M elems | 0.55 | 35.46 | 64.5× |

### BENCHMARK 4 — Memory Bandwidth (tensor copy)

| Size | Speedup |
|------|---------|
| 128 MB | 35.3× |
| 512 MB | 41.3× |
| 2048 MB | 64.2× |

### Speedup Summary

| Benchmark | CPU→GPU Speedup |
|-----------|----------------|
| gelu 256M elems | 64.5× |
| memcpy 2048 MB | 64.2× |
| bmm B=4 H=32 S=2048 D=64 | 56.8× |
| bmm B=4 H=32 S=512 D=64 | 45.5× |
| gelu 16M elems | 43.9× |
| memcpy 512 MB | 41.3× |
| memcpy 128 MB | 35.3× |
| bmm B=1 H=32 S=4096 D=128 | 32.9× |
| matmul 2048×2048 | 23.1× |
| matmul 4096×4096 | 22.8× |
| matmul 1024×1024 | 14.2× |
| gelu 1M elems | 11.5× |
| matmul 512×512 | 3.5× |
| **Average** | **35.3×** |

---

## Prerequisites (tatortot, WSL2 Ubuntu)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Python (Blackwell — RTX 5050 requires cu128)
source ~/sovereign-core/.venv/bin/activate
pip install --upgrade torch --index-url https://download.pytorch.org/whl/cu128
pip install numpy

# Start vLLM for live evolution runs (not needed for unit tests):
pip install vllm
vllm serve qwen2.5-32b-awq --port 8001 --dtype auto
```

---

## Known Issues / Follow-ups

- **Clippy**: clean on all targets as of 2026-10-26 (the older "~155 warnings in `genomic/`" note no longer reproduces).
- **VITASCALE Hostframe** (ADR 0010): Production hosting architecture (GCP) — not yet implemented.
- **Phase F L0**: Scaffold complete (ADR 0011); GCP Cloud Run backend and full self-awareness wiring pending.

---

## HF Account

`tastytator` — sovereign LoRA models (avery, forge, oracle, codex, sentinel, nexus),
datasets (sovereign-economy, aethyro-training), Space (tatortot).
