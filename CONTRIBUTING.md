# Contributing / engineering principles

This project inherits its engineering discipline from the founder's
sibling projects (Firmament, GH05T3, aetherflux-zero), where the biggest
wins came from measuring instead of assuming, and the biggest wastes came
from skipping that step.

## Rules

1. **Measure, don't assume.** Before claiming a change is faster, safer,
   or more efficient, benchmark it on real hardware. If it doesn't
   measurably help, revert it and record why (see `aetherflux-zero`'s own
   honest-negative-result PR as the model to follow) — a documented dead
   end is more valuable than a silent one.
2. **Every novelty claim is sourced or labeled as unverified.** See
   [docs/LITERATURE.md](docs/LITERATURE.md). Do not repeat "world first"
   or similar claims without a literature check backing them — this
   project already had to walk back an external draft's overclaims once
   (see ADR 0001); don't reintroduce that failure mode.
3. **Write decisions down, including rejected ones.** Every non-trivial
   architectural decision gets an ADR in `docs/architecture/`: Status /
   Context / Decision / Consequences. If a decision turns out stale,
   trust the code over the doc and update it.
4. **No telemetry by construction, not by config.** "Runs offline" is not
   a flag that can be flipped on customer request — no code path may make
   an outbound network call at runtime, full stop.
5. **CI gates every merge, from commit one.** No untested code path ships
   silently. If something can't be tested yet, the test should exist and
   skip cleanly with a stated reason — not be absent.
6. **Self-modification ships with rails, not on faith.** Any code that
   lets the graph topology modify itself must satisfy every rule in
   [ADR 0002](docs/architecture/0002-safety-rails-for-self-modification.md):
   bounded budget, automatic rollback, deterministic replay, ledger-logged,
   off by default.
7. **Library code returns `Result`, it does not panic on bad input.**
   Reserve `unwrap`/`panic!` for tests and truly-unreachable invariants.
8. **Docs and full green CI before the next phase, every time.** See
   [docs/ROADMAP.md](docs/ROADMAP.md) and the binding
   [docs/PHASE_GATE_PROTOCOL.md](docs/PHASE_GATE_PROTOCOL.md). A phase
   is not done until every checklist item is implemented *or*
   explicitly re-scoped in an ADR, tests are heavy and green, and a
   `docs/phases/PHASE_N_COMPLETE.md` certificate exists with sign-off.
   **No soft advance. No Phase N+1 while Phase N has open unexplained
   items.**
9. **Claims discipline extends to marketing copy.** No product claim
   ships through aethyro.com or anywhere else ahead of the tested code
   that makes it true — same standard as rule 1, applied to the business
   side (see Firmament ADR 0003 for the precedent this follows).

## Adding an ADR

Copy the format of an existing entry in `docs/architecture/`, number it
sequentially, and add a row to `docs/architecture/README.md`.
