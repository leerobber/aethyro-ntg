# Phase Gate Protocol

**Status:** Binding engineering process (2026-07-09).  
**Authority:** Same force as CONTRIBUTING.md rules 1 and 8.

## Purpose

Prevent “soft advance”: starting the next phase while prior work is
partially done, half-documented, or deferred without explanation.

## Rules

1. **No phase N+1 until phase N is COMPLETE.**  
   Complete means: every ROADMAP checkbox for that phase is either:
   - **[x] Implemented** with tests and docs, or  
   - **[x] Explicitly re-scoped** via ADR + ROADMAP update with full
     rationale (never a silent skip).

2. **Heavy tests before COMPLETE.**  
   - Unit + integration tests green (`cargo test` in `kernel/`).  
   - New behavior has dedicated tests (or a written skip-with-reason
     test if not yet testable — rule 5 CONTRIBUTING).  
   - Measured claims recorded in EXPERIMENTS.md when performance is
     claimed.

3. **Nothing left undocumented or unexplained.**  
   For every public module, type, and non-trivial decision in the phase:
   - Module-level rustdoc on *what it is and is not*  
   - ADR or phase doc if architectural  
   - ROADMAP checkbox text matches reality  

4. **Phase COMPLETE package (required artifact).**  
   When a phase is declared done, add/update:

   ```
   docs/phases/PHASE_<N>_COMPLETE.md
   ```

   Required sections:
   - Scope (what the phase promised)
   - Deliverables map (file → responsibility)
   - Test proof (commands + counts)
   - Measurements (or “none claimed”)
   - Deep dive: what could advance the project next
   - Explicit non-goals / deferred items (with target phase)
   - Sign-off: **COMPLETE** date + “Phase N+1 may begin: yes/no”

5. **Deep dive after every completion.**  
   Before opening phase N+1, the COMPLETE doc must answer:  
   *What did we learn, and what is the single highest-leverage next
   advance?* That answer is advisory for prioritization inside N+1,
   not a license to skip unfinished N items.

6. **Phase 4 and later stay closed** until Phases 0–3 each have a
   COMPLETE certificate under `docs/phases/` with sign-off **yes**.

## Checklist template (per phase)

- [ ] All ROADMAP items for phase resolved (implemented or re-scoped)
- [ ] `cargo test` green; new tests for new code
- [ ] Docs: DESIGN/STATUS/ROADMAP/ADRs consistent
- [ ] EXPERIMENTS.md updated if anything was measured
- [ ] `docs/phases/PHASE_N_COMPLETE.md` written
- [ ] Deep dive section filled
- [ ] Sign-off: Phase N+1 may begin

## Anti-patterns (forbidden)

- “Core done, stretch later, start next phase anyway”
- Checkboxes left open without ADR re-scope
- Claiming COMPLETE without the phase certificate doc
- Implementing Phase 4 training while Phase 2 SIS stretch is “TBD”
