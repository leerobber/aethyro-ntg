# Aethyro NTG: Operations Guide

**Version:** 1.0  
**Date:** 2026-07-16  
**Audience:** DevOps, SRE, Platform Engineers  

---

## 1. CI/CD Pipeline

### 1.1 GitHub Actions Workflow

Create `.github/workflows/ci-cd.yml`:

```yaml
name: CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  release:
    types: [published]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - uses: actions-rs/toolchain@v1
      with:
        profile: minimal
        toolchain: 1.75
        override: true
        components: rustfmt, clippy
    
    - uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          kernel/target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cd kernel && cargo fmt -- --check
    
    - name: Run clippy
      run: cd kernel && cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Run tests
      run: cd kernel && cargo test --all
    
    - name: Run doc tests
      run: cd kernel && cargo test --doc
    
    - name: Generate coverage
      run: |
        cargo install cargo-tarpaulin
        cd kernel && cargo tarpaulin -o Xml --out coverage.xml
    
    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        files: ./coverage.xml

  benchmark:
    runs-on: ubuntu-latest-m
    needs: test
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
    - uses: actions/checkout@v4
      with:
        fetch-depth: 0
    
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: 1.75
        override: true
    
    - name: Run benchmarks
      run: cd kernel && cargo bench --no-run
    
    - name: Store benchmark results
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: kernel/target/criterion/output.txt
        github-token: ${{ secrets.GITHUB_TOKEN }}
        auto-push: true

  build-container:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    
    steps:
    - uses: actions/checkout@v4
    
    - uses: docker/setup-buildx-action@v2
    
    - uses: docker/login-action@v2
      with:
        registry: ${{ env.REGISTRY }}
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
    
    - uses: docker/metadata-action@v4
      id: meta
      with:
        images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
        tags: |
          type=ref,event=branch
          type=semver,pattern={{version}}
          type=semver,pattern={{major}}.{{minor}}
          type=sha
    
    - uses: docker/build-push-action@v4
      with:
        context: .
        push: ${{ github.event_name != 'pull_request' }}
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}
        cache-from: type=gha
        cache-to: type=gha,mode=max

  security-scan:
    runs-on: ubuntu-latest
    needs: test
    steps:
    - uses: actions/checkout@v4
    
    - uses: rustsec/audit-check-action@v1
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
```

### 1.2 Local CI Check

```bash
#!/bin/bash
# tools/ci-check.sh

set -e

echo "=== Format Check ==="
cargo fmt --check

echo "=== Clippy Lint ==="
cargo clippy --all-targets --all-features -- -D warnings

echo "=== Tests ==="
cargo test --all

echo "=== Doc Tests ==="
cargo test --doc

echo "=== Coverage ==="
cargo tarpaulin -o Html

echo "All checks passed!"
```

Run before commit:

```bash
chmod +x tools/ci-check.sh
./tools/ci-check.sh
```

---

## 2. Automated Benchmarking

### 2.1 Regression Detection

Create `tools/benchmark-compare.sh`:

```bash
#!/bin/bash
# Compare benchmarks across branches

set -e

BASELINE_BRANCH=${1:-"main"}
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

if [ "$CURRENT_BRANCH" == "$BASELINE_BRANCH" ]; then
    echo "Error: Cannot compare branch with itself"
    exit 1
fi

echo "Benchmarking baseline branch: $BASELINE_BRANCH"
git stash
git checkout $BASELINE_BRANCH
cargo bench --release > /tmp/baseline.txt

echo "Benchmarking current branch: $CURRENT_BRANCH"
git checkout $CURRENT_BRANCH
cargo bench --release > /tmp/current.txt

# Compare results
echo ""
echo "=== Benchmark Comparison ==="
diff -u /tmp/baseline.txt /tmp/current.txt || true

# Detect regressions (>10% slower)
if grep -q "time:" /tmp/baseline.txt; then
    baseline_time=$(grep "time:" /tmp/baseline.txt | head -1 | awk '{print $3}')
    current_time=$(grep "time:" /tmp/current.txt | head -1 | awk '{print $3}')
    
    ratio=$(echo "scale=2; $current_time / $baseline_time" | bc)
    if (( $(echo "$ratio > 1.1" | bc -l) )); then
        echo "WARNING: Potential performance regression detected ($ratio x slower)"
        exit 1
    fi
fi

echo "No regressions detected"
```

### 2.2 Continuous Benchmarking

Add to GitHub Actions (see 1.1) to automatically:

1. Run benchmarks on every push to `main`
2. Store results in GitHub Pages
3. Alert on regressions > 10%

---

## 3. Performance Regression Detection

### 3.1 Automated Alerts

```yaml
# .github/workflows/perf-regression.yml
name: Performance Regression Detection

on:
  workflow_run:
    workflows: ["CI/CD"]
    types: [completed]

jobs:
  check-regression:
    if: github.event.workflow_run.conclusion == 'success'
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Download artifacts
      uses: actions/download-artifact@v3
      with:
        name: benchmark-results
    
    - name: Check regression
      run: |
        BASELINE=$(cat baseline.json | jq '.duration')
        CURRENT=$(cat current.json | jq '.duration')
        RATIO=$(echo "scale=2; $CURRENT / $BASELINE" | bc)
        
        if (( $(echo "$RATIO > 1.1" | bc -l) )); then
          echo "::error::Performance regression: $RATIO x slower"
          exit 1
        fi
```

### 3.2 SLO Monitoring

Define Service Level Objectives:

```yaml
# metrics/slos.yaml
slos:
  - name: "P99 Latency"
    metric: "aethyro_infer_latency_ms"
    quantile: 0.99
    target: 500    # milliseconds
    window: 5m
  
  - name: "Error Rate"
    metric: "aethyro_errors_total"
    target: 0.001  # <0.1%
    window: 5m
  
  - name: "Availability"
    metric: "up"
    target: 0.999  # 99.9%
    window: 1h
  
  - name: "TOBL Utilization"
    metric: "aethyro_compute_tobl_ratio"
    target: 0.80   # >80% using TOBL
    window: 5m
```

### 3.3 Alert Rules

```yaml
# prometheus/aethyro-slo-alerts.yaml
groups:
- name: aethyro-slos
  rules:
  - alert: HighLatency
    expr: histogram_quantile(0.99, aethyro_infer_latency_ms) > 500
    for: 5m
    annotations:
      summary: "Aethyro P99 latency > 500ms"
      dashboard: "https://grafana.internal/d/aethyro"
  
  - alert: HighErrorRate
    expr: rate(aethyro_errors_total[5m]) > 0.001
    for: 5m
    annotations:
      summary: "Aethyro error rate > 0.1%"
  
  - alert: LowTOBLUtilization
    expr: aethyro_compute_tobl_ratio < 0.80
    for: 15m
    annotations:
      summary: "TOBL acceleration < 80%, check CPU features"
```

---

## 4. Monitoring & Observability

### 4.1 Prometheus Configuration

```yaml
# prometheus/aethyro.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
- job_name: aethyro-prod
  static_configs:
  - targets: [aethyro-1:9090, aethyro-2:9090, aethyro-3:9090]
  
  metric_relabel_configs:
  # Drop high-cardinality metrics
  - source_labels: [__name__]
    regex: '.*_bucket'
    action: drop
  
  # Add environment label
  - source_labels: []
    target_label: env
    replacement: production

- job_name: aethyro-canary
  scrape_interval: 30s
  static_configs:
  - targets: [aethyro-canary:9090]

# Service discovery (Consul)
- job_name: aethyro-consul
  consul_sd_configs:
  - server: consul.service.consul:8500
    services: [aethyro-ntg]
```

### 4.2 Grafana Dashboards

Create `grafana/dashboards/aethyro-overview.json`:

```json
{
  "dashboard": {
    "title": "Aethyro NTG Overview",
    "panels": [
      {
        "title": "Request Latency (P50/P99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(aethyro_infer_latency_ms_bucket[5m]))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.99, rate(aethyro_infer_latency_ms_bucket[5m]))",
            "legendFormat": "P99"
          }
        ]
      },
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(aethyro_infer_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(aethyro_errors_total[5m]) / rate(aethyro_infer_requests_total[5m])"
          }
        ]
      },
      {
        "title": "TOBL vs Scalar Ratio",
        "targets": [
          {
            "expr": "aethyro_compute_tobl_ratio"
          }
        ]
      },
      {
        "title": "Memory Usage",
        "targets": [
          {
            "expr": "aethyro_memory_usage_bytes / 1024 / 1024 / 1024"
          }
        ]
      },
      {
        "title": "CPU Usage",
        "targets": [
          {
            "expr": "rate(aethyro_cpu_usage_seconds_total[5m])"
          }
        ]
      }
    ]
  }
}
```

### 4.3 Structured Logging

```rust
// In src/main.rs
use tracing_subscriber;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Aethyro NTG starting"
    );
}
```

Log output:

```json
{
  "timestamp": "2026-07-16T10:30:00.123456Z",
  "level": "INFO",
  "message": "Aethyro NTG starting",
  "version": "1.0.0",
  "target": "ntg_kernel",
  "module_path": "ntg_kernel::main",
  "file": "src/main.rs",
  "line": 42
}
```

---

## 5. Container Registry & Image Scanning

### 5.1 Image Build & Push

```bash
#!/bin/bash
# tools/build-release.sh

set -e

VERSION=$1
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

REGISTRY="ghcr.io/yourorg"
IMAGE_NAME="aethyro-ntg"
FULL_IMAGE="$REGISTRY/$IMAGE_NAME:$VERSION"

echo "Building image: $FULL_IMAGE"
docker build -t "$FULL_IMAGE" \
    --build-arg VERSION="$VERSION" \
    .

# Scan for vulnerabilities
echo "Scanning image for vulnerabilities..."
docker run --rm \
    -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image "$FULL_IMAGE"

echo "Pushing image to registry..."
docker push "$FULL_IMAGE"

# Tag as latest
docker tag "$FULL_IMAGE" "$REGISTRY/$IMAGE_NAME:latest"
docker push "$REGISTRY/$IMAGE_NAME:latest"

echo "Image published successfully"
```

### 5.2 Image Scanning in CI

```yaml
# .github/workflows/container-scan.yml
name: Container Security Scan

on:
  push:
    branches: [main]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - uses: docker/setup-buildx-action@v2
    
    - uses: docker/build-push-action@v4
      with:
        context: .
        push: false
        load: true
        tags: aethyro-ntg:scan
    
    - name: Scan with Trivy
      uses: aquasecurity/trivy-action@master
      with:
        image-ref: 'aethyro-ntg:scan'
        format: 'sarif'
        output: 'trivy-results.sarif'
    
    - name: Upload Trivy results
      uses: github/codeql-action/upload-sarif@v2
      with:
        sarif_file: 'trivy-results.sarif'
```

---

## 6. Automated Backups

### 6.1 Backup Strategy

```bash
#!/bin/bash
# tools/backup-models.sh

set -e

BACKUP_DIR="/backups/aethyro"
BUCKET="s3://aethyro-backups"
RETENTION_DAYS=30

mkdir -p "$BACKUP_DIR"

# Create backup
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/models_$TIMESTAMP.tar.gz"

echo "Creating backup: $BACKUP_FILE"
tar -czf "$BACKUP_FILE" /var/lib/aethyro/models/

# Verify backup
tar -tzf "$BACKUP_FILE" > /dev/null
echo "Backup verified"

# Upload to S3
echo "Uploading to S3..."
aws s3 cp "$BACKUP_FILE" "$BUCKET/" \
    --sse AES256 \
    --storage-class GLACIER_IR

# Create manifest
cat > "$BACKUP_DIR/backup_manifest.json" <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "file": "models_$TIMESTAMP.tar.gz",
  "size_bytes": $(stat -f%z "$BACKUP_FILE" 2>/dev/null || stat -c%s "$BACKUP_FILE"),
  "checksum": "$(sha256sum "$BACKUP_FILE" | cut -d' ' -f1)"
}
EOF

# Clean old backups
echo "Cleaning old backups..."
find "$BACKUP_DIR" -name "models_*.tar.gz" -mtime "+$RETENTION_DAYS" -delete

echo "Backup complete"
```

### 6.2 Scheduled Backup (Cron)

```bash
# Add to crontab -e
# Daily backup at 2 AM UTC
0 2 * * * /opt/aethyro/tools/backup-models.sh >> /var/log/aethyro-backup.log 2>&1

# Or with systemd timer
# /etc/systemd/system/aethyro-backup.service
[Unit]
Description=Aethyro Backup
After=network-online.target

[Service]
Type=oneshot
ExecStart=/opt/aethyro/tools/backup-models.sh
StandardOutput=journal
StandardError=journal

---
# /etc/systemd/system/aethyro-backup.timer
[Unit]
Description=Daily Aethyro Backup

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable timer:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now aethyro-backup.timer
sudo systemctl status aethyro-backup.timer
```

---

## 7. Cost Monitoring

### 7.1 Kubernetes Resource Metrics

```bash
#!/bin/bash
# tools/cost-analysis.sh

echo "=== CPU and Memory Usage ==="
kubectl top nodes

echo ""
echo "=== Pod Resource Usage ==="
kubectl top pods -n aethyro-ntg

echo ""
echo "=== Estimated Cost (8 hours @ $0.10/vCPU, $0.02/GB/hour) ==="

# Calculate vCPU costs
CPU_HOURS=$(kubectl top pods -n aethyro-ntg --no-headers | \
    awk '{print $2}' | \
    sed 's/m//' | \
    awk '{sum+=$1} END {print sum/1000}')

MEM_GB=$(kubectl top pods -n aethyro-ntg --no-headers | \
    awk '{print $3}' | \
    sed 's/Mi//' | \
    awk '{sum+=$1} END {print sum/1024}')

CPU_COST=$(echo "scale=2; $CPU_HOURS * 0.10" | bc)
MEM_COST=$(echo "scale=2; $MEM_GB * 8 * 0.02" | bc)

echo "CPU Cost: \$$CPU_COST"
echo "Memory Cost: \$$MEM_COST"
echo "Total (8h): \$$(echo "scale=2; $CPU_COST + $MEM_COST" | bc)"
echo "Est. Monthly (assuming consistent load): \$$(echo "scale=2; ($CPU_COST + $MEM_COST) * 90" | bc)"
```

---

## 8. On-Call Runbook

### 8.1 Alert: High Latency

**Alert:** `aethyro_infer_latency_ms_p99 > 500ms`

**Investigation:**

```bash
# 1. Check pod status
kubectl get pods -l app=aethyro-ntg -o wide

# 2. Check resource usage
kubectl top pods -l app=aethyro-ntg
kubectl describe node <node-name>

# 3. Check logs
kubectl logs -l app=aethyro-ntg --tail=100 -f

# 4. Check metrics
curl -s http://aethyro-1:9090/metrics | grep latency

# 5. Restart pod if needed
kubectl rollout restart deployment/aethyro-ntg
```

**Mitigation:**

- Scale up: `kubectl scale deployment aethyro-ntg --replicas=5`
- Increase timeouts temporarily
- Page on-call engineer

### 8.2 Alert: High Error Rate

**Alert:** `rate(aethyro_errors_total[5m]) > 0.001`

**Investigation:**

```bash
# Check error types
kubectl logs -l app=aethyro-ntg | grep ERROR | head -20

# Check recent changes
git log --oneline -10
kubectl rollout history deployment/aethyro-ntg

# Check dependencies
kubectl get pods -l app=ledger-authority
kubectl logs -l app=ledger-authority
```

**Mitigation:**

- Rollback if recent deployment: `kubectl rollout undo deployment/aethyro-ntg`
- Restart: `kubectl rollout restart deployment/aethyro-ntg`
- Check model integrity: `./aethyro-ntg --verify-model`

### 8.3 Alert: Ledger Verification Failed

**Alert:** `aethyro_ledger_verification_failures_total > 0`

**Investigation:**

```bash
# Check ledger status
./aethyro-ntg --verify-ledger /var/lib/aethyro/ledger

# Check for corruption
sha256sum -c /var/lib/aethyro/ledger/checksum.sha256

# Check disk health
df -h /var/lib/aethyro/ledger
smartctl -a /dev/sda
```

**Mitigation:**

- Restore from backup
- Flush ledger buffer: `curl -X POST http://aethyro:8080/v1/ledger/flush`
- Escalate to storage team

---

## 9. Runbook Templates

### 9.1 Incident Response Template

```markdown
# Incident: [INCIDENT_NAME]

**Date:** 2026-07-16T10:30:00Z
**Duration:** HHm
**Severity:** P1/P2/P3
**Impact:** [Describe impact]

## Timeline

- 10:30 - Alert fired
- 10:35 - On-call acknowledged
- 10:40 - Root cause identified
- 10:50 - Mitigation deployed
- 11:00 - Verified resolved

## Root Cause

[Clear description of what happened]

## Mitigation

[Steps taken to resolve]

## Prevention

[How to prevent in future]
```

### 9.2 Escalation Path

```
Level 1 (On-Call)
  └─> Page Level 2 (Senior Engineer) after 15m no resolution
       └─> Page Level 3 (Engineering Manager) after 30m
            └─> VP Engineering if customer-impacting
```

---

## 10. Operational Metrics Dashboard

Create `grafana/dashboards/operations.json` to track:

- Deployment frequency
- Lead time for changes
- Mean time to recovery (MTTR)
- Change failure rate

Example metrics:

```yaml
# Deployment Frequency (daily)
- Query: `count(increase(deployments_total[1d])) by (job)`

# Lead Time (hours)
- Query: `(deploy_timestamp - merge_timestamp) / 3600`

# MTTR (minutes)
- Query: `(resolved_timestamp - alert_timestamp) / 60`

# Change Failure Rate (%)
- Query: `failed_deployments / total_deployments * 100`
```

---

## Version History

- **v1.0 (2026-07-16):** Production operations guide
- **v0.9 (2026-07-09):** CI/CD pipeline draft

