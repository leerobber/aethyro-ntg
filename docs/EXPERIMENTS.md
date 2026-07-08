# Experiments log

Real, run, measured experiments against this project -- wins and
non-wins recorded the same way, per CONTRIBUTING.md rule 1 ("measure,
don't assume") and the precedent already set by the founder's
`aetherflux-zero` project (an honest negative depth-experiment result
sits next to a real measured 11% bits-per-character win, both kept).
An experiment that didn't pan out and is documented here is more
valuable than one that was quietly dropped.

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

**Conclusion:** this specific approach does not work and is **not**
shipped as a feature (see `kernel/src/ntg/graph.rs` -- no
`edge_interaction_score` function exists). The mechanism BitNet-style
absmean quantization exists for (compress one tensor's own weights) is
not the same problem as (compare two arbitrary short strings), and
applying one to the other silently breaks it.

**What this suggests for a follow-up experiment, not yet tried:** a
fixed/global byte-to-ternary mapping (same threshold for every string,
not each string's own mean) would preserve cross-string distinguishability.
Untried as of this writing -- record the result here, either way, before
treating it as fact.
