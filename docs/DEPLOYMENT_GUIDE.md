# Aethyro NTG: Production Deployment Guide

**Version:** 1.0  
**Date:** 2026-07-16  
**Status:** Production-Ready  

---

## Quick Start: 5-Minute Deployment

```bash
# 1. Clone and build
git clone https://github.com/yourorg/aethyro-ntg.git
cd aethyro-ntg
cargo build --release

# 2. Start in-process server
./target/release/kernel_host --config aethyro.yaml

# 3. Test inference
curl -X POST http://localhost:8080/v1/infer \
  -H "Content-Type: application/json" \
  -d '{"input": [0.5, -0.3, 0.8]}'
```

---

## 1. Container Deployment

### 1.1 Build Docker Image

Create `Dockerfile`:

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /build
COPY kernel Cargo.toml ./
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/kernel_host /usr/local/bin/
COPY models /models
COPY aethyro.yaml /etc/aethyro/aethyro.yaml

EXPOSE 8080 9090
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/kernel_host"]
CMD ["--config", "/etc/aethyro/aethyro.yaml"]
```

Build and push:

```bash
docker build -t aethyro-ntg:1.0.0 .
docker tag aethyro-ntg:1.0.0 gcr.io/your-project/aethyro-ntg:1.0.0
docker push gcr.io/your-project/aethyro-ntg:1.0.0
```

### 1.2 Kubernetes Deployment

Create `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aethyro-ntg
  labels:
    app: aethyro-ntg
    version: v1
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: aethyro-ntg
  template:
    metadata:
      labels:
        app: aethyro-ntg
    spec:
      containers:
      - name: aethyro
        image: gcr.io/your-project/aethyro-ntg:1.0.0
        imagePullPolicy: IfNotPresent
        
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        
        env:
        - name: RUST_LOG
          value: "info"
        - name: AETHYRO_LEDGER
          value: "1"
        - name: AETHYRO_THREADS
          value: "4"
        
        resources:
          requests:
            cpu: "2"
            memory: "2Gi"
          limits:
            cpu: "4"
            memory: "4Gi"
        
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 15
          periodSeconds: 30
          timeoutSeconds: 5
          failureThreshold: 3
        
        readinessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 2
        
        volumeMounts:
        - name: models
          mountPath: /models
          readOnly: true
        - name: ledger
          mountPath: /var/lib/aethyro/ledger
        - name: cache
          mountPath: /tmp/aethyro-cache
      
      volumes:
      - name: models
        configMap:
          name: aethyro-models
      - name: ledger
        emptyDir: {}
      - name: cache
        emptyDir:
          sizeLimit: 1Gi

---
apiVersion: v1
kind: Service
metadata:
  name: aethyro-ntg
  labels:
    app: aethyro-ntg
spec:
  type: ClusterIP
  selector:
    app: aethyro-ntg
  ports:
  - name: http
    port: 8080
    targetPort: http
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: metrics
    protocol: TCP

---
apiVersion: autoscaling.k8s.io/v2
kind: HorizontalPodAutoscaler
metadata:
  name: aethyro-ntg-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: aethyro-ntg
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

Deploy:

```bash
kubectl apply -f deployment.yaml
kubectl rollout status deployment/aethyro-ntg
```

---

## 2. Configuration Management

### 2.1 Runtime Configuration

Create `aethyro.yaml`:

```yaml
aethyro:
  version: "1.0"
  environment: "production"

model:
  path: "/models/model.ntg"
  preload: true
  cache_warmup: true

runtime:
  threads: 8
  max_batch_size: 32
  request_timeout_ms: 5000
  
  accelerator:
    prefer: "tobl"
    fallback: "scalar"
    allow_scalar_fallback: true
  
  scheduling:
    strategy: "density_adaptive"  # Routes to TOBL/scalar based on layer density
    sparse_threshold: 0.9

ledger:
  enabled: true
  backend: "filesystem"
  path: "/var/lib/aethyro/ledger"
  
  # Persistence
  flush_interval_ms: 100
  max_buffer_blocks: 1000
  
  # Verification
  verify_on_startup: true
  checksum_algorithm: "sha256"

api:
  listen: "0.0.0.0:8080"
  timeout_ms: 5000
  max_connections: 1000
  
  # TLS (optional)
  tls:
    enabled: false
    cert_path: "/etc/aethyro/cert.pem"
    key_path: "/etc/aethyro/key.pem"
  
  # CORS
  cors:
    enabled: true
    allowed_origins:
      - "https://yourdomain.com"

monitoring:
  metrics_enabled: true
  metrics_port: 9090
  
  # Structured logging
  logging:
    level: "info"  # debug, info, warn, error
    format: "json"
    
  # Traces (optional OpenTelemetry)
  tracing:
    enabled: false
    endpoint: "http://jaeger:4317"

security:
  # Request signing (optional)
  require_signature: false
  hmac_key_env: "AETHYRO_HMAC_KEY"
  
  # Rate limiting per client
  rate_limit:
    enabled: true
    requests_per_second: 100
    burst_size: 10
```

### 2.2 Environment Variables

```bash
# Required
export AETHYRO_MODEL_PATH=/models/model.ntg
export AETHYRO_LISTEN=0.0.0.0:8080

# Optional
export RUST_LOG=info
export AETHYRO_LEDGER=1
export AETHYRO_THREADS=8
export AETHYRO_METRICS_PORT=9090
export AETHYRO_HMAC_KEY=your-secret-key
export AETHYRO_PROFILE=production
```

---

## 3. Persistent Storage

### 3.1 Model Storage

```bash
# Initialize model directory
mkdir -p /var/lib/aethyro/models
cd /var/lib/aethyro/models

# Copy or download model package
aws s3 cp s3://your-bucket/model.ntg.tar.gz .
tar -xzf model.ntg.tar.gz

# Verify checksum
sha256sum -c model.ntg/checksum.sha256

# Set permissions
chmod 555 model.ntg/
```

### 3.2 Ledger Storage

```bash
# Initialize ledger directory
mkdir -p /var/lib/aethyro/ledger
chmod 700 /var/lib/aethyro/ledger

# Enable ledger persistence (optional)
export AETHYRO_LEDGER=1
export AETHYRO_LEDGER_PATH=/var/lib/aethyro/ledger
```

### 3.3 Backup Strategy

```bash
#!/bin/bash
# daily-backup.sh

BACKUP_DIR="/backups/aethyro"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Backup ledger
tar -czf $BACKUP_DIR/ledger_$TIMESTAMP.tar.gz \
  /var/lib/aethyro/ledger

# Backup model checksum
cp /var/lib/aethyro/models/model.ntg/checksum.sha256 \
  $BACKUP_DIR/checksum_$TIMESTAMP.sha256

# Upload to S3 (with retention policy)
aws s3 sync $BACKUP_DIR s3://your-backup-bucket/aethyro/ \
  --sse AES256 \
  --storage-class GLACIER_IR

# Clean local backups older than 30 days
find $BACKUP_DIR -type f -mtime +30 -delete
```

---

## 4. Network & Security

### 4.1 Network Architecture

```
┌──────────────────────────────────────────┐
│ Ingress Controller / Load Balancer       │
├──────────────────────────────────────────┤
│              nginx / Envoy               │
│  - TLS termination                       │
│  - Request routing                       │
│  - Rate limiting                         │
├──────────────────────────────────────────┤
│         Aethyro NTG Pods (3)             │
│  - Stateless inference                   │
│  - Horizontal scaling                    │
├──────────────────────────────────────────┤
│  Service Discovery (DNS / Consul)        │
└──────────────────────────────────────────┘
```

### 4.2 TLS Configuration

```bash
# Generate self-signed cert for testing
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Create Kubernetes secret
kubectl create secret tls aethyro-tls \
  --cert=cert.pem \
  --key=key.pem

# Update aethyro.yaml
# api:
#   tls:
#     enabled: true
#     cert_path: /etc/aethyro/certs/tls.crt
#     key_path: /etc/aethyro/certs/tls.key
```

### 4.3 Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: aethyro-ingress
spec:
  podSelector:
    matchLabels:
      app: aethyro-ntg
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          role: frontend
    ports:
    - protocol: TCP
      port: 8080
```

---

## 5. Performance Tuning

### 5.1 CPU Affinity

```bash
# Taskset - pin to specific cores
taskset -c 0-7 /usr/local/bin/kernel_host --config aethyro.yaml

# Or in Kubernetes (via numactl)
numactl --cpunodebind=0 --membind=0 kernel_host ...
```

### 5.2 Memory Optimization

```yaml
# In deployment.yaml
resources:
  requests:
    memory: "2Gi"
  limits:
    memory: "4Gi"

# Memory tuning
env:
- name: MALLOC_TRIM_THRESHOLD_
  value: "128m"
- name: MALLOC_MMAP_MAX_
  value: "65536"
```

### 5.3 Cache Warming

```bash
# Pre-load model on startup
kernel_host --config aethyro.yaml --warmup-cache

# Or manually in container entrypoint
#!/bin/bash
echo "Warming cache..."
curl -s -X POST http://localhost:8080/v1/infer \
  -H "Content-Type: application/json" \
  -d '{"input": [0.0]}' > /dev/null

exec kernel_host --config aethyro.yaml
```

---

## 6. Monitoring & Observability

### 6.1 Prometheus Metrics Endpoint

```
GET /metrics

# Response:
aethyro_infer_requests_total{status="200"} 10042
aethyro_infer_latency_ms_bucket{le="50"} 8000
aethyro_compute_tobl_ratio 0.85
aethyro_ledger_height 1234
aethyro_ledger_verification_failures_total 0
aethyro_graph_mutations_total 42
```

### 6.2 Scrape Configuration

```yaml
# prometheus.yml
scrape_configs:
- job_name: 'aethyro-ntg'
  scrape_interval: 15s
  static_configs:
  - targets: ['localhost:9090']
```

### 6.3 Alerting Rules

```yaml
# aethyro-alerts.yaml
groups:
- name: aethyro
  rules:
  - alert: AethyroHighLatency
    expr: aethyro_infer_latency_ms{quantile="0.99"} > 500
    for: 5m
    annotations:
      summary: "Aethyro P99 latency > 500ms"
  
  - alert: AethyroLedgerVerificationFailed
    expr: increase(aethyro_ledger_verification_failures_total[5m]) > 0
    for: 1m
    annotations:
      summary: "Ledger verification failure detected"
  
  - alert: AethyroHighMemory
    expr: aethyro_memory_usage_bytes / aethyro_memory_limit_bytes > 0.9
    for: 2m
    annotations:
      summary: "Aethyro memory usage > 90%"
```

---

## 7. Automated Scaling

### 7.1 Horizontal Pod Autoscaling

```bash
# Based on custom metrics
kubectl autoscale deployment aethyro-ntg \
  --min=3 \
  --max=10 \
  --cpu-percent=70

# Or with custom metrics
kubectl apply -f - <<EOF
apiVersion: autoscaling.k8s.io/v2
kind: HorizontalPodAutoscaler
metadata:
  name: aethyro-custom
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: aethyro-ntg
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Pods
    pods:
      metricName: aethyro_infer_requests_rate
      targetAverageValue: "1000"
EOF
```

### 7.2 Vertical Pod Autoscaling

```yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: aethyro-vpa
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: aethyro-ntg
  updatePolicy:
    updateMode: "Auto"
  resourcePolicy:
    containerPolicies:
    - containerName: aethyro
      minAllowed:
        cpu: 1
        memory: 1Gi
      maxAllowed:
        cpu: 8
        memory: 8Gi
```

---

## 8. Disaster Recovery

### 8.1 Backup & Restore Procedure

**Backup (before deployment):**

```bash
#!/bin/bash
set -e

BACKUP_ID="backup_$(date +%Y%m%d_%H%M%S)"
BACKUP_PATH="/backups/aethyro/$BACKUP_ID"

mkdir -p $BACKUP_PATH

# Backup model
cp -r /var/lib/aethyro/models $BACKUP_PATH/

# Backup ledger
if [ -d /var/lib/aethyro/ledger ]; then
  tar -czf $BACKUP_PATH/ledger.tar.gz /var/lib/aethyro/ledger
fi

# Create manifest
cat > $BACKUP_PATH/manifest.json <<EOF
{
  "backup_id": "$BACKUP_ID",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "aethyro_version": "1.0.0",
  "model_checksum": "$(sha256sum /var/lib/aethyro/models/model.ntg/checksum.sha256 | cut -d' ' -f1)"
}
EOF

echo "Backup created: $BACKUP_PATH"
```

**Restore:**

```bash
#!/bin/bash
set -e

BACKUP_ID=$1

if [ -z "$BACKUP_ID" ]; then
  echo "Usage: $0 <backup_id>"
  exit 1
fi

BACKUP_PATH="/backups/aethyro/$BACKUP_ID"

if [ ! -d "$BACKUP_PATH" ]; then
  echo "Backup not found: $BACKUP_PATH"
  exit 1
fi

# Stop service
systemctl stop aethyro-ntg || true

# Restore model
rm -rf /var/lib/aethyro/models
cp -r $BACKUP_PATH/models /var/lib/aethyro/

# Restore ledger
if [ -f $BACKUP_PATH/ledger.tar.gz ]; then
  rm -rf /var/lib/aethyro/ledger
  tar -xzf $BACKUP_PATH/ledger.tar.gz -C /
fi

# Verify
sha256sum -c /var/lib/aethyro/models/model.ntg/checksum.sha256

# Start service
systemctl start aethyro-ntg

echo "Restored from $BACKUP_ID"
```

### 8.2 RTO & RPO Targets

| Scenario | RTO | RPO |
|---|---|---|
| Corrupted model | 5 min (restore from backup) | 0 (immutable) |
| Ledger corruption | 15 min (replay from genesis) | 1 block (~100ms) |
| Pod crash | 30 sec (K8s restart) | 0 (stateless) |
| Node failure | 2 min (pod rescheduling) | 0 (stateless) |
| Data center loss | 30 min (cross-region restore) | 1 block |

---

## 9. Upgrade Procedure

### 9.1 Rolling Deployment

```bash
# Set new image
kubectl set image deployment/aethyro-ntg \
  aethyro=gcr.io/your-project/aethyro-ntg:1.0.1

# Monitor rollout
kubectl rollout status deployment/aethyro-ntg --timeout=5m

# Verify
kubectl get pods -l app=aethyro-ntg
curl http://aethyro-ntg:8080/health
```

### 9.2 Canary Deployment

```yaml
apiVersion: fluxcd.io/v1
kind: Canary
metadata:
  name: aethyro-ntg
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: aethyro-ntg
  progressDeadlineSeconds: 300
  skipAnalysis: false
  
  # Gradually increase traffic
  service:
    port: 8080
    targetPort: 8080
  
  analysis:
    interval: 1m
    threshold: 5
    maxWeight: 50
    stepWeight: 10
    
    metrics:
    - name: error_rate
      thresholdRange:
        max: 5
      interval: 1m
```

---

## 10. Runbook: Common Operations

### 10.1 Check Service Health

```bash
# Quick health check
curl -s http://localhost:8080/health | jq .

# Check Kubernetes rollout
kubectl rollout status deployment/aethyro-ntg

# Check resource usage
kubectl top pods -l app=aethyro-ntg

# View logs
kubectl logs -f deployment/aethyro-ntg -c aethyro
```

### 10.2 Restart Service

```bash
# Kubernetes
kubectl rollout restart deployment/aethyro-ntg

# Systemd
systemctl restart aethyro-ntg

# Docker
docker restart aethyro-ntg-container
```

### 10.3 Scale Up/Down

```bash
# Kubernetes horizontal scaling
kubectl scale deployment aethyro-ntg --replicas=5

# Check scaling
kubectl get deployment aethyro-ntg
```

### 10.4 Inspect Metrics

```bash
# Export metrics
curl -s http://localhost:9090/metrics | grep aethyro

# Query Prometheus
curl 'http://prometheus:9090/api/v1/query?query=aethyro_infer_latency_ms' \
  | jq '.data.result'
```

---

## Version History

- **v1.0 (2026-07-16):** Production deployment guide
- **v0.9 (2026-07-09):** Container deployment guide (draft)

