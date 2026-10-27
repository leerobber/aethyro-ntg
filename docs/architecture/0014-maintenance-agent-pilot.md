# 0014: Maintenance-agent pilot (design, kill criteria, spend caps)

| Field | Value |
|-------|-------|
| **Status** | Proposed (2026-10-27). Nothing below is built. Numeric thresholds are proposed defaults for the owner to confirm. |
| **Depends on** | ADR 0004 (tamper-evident ledger), ADR 0012 (`KEYMASTER_BACKEND_URL` contract) |
| **Purpose** | Decide, before writing code, what the pilot must prove and when it stops. |
| **Honesty rule** | This is an experiment to find out whether the idea earns money. It does not claim that it will. |

---

## Context

The owner wants agents that earn money while they sleep, built on cheap local inference. Nothing in
this repo shows that LLM agents can earn net-positive revenue unattended, and the existing "agent"
code (KAIROS lifecycle, Quad-Brain, the 500K-agent hierarchy) is bookkeeping, not behaviour (see
`CLAUDE.md`). The weakest assumption is demand and unit economics, so the pilot tests that first
and builds the minimum needed to test it.

**Chosen loop: automated code maintenance for small teams.** Fix failing CI, flaky tests, small
migrations, and turn well-specified issues into pull requests with tests. It was picked because
success is checkable by machine (CI goes green and a maintainer merges), cost per task is low, and
the ledger is a trust feature a customer can see ("every change is logged and replayable").
Rejected for now: open-source bounties (lumpy, maintainers dislike AI pull requests), security
bounties (low reliability), content and lead-list businesses (no objective check, platform and legal
risk), trading (real money loss).

Known weakness of the choice: dependency bumps and simple fixes are already covered by free tools,
so the pilot must show value on tasks those tools skip.

## What exists and what does not

| Piece | Status |
|-------|--------|
| Tamper-evident, replayable ledger (`kernel/src/ntg/ledger`) | Exists. Can record each agent action. |
| External-backend routing (`kernel_host`, `KEYMASTER_BACKEND_URL`) | Exists. Generic `{intent,payload}` POST; fail-closed. Local-first escalation policy is **not** built. |
| Local model serving on tatortot | Not set up. Target class: 7-8B at 4-bit on the 8 GB GPU (16 GB system RAM). A 32B model does not fit. |
| Task intake, sandbox runner, CI verifier, PR opener | **Not built.** |
| Spend and action caps | **Not built.** |

## Decision

Run a time-boxed pilot (3-4 weeks of operation) with a human approving every pull request. Do not
build anything beyond what the pilot needs. If the kill criteria below fail, stop and write up why.

### Autonomy ladder

| Level | Behaviour | In pilot |
|-------|-----------|----------|
| L0 | Suggest a patch in a comment | Yes (fallback) |
| L1 | Open a draft PR; a human approves and merges | **Yes, the pilot level** |
| L2 | Merge automatically when CI is green, on repos that opted in | **No.** Only after the criteria pass and the owner decides separately. |

### Spend and action caps (hard limits, enforced in code, not by prompt)

Proposed defaults, owner to confirm:

- Per-task cap: 2.00 USD of paid-model spend and 20 minutes of wall time.
- Per-day cap: 15.00 USD total. At the cap, the harness stops taking new tasks.
- At most 3 open agent PRs per repo and 10 per day.
- At most 15 files and 500 changed lines per PR.
- Forbidden paths: `.github/`, deploy and infrastructure configs, anything matching secrets patterns,
  lockfiles unless the task is a lockfile task.
- Sandbox has no credentials and a network allowlist (package registries and the model endpoints).
- A kill-switch file stops all work at the next step; the harness checks it before every action.
- Every action (read, edit, command run, model call with token counts and cost, PR opened) is appended
  to the ledger.

### Disclosure and conduct

- PRs are labelled as AI-assisted and say so in the description.
- Only repos whose owner has opted in. No unsolicited PRs to third-party projects.
- No scraping or outreach. No claims about the work that the logs do not support.

## What gets measured

Per task and in aggregate: tasks attempted, tasks with a merged PR, merge rate, all-in cost per
merged PR (paid-model spend plus an estimate of local compute), share of tasks solved by the local
model alone, human review minutes per merged PR, revert rate within 14 days, time to merge, and any
incident (a rule broken, a secret exposed, a complaint).

## Continue / stop criteria

Evaluated after at least 30 attempted tasks across at least 3 repos. Proposed defaults:

| Criterion | Continue if |
|-----------|-------------|
| Merge rate | at least 30% |
| Cost per merged PR | at most 25% of the price the owner would charge for one (owner sets the price first) |
| Review time | at most 10 minutes per merged PR |
| Revert rate | at most 10% |
| Incidents | zero security, secret, or policy incidents |

If any row fails: stop, write up the numbers, and do not extend the pilot. A pass does not
justify L2 autonomy or charging; it justifies a paid pilot on 2-3 repos at L1, counting only
revenue net of every cost.

## Local-model role

The router tries the local model first and escalates to a paid model on failure or low confidence.
Whether the local model is useful is a measured outcome (the local-solve share), not an assumption.
If it solves almost nothing, the pilot still stands on paid models and the "cheap local inference"
premise is dropped.

## Risks

- Competition: free and bundled AI tools cover easy tasks, so the value must come from harder ones.
- Trust: one bad change on a customer repo ends the relationship. L1 and the path restrictions
  exist for this.
- Liability and platform terms rest with the owner, not the agent.
- Cost drift: a loop that retries can burn the daily cap. Caps are enforced in code.

## Open questions for the owner

1. The price per merged PR or per repo per month, which the cost criterion depends on.
2. Which repos opt in first (this repo's backlog is the dogfood target).
3. The paid-model provider, and how API keys are supplied to the harness outside chat.
4. Daily and per-task caps above, or different numbers.

## Not claimed

No revenue estimate, no merge-rate expectation, and no claim that the local model is good enough.
Those are the things the pilot measures.
