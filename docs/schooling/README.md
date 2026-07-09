# NTG Doctorate Schooling

Professional, sourced, multi-run learning program for Aethyro NTG Phases **0–5**.

This is not marketing and not mythical data. The kernel **studies** real
repository documents and closed-form problems, then takes **advanced exams**.
Any phase below **75%** fails and must **full redo**.

## Quick start

```bash
# Linux / WSL / Git Bash (LF line endings)
cd kernel
cargo run --release --bin ntg_school -- \
  --docs ../docs \
  --out ../docs/schooling/runs \
  --runs 5
```

**Windows PowerShell** (do not rely on `bash tools/dev.sh` if you see `pipefail` errors — that is usually CRLF or non-bash):

```powershell
cd ~/aethyro-ntg
.\tools\dev.ps1 school

# or pure cargo (always works if Rust is installed):
cd kernel
cargo run --release --bin ntg_school -- --docs ../docs --out ../docs/schooling/runs --runs 5
```

## Read first

1. [PROTOCOL.md](PROTOCOL.md) — pass bar, redo, outputs  
2. [curriculum/](curriculum/) — per-phase data, teaching, exam design  
3. [runs/MASTER_NOTEBOOK.md](runs/MASTER_NOTEBOOK.md) — latest multi-run results  

## Layout

```
docs/schooling/
  README.md                 ← this file
  PROTOCOL.md               ← binding rules
  curriculum/PHASE_0..5.md  ← syllabus per phase
  runs/                     ← generated notebooks + JSON + manifest
kernel/src/ntg/schooling/   ← implementation
kernel/src/bin/ntg_school.rs
```

## Steady cadence (recommended)

| Cadence | Action |
|---------|--------|
| After every phase COMPLETE | Run `ntg_school --runs 5` and commit notebooks |
| Weekly on LOQ/WSL | `./tools/dev.sh school` (if wired) or `ntg_school` |
| After calib/feature changes | Re-run phases 4–5 at minimum |

## Honesty

- Engineering COMPLETE ≠ schooling PASS.
- Holdout metrics can fail 75% even when CI unit tests are green — that is the point of advanced exams.
- Document failures as carefully as wins.
