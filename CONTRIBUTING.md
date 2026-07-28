# Contributing Guidelines

**Version:** 1.0.0  
**Last Updated:** 2026-07-28  
**Authority:** Development Governance Board

All work must follow **PROTOCOL.md**.

## Quick Start

1. **Read:** PROTOCOL.md → ARCHITECTURE.md → GOVERNANCE.md
2. **Branch:** `git checkout -b phase-X-description`
3. **Cycle:** Plan → Implement → Validate → Document → Commit → Verify
4. **Test:** `cargo test --release && cargo clippy --all-targets && ./scripts/audit-modules.sh`
5. **PR:** Push and create GitHub PR with TASKLOG.md link
6. **Review:** Wait for Human Developer approval and merge

## Code Quality

- ✅ Run tests before pushing
- ✅ Update ARCHITECTURE.md if adding modules
- ✅ Update TASKLOG.md when done
- ✅ Write clear commit messages
- ✅ Run module audit: `./scripts/audit-modules.sh`

❌ Never:
- Push to main directly
- Skip tests
- Import DISABLED modules
- Force-push
- Merge without approval

---

**Questions?** See PROTOCOL.md Section 10 (Escalation Path)
