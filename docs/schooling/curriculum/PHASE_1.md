# Phase 1 curriculum — Ternary tensor core

## Learning objectives

1. Compute ternary matmul exactly on closed-form problems.
2. Apply absmean `encode` correctly.
3. Prove sparse self-dot ≡ dense self-dot on real text encodings.

## Data (real)

| Bank | Source |
|------|--------|
| Matmul problems | `phase1_matmul_problems()` — hand-derived, integer-exact in f32 |
| Encode problems | `phase1_encode_problems()` — closed-form absmean thresholds |
| Storage identity | `encode_fixed("BitNet b1.58 …")` live kernel call |

**Not used:** random mythical tensors without expected answers.

## Teaching / learning

- Practice bank = all but last 2 matmul problems + all but last encode.
- Remedial recompute on any practice miss.
- Storage practice with real ADR-title string.

## Advanced exam

- Full matmul bank including holdouts (`mm_*_holdout`).
- Full encode bank.
- Invalid ternary rejection; shape-mismatch error.
- Sparse ≡ dense self-dot identity.

## Pass

≥ 75% items. Fail ⇒ full redo.
