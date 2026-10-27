# Experiments log

Real, run, measured experiments against this project -- wins and
non-wins recorded the same way, per CONTRIBUTING.md rule 1 ("measure,
don't assume") and the precedent already set by the founder's
`aetherflux-zero` project (an honest negative depth-experiment result
sits next to a real measured 11% bits-per-character win, both kept).
An experiment that didn't pan out and is documented here is more
valuable than one that was quietly dropped.

## 2026-07-08: is ChronosLedger actually the "tamper-evident, hash-chained ledger" ADR 0001/0002 assumed it was?

**Why this check happened:** before merging the first substantial batch
of Phase 1-2 work, the question was asked: is there a foundational gap
worth closing now, while cheap, rather than discovering it mid-Phase-3?
ADR 0001 and ADR 0002 both stated, as fact, that this project's audit
ledger would "reuse the design already proven" in GH05T3's
ChronosLedger, describing it as tamper-evident and hash-chained. That
claim had never actually been checked against the real source in this
project — it was inherited from an external draft and repeated forward
across three documents without verification. This is exactly the kind
of claim CONTRIBUTING.md rule 2 exists to catch.

**Method:** read the actual implementation,
`GH05T3/backend/oss/core/chronos_ledger.py`, directly. Also checked two
other candidates in the same codebase for a genuine hash-chain:
`backend/oss/core/seal.py` ("LexGenSeal") and `backend/economy/ledger.py`.

**Result — real, and it corrects a standing false claim:**

- **ChronosLedger** is a real-time **mutable** mmap agent-state store:
  32-byte slots (desires, fitness, maturity, `parent_offset` for
  lineage, generation, a scratchpad bitfield), overwritten in place via
  `write_agent()`/`update_fitness()`/etc. There is no hashing anywhere
  in the file, no previous-record linkage, no tamper-evidence of any
  kind. It is genuinely excellent at what it's actually for: fast
  slot-level state and lineage tracing — not an audit trail.
- **LexGenSeal** (`seal.py`) is real and genuinely tamper-evident **per
  record**: each breakthrough record is SHA256-signed over its own
  content and written to an append-only-by-convention vault file. But
  records are not chained to each other — deleting one seal file from
  the vault directory is completely undetectable from the files that
  remain. It proves "this record wasn't altered," not "no record was
  removed from the sequence."
- **`economy/ledger.py`** is a plain SQLite table, append-only by
  convention (no `UPDATE`/`DELETE` in the code path), no cryptography
  at all.

**Conclusion: no genuine hash-chained, tamper-evident ledger exists
anywhere in the checked codebase.** ADR 0001 and ADR 0002's "reuses the
proven ChronosLedger design" claim was false for the tamper-evidence
half specifically (true only for the state-slot/lineage half). Both
ADRs and DESIGN.md have been corrected in place rather than quietly
patched, so the record shows the mistake and the fix, not just the fix.

**What was built as a direct result:** `kernel/src/ntg/chain.rs`
(`ChainLog`) — a genuine hash-chain primitive, tested against exactly
the properties a real audit ledger needs: altering a historical entry's
content breaks verification from that point forward; removing an entry
breaks verification; a given piece of content produces a different
chain value depending on what preceded it (proving the chain captures
sequence, not just content). Uses `std`'s non-cryptographic
`DefaultHasher` for now, honestly labeled as such (same pattern as
`Graph::fingerprint`) — swapping in a real cryptographic hash (SHA-256,
matching LexGenSeal's own choice, or BLAKE3) is a dependency decision
for Phase 3, not a redesign of this module.

**Why this was worth doing before merge, not after:** every phase from
here on assumes ledger-logging exists (ADR 0002 rule 5, ADR 0003's
execution-node auditing, Phase 3's exit criteria). Finding and fixing
this now — while only two ADRs and one design doc referenced the false
claim — is far cheaper than finding it after Phase 3 was built on top
of an assumption that didn't hold.

## 2026-07-08: does leaf-signal correlate with file extension?

**Hypothesis:** `LeafSignal` (case/punctuation/whitespace counts) might
carry enough information to distinguish content types cheaply, without
any ML.

**Method:** computed per-character signal ratios (uppercase, lowercase,
punctuation, whitespace, other) over every real file in this repo,
grouped by extension.

**Result — real, positive, but small-sample:**

| ext | n | upper | lower | punct | space | other |
|---|---|---|---|---|---|---|
| (none) | 1 | 0.017 | 0.786 | 0.041 | 0.147 | 0.009 |
| .gitignore | 1 | 0.029 | 0.747 | 0.066 | 0.158 | 0.000 |
| .md | 9 | 0.036 | 0.701 | 0.073 | 0.162 | 0.028 |
| .rs | 11 | 0.027 | 0.557 | 0.154 | 0.251 | 0.012 |
| .toml | 1 | 0.000 | 0.576 | 0.227 | 0.144 | 0.053 |

`.rs` files run higher punctuation (15.4% vs. 7.3% for `.md`) and higher
whitespace (25.1% vs. 16.2% -- indentation) with lower lowercase-letter
density (55.7% vs. 70.1%) -- the direction you'd expect from code's
brackets/semicolons vs. prose. `.toml` has the highest punctuation of
all (22.7%: `=`, `[]`, quotes).

**Honest caveat:** n=9 and n=11 are not statistically rigorous. The
direction is real and sensible, not noise, but this should not be
treated as a validated classifier -- it's a plausible cheap heuristic
worth remembering, not a shipped feature.

## 2026-07-08: ternary matmul as a real per-edge "interaction score"

**Hypothesis:** ternary-encoding two adjacent nodes' labels (via the
existing `encode()`) and running a real `matmul_scalar` between them
might produce an interpretable "interaction score" for that edge --
the first real compute over the graph, ahead of Phase 3/4.

**Method:** encoded each label's UTF-8 bytes as `(byte - 128) / 128`,
ternary-encoded via `encode()`, zero-padded to equal length, and ran a
1×k @ k×1 matmul. Tested on real and synthetic label pairs.

**Result — real, negative, and specifically diagnosed, not just "didn't
work":**

```
'Title'            x 'first'    -> score=4.000  enc_a=[-1,-1,0,-1,-1]     enc_b=[-1,-1,-1,-1,-1]
'Title'            x 'second'   -> score=4.000  enc_a=[-1,-1,0,-1,-1,0]   enc_b=[-1,-1,-1,-1,-1,-1]
'aaaaa'            x 'aaaaa'    -> score=5.000  enc_a=[-1,-1,-1,-1,-1]    enc_b=[-1,-1,-1,-1,-1]
'aaaaa'            x 'zzzzz'    -> score=5.000  enc_a=[-1,-1,-1,-1,-1]    enc_b=[-1,-1,-1,-1,-1]
'Section A'        x 'Section B'-> score=6.000  enc_a=[...,-1,0,0,-1,-1] enc_b=[...,-1,0,0,-1,-1] (identical)
```

`"aaaaa"` and `"zzzzz"` -- completely different strings -- produce the
*identical* encoding and score. **Root cause, found, not guessed:**
`encode()` thresholds each string against *its own* mean magnitude
(the same absmean design BitNet uses for a single weight tensor). Any
string made of characters with similar byte values collapses to the
same ternary pattern (mostly `-1`), because the normalization is
relative to that string alone -- there is no shared reference scale
across different labels, so cross-string comparison is structurally
impossible with this construction.

**Conclusion (at the time):** this specific approach does not work as
specified. The mechanism BitNet-style absmean quantization exists for
(compress one tensor's own weights) is not the same problem as (compare
two arbitrary short strings), and applying one to the other silently
breaks it.

**What this suggested for a follow-up experiment:** a fixed/global
byte-to-ternary mapping (same threshold for every string, not each
string's own mean) would preserve cross-string distinguishability. See
below -- this was tried, and it worked.

## 2026-07-08: follow-up — fixed-threshold encoding fixes the failure above

**Hypothesis:** replace `encode()`'s per-string mean threshold with a
fixed global one (same byte always maps to the same ternary value,
regardless of context), calibrated on the a-z byte range (center 109.5,
scale 13, threshold ±0.33).

**Method:** re-ran the exact same failing pairs from the experiment
above, in Python first, before writing any Rust.

**Result — real, positive, verified before shipping as code:**

```
'aaaaa'              x 'aaaaa'              ->    5.0
'aaaaa'              x 'zzzzz'              ->   -5.0   (was 5.0, identical, before)
'hello'              x 'hello'              ->    2.0
'hello'              x 'hxllo'              ->    0.0   (self-score > 1-char-edit score)
```

`"aaaaa"` vs `"zzzzz"` now score oppositely (`+5.0` vs `-5.0`) instead of
identically — the exact failure is fixed. Better than just "fixed":
the score is now *interpretable* in a way it wasn't designed to be —
self-similarity scores positive, a byte-for-byte "opposite" pairing
scores negative, and a single-character edit ("hello" -> "hxllo")
measurably lowers the score below the unedited self-score. This is a
real, if crude, working similarity signal, not just "no longer broken."

**Honest limits, stated precisely, not hidden:**
- This is a **byte-position correlation**, not semantic similarity — it
  knows nothing about meaning.
- It's sensitive to positional alignment: inserting one character near
  the start of a string shifts every later comparison out of sync,
  which could make genuinely similar strings score poorly if they
  differ in length early on. Not tested here.
- The fixed-threshold distribution is skewed (mostly `-1` across the
  full printable-ASCII range — 74 of 95 characters), because it's
  calibrated for a-z distinguishability, not balanced bit-usage. Not a
  drop-in replacement for `encode()`'s quantization use case.

**Shipped as code:** `encode_fixed` (`kernel/src/ntg/ternary.rs`) and
`edge_interaction_score` (`kernel/src/ntg/interaction.rs`), both tested
against the exact properties measured here (opposite-string negative
score, self-score-beats-edited-score, determinism).

## 2026-07-08: does edge_interaction_score say anything real about this repo's actual document structure?

**Hypothesis:** now that the score is fixed and interpretable on small
synthetic examples, does it capture anything meaningful about *real*
parent-child relationships (heading → its content, section → its
bullets) across this repo's actual ADRs, DESIGN.md, and ROADMAP.md?

**Method:** reimplemented `docparse.rs`'s parsing logic in Python
(headings/bullets/numbered items/fenced code, exactly matching the
Rust), parsed all 5 real docs into one combined graph (491 nodes, 486
real parent-child edges), computed `edge_interaction_score` for every
real edge, and compared against a same-sized sample of random
(non-adjacent) node pairs from the same corpus as a control.

**Result — real, and it corrects an initial over-read:**

Raw scores looked different at first glance (real edges: mean 4.48,
std 5.43; random pairs: mean 6.29, std 7.13) — but checking *why*
mattered. Correlation between raw score and `min(len_a, len_b)` across
the real edges: **0.563** (0.595 using `|score|`). Over half the
apparent "real vs. random" difference is explained by string length
alone (a direct consequence of the zero-padding: positions beyond the
shorter string's length always contribute zero, so the score is really
"agreement over the first `min(len)` bytes," which is dominated by how
big that overlap even is) -- not by any relationship between a heading
and its content.

After removing that confound (`normalized_edge_interaction_score` =
raw / `min(len_a, len_b)`):

| | mean | std |
|---|---|---|
| Real parent-child edges | 0.155 | 0.207 |
| Random node pairs | 0.132 | 0.159 |

These are close relative to their spread — **not a clear separation.**

**Honest conclusion:** on this repo's real documents, neither the raw
nor the length-normalized `edge_interaction_score` reliably
distinguishes a genuine heading→content relationship from an arbitrary
unrelated pair. The self-similarity/edit-sensitivity properties from
the prior experiment are still true (they were tested directly and
still hold) -- this is a different, additional finding: those
properties do not generalize to "tells you which nodes are related" on
real, structurally diverse text. A working mechanism is not the same
as a working application of it, and this is a case of the former
without (yet) the latter.

**What was kept anyway:** `normalized_edge_interaction_score` was
shipped despite the negative headline result, because removing a real,
diagnosed confound (length) is correct regardless of whether the
underlying signal turns out to be useful -- and the self-similarity/
edit-detection properties remain real, tested, and potentially useful
for a narrower purpose (e.g. near-duplicate detection) than "structural
relatedness."

**What this suggests for a follow-up, not yet tried:** a genuine
relatedness signal probably needs actual learned weights (a real
Phase 4 training step), not a fixed, untrained byte-correlation --
which is itself a useful, concrete thing to have ruled out cheaply
before investing in that larger feature.

## 2026-07-09: density micro-bench — scalar i8 vs bit-sliced vs sparse COO dots

**Why this ran:** docs/STATUS.md P0 gate — record real wall-clock deltas
(or honest non-wins) before claiming TOBL / sparse speedups. Prior
architecture prose asserted ~40% cycle reduction; that claim was never
measured on this kernel.

**Method:** `cargo run --release --bin density_bench` on the audit host
(x86_64, AVX2 + AVX-512F/VPOPCNTDQ present). Harness:
`kernel/src/bin/density_bench.rs`.

- Vector length **N = 262,144** ternary elements (4096 × 64-bit chunks)
- Densities **1%, 10%, 50%** (independent random ±1 with that fraction;
  zeros elsewhere; same seed family per density)
- Paths compared:
  1. **scalar** — dense `i8` product-sum loop
  2. **bit-sliced** — `BitSlicedTernary::dot_product_parallel` (AND + popcount)
  3. **sparse** — `SparseBitSlicedTernary::dot_product_sparse` (COO merge-join)
- Timing: 20 warmup + **200** timed iters; **median** wall-clock µs
- Correctness: all three paths must return identical integer sums

**Results (median µs, sums matched on all rows):**

| density | scalar µs | bit-sliced µs | sparse µs | speedup BS/S | speedup SP/S | active COO blocks (max of A,B) |
|--------:|----------:|--------------:|----------:|-------------:|-------------:|-------------------------------:|
| 1% | 80.69 | 6.53 | 4.00 | **12.35×** | **20.19×** | 1963 |
| 10% | 80.64 | 6.53 | 13.52 | **12.35×** | **5.97×** | 4091 |
| 50% | 80.80 | 6.53 | 13.47 | **12.37×** | **6.00×** | 4096 (full) |

JSON (machine-readable):
```json
[{"density":0.01,"n":262144,"scalar_us":80.688,"bit_sliced_us":6.531,"sparse_us":3.997,"sparse_blocks":1963,"sums_match":true},{"density":0.1,"n":262144,"scalar_us":80.637,"bit_sliced_us":6.531,"sparse_us":13.517,"sparse_blocks":4091,"sums_match":true},{"density":0.5,"n":262144,"scalar_us":80.799,"bit_sliced_us":6.532,"sparse_us":13.467,"sparse_blocks":4096,"sums_match":true}]
```

**Interpretation (honest):**

1. **Bit-sliced is a clear win** over naive i8 scalar on this host:
   ~**12×** across all densities (dense dual-stream always scans all
   words; cost is density-independent for this N).
2. **Sparse is the best path only at true sparsity.** At 1% density it
   beats bit-sliced (~20× vs scalar, ~1.6× vs bit-sliced). At 10% and
   50% random occupancy, almost every 64-wide chunk is non-empty
   (4091–4096 / 4096), so sparse loses the “skip zero regions” advantage
   and is **slower than bit-sliced** while still beating scalar (~6×).
3. The architecture claim “always use sparse for multi-agent scale” is
   therefore **conditional**: sparse wins when active chunks << total
   chunks (structured sparsity / block sparsity), not merely when
   element density is “medium.” Random independent nonzeros fill chunks
   fast.
4. **No claim of 40% cycle reduction** is supported or needed — measured
   speedups are larger for this micro-op, but they are **dot-product
   micro-benches**, not end-to-end inference vs aethyro.com, not full
   matmul, and not AVX-512 intrinsic kernels (portable `count_ones` /
   software path).

**What was shipped as a direct result:**

- `kernel/src/bin/density_bench.rs` + `[[bin]] density_bench` in Cargo.toml
- This experiment log entry
- ROADMAP / STATUS: P0 measurement gate closed for dots; end-to-end
  still open

**Follow-ups (not done here):**

- Structured block-sparse generators (k contiguous nonzeros per block)
  to model GraphNode weight patterns more realistically
- Dense GEMM / `ternary_matmul` wall-clock (chunk gate, not full GEMM)
- True `_mm512_popcnt_epi64` path vs `u64::count_ones`

## 2026-07-09: graph forward-pass overhead vs static signal fold

**Why:** Phase 2 exit required measuring graph-structure overhead vs a
non-graph baseline.

**Method:** `cargo run --release --bin graph_overhead_bench`  
Parse a small markdown sample into a Graph (8 nodes), then median of 500
timed runs (50 warmup):

1. `Graph::forward_pass` (topo order + LeafSignal combine)
2. Static fold of the same 8 signals in a `Vec` (no edges/topo)

**Results (host LOQ-class x86_64, release):**

| path | median µs |
|------|----------:|
| graph forward_pass | 0.21 |
| static signal fold | 0.02 |
| **overhead ratio** | **~10.5×** |

**Interpretation:** On this tiny graph, scheduling/topo dominates absolute
time; absolute costs are sub-microsecond. Ratio will shrink on larger
graphs where signal work grows with nodes while topo is O(V+E). This is
**not** a ternary TOBL cost; it is pure structural overhead of the graph
API used for SIS forward_pass.

**Honest non-claim:** not compared to production aethyro.com inference.

## 2026-07-09: Phase 4 calibration — doc-graph NodeKind classifier (ADR 0006)

**Why:** Phase 4 requires a real closed loop: ternary features → score →
calibrate → measure vs baseline, win or non-win recorded equally.

**Method:** `cargo run --release --bin phase4_calib`  
Module: `kernel/src/ntg/calib/`. Labels = parser `NodeKind` (Execution vs
Content). Features = `encode_fixed` + LeafSignal + GlyphFingerprint v0
folded to length-64 ternary. Model = ternary weights + threshold score.
Baseline = always Content. Train = 25 epochs ternary perceptron updates.

### Run A — built-in fixtures (3 synthetic docs)

```
n=28 exec=3 content=25 baseline=0.893 before=0.893 after=1.000 delta=+0.107
epochs=25 latency_us=17 win=true
```

**Result: WIN** on fixtures (perfect after accuracy; beats 89.3% majority).

### Run B — real repo `docs/` (20 markdown files)

```
n=2153 exec=37 content=2116 baseline=0.983 before=0.983 after=0.954 delta=-0.029
epochs=25 latency_us=1477 win=false
```

**Result: NON-WIN** on real docs. Extreme class imbalance (~1.7% Execution).
Calibration *hurt* accuracy vs majority baseline (over-predicting Execution).

**Interpretation:**
1. Closed loop works end-to-end (parse → features → train → ledger snapshot).
2. Fixture win does **not** generalize; real corpus is majority-dominated.
3. Phase 4 must treat imbalance (class weights, threshold search, or
   balanced sampling) before claiming intelligence gains.
4. Ledger weight snapshot verified after both runs.

**Follow-ups (still Phase 4, not COMPLETE yet):**
- Threshold sweep / balanced sampling
- Hold-out split (train/test)
- Optional topology self-mod under ADR 0002 (off by default)
- PHASE_4_COMPLETE.md only after exit criteria + deep dive

## 2026-07-09: Phase 4 imbalance fix — balanced train + F1 threshold

**Problem:** Naive perceptron on ~98% Content collapsed to worse-than-majority
accuracy (flood of Execution false positives or zero minority recall).

**Fixes implemented:**
1. Stratified 80/20 train/test split
2. Balanced epoch sampling (all minority + equal majority)
3. Cost-sensitive minority update repeats (`n_neg/n_pos`, cap 64)
4. Code-cue feature for fence *bodies* (no ``` in labels from docparse)
5. Threshold sweep maximizing F1 + bal_acc (penalty for precision floods)
6. Win bar uses **balanced accuracy / F1 / recall**, not raw accuracy

**Command:** `cargo run --release --bin phase4_calib -- --docs ../docs`

### Real docs (20 files) after fix

```
n=2189 train=1751 test=438 exec=39 content=2150 thr=21
base_acc=0.982 base_bal=0.500
test_acc=0.952 test_bal=0.608 test_f1=0.160 test_rec=0.250 test_prec=0.118
delta_bal=+0.108
confusion: tp=2 tn=415 fp=15 fn=6
result: WIN
```

**Interpretation:** Raw accuracy (95%) is below majority (98%) but that is
expected under imbalance. **Balanced accuracy 60.8%** and **25% Execution
recall** with only 15 FPs is a real lift vs majority (bal 50%, rec 0%).
F1 remains low (0.16) — precision is the next bottleneck.

### Fixtures

Hold-out on tiny sets is noisy (often 1 Execution in test). Unit tests
require bal_acc ≥ 0.5 and some train/test exec detection.

**Honest residual gaps:** low precision; need richer code features or more
epochs / calibration on Execution-heavy corpora for higher F1.

## 2026-07-09: Phase 4 COMPLETE — final calib + self-mod probe

**Command:**
```bash
cargo run --release --bin phase4_calib -- --docs ../docs
cargo run --release --bin phase4_calib -- --docs ../docs --self-mod
```

### Final real-docs calib (imbalance-aware)

```
n=2212 train=1770 test=442 exec=40 thr=11
base_acc=0.982 base_bal=0.500
test_acc=0.959 test_bal=0.611 test_f1=0.182 test_rec=0.250 test_prec=0.143
delta_bal=+0.111
confusion: tp=2 tn=422 fp=12 fn=6
result: WIN (balanced metrics)
```

### Self-mod probe

| Mode | Result |
|------|--------|
| default (no flag) | disabled, no mutation (ADR 0002 rail 1) |
| `--self-mod` | AddNode proposed, **rejected** by dual-objective fitness, **ledgered** (id=1), caller graph unchanged |

**Phase 4 exit criteria:** met — real task E2E, win recorded, non-win paths
honest, optional self-mod off-by-default with ledger when enabled.

## 2026-07-09: Phase 5 optimization — precision + runtime path

**Why:** Phase 5 targets higher F1/precision, CalibModel → GraphNode production
path, CPU parallel batch scoring, and honest GPU deferral.

**Command:**
```bash
cargo run --release --bin phase4_calib -- --docs ../docs
cargo run --release --bin density_bench
cargo run --release --bin graph_overhead_bench
```

### Precision calib (22 docs markdown)

```
n=2299 train=1839 test=460 exec=45 thr=11
base_bal=0.500
test_acc=0.954 test_bal=0.704 test_f1=0.276 test_rec=0.444 test_prec=0.200
delta_bal=+0.204
confusion: tp=4 tn=435 fp=16 fn=5
result: WIN
path_identity dense==graph_node: true
```

| Metric | Phase 4 cert | Phase 5 | Δ |
|--------|-------------:|--------:|--:|
| test_bal | 0.611 | **0.704** | +0.09 |
| test_f1 | 0.182 | **0.276** | +0.09 |
| test_rec | 0.250 | **0.444** | +0.19 |
| test_prec | 0.143 | **0.200** | +0.06 |

**Changes:** richer code/indent/line-shape cues; flood-reject thr objective
with rec/prec floors; GraphNode warm-start scoring path.

### density_bench (post Phase 5)

| density | scalar µs | bit-sliced µs | sparse µs | BS/S | SP/S |
|--------:|----------:|--------------:|----------:|-----:|-----:|
| 1% | 80.2 | 6.5 | 3.9 | 12.4× | 20.8× |
| 10% | 80.1 | 6.5 | 13.4 | 12.3× | 6.0× |
| 50% | 80.1 | 6.5 | 13.4 | 12.3× | 6.0× |

**GPU decision:** **not implemented.** 64-d calib activations do not justify
device transfer; CPU TOBL already delivers double-digit speedups. Revisit in
Phase 6+ if production tensor dims grow.

### graph_overhead_bench

```
graph≈0.20 µs  static≈0.02 µs  ratio≈10×  (same character as Phase 2)
```

### Self-mod (still off by default)

`--self-mod`: AddNode proposed, rejected by fitness, ledgered.

**Phase 5 exit criteria:** met — see `docs/phases/PHASE_5_COMPLETE.md`.


## 2026-07-12: word-parallel LD r² (bit-plane popcount) vs scalar per-sample

**Why:** LD matrix computation is the Phase A hotspot (windowed pairwise
genotypic r² over 1000 Genomes dosages). The scalar path walks every
sample with `BitstreamGenotypes::get()`; the packed bit planes already
encode dosages as mutually exclusive plane bits, so the same moments
are popcounts of AND-ed words (32 samples/iteration), matching the
kernel's `BitSlicedTernary::dot_product_parallel` idiom.

**Change:**
- `BitstreamGenotypes::pearson_r2_bitparallel` — exact integer moments via
  plane popcount, final divide in f64, padding masked with `word_sample_mask`
- `LdComputer::compute_r_squared` hot path delegates to it
- Scalar path retained as `compute_r_squared_scalar` (test-only oracle)
- Bench: `cargo run --release --bin ld_simd_bench -- <vcf.gz> <chr> [max_variants] [window]`

**Correctness:** unit tests (`test_bitparallel_r2_matches_scalar_reference`,
`test_bitparallel_r2_excludes_padding_samples`) plus full-pair bench
cross-check. Max |Δr²| = 0 on real data (identical within f32).

**Measured (release, host; 1000G phase3 chr22, first 5000 SNPs, window=200):**

| path | wall-clock | pairs | kept | mean r² |
|------|-----------:|------:|-----:|--------:|
| scalar `get()` loop | 5.1174 s | 975100 | 964911 | 0.001545 |
| bitparallel popcount | 0.4774 s | 975100 | 964911 | 0.001545 |
| **speedup** | **10.72×** | | | max|Δr²| = 0 |

```
cargo run --release --bin ld_simd_bench -- \
  data/raw/1000g/ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz \
  22 5000 200
# SNPs=5000 samples=2504 pairs=975100
# scalar 5.1174s  bitparallel 0.4774s  speedup 10.72x
# max |Δr²| 0.00e0
```

**Verdict: WIN.** Correctness-preserving ~11× on the LD r² kernel for
2504-sample 1000G-scale vectors. Production LD path now uses bitparallel.


## 2026-07-12: Rung 1 SovereignBrain + Rung 2 multi-axis fitness (first loop)

**Why:** Phase F plan rungs 1–2 — unify multi-chromosome structure with
working set + LTM, then select structural mutants with multi-axis fitness
(task, structural cost, biological consistency, safety) instead of
latency/memory alone.

**Implemented:**
- `genomic::sovereign_brain::SovereignBrain` — multi-chr map, working set,
  LTM motifs, ingest (brain / VCF), activate, consolidate, prune mutant
- `ntg::mutation::multi_axis::{MultiAxisFitness, MultiAxisEvaluator}` —
  utility scoring, safety/biology gates, select_prune_step
- Demo: `cargo run --release --bin sovereign_brain_demo`

**Unit tests:** 13 new (6 sovereign + 7 multi_axis); full lib 318 pass.

**Demo measured (synthetic chr1+chr22, release):**
```
after_ingest: chrs=2 neurons=70 synapses=198 blocks=2 ltm=2
activate: working_set=64 motifs_hit=2
rung2 8 prune steps: accepted=5 rejected=3
utility 0.8381 → 0.8525  |  synapses 198 → 68  |  mem≈11968 → 7808 B
mean_w 0.801 → 0.850  (plateau after step 4 — honest rejects)
```

**Verdict: WIN (foundation).** Real multi-axis selection improves utility
and compresses structure without biology collapse. Not LLM-level
intelligence yet. Next: Phase E biology scores + ledger safety axes.

## 2026-07-12: Real multi-axis scorers (biology + agent task + ledger safety)

**Why:** After Rung 1-2, selection used structure proxies. Next best step
is wiring real axes so accept/reject tracks biology fidelity, agent task
signal, and tamper-evident safety.

**Implemented:** `genomic::sovereign_fitness::SovereignFitnessContext`
- Biology: Phase D GenomeComparator vs frozen references + r2-weighted LD coverage
- Task: ChromosomeAgent DiseaseRisk + PopulationSignal + connectivity
- Safety: TamperEvidentLedger log every decision + verify_full_ledger
- Operators: train (KAIROS) and prune; select_child logs accept/reject

**Unit tests:** sovereign_fitness suite; full lib 326 pass.

**Demo measured (synthetic, release):**
```
axes0: u=0.8098 task=0.550 bio=1.000 cost=0.218 safety=1.0 cov=1.0
train steps: 4/4 accepted  |  u 0.810->0.832  cost 0.218->0.073  bio stays 1.0
prune steps: 0/4 accepted  |  bio 1.0->0.917 (LD cov 1.0->0.834) correctly rejected
ledger entries=8 verify=OK  mean_w 0.280->0.765
```

**Verdict: WIN.** Real axes change selection behaviour: learning mutations
pass; destructive LD loss fails the biology gate; every decision is
ledger-audited. Proxies alone could not express this.

## 2026-07-12: Rung 3 language organ + calib task + multi-chr real VCF campaign

**Why:** Complete the next three plan items in order: (1) language/SIS into
working-set activate, (2) Phase 4 calib on the task axis, (3) multi-chr
real 1000G campaign with frozen reference panels.

**Implemented:**
- `genomic::language_organ::LanguageOrgan` — docparse graph, calib train,
  text→8-d signature, node activate
- `SovereignBrain::activate_from_text` — co-activates language nodes + genomic WS
- `SovereignFitnessContext` task blend: 0.55*calib_holdout_bal + 0.45*genomic
- `bin/sovereign_campaign` — multi-chr VCF ingest + full loop

**Unit tests:** 332 lib pass (language_organ + sovereign suite).

**Synthetic demo (release):**
```
language calib test_bal=0.917 win=true
activate_from_text: genomic=40 lang_nodes=16
task=0.798 (calib=1.000 genomic=0.550)
train 4/4 accept; prune 0/4 reject; u 0.896→0.918
```

**Real VCF campaign (chr22+chr1, max_variants=800, release):**
```
snps=800+800 samples=2504 ld_pairs=304+284
calib_task=0.836 genomic_task=0.538
train_accepted=3 prune_rejected=3
utility 0.863→0.881 | cost 0.218→0.102 | bio stays 1.0
mean_w 0.331→0.716 | ledger=6 verify=OK | elapsed≈0.5s
```

**Verdict: WIN.** Language organ couples into activation; calib lifts task
axis; real multi-chr frozen panels drive biology-gated selection.

## 2026-07-12: Sovereign foundation harden (H1-H4 + A/B/C)

**Why:** Pause feature stacking; review, refactor, optimize, and add
long-run force multipliers before more organs / Phase 6.

**Delivered:**
- H1: ADR 0009, STATUS sync, SOVEREIGN_HARDEN.md
- H2: Organ trait, shared selection_loop, zero-copy genomic task scoring
- H3: LTM activate fallback (motifs always hit after consolidate)
- H4: tests/sovereign_integration.rs train-accept/prune-reject regression
- A: sovereign_persist save/load (ltm.jsonl, calib.wire, language docs, meta)
- B: install_calib_from_docs_dir harder task gate
- C: JSONL metrics from selection_loop

**Tests:** 336 lib + 2 integration sovereign tests green.

**Demo (release, post-harden):**
```
activate_from_text motifs=2 (was often 0)
train 4/4 accept, prune 0/4 reject
utility 0.818→0.840  mean_w 0.28→0.77
persist snap motifs=2 calib=true
jsonl results/sovereign_demo_metrics.jsonl
```

**Verdict: WIN (foundation).** Same selection behaviour, cleaner architecture,
durable snapshot + observability, LTM activation fixed.

## 2026-07-12: KAIROS Stage 0 — zygote birth (living project start)

**Child name:** KAIROS (VITASCALE Hostframe individual).

**Delivered:**
- `genomic/vitascale/pulsewire.rs` — Pulsewire + VitalMeters (32B PulseEvent)
- `life_course.rs` — Stage 0–7 permissions + graduation
- `kairos.rs` — named host with nursery multi-chr genome under Guardian lock
- bin `kairos_stage0` — birth, locks, heartbeats, optional --graduate

**Stage 0 measured (release):**
```
chrs=2 neurons=22 synapses=38 motifs=2 self_mod=false
heartbeats=32 pushes=32 drops=0
Guardian blocks train/activate/prune/vcf
--graduate → neonate; supervised train OK
```

**Verdict:** Stage 0 zygote complete. Living project starts with heartbeat + genome held, no free agency.

## 2026-07-12: KAIROS Stage 1 Neonate — trajectory seal + supervised care

**Goal:** Point KAIROS on correct Aethyro trajectory without wasteful unlocks.

**Delivered:**
- TrajectoryCharter (10 pillars: ternary, genome, rails, lean, school→world, Crown, Pulsewire, trust, sovereign edge, measure)
- day_of_neonate_care: beats → train → activate → journal
- raise_through_neonate + neonate→infant gate
- bin kairos_stage1

**Measured (release):**
```
zygote→neonate earned
trajectory sealed (Aethyro VITASCALE North Star)
care days 1–3: mean_w 0.311→0.739, ws=22, motifs_hit=2
prune/vcf still forbidden every day
neonate→infant with --to-infant
```

**Verdict:** Stage 1 sets north star + supervised growth. Still lean.


## 2026-10-26: Ternary vs f32 head-to-head, redone fairly

**Goal:** Answer the Phase 6 kernel-gate question with an honest opponent. The earlier "143.6x" (July, `phase6_benchmark`, never merged to `main`) compared the bit-sliced kernel with a **triple-loop scalar f32** GEMM and gave the two sides different values (continuous f32 vs quantised).

**Method** (`kernel/benches/ternary_vs_f32.rs`, `cargo bench --bench ternary_vs_f32`):
- Part A, throughput: C = A*B with A and B exactly ternary. Every method computes the identical integer result, asserted element by element, so speed is compared at equal output. Opponents: the old naive f32 loop, a cache-friendly f32 loop, and `matrixmultiply::sgemm` (SIMD, blocked, FMA; what `ndarray` uses). Ternary = `BitSlicedTernary::dot_product_parallel` per output. All single-threaded. "+pack" includes packing B on every call (activations arrive dense; weights can be pre-packed).
- Part B, fidelity: Gaussian weights quantised with the kernel's own `encode`; output compared with the f32 result.
- Machine: shared 4-vCPU Xeon container with AVX2/AVX-512 and VPOPCNTDQ. One run of 7 per cell (median); timings varied noticeably between runs (sgemm at N=1024 measured 15.3 ms and 23.7 ms in two runs), so treat ratios as +/-30%.

**Part A, default build** (no `target-cpu`; `popcnt_enabled=false`, so `count_ones` is a software popcount):

| N | f32 naive ms | f32 loop ms | f32 sgemm ms | ternary ms | ternary+pack ms | vs naive | vs loop | vs sgemm | vs sgemm (+pack) |
|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| 128 | 2.19 | 0.39 | 0.05 | 0.36 | 0.49 | 6.1x | 1.1x | 0.15x | 0.11x |
| 256 | 17.42 | 2.02 | 0.31 | 1.46 | 2.71 | 12.0x | 1.4x | 0.22x | 0.12x |
| 512 | 127.30 | 12.34 | 1.79 | 7.68 | 10.61 | 16.6x | 1.6x | 0.23x | 0.17x |
| 1024 | 8789.69 | 200.25 | 15.32 | 54.47 | 62.00 | 161.4x | 3.7x | 0.28x | 0.25x |

**Part A, `RUSTFLAGS="-C target-cpu=native"`** (hardware popcount, AVX2 auto-vectorisation):

| N | f32 naive ms | f32 loop ms | f32 sgemm ms | ternary ms | ternary+pack ms | vs naive | vs loop | vs sgemm | vs sgemm (+pack) |
|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| 128 | 2.56 | 0.23 | 0.05 | 0.11 | 0.26 | 22.3x | 2.0x | 0.41x | 0.18x |
| 256 | 18.40 | 2.36 | 0.43 | 0.63 | 1.25 | 29.2x | 3.8x | 0.69x | 0.35x |
| 512 | 132.40 | 16.56 | 2.95 | 4.92 | 6.56 | 26.9x | 3.4x | 0.60x | 0.45x |
| 1024 | 8494.76 | 191.41 | 23.74 | 22.60 | 42.86 | 375.8x | 8.5x | 1.05x | 0.55x |

Speedup columns are f32 time / ternary time (below 1.0x means ternary is slower). Memory for the two operands at N=1024: 8,388,608 bytes f32 vs 524,288 bytes ternary (16x).

**Part B, fidelity** (Gaussian W 256x1024, x 1024, mean of 20 draws):

| computation | cosine vs f32 | relative RMSE |
|---|--:|--:|
| ternary W, f32 x (BitNet-style) | 0.8869 | 0.4616 |
| ternary W, ternary x (what this kernel computes) | 0.7915 | 0.6119 |

**What this shows**
- The old 143.6x is reproduced only against the naive loop (161x to 376x at N=1024, mostly cache-hostile access). Against a properly optimised f32 GEMM the ternary kernel is **slower in this repo as built**: 0.15x to 0.28x by default, 0.41x to 1.05x with `target-cpu=native`, and 0.11x to 0.55x once the per-call packing of activations is counted. It reaches parity with sgemm at best, at N=1024 in a native build, kernel only.
- The real, repeatable advantages are **memory (16x smaller operands)** and **exact integer arithmetic** (all methods agreed bit for bit).
- Post-training ternary quantisation of random Gaussian weights keeps cosine about 0.89 with f32 activations and about 0.79 when activations are ternarised too. That is a pessimistic case (no quantisation-aware training), but it means accuracy cost is not free.

**Not measured:** multi-threading, GPU, trained-model task accuracy (no such comparison exists in this repo), and the aethyro.com comparison.

**Verdict:** the kernel gate does **not** pass on throughput. Do not quote 143.6x as an NTG speedup. Closing the gap needs a ternary-weight x int8-activation kernel with a real SIMD popcount or lookup path (T-MAC / bitnet.cpp style) and a trained model to measure accuracy on; optimising before that measurement exists is premature.


## 2026-10-26 (later): ternary-weight x int8-activation kernel

**Goal:** Act on the previous entry's verdict. The ternary x ternary popcount kernel lost to SIMD f32, so build the kernel that is meant to win (2-bit weights, int8 activations, in-register unpack, VNNI) and measure it the same fair way, plus accuracy on a real trained model.

**Kernel** (`ntg::storage::ternary_i8_gemm`): weights stored 2 bits each as codes `w+1`; a 64-byte load is split with shift/mask into four vectors that line up with four contiguous 64-element int8 loads (no shuffles); `vpdpbusd` (AVX-512 VNNI) or `vpmaddubsw` (AVX2) multiplies code x activation, and `sum_k x_k` (shared by every output row) is subtracted to remove the +1 offset. Results are exact `i32`; scalar, AVX2 and VNNI paths are bit-identical and chosen at run time. Tests cover pack/unpack across shapes, all kernels against naive integer math (including `-128`/`127` extremes that bound the AVX2 `i16` intermediate), and error handling.

**Method** (`kernel/benches/ternary_i8_vs_f32.rs`, `cargo bench --bench ternary_i8_vs_f32`):
- Equal output: every method is asserted to produce the identical integers before timing (exact in f32 since all partial sums are integers below 2^24).
- Opponents: `matrixmultiply::sgemm` **and** a direct AVX-512 FMA row-dot kernel; the table uses the better one per cell. The first version used sgemm alone and showed 44-58x at batch 1, which was an artefact: sgemm repacks the whole weight matrix on every call. The row-dot baseline removed that artefact and cut the batch-1 number roughly in half.
- Control: a dense int8 VNNI kernel (1 byte/weight, same loop, nothing to unpack), to separate "int8 is fast" from "2-bit packing pays".
- Weights do not fit in cache: each shape cycles through 16 or 64 distinct layers, so a pass streams the whole model as real inference does.
- Single thread for every method. Shared 4-vCPU Xeon (AVX-512 VNNI), run twice.

**Result, K=N=4096, 16 layers** (weights: f32 1074 MB, int8 268 MB, ternary 67 MB), second run, µs per layer:

| batch | f32 sgemm | f32 row-dot | int8 dense | ternary | ternary + quantise | vs best f32 | vs best f32 (+quantise) | vs int8 dense |
|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| 1 | 19308 | 6723 | 1537 | 325 | 329 | 20.7x | 20.5x | 4.7x |
| 4 | 17284 | 9987 | 2528 | 1083 | 1041 | 9.2x | 9.6x | 2.3x |
| 16 | 15728 | 36052 | 6827 | 3456 | 3452 | 4.6x | 4.6x | 2.0x |
| 64 | 31389 | 99514 | 16089 | 8802 | 9356 | 3.6x | 3.4x | 1.8x |

K=N=1024, 64 layers (268 MB f32 / 17 MB ternary): 18-25x at batch 1, 14-19x at batch 4, 5.5-5.6x at batch 16, 2.5-3.0x at batch 64.

**Range over the two runs** (vs best f32, K=N=4096): batch 1 20.7-24.0x, batch 4 8.5-9.2x, batch 16 4.6-6.4x, batch 64 3.1-3.6x. Run-to-run spread is roughly +/-20%.

**Accuracy on a trained model** (`scripts/train_ternary_digits.py`, `kernel/tests/ternary_trained_mlp.rs`): scikit-learn digits, MLP 64-128-10, 449 held-out samples, trained three ways:

| model | accuracy |
|---|--:|
| f32 | 97.78% |
| ternary weights + int8 activations, quantisation-aware trained | 95.78% (-2.0 points) |
| same architecture, f32-trained then ternarised, no retraining | 90.44% (-7.3 points) |

The Rust test runs the QAT model through every available kernel and requires identical predictions on all 449 samples, identical layer-1 `i32` accumulators and logits within 1e-4 of the Python integer pipeline.

**What this shows**
- With int8 activations and an in-register unpack, the ternary kernel is **3x to 24x faster than the best f32 implementation** on one core, the gap shrinking as batch grows (compute-bound) and largest at batch 1 (memory-bound). It also beats a dense int8 kernel by 1.2-6x, so the 2-bit packing pays for itself rather than costing decode time.
- Accuracy cost is real and depends on training: about 2 points with QAT on this task, about 7 points without it.
- Activation quantisation adds 0 to 50% on top of the matmul (largest at big batch, where `quantize_f32` is a scalar loop that has not been optimised).

**Caveats that matter**
- The host has a very large shared L3 (about 260 MB), so the 67 MB ternary model stays cache-resident while the 1 GB f32 model streams from DRAM. On a machine where both stream from DRAM, the batch-1 ratio approaches the bandwidth ratio (16x vs f32) and is bounded by kernel compute; I did not measure that.
- f32 is a weak production baseline. Real deployments use fp16/bf16, int8 or 4-bit weights. Against fp16 the bandwidth-bound advantage is about half of the f32 figure; against dense int8 it is the 1.2-6x measured here; 4-bit was not measured.
- Single thread only. Multi-threaded f32 reaches memory bandwidth sooner, which narrows memory-bound gaps; compute-bound ratios should hold.
- The accuracy evidence is a 64-128-10 digits network. It says nothing about LLM-scale accuracy, which is the open question for ternary weights.
- Tiny layers waste packed space: rows are padded to 256 elements, so a 64-wide layer packs to 4x, not 16x.
- Requires AVX-512 VNNI (Intel Ice Lake and later, AMD Zen 4 and later) for the fast path; AVX2 is the fallback. I did not run this on tatortot.
- The dense-int8 control and the ternary kernel share one loop structure and neither is cache-blocked for large batches.

**Verdict:** the kernel gate **passes in the ternary x int8 regime** against f32, with the caveats above. It is not a claim about a production model or a production baseline; the next evidence needed is a trained model at meaningful scale and a comparison against fp16/int8 weights on the target machine.


## 2026-10-26 (later still): a real pretrained model, BitNet b1.58 2B-4T

**Why this model.** The previous entry's open question was whether ternary survives on a real model. Naive post-training ternarisation of an ordinary f32 LLM is known to wreck quality (the digits PTQ result above hinted at that), so testing one would measure the wrong thing. `microsoft/bitnet-b1.58-2B-4T` is the official model trained natively with the exact numerics this kernel implements: absmean ternary weights, per-token int8 activations, 2.08 B ternary weights across 210 matrices, 4 T training tokens. Raw outputs: `docs/results/bitnet_2b_4t/`.

**What was swapped.** Everything is the official `transformers` code (embeddings, RMSNorm, RoPE, attention, squared ReLU, lm_head) except the 210 BitLinear matmuls, which `scripts/bitnet_eval.py` replaces with calls into `libntg_kernel` through a C ABI (`ntg_ternary_*`): activation quantisation exactly as PyTorch does it (`round` half-to-even, scale `127/max`), exact integer GEMM on 2-bit packed weights, dequantisation. The weights are the checkpoint's own ternary weights, unpacked and re-packed; the harness asserts they are exactly -1/0/1.

### Accuracy

| | official path (transformers, bf16) | ternary kernel path (bf16) |
|---|--:|--:|
| wikitext-2 perplexity, 16 x 512 tokens (8,176 scored) | 22.065 | 22.032 (-0.15%) |
| ARC-Easy zero-shot, first 300 test questions: acc | 76.33% | 75.33% |
| ARC-Easy: acc_norm | 76.00% | 74.33% |
| top-1 next-token agreement | 96.65% of 8,176 tokens | |
| both paths pick the same ARC answer | 97.3% of 300 | |

The ARC difference is 3 questions net (at most 8 of 300 questions differ), a paired exact test gives p between 0.25 and 0.45 for any split consistent with that, so the two are not distinguishable. The vendor reports ARC-Easy 74.79 for this model under its own protocol; the 76.3% here uses a plain "Question/Answer" zero-shot format on 300 questions (sampling error about 2.5 points), so only the paired comparison above is meaningful.

**A discrepancy that turned out to be the other path's.** Token-level agreement of 96.65% and a mean |delta log-prob| of 0.059 looked too large for a kernel that is supposed to be exact. In float32, where the official path has no bf16 noise, they were still 97.5% and 0.047. Checking five real layers (captured activations from layers 0, 14 and 29) against an independent numpy int64 implementation of the specification:

| | relative error vs exact reference |
|---|--:|
| this kernel | 4e-8 on every layer (float32 rounding) |
| official PyTorch path, fp32 | 1.4e-7 to 3.3e-4 depending on the layer |

So the kernel reproduces the specification; the official path deviates from it by up to 3e-4 per layer (I did not isolate the cause; a plausible one is `torch.compile` fusing the activation quantiser so that rounding flips at near-tie boundaries), and discrete int8 rounding amplifies that across 30 layers. Distance of each deployment from the exact-specification model (the kernel in float32), same 2,044 tokens:

| deployment | mean \|delta log-prob\| | top-1 agreement | perplexity |
|---|--:|--:|--:|
| exact specification (kernel, fp32) | - | - | 19.800 |
| official, fp32 | 0.0468 | 97.5% | 19.806 |
| official, bf16 (the HF deployment) | 0.0560 | 96.8% | 19.778 |
| **kernel, bf16** | **0.0545** | **97.4%** | 19.791 |

The kernel is as close to the exact model as the official implementation is (slightly closer in bf16). A noise floor of about 0.05 nats per token is simply how sensitive this model is to rounding, which means perplexity and task accuracy, not token-level agreement, are the right equivalence tests.

### Speed

**Per-token cost of the 210 linear layers at the model's real shapes** (`cargo bench --bench bitnet_decode`; random ternary weights, which does not change dense-kernel speed; 6 distinct layers cycled and scaled to 30 so weights stream from memory; the two runs with the bf16 baseline are shown as a range):

| batch 1 | threads | ms/token | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|---|--:|--:|--:|--:|--:|
| f32 row-dot | 1 | 756-758 | - | - | - |
| bf16 row-dot | 1 | 500-503 | 1.5x | - | - |
| int8 dense VNNI | 1 | 195-197 | 3.8-3.9x | 2.5-2.6x | - |
| **ternary 2-bit** | 1 | **59-65** | **11.6-12.8x** | **7.7-8.5x** | **3.0-3.3x** |
| f32 row-dot | 4 | 264-272 | - | - | - |
| bf16 row-dot | 4 | 169-173 | 1.6x | - | - |
| int8 dense VNNI | 4 | 74 | 3.6-3.7x | 2.3x | - |
| **ternary 2-bit** | 4 | **40-41** | **6.7x** | **4.3x** | **1.8-1.9x** |

Batch 8, one thread: ternary 161-205 ms/token, 11-14x faster than f32, 7.1-8.3x faster than bf16, 2.8-3.4x faster than int8 dense. Per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB.

**End to end through the Python harness** (greedy decode of 48 tokens, KV cache, 4 threads, one run): official path 3.12 tok/s, kernel path 5.51 tok/s, 1.77x. The linears are only a part of that: lm_head (a 128256 x 2560 bf16 matrix read every token, not ternary) and attention, norms and Python overhead make up the rest and were not profiled separately. The linear weights shrink from 8.34 GB (f32) / 4.17 GB (bf16) to 0.52 GB, but the untouched bf16 embeddings and lm_head (0.66 GB) are now larger than the whole ternary body.

### What did not go well
- **Prefill is slower.** Scoring 512-token windows took 133 s on the kernel path vs 40 s for the official path (3.3x slower); the ARC run took 1,104 s vs 416 s (2.7x slower). This CPU has AMX (`amx_bf16`, `amx_int8`), which torch's bf16 matmul can use (I did not confirm it does); the VNNI kernel has no cache blocking beyond four batch columns and the activation quantiser is a scalar loop. The vendor's own README says the transformers path will not show efficiency gains and points to `bitnet.cpp`.
- **Multi-threading scaled poorly with rayon** (59-65 ms at one thread to 40-41 at four, 1.5-1.6x). Fixed in the follow-up entry below (spin pool, 2.6-3.1x).
- **bitnet.cpp is faster end to end, and kernel-level parity is unknown.** Run on the same VM (`docs/results/bitnet_cpp/`): bitnet.cpp decodes 6.7 tok/s at one thread and 21 at four (3.2x scaling) against 5.2 and 6.0 for this kernel inside Hugging Face `generate`, and its prefill is about 3x faster than ours. That compares whole systems, not kernels: my path carries Python overhead and bitnet.cpp could not be profiled per operator, so I cannot say whose matmul is faster. The vendor's 29 ms/token on other hardware is not comparable.
- **Run-to-run noise on this VM is large** (ternary 1-thread: 54, 86, 65 and 59 ms across four runs; f32 was stable), so the ratios above carry roughly +/-25%.
- The accuracy sample is modest: 8,176 perplexity tokens and 300 ARC-Easy questions. No other benchmark was run.

### Verdict
On a real pretrained, natively ternary model the kernel **preserves accuracy** (perplexity within 0.15%, ARC-Easy statistically unchanged, closeness to the exact model equal to the official implementation's) while cutting the linear-layer weights 16x versus f32 and 8x versus bf16, and running the decode-time matmuls **7.7-8.5x faster than a bf16 kernel and 3.0-3.3x faster than dense int8 on one core (4.3x and 1.8-1.9x on four)**. End-to-end it is only 1.8x faster in this harness, prefill is slower than torch's AMX path, and it has not been compared with `bitnet.cpp`. The remaining engineering is clear: a prefill kernel (blocking, AMX or wider tiles), a spinning thread pool, a vectorised activation quantiser, and a quantised lm_head.

## 2026-10-26 (last): thread scaling, a spinning pool

**Problem.** The ternary kernel scaled 1.5x from one to four threads while `bitnet.cpp` scaled 3.2x on the same VM (`docs/results/bitnet_cpp/`). Each of the 210 products per token takes about 250 microseconds, and rayon pays two thread wake-ups per product (hand-off, then the caller blocks on a latch).

**Change.** `ntg::storage::SpinPool`: persistent workers that spin, then sleep; the calling thread runs a share of the job and spins (not blocks) on completion. `PackedTernaryMatrix::gemm_i8_spin` cuts the output rows into about four chunks per thread that threads claim from a shared counter, so a descheduled thread delays one chunk, not a quarter of the matrix. Results are bit-identical to the serial path (new tests for 2, 3 and 4 threads, every kernel, batch 1 to 5; pool tests for wake-after-sleep, concurrent callers and panics). The C ABI now uses it (`ntg_ternary_set_threads`).

**Matmul-only result** (`cargo bench --bench bitnet_decode`; ms per token for the 210 linears, median of 9 interleaved rounds, minimum in brackets; rayon is the previous pool):

| threads | rayon | spin pool | spin speed-up vs 1 thread |
|--:|--:|--:|--:|
| 1 | 33.0 (32.1) | 33.0 (32.1) | 1.00x |
| 2 | 34.1 (27.2) | 20.6 (17.1) | 1.60x |
| 3 | 28.1 (26.5) | 15.3 (12.6) | 2.15x |
| 4 | 31.7 (28.4) | 12.6 (11.5) | 2.61x |

Two earlier runs with a larger idle spin budget gave 4-thread medians of 14.7 and 14.8 ms (speed-up 2.63x and 3.14x); raw output in `docs/results/bitnet_2b_4t/decode_spinpool_run{3,4,5}.md`. Rayon stayed at 1.0-1.2x in all three. At four threads the pool streams weights at 29-46 GB/s, about the f32 row-dot's 35 GB/s, so scaling is now limited by memory bandwidth rather than hand-off cost. The one-thread time itself moved between 30 and 77 ms across runs on this VM, so use the interleaved table, not absolute milliseconds.

**End to end is a different story** (`scripts/bitnet_eval.py`, Hugging Face `generate`, 64 tokens, tok/s, one run per cell, new `--kernel-threads` flag):

| torch threads / kernel threads | rayon pool (before) | spin pool |
|---|--:|--:|
| 1 / 1 | 5.17 | 5.1 |
| 4 / 4 | 5.96 | 4.8 (4.0-4.2 with a longer idle spin) |
| 2 / 2 | - | 7.3-7.8 |
| 1 / 4 | - | 6.7-7.1 |

* Torch's own threads and a spinning pool compete for the same four cores, so the old default (both at 4) got **worse**: 4.8 against 5.96. Splitting the cores (2 + 2, or 1 + 4) gives 7.3-7.8 tok/s, +25-30% over the old best. I shortened the default idle spin (about 30 microseconds, then sleep) for this reason; `NTG_SPIN_ITERS` and `NTG_YIELD_ITERS` override it. The harness default is still `--kernel-threads = --threads`, so pass a split explicitly.
* The linears are now about 20 ms of a roughly 130-140 ms token; the rest is attention, the 128256x2560 output matrix (66 ms in bf16 at one thread in torch) and Python. Faster matmuls cannot show up much end to end until those are replaced.
* `bitnet.cpp` still decodes the whole model faster than this harness at the same thread count (12.1-12.6 tok/s at two threads, 20.9-21.7 at four against 7.3-7.8 here). The matmul scaling gap is closed; the whole-model gap is not.
* One machine, shared VM, single end-to-end runs; not tatortot.

## 2026-10-27: the output matrix to int8

**Where the time goes.** A decode token in the Hugging Face harness with the ternary kernel
(torch 2 threads / kernel 2 threads) measured 132-145 ms wall: about 60 ms in the ternary
kernel's C ABI calls, about 11 ms in their Python wrappers, 35-37 ms in the output projection
(`lm_head`, a 128256 x 2560 bf16 matrix, 0.66 GB read per token), and the rest attention, norms
and Python. With torch 1 / kernel 4 the output projection took 62 ms because torch had one thread.

**Change.** `ntg::storage::Int8Matrix` (PR #32): per-row symmetric int8 weights, int8
activations, exact `i32` accumulation (VNNI / AVX2 / scalar), spinning-pool parallel. Opt-in via
`scripts/bitnet_eval.py --int8-lm-head`. Unlike the ternary kernel this is lossy.

**Speed** (shared 4-vCPU VM; wall ms per token, output-projection ms in brackets):

| torch / kernel threads | bf16 head | int8 head |
|---|--:|--:|
| 2 / 2 | 145 (36), 135 (37) | 107 (17), 111 (17) |
| 1 / 4 | 135 (62) | 88 (9); repeats under other idle-spin budgets 72, 77, 67 (8-9) |

Greedy generation in the full harness (2 / 2): 8.0 tok/s against 7.3-7.8 before.

**Accuracy** (wikitext-2, 8,176 tokens; ARC-Easy, 300 questions; 2 / 2 threads):

| | perplexity | top-1 agreement with official | mean abs logprob diff | ARC-Easy acc / acc_norm |
|---|--:|--:|--:|--:|
| official bf16 | 22.066 | - | - | 76.00 / 76.00 |
| ternary kernel, bf16 head | 22.032 | 96.65% | 0.059 | 75.33 / 74.33 (earlier run, 4 threads) |
| ternary kernel + int8 head | 22.045 | 95.57% | 0.074 | 76.33 / 75.33 |

The int8 output matrix costs about one point of top-1 agreement and 0.015 of mean logprob
difference; perplexity and ARC-Easy are unchanged within noise (the two ARC runs pick the same
answer on 97.7% of questions, 7 of 300 differ). Raw:
`docs/results/bitnet_2b_4t/perplexity_bf16_int8_lm_head.json`, `arc_easy_300_int8_lm_head.json`.

**What did not go well.**
* The same configuration measured 88, 72, 77 and 67 ms per token across runs: treat the speed
  rows as roughly +/-20%.
* The ternary kernel's C ABI calls still cost 33-57 ms per token in the model against 12-20 ms for the
  same matmuls in `bitnet_decode`. **The explanation first written here was wrong**; see the follow-up
  below.
* `bitnet.cpp` still decodes the whole model faster at equal thread counts
  (`docs/results/bitnet_cpp/`).
* Prefill is unchanged. One machine; not tatortot.

### Follow-up (same day): where the C ABI gap comes from

`scripts/bench_ffi_overhead.py` times one token's 210 matmuls through the C ABI with no torch in the
loop (ms per token; the matmul-only `bitnet_decode` figures are 12.6-14.8 at 4 threads, 20.6 at 2):

| kernel threads | default idle spin (1024) | spin 16384 | spin 262144 + 1024 yields |
|--:|--:|--:|--:|
| 1 | 64 | - | - |
| 2 | 61-62 | 51.5 | 46.0 |
| 4 | 45.5-50.8 | 31.1 | 30.7 |

* **Workers falling asleep between calls is the largest piece.** Each C ABI call does serial work
  (the activation quantiser) before it dispatches; with the default idle budget the workers sleep
  during it and every dispatch pays a wake-up. A longer spin takes four threads from about 46-51 to
  about 31 ms. The earlier test that "a 64x longer spin changed it by about 6 ms" was run inside
  the Hugging Face loop, where torch's own work between linears is far longer than any spin budget,
  so it could not show this. That sentence in the entry above is withdrawn.
* **The quantiser is about 4 ms per token.** With a tiny output matrix a call costs 13.9 us for
  2,560 inputs and 43.4 us for 6,912, about 5-6 ns per element (`f32::round_ties_even` on the
  baseline x86-64 target compiles to a libm call). That is roughly 3.8 ms per token across 210 calls.
* **Not explained:** about 10-15 ms of the 4-thread gap remains after a long spin and the quantiser;
  dequantisation, allocation and the Python wrapper (7-11 ms in the model) are the candidates and
  are not separately measured.
* One-thread time swung with the VM again (64 ms here against 30-86 ms for the matmul-only
  benchmark across runs), so absolute milliseconds are roughly +/-30%.
* Obvious fixes, not built: wake the workers at the start of each call so the wake-up overlaps the
  quantiser; a vectorised quantiser (AVX-512 round-to-even); a per-token "stay awake" mode (trades
  CPU stolen from torch).

