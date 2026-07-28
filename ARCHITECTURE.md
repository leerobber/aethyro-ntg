# System Architecture

**Version:** 1.0.0  
**Last Updated:** 2026-07-28  
**Status:** Active Development  
**Maintainer:** Development Governance Board

---

## Module Registry

### Core Modules (STABLE) — Production Ready

Aethyro-NTG consists of multiple modules supporting neuromorphic ternary graph processing:

| Module | Purpose | Status | Notes |
|--------|---------|--------|-------|
| `ntg::storage` | Ternary bit storage | ✅ STABLE | Core storage format |
| `genomic::vcf` | VCF file parsing | ✅ STABLE | Genomic variant reading |
| `genomic::ld` | Linkage disequilibrium | ✅ STABLE | Statistical genetics |
| `mutation::adaptive` | Adaptive strategies | ✅ STABLE | Domain-aware mutations |
| `mutation::portfolio_learning` | Portfolio management | ✅ STABLE | Strategic diversification |

### External Dependencies (PRODUCTION)

| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| tokio | 1.38+ | Async runtime | ✅ STABLE |
| actix-web | 4.4+ | Web framework | ✅ STABLE |
| serde | 1.0+ | Serialization | ✅ STABLE |
| rayon | 1.7+ | Parallel computing | ✅ STABLE |

---

## Dependency Health Check

### Import Validation Rules

1. **All imports must reference STABLE or BETA modules only**
2. **Module must be registered in this file before importing**
3. **No circular dependencies allowed**

### Validation Procedures

- **Pre-commit:** Run `./scripts/validate-imports.sh`
- **CI/CD:** Run `./scripts/audit-modules.sh` on every push
- **Review:** Check ARCHITECTURE.md updated in all PRs

---

## Module Status Definitions

- **STABLE**: Production-ready, 100% test pass rate, used by 2+ binaries
- **BETA**: Tested and integrated, may be used by 1-2 binaries
- **EXPERIMENTAL**: Under development, limited testing
- **DISABLED**: Not available for import, binary cannot run

---

**Maintained by:** Development Governance Board  
**Last Updated:** 2026-07-28
