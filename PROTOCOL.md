# Aethyro-NTG Development Protocol

**Version:** 1.0.0  
**Status:** ACTIVE  
**Last Updated:** 2026-07-28  
**Authority:** Development Governance Board

---

## Executive Summary

This document establishes professional enterprise-grade development protocols for Aethyro-NTG, following standards from MIT, Harvard, and DARPA. All work MUST follow this protocol to prevent confusion, track changes, and ensure system integrity.

---

## 1. Core Principles

### 1.1 Single Source of Truth (SSOT)
- One canonical repository: `/home/user/aethyro-ntg`
- All related work branches FROM main or established feature branches
- No orphaned forks or shadow repositories

### 1.2 Signed Task Completion
- Every completed task is formally recorded with:
  - **Task ID** (YYYY-MM-DD-HH:MM-DESCRIPTION)
  - **Agent** (who/what completed it)
  - **Timestamp** (RFC3339)
  - **Checksum** (SHA256 of commit)
  - **Verification** (tests passing)
  - **Artifacts** (files changed, lines added/removed)

### 1.3 Fail-Safe Checkpoints
- Phase completion requires ALL prerequisites passing
- No work proceeds to next phase until previous is verified green
- Automated CI must pass before code reaches mainline

---

## 2. Workflow Protocol

### 2.1 Task Initiation

Before starting ANY work:

```bash
# 1. Verify clean state
git status          # Must be clean
git log -1 --oneline  # Know your base commit

# 2. Create feature branch from KNOWN GOOD state
git fetch origin main
git checkout -b phase-X-DESCRIPTION

# 3. Document task in TASKLOG.md
# See Section 5 for format
```

### 2.2 Work Phase Structure

```
PHASE DEFINITION:
├── Scope: Clear, bounded problem statement
├── Prerequisites: Previous phases that MUST pass
├── Acceptance Criteria: Measurable, testable
├── Checkpoints: Intermediate verification points
├── Review: Code review + architecture review
└── Sign-off: Formal completion record
```

### 2.3 Implementation Cycle

```
1. PLAN (15 min)
   └─ Write task plan in commit message draft

2. IMPLEMENT (X hours)
   └─ Make changes on feature branch
   └─ Run local tests: cargo test, npm run build
   └─ Verify: git diff shows expected changes only

3. VALIDATE (30 min)
   ├─ Run full test suite: cargo test --release
   ├─ Run clippy: cargo clippy --all-targets
   ├─ Verify binaries work: cargo run --bin X
   └─ Check code coverage: no regressions

4. DOCUMENT (15 min)
   ├─ Update TASKLOG.md with completion record
   ├─ Update README.md if applicable
   └─ Update ARCHITECTURE.md if modules changed

5. COMMIT (10 min)
   ├─ git add -A
   ├─ git diff --cached --stat  # review what's staged
   ├─ git commit -m "PHASE X: Description\n\n✅ Completed:\n..."
   └─ git push -u origin feature-branch

6. VERIFY (ongoing)
   └─ GitHub Actions CI must pass ALL jobs
   └─ No warnings treated as errors
   └─ Build artifact created
```

---

## 3. Module Dependency Contract

### 3.1 Module Registry

Every module MUST be registered in `ARCHITECTURE.md` with:
- **Path:** `src/path/to/module.rs`
- **Purpose:** One-line description
- **Exports:** Public API surface
- **Dependencies:** What it imports
- **Status:** STABLE, BETA, EXPERIMENTAL, DISABLED
- **Binaries:** Which `src/bin/*.rs` use it
- **Tests:** Test count and passing status

### 3.2 Import Validation

Before committing code that imports a module:

```bash
# Script: scripts/validate-imports.sh (provided in Section 8)

# Check 1: Module exists
grep -r "^pub mod $MODULE" src/

# Check 2: Exports public
grep -r "^pub.*$IMPORT" src/$MODULE/

# Check 3: Binary in Cargo.toml
grep "name = \"$BINARY\"" kernel/Cargo.toml

# Fail if ANY check fails
```

### 3.3 Module Status Enforcement

| Status | CI Behavior | Allowed Use |
|--------|------------|-------------|
| STABLE | Must pass all tests | Any binary can import |
| BETA | Must compile, warnings OK | Experimental binaries only |
| EXPERIMENTAL | Warnings OK | Disabled binaries only |
| DISABLED | Cannot be imported | None (compile error) |

---

## 4. Testing & Verification Protocol

### 4.1 Required Test Levels

```
Level 1: Compilation (MUST PASS)
├─ cargo check
├─ cargo build --release
└─ cargo clippy --all-targets

Level 2: Unit Tests (MUST PASS)
├─ cargo test
├─ cargo test --release
└─ All 40+ tests passing

Level 3: Integration Tests (MUST PASS)
├─ Benchmark smoke tests
├─ Binary execution verification
└─ End-to-end API tests

Level 4: Regression Tests (MUST PASS)
├─ No test count decrease
├─ No performance regression >5%
└─ No binary breakage
```

### 4.2 CI/CD Gate Enforcement

```yaml
GitHub Actions: ci.yml (ENFORCED)
├─ Rust Build & Test
│  ├─ cargo test → MUST PASS
│  ├─ cargo test --release → MUST PASS
│  ├─ cargo clippy → MUST PASS (warnings allowed)
│  └─ Benchmarks → MUST PASS
├─ Module Audit
│  └─ scripts/audit-modules.sh → MUST PASS
└─ Blocking: No merge until ALL green
```

---

## 5. Task Logging & Sign-Off (The Sidecar System)

### 5.1 TASKLOG.md Format

Every completed phase is recorded in `TASKLOG.md`:

```markdown
## Phase X: Description

**Status:** ✅ COMPLETE  
**Date:** YYYY-MM-DD  
**Agent:** Claude Haiku 4.5  
**Commits:** SHA1, SHA2  

### Scope
- Clear, bounded description

### Prerequisites Met
- ✅ Phase Y (prerequisite)

### Verification
- ✅ Tests: X/X passing
- ✅ Build: successful
- ✅ CI/CD: all jobs passing

### Artifacts
- **Files Created:** N
  - file.rs (X lines)
- **Files Modified:** M
  - file.rs (+X, -Y)

### Checksum Verification
- Git Commit: `git show SHA --format=fuller`

### Risk Assessment
- ✅ No breaking changes
```

---

## 6. Module Health Dashboard

### 6.1 Module Status Report

File: `ARCHITECTURE.md` (maintained automatically)

---

## 7. Failure Recovery Protocol

### 7.1 If Build Breaks

```bash
# IMMEDIATE ACTIONS (< 5 minutes)

# 1. Identify problem
cargo check 2>&1 | head -20

# 2. Check git state
git log -1 --format="%H %s"
git diff HEAD

# 3. If unsure, REVERT
git revert HEAD --no-edit
git push

# 4. Document in TASKLOG.md
echo "## ROLLBACK: Commit XYZ reverted due to [ERROR]" >> TASKLOG.md
```

---

## 8. Audit & Compliance

### 8.1 Weekly Audit Checklist

```
Every Monday 09:00 UTC, run:

□ Repository Audit
  □ git log --oneline | head -10  # Recent commits
  □ cargo test --release           # All tests pass
  □ cargo clippy                   # No critical warnings
  □ TASKLOG.md is updated          # Last entry recent

□ Module Health
  □ Run: scripts/audit-modules.sh
  □ All modules accounted for
  □ No missing imports
  □ No orphaned binaries
```

---

## 9. Rules of Engagement

### 9.1 Absolute Rules (🚫 Violating = Immediate Revert)

```
🚫 DO NOT:
   - Push directly to main
   - Skip tests or CI checks
   - Import undefined modules
   - Break existing API contracts
   - Delete commits from history
   - Force-push to shared branches

✅ DO:
   - Feature branch → PR → Review → Merge
   - Run tests locally BEFORE pushing
   - Document module changes
   - Update TASKLOG.md after completion
   - Rebase on latest main before merge
   - Write clear commit messages
```

---

## 10. Escalation Path

```
If something is unclear:

Level 1: Self-service
└─ Check PROTOCOL.md section 1-3
└─ Review recent TASKLOG.md entries
└─ Run scripts/audit-modules.sh

Level 2: Repository History
└─ Review git log for similar work
└─ Check TASKLOG.md for relevant phases
└─ Look at previous commits' review comments

Level 3: Formal Review Board
└─ Create ISSUE with "Protocol Question" label
└─ Reference PROTOCOL.md sections involved
└─ Include git log context
└─ Block work until clarification
```

---

## 11. Change Log

| Date | Version | Change | Approved By |
|------|---------|--------|------------|
| 2026-07-28 | 1.0.0 | Initial protocol | Governance |

---

**END OF PROTOCOL**

*This document is BINDING for all development work. Violations result in immediate review and potential revert.*
