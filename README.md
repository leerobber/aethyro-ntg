# aethyro-ntg

The Aethyro NTG (Neural Ternary Graph) Engine: a ternary-weight,
self-evolving-graph-topology inference engine, wrapped in a
tamper-evident audit ledger, built for provably air-gapped deployment.

**What's actually new here, stated precisely** (see
[docs/architecture/0001-vision-and-pivot.md](docs/architecture/0001-vision-and-pivot.md)
and [docs/LITERATURE.md](docs/LITERATURE.md) for the full, sourced
version): ternary weight quantization is proven, production technology
(Microsoft's BitNet b1.58). Self-evolving graph topology is an active
2025-2026 research area, not an invention of this project. What (as of a
2026-07-07 literature check) doesn't appear to exist yet is the specific
combination of both, inside a deterministic-replay, ledger-audited safety
envelope engineered for fully air-gapped/sovereign edge deployment. That
combination — not any single piece — is this project's bet.

## Why this repo, not Firmament

This supersedes the founder's prior plan
([leerobber/Firmament](https://github.com/leerobber/Firmament), a
legal-vertical-first bet) in favor of building the underlying engine
first and letting the product/vertical decision follow from what it can
actually do. Firmament's ADRs remain as historical record.

## Where this fits with aethyro.com

[aethyro.com](https://aethyro.com) is a live product today (Personal,
CPA, Dev, Research tiers, real paying customers; Legal/Healthcare are
waitlist-only, no code yet). This engine's first real target is those
existing tiers — a memory/compute efficiency upgrade on hardware they
already run on — not a new vertical's sales motion. See
[docs/DESIGN.md](docs/DESIGN.md) for how that fits together.

## Status

Pre-alpha. Phase 1.1 (ternary scalar reference) is implemented — see
[docs/ROADMAP.md](docs/ROADMAP.md) for the full phased build plan, gates,
and current status. Nothing here is benchmarked against production
inference yet; no product or go-to-market decision has been made.

## Structure

- `kernel/` — Rust ternary tensor / graph / self-modification engine.
- `docs/architecture/` — ADRs: what's decided, why, and what was rejected.
- `docs/DESIGN.md` — technical architecture.
- `docs/ROADMAP.md` — phased build plan with gates and to-do checklists.
- `docs/LITERATURE.md` — sourced grounding for every novelty claim made
  anywhere in this repo.

## Engineering principles

See [CONTRIBUTING.md](CONTRIBUTING.md).
