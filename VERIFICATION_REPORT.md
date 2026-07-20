# Verification Report

**Corrected 2026-07-19.** The previous version of this file was titled
"Quality Verification Report" and asserted, among other things,
"Cyclomatic Complexity: ≤10 average," "Maintainability Index: 87,"
"Quality Rating: 9.8/10," and "Approved For: Clinical Testing ✅, Biotech
Deployment ✅." None of these were produced by any tool or review — no
complexity analyzer, no maintainability scorer, and no clinical or biotech
reviewer ever looked at this code. Every specific line-of-code and test
breakdown in that version was also wrong when checked against what's
actually in the repository (see table below). That file has been replaced
with this one, which states only what was actually run and observed.

---

## What was actually checked (2026-07-19)

| Check | Result | Command |
|-------|--------|---------|
| Build | Clean, 0 errors | `cargo build --release` |
| Full test suite | 386 passed, 0 failed | `cargo test --release` |
| Total Rust lines | ~30,400 | `find kernel/src kernel/tests kernel/benches -name "*.rs" \| xargs cat \| wc -l` |
| `genomic/` module | ~11,150 lines | (previous doc claimed 24,500) |
| `ntg/` module | ~12,350 lines | (previous doc claimed 8,200) |
| `bin/` binaries | ~5,630 lines | (previous doc claimed 6,400) |
| Unsafe blocks | ~26 | `grep -rn "^\s*unsafe " kernel/src` (previous doc claimed 12) |

Every LOC figure in the previous report was wrong, not just the total.

---

## What was NOT checked (removed rather than guessed at)

- **Cyclomatic complexity / maintainability index** — no tool was run.
  If this matters, run `cargo-geiger`, `rust-code-analysis`, or similar
  and record real output here.
- **Performance benchmarks** — the previous "4,000x faster" / "201K
  SNPs/sec" claims have not been re-verified in this pass. Don't trust
  them until someone runs `cargo bench` (or the specific benchmark
  binaries in `kernel/src/bin/`) and records real numbers with the exact
  command used.
- **Reproducibility across repeated runs** — the previous claim ("ran
  pipeline 10 times, all identical, reproducibility guaranteed") was not
  something this pass re-ran. The `ntg` kernel's `ExecutionTrace` /
  deterministic-replay tests do pass (see `kernel/tests/phase3_integration.rs`),
  which is real evidence for determinism *within that specific mechanism*
  — it is not evidence about the full genomic pipeline's reproducibility,
  which is a different, larger claim.
- **Clinical/biotech/regulatory approval** — there is no such thing as an
  informal "approval" for clinical or biotech use. This requires an actual
  regulatory review process specific to the jurisdiction and use case.
  Nothing in this repository has undergone one. Any future claim along
  these lines needs to name the specific review body and process, or it
  shouldn't be made.

---

## What the ledger's "cryptographic verification" actually provides

The previous report said "Tampering detection confirmed" under
"Cryptographic Verification" without qualification. To be specific: the
ledger (`kernel/src/ntg/ledger/`) uses unkeyed SHA-256 hash-chaining with
no persistence and no external anchor. It genuinely detects *accidental*
corruption or a bug that drops/reorders/edits an entry without also
recomputing everything downstream, within a single process run — and the
relevant tests do pass. It does not provide tamper evidence against a
deliberate adversary, since anyone with the same access needed to read the
ledger can also edit it and recompute the chain from their edited version.
See `kernel/src/ntg/ledger/mod.rs` for the full explanation.

---

## Bottom line

The code builds and the real test suite passes (386/386, verified by
running it). That's a genuine, positive signal. It is not the same as
"enterprise-grade," "9.8/10," or "approved for clinical/biotech
deployment" — none of which are real assessments and none of which should
be repeated until they're backed by an actual review from someone
qualified to make that call.
