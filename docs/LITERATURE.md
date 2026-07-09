# Literature grounding

This document exists so novelty claims about the Aethyro NTG Engine are
checkable, not asserted. Searched and verified 2026-07-07. If a claim in
any ADR or design doc isn't backed by something here (or by this
project's own measured results), treat it as unverified and flag it.

## Ternary weight quantization — proven, not novel

- **BitNet b1.58** (Microsoft): weights constrained to {-1, 0, +1}
  (~1.58 bits/weight, log2(3)), ~90% memory savings vs. FP16, trained
  natively (not post-training quantized) via a `BitLinear` layer replacing
  `nn.Linear`. [Technical report (arXiv 2504.12285)](https://arxiv.org/pdf/2504.12285)
- **BitNet b1.58 2B4T**: first natively-trained 1-bit LLM at 2B
  parameters, trained on 4 trillion tokens, released April 2025 under the
  MIT license; a January 2026 CPU optimization added 1.15x-2.1x further
  throughput. [1.58-bit LLM — Wikipedia](https://en.wikipedia.org/wiki/1.58-bit_large_language_model)
- Quantization mechanism: weights via absmean (scale by the tensor's own
  average magnitude), activations via per-token absmax to 8-bit. This
  project's `encode()` function (kernel/src/ntg/ternary.rs) follows the
  same absmean-style shape, not a byte-for-bit reimplementation of
  BitLinear.
- Energy: BitNet 2B measured at ~0.028 J/inference vs. 0.347 J for a
  comparable Qwen2.5 model. [JMLR paper](http://www.jmlr.org/papers/volume26/24-2050/24-2050.pdf)

**Conclusion: do not claim ternary weights as novel anywhere in this
project's docs or marketing.** The claim to make instead: this project
*reuses* a proven, production-grade technique rather than inventing a
new quantization scheme.

## Self-modifying / dynamically-evolving graph topology — active research, not novel

- **Self-organizing dynamic graph neural cellular automata** (Dec 2025):
  a neural cellular automaton with dynamic, learnable graph connectivity
  that adapts its computational topology in response to input history —
  history-dependent behavior without explicit recurrence. [IJCSE](https://www.ijcsejournal.org/self-organizing-graph-neural-cellular-automata/)
- **EvoNet** (2025): self-evolving networks that autonomously adjust
  structure during training via genetic-inspired mutations, improved
  robustness shown in reinforcement-learning tasks.
- **Knowledge-aware Evolutionary Graph Neural Architecture Search**
  (2024/2025). [arXiv 2411.17339](https://arxiv.org/pdf/2411.17339)
- **SEKI** — self-evolution and knowledge-inspired NAS via LLMs (2025).
  [arXiv 2502.20422](https://arxiv.org/pdf/2502.20422)
- **Dynamic nested hierarchies** — self-evolving ML architectures for
  lifelong learning (2026). [Frontiers in AI](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2026.1804338/full)
- Related, adjacent field: Neural Architecture Search (NAS) broadly
  searches static architectures offline; the above work is the newer,
  online/dynamic-adaptation subset most relevant here.

**Conclusion: do not claim self-modifying graph topology as unprecedented.**
This is an active, multi-paper 2025-2026 research area. The claim to make
instead: this project applies known self-evolving-topology ideas inside
a much stricter safety/audit envelope than the research work above
generally specifies (see [ADR 0002](architecture/0002-safety-rails-for-self-modification.md)),
aimed at production deployment, not a research benchmark.

## What has not been found (as of this search pass) — the actual whitespace claim

No published or public work was found combining all three of:
1. Ternary weight quantization (proven — BitNet lineage above),
2. Bounded, self-evolving graph topology (active research — above), and
3. A tamper-evident, deterministic-replay audit ledger, engineered
   specifically for provably air-gapped/sovereign edge deployment.

This is stated as **"not found in this search pass," not "proven not to
exist anywhere."** Treat it as the current best understanding, subject to
revision if later research surfaces a direct prior-art match — and update
this document if that happens, per CONTRIBUTING.md's "write decisions
down, including when they turn out wrong" principle.

## Document/path/glyph ingestion (ADR 0003) — real prior art, not invented

- **PIXEL** (Rust et al., ICLR 2023): text rendered as images, processed
  as ViT patches — proves glyph geometry can be a legitimate model
  substrate. Documented weakness: hard to generate/reconstruct from
  patches, not just recognize. [OpenReview](https://openreview.net/pdf?id=FkSp8VW8RjH),
  [Text Rendering Strategies for Pixel LMs](https://aclanthology.org/2023.emnlp-main.628/)
- **ByT5**: tokenizer-free, byte-level T5 variant — case, punctuation,
  whitespace preserved exactly since there's no subword vocabulary to
  normalize them out of. Documented cost: up to 6-9x slower on long
  sequences vs. subword tokenization. [ACL Anthology](https://aclanthology.org/2022.tacl-1.17.pdf)
- **CANINE**: character-level, tokenization-free encoder, convolutional
  downsampling before a transformer stack. [Referenced via tokenizer-free
  architecture surveys, 2024-2025].
- **MrT5**: dynamic token merging built specifically to cut byte-level
  models' sequence-length cost. [arXiv 2410.20771](https://arxiv.org/pdf/2410.20771)
- **GraphMD / "Literate Execution"** (2026): Markdown parsed directly
  into executable knowledge graphs — sections become nodes, fenced code
  blocks become executable ops, references become edges, bridged to
  RDF/OWL. This is real, current prior art for "docs parsed into
  execution ops, paths as graphs" — not something this project invented.
  [Literate Execution (arXiv 2604.26967)](https://arxiv.org/pdf/2604.26967),
  [GraphMD](https://github.com/graphmd-lpe/graphmd)

**Conclusion:** no single piece of ADR 0003's design is novel on its own.
The claim is the specific synthesis — structural-first (GraphMD-style)
with lazy byte-exact + glyph-fingerprint leaf resolution (ByT5/CANINE +
PIXEL-lite) — feeding directly into NTG's own graph substrate rather than
existing as a separate front-end. Same epistemic shape as ADR 0001's
whitespace claim: proven parts, unproven combination.

## Prior art within the founder's own project portfolio

- **ChronosLedger** (GH05T3): a working 32-byte mmap binary agent-state
  store, already operated on directly (slot inspection, vacancy scans,
  lineage tracing). This is the audit-ledger design this project reuses
  rather than reinventing (see ADR 0002).
- **GenesisThread / MutationEngine** (GH05T3): a working mutation +
  fitness-scoring + orphan-pruning evolution loop. Conceptual prior art
  for this project's self-modification engine (Phase 3).
- **aetherflux-zero**: the founder's existing small-scale architecture
  experimentation project already practices exactly the discipline this
  project commits to — PR #2 there is an honest negative result (a depth
  experiment that didn't help), PR #3 is a real measured 11%
  bits-per-character improvement from a BPE tokenizer change. This
  project should report results the same way: measured wins reported as
  wins, measured non-wins reported as non-wins.
