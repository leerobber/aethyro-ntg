# Aethyro NTG: Production Handbook

**Version:** 1.0  
**Date:** 2026-07-16  
**Status:** Production-Ready  

---

## Overview

This handbook contains complete documentation for deploying, operating, and developing Aethyro NTG in production. It consolidates five key documentation guides into a unified reference.

### Quick Navigation

| Audience | Document | Purpose |
|---|---|---|
| **Architects** | [ARCHITECTURE_GUIDE.md](ARCHITECTURE_GUIDE.md) | System layers, data flow, API contracts, performance model |
| **DevOps/SRE** | [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) | Container builds, K8s deployment, config management, disaster recovery |
| **End Users** | [USER_GUIDE.md](USER_GUIDE.md) | API reference, client libraries, examples, troubleshooting |
| **Engineers** | [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) | Local development, testing, profiling, contribution guidelines |
| **Platform** | [OPERATIONS_GUIDE.md](OPERATIONS_GUIDE.md) | CI/CD pipeline, monitoring, automated benchmarking, on-call runbooks |

---

## Getting Started by Role

### For DevOps Engineers

**Goal:** Deploy Aethyro NTG to production

**Sequence:**

1. Read [DEPLOYMENT_GUIDE.md § 1: Container Deployment](DEPLOYMENT_GUIDE.md#1-container-deployment)
2. Read [DEPLOYMENT_GUIDE.md § 2: Configuration Management](DEPLOYMENT_GUIDE.md#2-configuration-management)
3. Deploy using: [k8s/deployment.yaml](../k8s/deployment.yaml)
4. Set up monitoring using: [k8s/monitoring.yaml](../k8s/monitoring.yaml)
5. Configure backup strategy: [DEPLOYMENT_GUIDE.md § 8: Disaster Recovery](DEPLOYMENT_GUIDE.md#8-disaster-recovery)

**Expected time:** 2-4 hours

### For Application Developers

**Goal:** Integrate Aethyro NTG into your application

**Sequence:**

1. Read [USER_GUIDE.md § 1: API Reference](USER_GUIDE.md#1-api-reference)
2. Choose client library: [USER_GUIDE.md § 2-3: Clients](USER_GUIDE.md#2-pythonclient-library)
3. Run examples: [USER_GUIDE.md § 5: Examples](USER_GUIDE.md#5-examples)
4. Troubleshoot issues: [USER_GUIDE.md § 4: Troubleshooting](USER_GUIDE.md#4-troubleshooting)

**Expected time:** 30 minutes - 2 hours

### For Kernel Engineers

**Goal:** Develop and optimize the core engine

**Sequence:**

1. Set up environment: [DEVELOPER_GUIDE.md § Quick Start](DEVELOPER_GUIDE.md#quick-start-for-developers)
2. Understand architecture: [ARCHITECTURE_GUIDE.md § 1-2: Layers & Data Flow](ARCHITECTURE_GUIDE.md#1-system-layers)
3. Add features: [DEVELOPER_GUIDE.md § 2.1: Feature Development](DEVELOPER_GUIDE.md#21-adding-a-new-feature)
4. Profile performance: [DEVELOPER_GUIDE.md § 5: Profiling](DEVELOPER_GUIDE.md#5-profiling--optimization)
5. Run test suite: [DEVELOPER_GUIDE.md § 3: Testing](DEVELOPER_GUIDE.md#3-testing-guide)

**Expected time:** Depends on task complexity

### For SRE/On-Call

**Goal:** Maintain uptime and respond to incidents

**Sequence:**

1. Read [OPERATIONS_GUIDE.md § 1-2: CI/CD & Benchmarking](OPERATIONS_GUIDE.md#1-cicd-pipeline)
2. Understand monitoring: [OPERATIONS_GUIDE.md § 4: Monitoring](OPERATIONS_GUIDE.md#4-monitoring--observability)
3. Learn runbooks: [OPERATIONS_GUIDE.md § 8: On-Call Runbook](OPERATIONS_GUIDE.md#8-on-call-runbook)
4. Review alert rules: [k8s/monitoring.yaml](../k8s/monitoring.yaml)

**Expected time:** 4-6 hours initial, then ongoing

---

## Key Characteristics

### Performance

```
Throughput:    12-20x speedup vs scalar baseline (TOBL acceleration)
Latency P99:   ~150ms (dense), ~50ms (sparse > 90%)
Memory:        87.5% compression vs FP32 (ternary packing)
```

### Deployment

```
Container:     Multi-stage Docker build, minimal runtime image
Orchestration: Kubernetes with HPA, 3-10 pod scaling
HA:            Read-only replica sets, stateless design
Backup:        Automated daily snapshots to S3
```

### Safety

```
Audit:         Deterministic replay ledger (opt-in)
Verification:  SHA-256 chain integrity on startup
Access:        HMAC-SHA256 request signing (optional)
Isolation:     Non-root container, seccomp profiles
```

---

## Essential Operations

### Deploy New Version

```bash
# 1. Build and push container
./tools/build-release.sh 1.0.1

# 2. Update K8s deployment
kubectl set image deployment/aethyro-ntg \
  aethyro=ghcr.io/yourorg/aethyro-ntg:1.0.1 -n aethyro-ntg

# 3. Monitor rollout
kubectl rollout status deployment/aethyro-ntg -n aethyro-ntg

# 4. Verify
curl http://aethyro-ntg:8080/health
```

### Scale Deployment

```bash
# Automatic (via HPA)
kubectl get hpa -n aethyro-ntg

# Manual
kubectl scale deployment aethyro-ntg --replicas=5 -n aethyro-ntg
```

### Check Health

```bash
# Pods
kubectl get pods -l app=aethyro-ntg -n aethyro-ntg

# Metrics
curl http://aethyro-ntg:9090/metrics

# Logs
kubectl logs -l app=aethyro-ntg -n aethyro-ntg --tail=100 -f
```

### Handle Incident

1. **Check status:** `kubectl get pods -n aethyro-ntg`
2. **View logs:** `kubectl logs -f deployment/aethyro-ntg -n aethyro-ntg`
3. **Restart pod:** `kubectl rollout restart deployment/aethyro-ntg -n aethyro-ntg`
4. **Rollback:** `kubectl rollout undo deployment/aethyro-ntg -n aethyro-ntg`
5. **Page on-call:** Send alert via incident management tool

---

## Configuration Checklist

### Before Production Deployment

- [ ] Container image built and pushed to registry
- [ ] Kubernetes manifests reviewed and tested in staging
- [ ] TLS certificates provisioned (if needed)
- [ ] Persistent volumes configured and backed up
- [ ] Prometheus/Grafana dashboards deployed
- [ ] Alert rules configured
- [ ] On-call rotation set up
- [ ] Backup procedures tested (restore successfully)
- [ ] Network policies applied
- [ ] Resource limits verified

### Daily Operations

- [ ] Monitor alert dashboard
- [ ] Review error logs
- [ ] Check backup completion
- [ ] Verify ledger height (if enabled)
- [ ] Monitor cost/resource trends

### Weekly Operations

- [ ] Review performance trends
- [ ] Check for pending updates
- [ ] Test backup restoration
- [ ] Review incident summaries
- [ ] Check for security advisories

### Monthly Operations

- [ ] Full disaster recovery drill
- [ ] Capacity planning review
- [ ] Dependency update audit
- [ ] Cost analysis
- [ ] Performance regression analysis

---

## Support Matrix

### Component Versions

| Component | Minimum | Recommended | Maximum |
|---|---|---|---|
| Rust | 1.70 | 1.75+ | Latest |
| Kubernetes | 1.24 | 1.26+ | Latest |
| Docker | 20.10 | 24.0+ | Latest |
| Linux | kernel 5.4 | 5.15+ | Latest |

### Platform Support

- **CPU:** x86-64 (TOBL requires AVX2 or better)
- **OS:** Linux (glibc 2.31+) or musl
- **RAM:** Minimum 2GB, recommended 4GB+
- **Disk:** Minimum 1GB for models, 500MB for ledger

### Browser Compatibility (Dashboards)

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

---

## Documentation Map

```
docs/
├── README.md                          # Main project README
├── ARCHITECTURE_GUIDE.md              # System design
├── DEPLOYMENT_GUIDE.md                # Ops deployment
├── USER_GUIDE.md                      # API & client usage
├── DEVELOPER_GUIDE.md                 # Engineering guide
├── OPERATIONS_GUIDE.md                # CI/CD & monitoring
├── PRODUCTION_HANDBOOK.md             # This file
├── STATUS.md                          # Current project status
├── ROADMAP.md                         # Future phases
├── DESIGN.md                          # Technical details
├── LITERATURE.md                      # Academic references
└── architecture/                      # Architecture Decision Records
    ├── 0001-vision-and-pivot.md
    ├── 0002-safety-rails.md
    └── ...
```

---

## Troubleshooting Matrix

| Symptom | Check | Solution |
|---|---|---|
| High P99 latency | CPU throttling, memory pressure | Scale up, increase thread count |
| High error rate | Application logs, network connectivity | Check model integrity, restart pod |
| Ledger verification error | Disk health, file permissions | Restore from backup |
| Pod OOM killed | Memory limit, batch size | Increase memory limit or reduce batch |
| No TOBL acceleration | CPU feature detection | Verify AVX2+ CPU, check `dmesg` |
| Connection refused | Port listening, firewall | Check service, verify network policy |

---

## Escalation Procedures

```
Level 1 (On-Call)
  Investigate for 15 min
  └─> Page Level 2 if unresolved

Level 2 (Senior Engineer)
  Coordinate response for 30 min
  └─> Page VP Engineering if customer-impacting

Level 3 (VP Engineering)
  Coordinate cross-functional response
  └─> CEO notification if critical
```

---

## Key Contacts & Resources

**Internal:**
- Slack: #aethyro-ntg-eng
- On-call: [PagerDuty escalation](https://pagerduty.com/...)

**External:**
- Documentation: https://docs.aethyro.com
- GitHub: https://github.com/yourorg/aethyro-ntg
- Issues: https://github.com/yourorg/aethyro-ntg/issues

**Support:**
- Email: support@aethyro.com
- Discord: https://discord.gg/aethyro
- Status Page: https://status.aethyro.com

---

## Change Log

### v1.0 (2026-07-16)

**Production Release**

- [x] ARCHITECTURE_GUIDE.md — Complete system design
- [x] DEPLOYMENT_GUIDE.md — Production deployment procedures
- [x] USER_GUIDE.md — API reference and examples
- [x] DEVELOPER_GUIDE.md — Engineering development guide
- [x] OPERATIONS_GUIDE.md — CI/CD and monitoring setup
- [x] CI/CD pipeline (.github/workflows/ci-cd.yml)
- [x] Container image (Dockerfile)
- [x] Kubernetes manifests (k8s/deployment.yaml)
- [x] Monitoring & alerting (k8s/monitoring.yaml)
- [x] On-call runbooks (OPERATIONS_GUIDE.md § 8)
- [x] Automated benchmarking (OPERATIONS_GUIDE.md § 3)

**Completeness:** 100%

---

## Next Steps

1. **Review** — Read the appropriate guide for your role
2. **Plan** — Schedule deployment/development work
3. **Communicate** — Notify stakeholders of timeline
4. **Execute** — Follow procedures in chosen guide
5. **Monitor** — Use monitoring setup (§ 4)
6. **Improve** — Document lessons learned, update runbooks

---

## Document Governance

**Owners:**
- Architecture: @design-team
- Deployment: @devops-team
- Operations: @platform-team
- Development: @engineering-team

**Review Cycle:** Quarterly or on major releases

**Update Process:**
1. Create GitHub issue with proposed changes
2. Get approval from document owner
3. Submit PR with updates
4. Merge after review

**Version Control:** Git history in `docs/` directory

---

## Support This Project

Questions or feedback about this handbook?

- **Found an error?** → Create [GitHub issue](https://github.com/yourorg/aethyro-ntg/issues)
- **Have suggestions?** → [Discussions](https://github.com/yourorg/aethyro-ntg/discussions)
- **Want to contribute?** → See [CONTRIBUTING.md](../CONTRIBUTING.md)

---

**Last Updated:** 2026-07-16  
**Maintained By:** Platform & DevOps Teams  
**Status:** ACTIVE ✓

