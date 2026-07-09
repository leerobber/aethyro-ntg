# Phase 2 curriculum — Graph & SIS front-end

## Learning objectives

1. Parse real markdown into typed graphs without panic.
2. Enforce fence → Execution structural rule.
3. Pathparse real repo path shapes; forward_pass deterministic; fingerprint stable.

## Data (real)

| Split | Source |
|-------|--------|
| Train 80% | Sorted real `docs/**/*.md` |
| Holdout 20% | Remainder of same corpus |
| Paths | Real paths: `kernel/src/ntg/ternary.rs`, ADRs, `tools/dev.sh`, etc. |
| Fixture | Small real markdown snippet with a rust fence |

## Teaching / learning

- Parse every train doc into one graph; record node growth.
- Pathparse practice set (6 paths).
- forward_pass once on train graph.

## Advanced exam

- Holdout docparse (nodes ≥ 1; if fences present, ≥1 Execution).
- Full path corpus.
- Content+Execution kinds on fixture.
- forward_pass deterministic; fingerprint stable.

## Pass

≥ 75% items. Fail ⇒ full redo.
