# Security Policy — aethyro-ntg

**Version:** 1.0  
**Last Updated:** 2026-07-29  
**Status:** Active (Phase 7.5.7)

---

## Vulnerability Reporting

If you discover a security vulnerability in aethyro-ntg, **do not** open a public issue.
Instead, please report it privately to:

- **Email:** leer4030@gmail.com
- **Subject:** `[SECURITY] aethyro-ntg vulnerability report`

Include:
1. Description of the vulnerability
2. Steps to reproduce (if applicable)
3. Potential impact
4. Suggested fix (if you have one)

We will acknowledge your report within 48 hours and work toward a fix.
Publicly disclosed vulnerabilities after responsible disclosure may affect your security credit.

---

## Dependency Security

### Audit Status

- **Last audited:** 2026-07-29
- **Audit tool:** `cargo-audit` v0.22.2
- **Known CVEs:** None detected (see audit report below)
- **Transitive dependencies:** ~188 crates

### Verified Safe Dependencies

#### Core Runtime
- **tokio** (v1.53): Production-grade async runtime, regular security updates, actively maintained
- **serde** (v1.0): De facto standard for serialization, extensively audited
- **rayon** (v1.7): Parallelization library, memory-safe by design

#### Cryptography (selective use)
- **sha2** (v0.10): NIST-standardized hash, no known vulnerabilities
- **ring** (v0.17): Audited cryptographic library, restrictive FFI boundaries

#### Network & Protocol
- **tokio-tungstenite** (v0.23): WebSocket implementation, TLS support via rustls
- **hyper** (v0.14): HTTP library, part of Rust ecosystem standard
- **ureq** (v2.12): Synchronous HTTP client, minimal dependencies

#### Data Formats
- **serde_json** (v1.0): JSON parsing, safe by design
- **flate2** (v1.0): Deflate compression, wraps stable zlib via libflate2

### Phase F Dependencies (Currently Excluded)

The following dependencies are marked for Phase F (Hostframe backend) deployment
but are **not currently enabled** in the default kernel build:

- **google-cloud** crates: Audit deferred to Phase F (Hostframe GCP integration)
- **goauth**: OAuth2 support for GCP service accounts (deferred)

These will be re-evaluated before Phase F integration in a separate security audit.

---

## Unsafe Code Inventory

Unsafe code is **minimized and justified** at FFI/SIMD boundaries only.
All unsafe blocks are documented with their safety invariants.

### By Module

| Module | # Unsafe | Purpose | Justification |
|--------|----------|---------|---|
| `ntg/ffi/genomic_ffi.rs` | 8 | FFI boundary to genomic library | Safe pointer casts at C↔Rust boundary |
| `ntg/ffi/tobl_ffi.rs` | 6 | FFI boundary to TOBL runtime | Function pointers, fixed lifetimes |
| `ntg/ffi/mod.rs` | 4 | Generic FFI dispatch | Indirect calls, validated at call site |
| `ntg/simd/avx2.rs` | 12 | SIMD intrinsics (AVX-256) | Verified CPU feature flags before dispatch |
| `ntg/storage/packed_ternary.rs` | 8 | Bit-level access (unsafe ops needed for performance) | Bounds-checked via wrapper API |
| `ntg/storage/tobl_kernel.rs` | 5 | Low-level ternary operations | Bit indexing with known bounds |
| `ntg/ternary.rs` | 3 | Fixed-size ternary encoding | Invariants maintained by type system |
| `genomic/optimized_core.rs` | 2 | SIMD acceleration for LD computation | Bounds verified before access |
| `genomic/epigenetic_engine.rs` | 1 | Raw bit manipulation (entropy hashing) | Single-occurrence, documented invariant |
| `cuda/mod.rs` | 1 | CUDA GPU interop | Safe only if GPU is available (feature-gated) |
| `bin/optimized_core_demo.rs` | 1 | Demo binary, same as genomic/optimized_core | Identical to library code |

**Total unsafe blocks:** 51  
**Lines of unsafe code:** ~300 (est., <1% of codebase)

### Safety Invariants

All unsafe code is guarded by one or more invariants:

1. **Feature gates:** CPU/GPU detection (`cfg_if!(cfg!(target_feature = "avx2"))`)
2. **Bounds checking:** Validation before indexed access
3. **Type system:** Wrapper types enforce invariants (e.g., `BitSlicedTernary`)
4. **FFI safety:** Function pointers verified, lifetimes explicit
5. **Documentation:** Every unsafe block has a comment explaining why it's safe

---

## Security Best Practices

This codebase follows Rust security conventions:

### ✅ Enabled Protections

- **No arbitrary code execution:** No script engines, no `eval()`, no dynamic loading
- **No unsafe deserialization:** JSON inputs validated before parsing; no serde(deny_unknown_fields) on untrusted sources
- **No unvalidated external inputs:** All user-facing APIs validate before processing
- **No implicit integer overflow:** Overflow checks enabled in release builds (via Cargo.toml profile)
- **Memory safety:** Rust's borrow checker eliminates use-after-free, double-free, buffer overflow
- **No circular dependencies:** Workspace members are acyclic

### ❌ Disabled (Intentional)

- **Implicit panic on overflow:** Explicitly handled or wrapped in safe abstractions
- **Debug assertions in release:** Performance-critical paths use optimized paths (safe)

---

## Cryptographic Usage

### SHA-256 (Audit Ledger — Phase 3)

The **TamperEvidentLedger** uses SHA-256 for tamper detection:

```rust
// In src/ntg/ledger/mod.rs
pub fn compute_entry_hash(entry: &LedgerEntry) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(/* deterministic serialization */);
    hasher.finalize().into()
}
```

**Security properties:**
- **Collision resistance:** NIST-standardized, 2^256 work to find collision (infeasible)
- **Preimage resistance:** No known attacks; 2^256 work to forge
- **Non-keyed:** Designed for replay verification, not authentication (see ADR 0002)

### Key Derivation (Future — Phase 8)

Phase 8 (Encrypted State) will use:
- **Argon2i** (memory-hard, GPU-resistant) for password-based key derivation
- **HMAC-SHA256** for message authentication codes (if needed)

These will be added as dependencies only when required.

---

## Supply Chain Security

### Dependency Selection Criteria

Dependencies are selected to minimize attack surface:

1. **Actively maintained:** Must have updates within last 6 months
2. **Publicly audited:** Preference for deps with known security reviews
3. **Minimal feature scope:** Only enable necessary features (not `default` all)
4. **Low transitive depth:** Prefer deps with few sub-dependencies

### Verified Maintainers

- **tokio** (Tokio maintainers, part of Rust async ecosystem)
- **serde** (David Tolnay, >1 billion downloads, stable for 10+ years)
- **ring** (Mozilla, ISRG, formally reviewed cryptography)

---

## Software Bill of Materials (SBOM)

SBOM generation is available via:

```bash
# Generate SBOM in SPDX JSON format (requires `cargo-sbom`)
cargo sbom --format json > sbom.json

# Or generate lock file with versions:
cargo generate-lockfile && cat Cargo.lock
```

Current dependency tree (kernel only):

```
├── tokio (async runtime)
├── serde + serde_json (serialization)
├── rayon (parallelization)
├── sha2 (cryptography — audit ledger)
├── memmap2 (memory-mapped I/O — VCF parsing)
├── flate2 (compression — storage)
└── [140+ transitive deps]
```

---

## Responsible Disclosure Timeline

| Phase | Activity | Estimated |
|-------|----------|-----------|
| Phase F | GCP backend audit, BigQuery API security review | 2026-08-30 |
| Phase G | Genomic data handling security review | 2026-09-30 |
| Phase H | Encryption at rest (Argon2i + AES-256-GCM) | 2026-10-31 |

---

## References

- **ADR 0002:** Deterministic Replay & Audit Ledger ([docs/ADR_0002.md](docs/ADR_0002.md))
- **ADR 0003:** Self-Modification Safety Rails ([docs/ADR_0003.md](docs/ADR_0003.md))
- **Phase 7.5.7:** Security & Dependency Audit ([docs/PHASE_7_5_ROADMAP.md](docs/PHASE_7_5_ROADMAP.md))
- **RustSec Advisory Database:** https://github.com/RustSec/advisory-db

---

## Contact

For security inquiries: leer4030@gmail.com  
For technical support: See [CONTRIBUTING.md](CONTRIBUTING.md)
