# Governance Structure

**Effective Date:** 2026-07-28  
**Authority:** Development Governance Board  
**Meeting Cadence:** First Friday of each month, 09:00 UTC

---

## Executive Summary

This document establishes the governance framework for Aethyro-NTG, following enterprise standards. All development work is subject to board review and approval, with clear authority structures, escalation paths, and accountability mechanisms.

---

## Review Board Members

### Core Members

| Role | Responsibilities |
|------|------------------|
| **AI Development Agent** (Claude) | Implements phases, maintains code, escalates decisions |
| **Human Developer** (Project Owner) | Strategic direction, phase sign-off, escalations |

### Phase Approval Process

1. **Phase Definition** - Clear scope, prerequisites, acceptance criteria
2. **Implementation** - Code changes following 6-step cycle
3. **TASKLOG Entry** - Document work with checksum
4. **Push and PR** - Feature branch → GitHub PR
5. **Review** - Human Developer reviews code
6. **Sign-Off** - Approval and merge to main
7. **Merge** - Clean history maintained

---

## Escalation Path

### Level 1: Self-Service
- Check PROTOCOL.md sections 1-3
- Review TASKLOG.md for precedent
- Run scripts/audit-modules.sh

### Level 2: Repository History
- Review git log for similar work
- Check prior TASKLOG entries
- Read PR review comments

### Level 3: Formal Review
- Create GitHub Issue with "Protocol Question" label
- Reference PROTOCOL.md sections
- Wait for Human Developer decision

---

**Maintained by:** Development Governance Board  
**Last Updated:** 2026-07-28
