# ADR 0007 — Observability collector + ternary genome prototype

**Status:** Accepted (prototype, 2026-07-09)  
**Context:** Design dump for stats aggregation, WASM visualization, avatar UI,
and perpetual DNA-graph evolution. Need a **diff-ready** first slice that
grounds the model in existing aethyro-ntg primitives without soft-advancing
Phase 6 or disabling ADR 0002 rails.

## Decision

### Implemented now (pure Rust, tested)

| Piece | Path | Notes |
|-------|------|--------|
| Lock-free stats aggregator | `ntg/observability/stats.rs` | Records real [`OpStats`](../../kernel/src/ntg/ffi/stats.rs) |
| Snapshot helpers | `StatsSnapshot` | ops/s, avg latency, dominant path, errors |
| DNA graph node | `ntg/genome/mod.rs` | `PackedTernary` payload + `parent_hash` + generation |
| Density-biased delta | `propose_density_delta` | Deterministic; favors filling zeros → ±1 |

### Explicitly deferred (not in this crate yet)

| Item | Why deferred |
|------|----------------|
| WASM + Three.js / `wasm-bindgen` / web-sys UI | New heavy deps; Phase 6 integration concern; fictional `three` crate API in sketch |
| kiss3d avatar / emotion system | Product UI, not kernel correctness; no Phase 6 host yet |
| Perpetual always-on mutation loop | **ADR 0002 rail 1** — self-mod remains **off by default** |
| Full SignedEntry schema change for DNA fields | Needs ledger migration design; use `lineage_line()` text for now |

## Mapping (biology metaphor → code)

| Metaphor | Implementation |
|----------|----------------|
| Gene / DNA | `DNAGraphNode.ternary_genome: PackedTernary` |
| Mutation | `DNAGraphNode::mutate` + `GenomeDelta` |
| Lineage / lifeline | `parent_hash` + `generation` (+ future ledger entry body) |
| Phenotype fitness | Existing dual-objective fitness + `StatsSnapshot` |
| Safety rails | Existing `SelfModConfig` / `MutationCycle` / budget / ledger |

## Safety constraints (binding)

1. Genome mutation APIs are **pure** — they do not touch live `Graph` or ledger.
2. Any live apply path **must** go through enabled `MutationCycle` + ledger log.
3. Stats collection is always-on-safe (relaxed atomics); never panics on hot path.
4. No capability claim that the system “is conscious” or “is alive” — these are
   engineering metaphors for continuous learning under rails.

## Consequences

- Hosts can aggregate FFI `OpStats` without locks.
- Genome prototype unblocks schooling/experiments on density-biased edits.
- WASM/avatar remain product work after Phase 6 host decision.
- Perpetual evolution stays a **design goal**, not a default runtime mode.

## Next atomic steps (when ready)

1. Optional: append `lineage_line()` into ledger mutation description field.
2. Optional: sampled propose-evaluate-apply **behind** `SelfModConfig.enabled`.
3. Phase 6: host-side dashboard consuming `StatsSnapshot::summary_line()`.
