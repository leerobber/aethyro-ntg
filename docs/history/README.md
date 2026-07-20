# Historical session logs — not current status

The files in this directory are dated, phase-by-phase progress logs and
session summaries from earlier development. They were previously scattered
across the repository root, cluttering it and, in several cases, standing
in for current status when they were badly out of date.

**These are historical records, not documentation.** Many contain
unverified or inflated claims typical of in-the-moment session summaries:
fabricated-sounding precision ("Quality Rating: 9.8/10"), performance
numbers that were never re-measured, and "production-ready" / "approved"
language that no external review ever confirmed. They have not been
individually fact-checked or corrected — they're kept for historical
reference (what was being worked on, when, and what the author believed
at the time), not as claims you should rely on.

**For current, accurate status, see:**
- [`docs/STATUS.md`](../STATUS.md) — the maintained, actively honest status
  document. It already explicitly names "docs and marketing language
  outrunning measurements" as the project's primary risk.
- [`docs/phases/`](../phases/) — the formal Phase 0-5 completion
  certificates, a different and current tracking scheme from the
  `PHASE_A/B/C/D/E` files in this directory.
- [`../../README.md`](../../README.md) and
  [`../../VERIFICATION_REPORT.md`](../../VERIFICATION_REPORT.md) — both
  rewritten 2026-07-19 to state only what was actually verified by running
  the code, with a record of what was previously claimed and found false.

If you're citing anything from this directory (a number, a benchmark, a
claim of completeness), re-verify it against the current code first.
