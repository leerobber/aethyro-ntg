# Phase 3 curriculum — Ledger & self-mod rails

## Learning objectives

1. Log and verify real tamper-evident ledger entries.
2. Prove self-mod is OFF by default (ADR 0002 rail 1).
3. Rejected mutations still leave a ledger trail.
4. MutationCycle cannot start when disabled.

## Data (real)

Live `TamperEvidentLedger` API only — no simulated fake chain.

## Teaching / learning

- Log 5 study mutations; `verify_full_ledger`.
- Read `SelfModConfig::default().enabled == false`.

## Advanced exam

- Rail 1 default off.
- 3-entry verify.
- RejectedFitnessGate logged + verifies.
- Multi-entry chain.
- MutationCycle::new errors when disabled.
- Outcome enum discrimination.

## Pass

≥ 75% items. Fail ⇒ full redo.
