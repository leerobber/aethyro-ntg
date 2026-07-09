# 0005: Canonical ternary storage types

**Status:** Accepted (2026-07-09).  
**Phase:** 1 (closes dual-PackedTernary ambiguity).

## Context

Two types were both named `PackedTernary` with **incompatible** encodings:

| Module | Layout | Encoding |
|--------|--------|----------|
| `ntg::packed::PackedTernary` | 4 values / `u8` | `0b00=0`, `0b01=+1`, `0b11=-1` |
| `ntg::storage::PackedTernary` | 32 values / `u64` | `0b00=0`, `0b01=-1`, `0b10=+1` |

Silent interchange would corrupt tensors. Docs and APIs must name the
canonical roles clearly.

## Decision

1. **`ntg::packed::PackedTernary`** — **legacy byte-pack** (Phase 1.2
   original). Keep for backward tests and density demos. Prefer alias
   docs name: “legacy packed”.
2. **`ntg::storage::PackedTernary`** — **TOBL / FFI packed** used by
   `tobl_ffi` and density hooks. Canonical for C ABI and TOBL kernels.
3. **`ntg::storage::BitSlicedTernary`** — **canonical dense compute**
   path for dual-stream popcount TOBL (CPU).
4. **`ntg::storage::SparseBitSlicedTernary`** — **canonical sparse /
   GraphNode.weights** path for native runtime and ledgered mutations.
5. Do **not** auto-convert between encodings without an explicit
   `from_*` function that is unit-tested.

## Consequences

- New code attaches weights as `SparseBitSlicedTernary` unless a
  documented reason requires another type.
- ROADMAP Phase 1 “Canonical-storage ADR” checkbox is closed by this ADR.
- Future unification (single type) is Phase 5+ only if measured win;
  not required for Phase 1 COMPLETE.
