# Sovereign stack — foundation harden checklist

**As of:** 2026-07-12  
**Scope:** Rung 1–3 + real multi-axis + multi-chr campaign.

## Module map (canonical)

| Module | Role |
|--------|------|
| `genomic/sovereign_brain.rs` | Multi-chr map, working set, LTM, mutants, activate |
| `genomic/language_organ.rs` | SIS docs + calib + text signature |
| `genomic/sovereign_fitness.rs` | Real axes + ledgered selection |
| `genomic/organ.rs` | Shared `Organ` trait |
| `genomic/selection_loop.rs` | Shared train/prune loop + JSONL |
| `genomic/sovereign_persist.rs` | Durable save/load |
| `ntg/mutation/multi_axis.rs` | Utility / gates |
| `bin/sovereign_brain_demo` | Synthetic + optional VCF demo |
| `bin/sovereign_campaign` | Multi-chr real VCF campaign |

## Proven vs assumed

| Claim | Status |
|-------|--------|
| Multi-chr ingest + structure metrics | Proven (tests + campaign) |
| Train accept / prune reject under bio gate | Proven (demo + campaign) |
| Ledger integrity across selection | Proven (tests) |
| Language co-activate with genomic WS | Proven (tests + demo) |
| Calib lifts task axis | Proven on fixtures; not schooling exam |
| LTM motif hits on activate | Hardened in this pass |
| Unrestricted durable memory | Save/load path added this pass |
| Production aethyro.com parity | **Not claimed** |

## Hardening pass items

- [x] ADR 0009 + STATUS sync  
- [x] Shared selection loop  
- [x] Organ trait  
- [x] LTM activate reliability  
- [x] Integration regression (train/prune pattern)  
- [x] Durable snapshot (LTM + calib + metrics)  
- [x] JSONL generation log  
- [x] Docs-corpus harder task option  

## Non-goals this pass

- Rung 4 new structure families  
- Phase 6 product host  
- GPU / AVX-512 full kernels  
