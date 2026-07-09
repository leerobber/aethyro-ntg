# Architecture Decisions

Durable record of decisions made from **real code and tests**, not
aspiration. If code and ADR diverge, fix the ADR (or the code) and note
it in [STATUS.md](../STATUS.md).

| # | Decision | Status |
|---|----------|--------|
| [0001](0001-vision-and-pivot.md) | Vision: NTG Engine, not a vertical-first bet | **Accepted** |
| [0002](0002-safety-rails-for-self-modification.md) | Five safety rails for self-modifying topology | **Accepted + implemented** (Phase 3; tests in `phase3_integration.rs`) |
| [0003](0003-sis-frontend.md) | SIS front-end: docs / paths / glyphs into the graph | **Accepted; partially implemented** (doc/path/fs-event/leaf signal ✅; lazy glyph / PIXEL-lite ❌) |
| [0004](0004-phase3-tamper-evident-ledger.md) | Tamper-evident ledger composition | **Accepted + implemented** |
| [0005](0005-canonical-ternary-storage.md) | Canonical ternary storage types | **Accepted** |
| [0006](0006-phase4-calibration-task.md) | Phase 4 doc-graph calibration task | **Accepted + implemented** |
| [0007](0007-observability-genome-prototype.md) | Stats collector + ternary DNA genome prototype | **Accepted (prototype)** |

## How to add a new ADR

1. Copy the format of an existing entry.  
2. Number sequentially.  
3. Prefer documenting **after** implementation and tests.  
4. Update this table and [STATUS.md](../STATUS.md).
