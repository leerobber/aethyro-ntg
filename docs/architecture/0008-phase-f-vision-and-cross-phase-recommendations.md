# 0008: Phase F direction — cross-phase performance/intelligence recommendations, real-time self-awareness instrumentation, self-healing, and the synthetic-biology/robotics long-horizon vision

**Status:** Proposed (ideas + recommendations, not yet implemented).
**Phase:** Written alongside Phase E COMPLETE — see `PHASE_E_EXTENDED_VALIDATION_COMPLETE.md`.
**Depends on:** Kernel Phases 0–5 COMPLETE, Genomic Phases A–E COMPLETE.

## Context

This doc answers four things asked together: (1) what to retrofit into
Phases A–E and kernel Phases 0–5 for real performance/intelligence gains,
(2) what real-time metrics/graphs would give the system multi-axis
("360°/4D+") self-awareness, (3) how to make it self-healing, and (4) the
long-horizon idea of growing this toward a synthetic-biology-inspired
AI/ML-in-robotics architecture ("synthetic DNA" driving a cognitive
substrate).

Following this repo's own convention (see `README.md`,
`docs/architecture/0006-*`): recommendations below are graded honestly.
Some are buildable this week from code that already exists. Some are
multi-week efforts. The synthetic-biology/robotics vision is explicitly
long-horizon and speculative — it is staged so each step is independently
useful even if later steps never happen.

---

## Part 1 — Cross-phase performance & intelligence recommendations

The theme across all of these: **the ntg kernel (ternary storage, SIMD
dispatch, mutation/evolution, ledger) and the genomic module were built as
two separate stacks that happen to share a process.** The single highest-
leverage move available is *cross-wiring* them — using kernel machinery to
accelerate genomic computation, and using genomic validation metrics as
real fitness signal for the kernel's self-mutation loop. Specific ideas:

### Phase A (VCF → LD → Blocks)
- **Bit-sliced ternary genotypes.** `bitsliced_genotypes.rs` already
  packs genotypes densely, but LD (`ld_compute.rs`) is computed with
  scalar loops. The kernel's `ntg::storage::bit_sliced_ternary` and
  `ntg::simd::dispatcher` (AVX2-dispatched popcount/dot-product) were
  built for exactly this shape of problem. Route genotype dot-products
  through the SIMD dispatcher instead of a hand-rolled scalar loop —
  likely a 4–10x wall-clock win on the 3.8GB of real VCF data sitting in
  `data/raw`, not just the 10-SNP synthetic stand-ins currently exercised.
- **Real data path.** Phase A→E all currently run on deterministic
  synthetic proxies (`synthetic_freq`, `synthetic_recomb_rate`, etc. —
  explicitly labeled as such in the code). The actual 1000-Genomes-style
  data already downloaded is unused by the phase binaries. Wiring
  `VcfParser` → real `data/raw/*.gz` is the single biggest "is this real"
  upgrade available and unblocks everything else in this doc that talks
  about "real biology."

### Phase B (Chromosome Brain + Agents)
- **Ledger every domain-agent diagnosis.** `agents.rs`/`domain_agents.rs`
  produce `DiseaseDiagnosis`/`AgentResponse` but nothing writes them to
  `ntg::ledger`. The ledger already exists, is tamper-evident, and is
  exactly the audit trail a disease-detection agent needs before its
  output could ever be trusted downstream. This is a wiring task, not new
  design.
- **StateBlots for agent lineage.** `ntg::ledger::stateblots` already
  implements per-agent generation lineage + `verify_lineage`. Chromosome
  agents currently have no persisted lineage across brain re-inits. Using
  `StateSlotStore` per `ChromosomeAgent` gives free provenance and is the
  prerequisite for the self-healing design in Part 3.

### Phase C (Synthesis & Evolution)
- **Reuse the kernel's mutation/evolution engine.** `genomic::evolution`
  (`EvolutionSim`, `FitnessModel`) reimplements a fitness-driven search
  loop that is structurally identical to `ntg::mutation` (`FitnessEvaluator`,
  `FitnessScore::dominates`, budget-bounded mutation cycles already used
  for graph topology evolution). Two parallel evolution engines is
  duplicated complexity; converging on one (probably the kernel's, since
  it already has ledger integration and budget accounting) means genome
  synthesis inherits self-mod safety rails for free instead of needing
  its own.

### Phase D (Quality Control & Validation)
- **`inverse_normal_cdf` fixed this session** (missing `ln()` in the
  Abramowitz-Stegun approximation — was silently returning NaN for
  `p >= 0.5` and failing every power check above the median). Worth a
  regression test asserting `inverse_normal_cdf(0.5..1.0)` is finite and
  monotonic, since this class of bug (silent NaN, not a panic) is exactly
  the kind that hides in synthetic-data-only test suites.
- **f64 for the tail of the power calculation.** `f32` is fine for allele
  frequencies but the CDF approximation compounds rounding error; for
  very small alpha (genome-wide significance, ~5e-8) `f32` precision is
  marginal. Low cost to widen `PowerAnalysis` internals to `f64`.

### Phase E (Extended Validation) — this session's work
- **Recombination + haplotype block comparison now implemented**
  (`RecombinationComparator`, `HaplotypeBlockComparator` in
  `extended_validation.rs`) closing the two gaps flagged in
  `STATUS_ALL_PHASES.md`.
- **Next real intelligence gain here specifically:** feed
  `ExtendedValidationReport` scores (similarity, recombination match,
  haplotype-block match, locus power) into the kernel's
  `FitnessEvaluator` as additional fitness dimensions when the synthesis
  engine (Phase C) is converged onto the kernel's mutation loop per
  above. That is the concrete mechanism that turns "genomic validation"
  from a one-shot report into a training signal — see Part 4.

### Kernel Phases 0–5 (ternary/storage, graph, ledger, calibration, runtime)
- **`ntg::runtime`'s hardware-path selection (`resolve_hardware`,
  `high_density_selects_non_sparse_device`) is not currently informed by
  anything genomic**, but genomic workloads are exactly the
  high-dimensional sparse/dense-mixed data this dispatcher was designed
  to route. Once Phase A routes through it (above), the dispatcher's
  existing density heuristics apply unchanged.
- **`ntg::mutation::budget`'s wall-time/consumed accounting** should wrap
  the genomic pipeline binaries once they're on the shared mutation loop,
  so a runaway synthesis/evolution run gets the same budget guardrails
  self-mutating graph topology already has.

---

## Part 2 — Real-time readings for performance, intelligence, and multi-axis ("360°/4D+") self-awareness

Reframing "360°/4D+ awareness" concretely: the system already has (or, per
Part 1, will soon have) **four genuinely distinct observability axes**,
each backed by a real subsystem rather than a metaphor:

| Axis | What it answers | Source (exists today) |
|---|---|---|
| **Structural** | What am I made of, right now? | `ntg::simd::dispatcher` (selected SIMD path), `ntg::storage` density, ternary graph node/edge counts |
| **Temporal** | What did I do, in what order, can I prove it? | `ntg::ledger` (tamper-evident hash chain), `ntg::ledger::replay` (determinism check) |
| **Evolutionary** | Am I getting better? | `ntg::mutation::evaluator::FitnessScore`, budget-consumed-vs-remaining |
| **Biological/domain** | How well do my internal representations track the biology I claim to model? | Phase E's `ExtendedValidationReport` (similarity, Fst, power, recombination/block match) |

"4D+" = these four axes plotted together, with **time as the animating
dimension** (a 5th axis if you want it literal) rather than a static
snapshot. Concrete instrumentation:

1. **Emit a JSONL metrics stream, not just console printouts.** Every
   phase binary and the mutation loop already compute the numbers above;
   append one line per run/generation to `results/metrics.jsonl` (a
   sibling of the `results/metrics.csv` that already exists from an
   earlier session). This is the cheapest possible "real-time" story —
   `tail -f` on that file is already a live feed.
2. **Radar/spider chart = literal "360°" view.** One radar chart with the
   four axes above as spokes (structural = SIMD path/density normalized
   0–1, temporal = ledger verify-OK boolean, evolutionary = fitness
   weighted score normalized, biological = mean genome-wide similarity)
   redrawn per run. `results/report.html` already exists as a Phase B
   report artifact and is a natural home to extend — it's already pure
   HTML/CSS with no external dependency, matching this repo's
   zero-dependency stance.
3. **Sparklines over generations for the evolutionary axis** — a simple
   inline waveform of `FitnessScore::weighted_score()` across mutation
   generations is far more informative than a single number, and costs
   nothing beyond keeping the existing per-generation `GenerationStats`
   that `evolution.rs` already produces.
4. **Ledger-chain integrity as a literal waveform.** `ntg::ledger::replay`
   already computes deterministic fingerprints per node; plotting
   fingerprint-agreement over time turns "tamper-evident" from a boolean
   into a continuously-monitored signal — a flat line at 1.0 is healthy,
   any dip is instantly visible.
5. **Density/precision heatmap for structural awareness.** `ntg::storage`
   already tracks ternary density per layer/node; a heatmap (layer ×
   generation) shows structural drift over the graph's self-mutation
   history at a glance — exactly the kind of view you'd want before
   trusting a self-modifying system with anything real-world.

All five of these are renderable as static HTML/SVG with no new
dependency (this repo is intentionally zero-dependency-Rust), consistent
with `results/report.html`'s existing approach.

---

## Part 3 — Self-healing

The mechanism for this already exists and is unused for this purpose:
**`ntg::ledger::stateblots::StateSlotStore`** provides per-agent
generation lineage plus `verify_lineage()`. Concrete design:

1. Before any mutation/synthesis step that could regress fitness (Phase C
   evolution generation, Phase E validation re-run, a chromosome agent
   re-init), **write a `StateSlot` checkpoint** — this is a few lines of
   wiring, not new infrastructure.
2. **Define a regression trigger** using the four axes from Part 2: if
   `FitnessScore::weighted_score()` drops beyond a budget-scaled
   threshold, or `ExtendedValidationReport::mean_similarity` drops below
   its Phase D/E validation-criteria table threshold, or
   `replay::verify()` fails — treat that as a health-check failure.
3. **Roll back via `lineage()` + `verify_lineage()`**, not a blind
   overwrite: walk the verified lineage chain back to the last slot that
   passed all three checks, and resume from there. Because
   `verify_lineage` is already tamper-evident, the rollback target is
   provably the state it claims to be — self-healing that trusts its own
   history, not just "last known good" by convention.
4. This is genuinely "self-healing" in the narrow, honest sense: bounded,
   auditable, reversible-by-construction recovery — not autonomous
   open-ended repair. Consistent with this repo's existing safety-rails
   ADR (`0002-safety-rails-for-self-modification.md`), this should stay
   framed that way rather than oversold.

---

## Part 4 — Long-horizon vision: synthetic biology → AI/ML/robotics

This is the speculative part, staged so each stage stands alone:

**Stage 1 (buildable now, weeks not months):** Real data, not synthetic
proxies. Wire Phase A's `VcfParser` to the real `data/raw` 1000-Genomes-
style files instead of the deterministic `synthetic_freq` stand-ins used
throughout Phases A–E today. Every validation number in
`PHASE_E_EXTENDED_VALIDATION_COMPLETE.md` currently measures "does the
pipeline reproduce its own synthetic input" — real data is what turns
these into measurements of the actual biology.

**Stage 2 (the actual cross-wiring idea):** Once Phase C is converged
onto the kernel's mutation/evolution loop (Part 1), make genomic
validation scores (Phase E: similarity, Fst, recombination/block match,
locus power) **literal additional dimensions in `FitnessScore`**, evaluated
by the same `FitnessEvaluator` already used for ternary graph topology
mutation. Concretely: the self-evolving graph's fitness function stops
being purely abstract and starts being partially graded on "does the
resulting structure remain consistent with real population-genetic
constraints." This is the mechanism, not a metaphor — it's a few new
fields on `FitnessScore` and a call from Phase E into `ntg::mutation`.

**Stage 3 (phenotype ↔ behavior bridge):** `genomic::phenotype`'s
`GxEEngine` already models genotype × environment → phenotype. Extend
that mapping so certain ternary-graph structural motifs ("synthetic
genes" — recurring subgraph patterns discovered by the mutation engine)
correlate with measurable *system* behavior (latency, robustness under
perturbation, density/precision tradeoffs) the same way a real genotype
correlates with a real phenotype. This is where "AI/ML biology" stops
being a name and starts being an actual measured correlation you could
put a number on.

**Stage 4 (robotics/embodiment — explicitly long-horizon):** If Stages
1–3 hold up, the natural extension is a physical/robotic agent whose
control policy is driven by the ternary-graph "genome," with the ledger
providing the tamper-evident provenance that would be a hard safety
requirement for any embodied self-modifying system. This stage needs
domain expertise (robotics control theory, functional safety) well beyond
what this session can responsibly design — flagging it as a direction,
not a plan, is the honest thing to do here. The concrete, buildable next
step toward it is nothing more exotic than Stage 1: get real biological
data flowing through the pipeline that already exists.

---

## Summary

| Ask | This doc's answer |
|---|---|
| Recommendations to improve past phases | Part 1 — concrete, per-phase, tied to existing files |
| Real-time stats/graphs/waves for performance & intelligence | Part 2 — JSONL stream + radar chart + sparklines + ledger waveform + density heatmap, all zero-dependency HTML |
| 360°/4D+ self-awareness | Part 2's four-axis framing (structural/temporal/evolutionary/biological) plus time as the animating dimension |
| Self-healing | Part 3 — StateSlot checkpoints + verified-lineage rollback, bounded and auditable |
| Synthetic-biology/robotics AI/ML vision | Part 4 — four honest stages, each independently useful, robotics explicitly flagged as long-horizon |

None of Part 2–4 is implemented yet; this document is the recommendation,
not a completed phase. If you want to move on any specific piece, the
next step is picking one item above to actually build.
