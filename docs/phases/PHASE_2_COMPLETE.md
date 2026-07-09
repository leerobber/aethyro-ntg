# Phase 2 COMPLETE Certificate

**Sign-off date:** 2026-07-09  
**Phase N+1 may begin:** **YES** (Phase 3)

## Scope

Graph structure + SIS front-end (ADR 0003): typed graph, doc/path parse,
fs-event pure layer, leaf signal, lazy body, glyph fingerprint v0,
byte-merge cost mitigation, forward pass, fingerprint, interaction
score (with honest non-win), weighted GraphNode, execution ledger log,
overhead measurement.

## Deliverables map

| Item | Path |
|------|------|
| Graph + adj_list | `ntg/graph/mod.rs` |
| GraphNode weights | `ntg/graph/node.rs` |
| Doc parse | `ntg/docparse.rs` |
| Path parse | `ntg/pathparse.rs` |
| Fs events (pure) | `ntg/fsevents.rs` |
| LeafSignal | `ntg/leafsignal.rs` |
| Lazy leaf body | `ntg/lazyleaf.rs` |
| Glyph fingerprint v0 | `ntg/glyph.rs` |
| Byte merge (MrT5-inspired) | `ntg/bytemerge.rs` |
| Execution ledger log | `Graph::log_execution_nodes` |
| Overhead bench | `src/bin/graph_overhead_bench.rs` |
| Self-parse tests | `tests/self_parse.rs` |

## Test proof

```bash
cd kernel && cargo test
cargo run --release --bin graph_overhead_bench
```

Dedicated tests: lazy resolve, execution ledger log, glyph determinism,
bytemerge reduction, self_parse, graph suite.

## Measurements

EXPERIMENTS.md: graph overhead ~10.5× vs static fold on 8-node sample
(absolute sub-µs). Density TOBL benches under Phase 1 cert.

## Explicit re-scopes

| Item | Decision |
|------|----------|
| Trained PIXEL / full raster glyph encoder | **Phase 4+ research** — requires model weights not in-repo; **GlyphFingerprint v0** is the Phase 2 channel (deterministic shape-class proxy), fully documented as non-PIXEL |
| OS filesystem watcher (`notify` crate) | **Out of Phase 2** — pure `fsevents` translation remains; OS wiring is a dependency decision for deployment phase |
| Learned MrT5 merges | **Phase 4+** — deterministic merge measured; learned merge needs training loop |

## Deep dive

**What advances the project most after Phase 2:** wire ternary weights
(`GraphNode`) into a **labeled task** on doc graphs (Phase 4), using
ledgered self-mod (Phase 3) only under config. SIS is structural; intelligence
still needs calibration.

## Sign-off

**COMPLETE.** Phase 3 may begin under the gate protocol.
