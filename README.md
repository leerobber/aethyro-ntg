# aethyro-ntg

The Aethyro NTG (Neural Ternary Graph) Engine: a ternary-weight,
self-evolving-graph-topology inference engine, wrapped in a
tamper-evident audit ledger, engineered for air-gapped / sovereign edge
deployment.

**Current status (authoritative):**  
→ **[docs/STATUS.md](docs/STATUS.md)** — full research-agency report, test
proof matrix, gaps, and next priorities.  
→ **[docs/ROADMAP.md](docs/ROADMAP.md)** — phased gates.

**As of 2026-07-09:** pre-alpha research kernel, **capability v10**,
Phase 0–5 COMPLETE (calib + precision + GraphNode warm-start path). Not
benchmarked against production aethyro.com inference; no GTM decision.

## What's actually new (precise claim)

See [docs/architecture/0001-vision-and-pivot.md](docs/architecture/0001-vision-and-pivot.md)
and [docs/LITERATURE.md](docs/LITERATURE.md). Ternary quantization and
dynamic graph topology each have prior art. This project's bet is the
*combination* of both inside a deterministic-replay, ledger-audited
safety envelope for fully air-gapped deployment.

## Why this repo, not Firmament

Supersedes the prior legal-vertical-first plan
([leerobber/Firmament](https://github.com/leerobber/Firmament)) in favor
of building the engine first and letting product/vertical follow from
measured capability.

## Where this fits with aethyro.com

[aethyro.com](https://aethyro.com) is live (Personal, CPA, Dev, Research).
This engine's first intended target is an efficiency upgrade on hardware
those tiers already run — not a new vertical sales motion.

## Quick start

```bash
cd kernel
cargo test
cargo build --release
```

From repo root (tests + benches + calib + schooling):

```bash
./tools/dev.sh check    # test + clippy
./tools/dev.sh model    # train → artifacts/models + eval + predict
./tools/dev.sh model-ab # A/B two epoch settings
./tools/dev.sh school   # doctorate study+exam phases 0–5 (75% gate, notebooks)
```

Schooling notebooks: [docs/schooling/](docs/schooling/) — real data only, fail <75% full redo.

Optional layer ingest contract check:

```bash
echo '{"layers":[{"nodes":[{"id":0},{"id":1}]}]}' | python3 tools/ingest.py
```

## Repository layout

| Path | Purpose |
|------|---------|
| `kernel/` | Rust crate: ternary core, storage, graph, ledger, mutation, runtime, calib |
| `tools/ingest.py` | Sequential `GraphNode.id` contract for native forward |
| `tools/dev.sh` | One-shot test / calib / model / bench workflows |
| `artifacts/models/` | Local CalibModel dumps (`dev.sh model`; not required in git) |
| `docs/STATUS.md` | **Where the project is** (read first) |
| `docs/ROADMAP.md` | Phased build plan and open gates |
| `docs/PHASE5_PREP.md` | Pre-positioned Phase 5 hooks |
| `docs/DESIGN.md` | Technical architecture |
| `docs/LITERATURE.md` | Sourced novelty grounding |
| `docs/EXPERIMENTS.md` | Measured wins and non-wins |
| `docs/architecture/` | ADRs 0001–0006 |
| `kernel/FFI_*.md`, `TOBL_FFI_REFERENCE.md` | C ABI notes |

## Implemented stack (summary)

1. **Ternary core** — scalar golden `matmul_scalar`, encoding  
2. **Storage** — packed 2-bit, dual-stream bit-sliced, sparse COO  
3. **SIMD / TOBL / FFI** — runtime dispatch, C ABI, OpStats  
4. **Graph + SIS** — topology, doc/path parse, fs-event pure layer, adj_list  
5. **Native runtime** — `forward_native_parallel` + density-based `AccelManager`  
6. **Ledger + self-mod** — SHA-256 chain, budgets, fitness, **off by default**

## Explicitly not done

- Full AVX-512 VPOPCNTDQ kernels (detect yes, full kernels no)  
- GPU/NPU (re-scoped: CPU TOBL 12–20×; revisit at large tensors)  
- Phase 6 integration / product head-to-head vs aethyro.com  
- Self-mod enabled by default (stays off)
- Lazy PIXEL-lite glyph fingerprints (ADR 0003 design only)

## Engineering principles

See [CONTRIBUTING.md](CONTRIBUTING.md). Rule: **measure, don't assume**;
docs and CI green before calling a phase done.

## License

Proprietary — see [LICENSE](LICENSE).
