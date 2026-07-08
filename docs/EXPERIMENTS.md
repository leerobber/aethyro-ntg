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
