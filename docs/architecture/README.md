# Architecture Decisions

This folder is the durable record of what's actually been built in this
repo, why, and what was tried and rejected. Each entry is a lightweight
ADR (Architecture Decision Record): Status / Context / Decision /
Consequences.

Written from real code and real test runs, not aspiration — if a
decision below turns out to be stale, trust the code over this document
and update the doc.

| # | Decision | Status |
|---|---|---|
| [0001](0001-vision-and-pivot.md) | Vision: the Aethyro NTG Engine, and why this isn't a vertical bet | Accepted |
| [0002](0002-safety-rails-for-self-modification.md) | Safety rails for self-modifying graph topology | Accepted, not yet implemented |
| [0003](0003-sis-frontend.md) | SIS front-end: docs/paths/glyphs into the NTG graph | Accepted (design), not yet implemented |

## How to add a new one

Copy the format of any existing entry, number it sequentially, and add a
row to the table above. Prefer documenting a real decision after it's
been implemented and tested over speculating about one in advance.
