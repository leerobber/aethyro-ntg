> **Note added 2026-10-27 when this plan was committed (not part of the original text).**
> Numbers in §1 and the A2 exit criterion predate PRs #29-#32: the kernel now has 678 test
> markers and 674 executed tests (not 540 / 544); see the addendum in
> [BASELINE_2026-10-26.md](BASELINE_2026-10-26.md). Findings made while implementing A2/A5/A6 are in
> that addendum and in ADRs [0012](architecture/0012-keymaster-contract.md) and
> [0013](architecture/0013-telemetry-loop.md). Where this plan and
> [STATUS.md](STATUS.md) disagree, STATUS wins (standing rule 8).

# aethyro-ntg — Foundation Lock + Build Plan (v2)

**Date:** 2026-10-26 (evening)
**Supersedes as the forward plan:** the phase list in `audit-and-build-plan.md`
(the audit itself remains the record of what was found at HEAD 929bee6).
**Current HEAD:** `c1189f4` (main) — 7 PRs landed 2026-10-26 (#24, #26–#31).
**Current truth doc in the repo:** `docs/BASELINE_2026-10-26.md`.
**Author's note:** recommendations marked "Recommendation" are judgment calls,
not Terry's decisions. Everything else is either measured fact or the
mechanical consequence of the audit's findings. Terry decides; this plan
proposes.

---

## 1. Where things stand (measured, 2026-10-26 evening)

| Item | State | Evidence |
|---|---|---|
| Tests executed (kernel) | 540 passed, 0 failed, 0 ignored | `docs/BASELINE_2026-10-26.md`, measured at `5c3b35e`, Linux container, Rust 1.97, no GPU |
| Test markers | 544 (`#[test]` + `#[tokio::test]`); 4 not in executed count | Baseline doc: 2 are `#[ignore]` GPU tests in `src/cuda/mod.rs`; 2 untraced (likely cfg-gated or non-default targets) |
| Clippy | Clean on lib+bins, release, and all-targets | Baseline doc; the "~155 warnings in `genomic/`" note does not reproduce on current main |
| Size | `kernel/src` 34,965 lines / 139 files; unsafe 19 blocks + 5 `unsafe fn` | Baseline doc (matches the audit's independent measurement exactly) |
| Capability version | 10 (`ternary_capability()`) | Baseline doc — resolves the v8/v9/v10 drift |
| Whole-stack smoke | `scripts/smoke-whole.sh` green: health, classify, score, verify_ledger pass; `/api/chat` skipped (no Ollama); green in CI as the `whole-stack` job | Baseline doc + `.github/workflows/ci.yml` |
| Build-all | `scripts/build-all.sh` exists | PR #24 |
| Doc-count enforcement | `scripts/check-doc-counts.sh` enforces documented test counts in CI | PR #26 |
| Railway config | Fixed (new schema, `/health` healthcheck, no `kernel_host` startCommand bug) | `railway.json` on main |
| Deploy verification | `scripts/verify-deploy.sh` exists | PR #24 |
| Open PRs | 1: #25, Dependabot rustls 0.23.42 → 0.23.46 (routine) | GitHub, 2026-10-26 |
| Dangerous old PRs | #23 and #16 closed unmerged; dashboard intact; their branches still exist remotely | GitHub + tree check 2026-10-26 |
| New capability landed today | Quad-Brain wired into kernel_host (#27); ternary×int8 GEMM, VNNI/AVX2 (#28); kernel inside BitNet b1.58 2B-4T + C ABI + parallel GEMM + decode benchmarks (#29); bitnet.cpp comparison on the same VM (#30); spinning thread pool, 2.6–3.1× four-thread matmul scaling (#31) | `git log 929bee6..c1189f4`, 58 files, +10,604/−1,636 |
| Live deployment | NONE evidenced (no live URL recorded anywhere) | Absence checked 2026-10-26 |
| Stale branches | ~15 remote branches, all July-era (phase scaffolds, claude sessions, gpu-acceleration, wip) | `git branch -r` |

**What the audit plan already delivered (done, not to be redone):**
Phase 0 baseline · Phase 1 PR closures (PRs closed; branch deletion still
pending — see A3) · Phase 2 whole-stack smoke + CI job · Phase 3 build-all ·
Phase 4 Railway fix (live proof still pending — see A1) · Phase 6 docs
reconciliation + test-count enforcement.

**What remains from the audit plan:** live-deployment proof · Phase 5 wiring
leftovers (telemetry loop visibility, keymaster contract, webhooks) ·
canonical PackedTernary ADR · Phase 7 capability work (AVX-512, NEON,
WASM/FFI, aethyro.com head-to-head).

---

## 2. The principle: Foundation Lock before capability work

The repo absorbed 10,604 new lines in a single day. That is a successful
day — and it is exactly when foundations crack: fresh code (spin pool,
ternary GEMM, FFI layer, Quad-Brain wiring) with no soak period, no
release, no live deployment, and no extended test run.

**Foundation Lock** is a short, explicitly bounded hardening phase. Its
rule is simple: **no new capability PRs until the Lock's exit criteria
(§4) are all met.** This is not a freeze on all work — it is a freeze
on growth while the ground is made unbreakable. The audit's central
lesson was that this repo's risk has never been a lack of ideas or code;
it has been claims outrunning measurements and stale branches deleting
work. The Lock is designed to make both impossible by construction:
everything it produces is either a measured fact, a passing gate, or a
deleted hazard.

**Why this order (my plan, stated plainly):** deploy proof first, because
it is the only "working as a whole" criterion with zero evidence, and it
validates the day's most consequential fix. Truth gaps second, because
the baseline's value is exactness and it currently has a 4-marker hole.
Hygiene third, because the two closed PRs' branches are still loaded
weapons. Release fourth, because a tagged artifact is what turns "main
is green" into "something stable exists." Contracts and stubs fifth,
because they are the remaining undocumented seams. Capability work
after — in the order that most reduces risk, not the order that is most
exciting.

---

## 3. The definition of "working as a whole" (carried from the audit, updated)

| # | Criterion | Status now |
|---|---|---|
| 1 | One local stack: a single documented sequence starts kernel_host + dashboard (+ keymaster; Ollama optional) and serves the UI | MET (smoke script + build-all; dashboard verified binding) |
| 2 | One smoke script passes health/classify/score/verify_ledger (+ chat when Ollama is up) + keymaster health | MET in CI (chat skipped without Ollama — acceptable, documented) |
| 3 | One CI job runs the whole-stack smoke (minus Ollama) on every push to main | MET (`whole-stack` job) |
| 4 | One live deployment, verified by hitting its health endpoints from outside | **NOT MET — this is A1** |
| 5 | Docs match measurements (test counts + phase vocabulary from one generated source) | MET for test counts (CI-enforced); phase vocabulary — confirm single scheme in A6 |
| 6 | No open PR can delete main-branch work; stale branches closed or rebased | PARTIAL (PRs closed; branches still exist — A3 finishes it) |

---

## 4. Phase A — Foundation Lock (the next move)

### A1. Prove one live deployment [BEST FIRST STEP]

**What:** deploy the stack to ONE target and verify it from outside.
**Recommendation — target: Railway.** Reasons: its config was fixed
today (new schema, `/health` healthcheck, the `kernel_host` startCommand
bug removed), and `scripts/verify-deploy.sh` already exists for exactly
this. Fallback: Fly — its config already looked consistent (context
`hostframe-backend`, `/health` is served by hostframe's router), but it
has no verification script and no deploy history. GCP is NOT a
candidate for this step: billing is disabled on the aethyro-ntg project
and it needs 4 secrets before it can even attempt a run.
**How:**
1. Terry confirms the target (Railway recommended) — the deploy itself
   needs his account on that target; this plan does not assume access.
2. Deploy from current main (`c1189f4` or later).
3. Run `scripts/verify-deploy.sh` (extend it if it does not already curl
   from outside the deploy host — a health check from inside the
   container proves nothing about reachability).
4. From an outside machine, curl the live `/health` (hostframe) and/or
   `/api/health` (dashboard) and record the URL + timestamp + commit.
5. Write the URL, timestamp, and commit into `docs/DEPLOY.md` (new in
   PR #24 — extend it, don't create a second deploy doc).
**Exit:** one live URL, curled from outside, recorded in the repo.
**Why first:** it is the only criterion in §3 with zero evidence, and
it is the only step that converts "fixed" into "proven." Everything
after it (telemetry visibility, dashboard access, real user testing)
needs a live target to point at.

### A2. Close the baseline's truth gaps

**What:** the baseline doc is honest about a 4-marker hole. Close it.
**How:**
1. Trace the 2 untraced test markers: find which targets/cfgs they
   belong to and name them in the baseline doc, or note precisely why
   they are not executed. "Likely cfg-gated" is a hypothesis, not a
   finding — the Lock requires the finding.
2. Decide the 2 ignored GPU tests (`src/cuda/mod.rs`): either enable
   them on a GPU-capable runner (tatortot has the RTX 5050) or document
   why they stay ignored and what would un-ignore them. Ignored tests
   with no reason are how coverage quietly rots.
3. Resolve the Oct 25 `updated_at` mystery: check GitHub repo →
   Insights / branches / releases / events for any Oct 25 event. If
   nothing explains it, write one line in the baseline doc saying it
   is unexplained and move on. An unexplained timestamp is fine;
   an unexamined one is not.
**Exit:** baseline doc accounts for all 544 markers (540 executed + 2
ignored-with-reason + 2 traced) and the Oct 25 question has an answer
or a recorded "unexplained."

### A3. Finish PR and branch hygiene

**What:** make the audit's critical findings unrepeatable.
**How:**
1. Merge Dependabot PR #25 (rustls 0.23.42 → 0.23.46) after CI passes.
   It is routine; do not let routine updates linger — lingering
   routine PRs are how a PR list goes stale.
2. Delete the remote branches for closed PR #23
   (`claude/merge-aetherflux-zero-prs-5h10zg`) and #16
   (`phase-5-governance`). Closing the PRs left the branches live; a
   future accidental merge would still delete main-branch work.
3. Triage the remaining ~13 July-era remote branches: delete, or keep
   with a one-line reason each (e.g., `feature/gpu-acceleration` may
   still be wanted for Phase B). No branch stays by inertia.
4. Adopt the standing rule (see §7): any PR older than one week gets
   closed or rebased — no exceptions for "I'll get to it."
**Exit:** `git branch -r` contains only main, branches with a current
PR, and branches with a recorded reason. Zero branches whose merge
would delete main-branch work.

### A4. Lock the day's new code in

**What:** the 10,604 new lines need a soak and a stable artifact.
**How:**
1. Extend smoke coverage where it is cheap and meaningful: the smoke
   script already covers health/classify/score/verify_ledger. Add a
   check that exercises the new ternary GEMM path (e.g., a classify
   or score call that routes through the ternary×int8 kernel) IF such
   a call exists in the dashboard API; if it does not, record that
   the GEMM path is covered by unit/bench tests only, not by the
   smoke — and say so in the baseline doc.
2. Tag a first release — **recommendation: `v0.1.0`** — from the Lock's
   exit commit. This exercises `windows-release.yml` (which publishes
   on `v*` tags: portable ZIP with binaries + PWA assets) and creates
   the first stable artifact anyone can download without building
   from source. If the release job fails, fixing it is part of the
   Lock, not a separate task.
3. **Soak rule (applies from the tag until the Lock exits):** no new
   capability PRs. Bug fixes, doc corrections, and the Lock's own
   steps are allowed. A capability PR opened during the Lock is
   closed with a pointer to this plan, not debated.
**Exit:** one tagged release exists with downloadable artifacts; the
smoke script's coverage statement (what it covers, what it does not)
is written down; no capability PR merged during the Lock.

### A5. Contracts and stubs (the remaining seams)

In priority order — each is small, each removes a silent gap:

1. **Keymaster contract, in writing.** One doc page (or a section in
   ARCHITECTURE.md — do not create a second architecture doc):
   what `KEYMASTER_BACKEND_URL` must serve, the request/response
   shape, what kernel_host does when keymaster is down (fail open or
   fail closed — this must be a stated decision, not an emergent
   behavior), and what the dashboard's keymaster health dot actually
   checks.
2. **Telemetry loop, visible or declared one-way.** Trace kernel_host
   (or dashboard) → hostframe `/ingest` → BigQuery → a visible
   surface (dashboard metrics view or a documented query). If the
   loop is intentionally ingest-only, write that down. An
   undocumented loop reads as a broken loop.
3. **Webhooks: finish or delete.** `webhooks/payload-.json` is still a
   stub with a malformed name, and the HEAD commit that introduced
   versioning ("Versioned webhook payload:") never finished the
   story. Either: version field + one real example payload + one doc
   page + one parsing test — or delete the directory. No third
   option, no lingering stub.
4. **Canonical PackedTernary ADR.** STATUS.md's P0, still open: which
   implementation (`packed.rs` vs `storage/packed_ternary.rs`) owns
   the product path long-term, and what happens to the other one
   (deprecated, deleted, or namespaced). This is a one-page decision
   record, not a refactor — the refactor, if any, is Phase B work.
**Exit:** each of the four is either done or explicitly declared
intentional (with the reason written down). No silent gaps remain.

### A6. Docs lock

**What:** make the new truth hard to drift again.
**How:**
1. Confirm a single phase vocabulary everywhere (STATUS, README,
   CLAUDE, ROADMAP): the 0–5 + 6.x + F scheme from CLAUDE.md is the
   recommendation — retire "7.5.x" wherever it still appears, or
   adopt it everywhere if Terry prefers it. One scheme, everywhere.
   (Note: `docs/PHASE_7_5_ROADMAP.md` was touched by today's work —
   check whether it reintroduces the 7.5 scheme and reconcile it.)
2. Update STATUS.md from the baseline: current date, 540 executed
   tests, capability v10, Lock status. STATUS remains the single
   source of truth; BASELINE is its evidence appendix, not a rival.
3. DEPLOY.md gets the A1 live URL (or, if A1 fails, the failure and
   the next attempt — a failed deploy recorded honestly is still
   progress; an unrecorded one is not).
4. Keep `check-doc-counts.sh` enforcement green — any doc edit that
   changes a test count without the marker fails CI, by design.
**Exit:** a reader opening STATUS, README, or CLAUDE gets the same
test count, the same phase, and the same clippy status. Any future
drift fails CI instead of surprising a reader.

### Phase A exit criteria (all required)

- [ ] One live URL, curled from outside, recorded in `docs/DEPLOY.md`.
- [ ] All 544 markers accounted for (540 executed + 2 ignored-with-reason + 2 traced).
- [ ] Oct 25 `updated_at` answered or recorded unexplained.
- [ ] Dependabot #25 merged; dangerous branches deleted; remaining branches triaged.
- [ ] One tagged release (`v0.1.0` recommended) with downloadable artifacts.
- [ ] Smoke coverage statement written (what it covers, what it does not).
- [ ] Keymaster contract written; telemetry loop visible or declared one-way; webhooks finished or deleted; PackedTernary ADR written.
- [ ] Single phase vocabulary everywhere; STATUS updated from baseline.
- [ ] Zero capability PRs merged during the Lock.

**When Phase A exits, capability work resumes.** Not before.

---

## 5. Phase B — Capability work (only after Phase A exits, in this order)

Ordered by risk reduction, not excitement. Each item's exit is a
measurement or a merged, tested change — never a claim.

### B1. Real AVX-512 VPOPCNTDQ kernels
- **Why first:** STATUS P1, and the highest-leverage honest performance
  work — the host already advertises AVX-512F/VPOPCNTDQ, but the dense
  dual-stream path is detect-only with portable `count_ones`. The
  day's GEMM work (VNNI/AVX2) makes this the natural next kernel, not
  a detour.
- **Exit:** kernel exists, bit-identity tests pass against the scalar
  reference, benchmark recorded in EXPERIMENTS.md with hardware +
  command (not a bare speedup number).

### B2. NEON full path
- **Why:** the ARM path is a stub/fallback today. The project's
  edge story (phone-class devices, Android PWA install) is hollow
  without a real ARM kernel.
- **Exit:** same bar as B1 — bit-identity + recorded benchmark, or a
  documented decision that ARM stays fallback (with the reason).

### B3. OpStats + device name into ledger entries on forward
- **Why:** STATUS P1, and it converts the ledger from "what happened"
  into "what happened, on what, how fast" — which is what makes the
  ledger useful for the head-to-head in B5 instead of merely
  decorative.
- **Exit:** forward entries carry OpStats + device name; a test
  asserts their presence; replay still verifies.

### B4. WASM/FFI host product path (Phase 6 integration)
- **Why:** this is the project's own gate protocol item — host loading
  of frozen models + production compare. The C ABI added in PR #29
  is the on-ramp; this item walks through it.
- **Gate rule (non-negotiable, from the project's own protocol):** no
  soft advance without a phase certificate. A `PHASE_6_COMPLETE.md`
  certificate is written from measured results before Phase 7 work
  begins.
- **Exit:** a host (WASM or FFI consumer) loads a frozen NTG model,
  runs it, and the result is recorded — win, loss, or tie, reported
  honestly.

### B5. aethyro.com head-to-head (the big one)
- **Why:** the project has never had it, and its own verdict
  (RESEARCH-READY / PRE-PRODUCT) rests on its absence. Every other
  performance number in the repo is either a micro-benchmark, a
  same-family comparison (bitnet.cpp), or a PyTorch GPU number that
  is not an NTG number at all.
- **How (sketch, to be detailed when Phase B reaches it):** pick one
  inference task the site actually serves; run it through NTG and
  through the site's current path on comparable hardware; record
  latency, cost, and quality with commands + hardware. Report win,
  loss, or tie — all three are acceptable outcomes; an unreported
  outcome is not.
- **Exit:** a head-to-head report exists in the repo, whatever it
  says. Until then, the PRE-PRODUCT verdict stays printed on the
  project — correctly.

### B6. GPU path (un-ignore or justify)
- **Why:** the 2 ignored CUDA tests from A2 either get a runner
  (tatortot's RTX 5050) or a permanent, reasoned ignore. The GPU
  story (bench scripts, Blackwell setup docs) is currently
  PyTorch-side; the NTG CUDA path (`src/cuda/`) should either be
  exercised or declared experimental.
- **Exit:** CUDA tests run somewhere in CI or on a documented
  schedule, or their ignore carries a written reason and a
  re-enable condition.

---

## 6. Phase C — Product readiness (later; not a promise)

The verdict changes from RESEARCH-READY / PRE-PRODUCT only when the
evidence changes. The items that would change it:

1. **Threat-model review** before any air-gap or sovereignty product
   claim (STATUS's own requirement — the ledger's scope label must
   survive this review, not be inflated by it).
2. **Independent review** before ANY biotech/clinical language
   returns. This is a hard gate, not a milestone: no external audit,
   no clinical words. The README's removal of those claims stays
   in force until this happens.
3. **Production benchmark win/loss** (B5) plus a repeatable
   deployment (A1, kept live) plus a release cadence (A4, repeated).
4. **Marketing discipline, enforced:** every public claim traces to
   a measurement in the repo (the `check-doc-counts.sh` pattern,
   generalized). The project's primary risk — docs outrunning
   measurements — is managed by making it mechanically hard, not by
   good intentions.

Phase C has no date. Dating it now would repeat the pattern this plan
exists to break.

---

## 7. Standing rules (carried forward, no expiry)

1. **Safety rails stay on.** Self-modification off by default,
   bounded budget, auto-rollback, deterministic replay, ledger on
   every mutation. Any change to a rail needs a test and a reason.
2. **Ledger scope label stays.** Self-consistency checking, not
   cryptographic tamper evidence against a deliberate adversary —
   until a signing key + external anchor exist, and then only after
   review.
3. **No efficiency claims without measurement.** Numbers carry
   hardware + command + date, or they don't ship.
4. **No clinical/biotech claims without independent review.** (See §6.2.)
5. **Review is a release gate.** No direct-to-main capability work
   without review; no merge of a stale branch without rebasing onto
   current main first (this rule exists because F1/F2 happened).
6. **Phase-gate protocol.** No soft advance without certificates
   (the project's own rule, kept).
7. **Stale-branch rule.** Any PR older than one week gets closed or
   rebased. Branches are not storage.
8. **One truth doc.** STATUS.md is the truth; BASELINE.md is its
   evidence; everything else points at them. A second truth doc is
   a bug.

---

## 8. Sequencing summary

| Order | Step | Size | Blocks |
|---|---|---|---|
| 1 | A1 live deploy proof (Railway recommended) | Small (hours, mostly waiting on the deploy) | Needs Terry's target confirmation + account |
| 2 | A2 baseline truth gaps | Small (an hour of investigation) | Nothing |
| 3 | A3 hygiene (Dependabot + branch deletion) | Small (minutes, plus review) | Nothing |
| 4 | A4 release + soak | Small (tag + release job) | A1 green (release from a proven state) |
| 5 | A5 contracts + stubs | Medium (four small docs/decisions) | Nothing structural |
| 6 | A6 docs lock | Small (edits + CI green) | A1–A5 recorded |
| — | **Phase A exit** | — | — |
| 7 | B1 AVX-512 kernels | Medium | Phase A exit |
| 8 | B2 NEON path | Medium | Phase A exit |
| 9 | B3 OpStats in ledger | Small–medium | Phase A exit |
| 10 | B4 WASM/FFI product path + certificate | Large | Phase A exit; certificate gates Phase C |
| 11 | B5 aethyro.com head-to-head | Medium–large | B4 (a host path to compare through) |
| 12 | B6 GPU path | Small–medium | A2's GPU decision |
| — | Phase C (verdict change) | — | B5 + threat-model review + release cadence |

---

## 9. Immediate next action

Terry confirms the deploy target — **recommendation: Railway** (fixed
today, healthcheck present, verification script exists; Fly is the
fallback) — and the deploy is run and verified from outside. That
single action is the best next step, and it is the only step in this
plan that needs Terry's account rather than just his approval. Every
other Phase A step can proceed in parallel or immediately after.
