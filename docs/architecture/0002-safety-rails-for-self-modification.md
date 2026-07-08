# 0002: Safety rails for self-modifying graph topology

**Status:** Accepted. Not yet implemented — this ADR defines the
requirements the self-modification engine (Phase 3, see ROADMAP.md) must
meet before it ships enabled, at all.

## Context

This project's graph topology is meant to evolve (add/remove nodes and
edges, re-route computation) rather than stay static — that is the
"self-modifying" half of the engine's bet ([ADR 0001](0001-vision-and-pivot.md)).
Unconstrained self-modification of a running system's own computation
graph is a real safety and auditability concern, especially given the
target deployment (fully air-gapped, sovereign, regulated environments)
where nobody can inspect a live network connection to sanity-check what
changed. This ADR adapts the same rails already specified for Firmament's
evolution loop (Firmament ADR 0002), applied here to graph topology
mutation specifically.

## Decision

The self-modification engine must meet all of the following before it
ships enabled by default — not after, not as a fast-follow:

1. **Off by default.** The graph ships as a fixed, inference-only
   topology first. Self-modification is an explicit opt-in mode, not the
   default posture, until it has earned trust on real measured outcomes.
2. **Bounded compute/time budget per modification cycle.** Each cycle
   (propose a topology change, evaluate it, accept or reject) runs within
   a hard, configurable ceiling. It cannot run indefinitely or consume
   unbounded resources on a customer's box.
3. **Automatic rollback on regression.** Every proposed modification is
   evaluated against a real fitness signal — measured task performance or
   measured resource cost on the actual target hardware, not a proxy —
   before acceptance. A regression reverts to the last known-good
   topology automatically, no human in the loop required to avoid
   degradation.
4. **Deterministic replay.** Same topology + same input must produce the
   same output. The ternary scalar core (Phase 1.1) is deliberately pure
   and RNG-free for exactly this reason — determinism has to hold at the
   lowest layer for it to mean anything once graph mutation sits on top
   of it.
5. **Every modification event is ledger-logged.** Each accepted or
   rejected topology change — with its fitness score and the resource
   budget it consumed — is an entry in a tamper-evident audit ledger, not
   an optional or separate log. This reuses the design already proven in
   the founder's GH05T3 project (ChronosLedger: a 32-byte mmap binary
   agent-state store already used in production for slot inspection and
   lineage tracing), rather than inventing a new ledger format.

## Consequences

- The first working version of the self-modification engine is scoped
  small: evolve once at a calibration step against the target hardware,
  freeze, log the result, require an explicit re-trigger for another
  cycle. Continuous background self-modification is a later, harder-to-
  justify step once rules 1-5 are proven out.
- The fitness evaluator must be real and fast enough to run inside the
  budget in rule 2 — not a slow proxy that forces the budget to be
  unreasonably large.
- Because this reuses ChronosLedger's format rather than a new one,
  Phase 3 (see ROADMAP.md) includes porting/adapting that ledger code
  into this repo rather than designing a new binary format from scratch.
