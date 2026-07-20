# Aethyro NTG: User Guide

**Version:** 1.0  
**Date:** 2026-07-16  
**Audience:** Developers, Data Scientists, Operators  

---

## Quick Start

### Installation (Standalone Binary)

```bash
# Download latest release
curl -L https://releases.aethyro.com/v1.0.0/aethyro-ntg-linux-x86_64.tar.gz | tar xz

# Make it executable
chmod +x aethyro-ntg

# Verify
./aethyro-ntg --version
# aethyro-ntg 1.0.0
```

### Running Your First Inference

```bash
# Start the server
./aethyro-ntg --model ./models/model.ntg --listen 0.0.0.0:8080

# In another terminal, send a request
curl -X POST http://localhost:8080/v1/infer \
  -H "Content-Type: application/json" \
  -d '{
    "input": [0.5, -0.3, 0.8, 0.1, -0.5]
  }'

# Response:
# {
#   "output": [0.127, 0.893, 0.021, 0.756, 0.334],
#   "batch_id": "req-12345",
#   "latency_ms": 42,
#   "compute_type": "tobl"
# }
```

---

## 1. API Reference

### 1.1 Inference Endpoint

**Endpoint:** `POST /v1/infer`

**Request:**

```json
{
  "input": [0.5, -0.3, 0.8],
  "batch_id": "optional-uuid",
  "return_internals": false
}
```

**Parameters:**

| Field | Type | Required | Description |
|---|---|---|---|
| `input` | float[] | Yes | Input vector (must match model input size) |
| `batch_id` | string | No | Optional request ID for tracking |
| `return_internals` | bool | No | Include internal graph activations (debug mode) |

**Response (200 OK):**

```json
{
  "output": [0.127, 0.893, 0.021],
  "batch_id": "req-12345",
  "latency_ms": 42,
  "compute_type": "tobl",
  "model_version": "1.0.0"
}
```

**Error Response (400 Bad Request):**

```json
{
  "error": "input_shape_mismatch",
  "expected": [1024],
  "received": [512],
  "timestamp": "2026-07-16T10:30:00Z"
}
```

**Common Errors:**

| Error | Cause | Solution |
|---|---|---|
| `input_shape_mismatch` | Input vector has wrong size | Check model input dimension |
| `invalid_json` | Request JSON is malformed | Validate JSON syntax |
| `model_not_loaded` | Model failed to load | Check model path and permissions |
| `timeout` | Request took > 5 seconds | Reduce batch size or increase timeout |

---

### 1.2 Batch Inference

**Endpoint:** `POST /v1/batch-infer`

**Request:**

```json
{
  "inputs": [
    [0.5, -0.3, 0.8],
    [0.1, 0.2, 0.3],
    [-0.5, 0.0, 0.5]
  ],
  "batch_id": "batch-12345"
}
```

**Response:**

```json
{
  "outputs": [
    [0.127, 0.893, 0.021],
    [0.456, 0.123, 0.789],
    [0.999, 0.001, 0.500]
  ],
  "batch_id": "batch-12345",
  "latency_ms": 52,
  "compute_type": "tobl"
}
```

**Limitations:**

- Max batch size: 32 (configurable)
- All inputs must have same shape
- Processed sequentially, not in parallel

---

### 1.3 Health Check

**Endpoint:** `GET /health`

**Response (200 OK):**

```json
{
  "status": "healthy",
  "components": {
    "graph": "loaded",
    "compute": "ready",
    "ledger": "synced"
  },
  "uptime_seconds": 3600,
  "requests_processed": 10042
}
```

---

### 1.4 Model Info

**Endpoint:** `GET /v1/model/info`

**Response:**

```json
{
  "name": "genomic-disease-v1.0",
  "version": "1.0.0",
  "input_shape": [1024],
  "output_shape": [3],
  "total_parameters": 2097152,
  "ternary_parameters": 2097152,
  "sparsity": 0.85,
  "model_size_bytes": 262144,
  "estimated_latency_ms": 45
}
```

---

### 1.5 Metrics Endpoint

**Endpoint:** `GET /metrics`

**Response (Prometheus format):**

```
# HELP aethyro_infer_requests_total Total inference requests
# TYPE aethyro_infer_requests_total counter
aethyro_infer_requests_total{status="200"} 10042
aethyro_infer_requests_total{status="400"} 12
aethyro_infer_requests_total{status="500"} 0

# HELP aethyro_infer_latency_ms Inference latency in milliseconds
# TYPE aethyro_infer_latency_ms histogram
aethyro_infer_latency_ms_bucket{le="10"} 1024
aethyro_infer_latency_ms_bucket{le="50"} 8000
aethyro_infer_latency_ms_bucket{le="100"} 10000
aethyro_infer_latency_ms_bucket{le="+Inf"} 10042

aethyro_compute_tobl_ratio 0.85
aethyro_compute_scalar_ratio 0.15
```

---

## 2. Python Client Library

### 2.1 Installation

```bash
pip install aethyro-ntg
```

### 2.2 Basic Usage

```python
from aethyro import AethyroClient

# Initialize client
client = AethyroClient(
    host="http://localhost:8080",
    timeout=5.0
)

# Single inference
input_data = [0.5, -0.3, 0.8]
output = client.infer(input_data)
print(output)
# [0.127, 0.893, 0.021]

# Batch inference
inputs = [
    [0.5, -0.3, 0.8],
    [0.1, 0.2, 0.3]
]
outputs = client.batch_infer(inputs)
print(outputs)
# [[0.127, 0.893, 0.021], [0.456, 0.123, 0.789]]

# Get model info
info = client.get_model_info()
print(f"Model: {info['name']}, Params: {info['total_parameters']}")
```

### 2.3 Advanced Usage

```python
import numpy as np
from aethyro import AethyroClient

# Connect with authentication
client = AethyroClient(
    host="https://api.aethyro.com",
    api_key="sk_prod_xxxxx",
    timeout=10.0
)

# Inference with batch tracking
output, metadata = client.infer(
    input_data,
    return_metadata=True,
    batch_id="batch-001"
)

print(f"Latency: {metadata['latency_ms']}ms")
print(f"Compute type: {metadata['compute_type']}")

# Error handling
try:
    output = client.infer([0.1, 0.2])  # Wrong size
except Exception as e:
    print(f"Error: {e.args[0]['error']}")
    # Error: input_shape_mismatch
```

---

## 3. JavaScript/TypeScript Client

### 3.1 Installation

```bash
npm install @aethyro/ntg-client
```

### 3.2 Usage

```typescript
import { AethyroClient } from '@aethyro/ntg-client';

const client = new AethyroClient({
  baseURL: 'http://localhost:8080',
  timeout: 5000
});

// Single inference
const output = await client.infer([0.5, -0.3, 0.8]);
console.log(output);
// [0.127, 0.893, 0.021]

// Batch inference
const outputs = await client.batchInfer([
  [0.5, -0.3, 0.8],
  [0.1, 0.2, 0.3]
]);
console.log(outputs);

// Error handling
try {
  await client.infer([0.1, 0.2]); // Wrong size
} catch (e) {
  console.error(e.error);
  // "input_shape_mismatch"
}
```

---

## 4. Troubleshooting

### 4.1 Connection Issues

**Problem:** `Connection refused`

```bash
# Check if server is running
ps aux | grep kernel_host

# Check if port is listening
netstat -tuln | grep 8080

# Start server
./aethyro-ntg --model models/model.ntg --listen 0.0.0.0:8080
```

### 4.2 Model Loading Errors

**Problem:** `Model not found`

```bash
# Verify model path
ls -la /path/to/model.ntg/

# Check required files
ls -la /path/to/model.ntg/graph.json
ls -la /path/to/model.ntg/weights.bin
ls -la /path/to/model.ntg/metadata.json

# Verify checksum
sha256sum -c /path/to/model.ntg/checksum.sha256
```

### 4.3 Performance Issues

**Problem:** High latency (> 500ms)

```bash
# Check server metrics
curl -s http://localhost:8080/metrics | grep latency

# Check CPU usage
top -p $(pidof kernel_host)

# Check memory
ps aux | grep kernel_host | grep -v grep

# Solutions:
# 1. Increase thread count
#    export AETHYRO_THREADS=16
# 2. Reduce batch size
#    curl -X POST http://localhost:8080/v1/infer \
#      -d '{"input": [...]}'
# 3. Enable CPU affinity (Linux)
#    taskset -c 0-7 ./aethyro-ntg ...
```

### 4.4 Memory Issues

**Problem:** Out of memory crash

```bash
# Check memory limit
cat /proc/limits
# or
ulimit -a

# Reduce cache
export AETHYRO_CACHE_SIZE=256m

# Reduce batch size
# (See 4.3 above)

# Monitor memory
watch -n 1 'ps aux | grep kernel_host'
```

### 4.5 Ledger Verification Issues

**Problem:** `Ledger verification failed`

```bash
# Check ledger
ls -la /var/lib/aethyro/ledger/

# Verify chain integrity
./aethyro-ntg --verify-ledger /var/lib/aethyro/ledger

# Reset ledger (WARNING: loses history)
rm -rf /var/lib/aethyro/ledger/*

# Restart service
systemctl restart aethyro-ntg
```

---

## 5. Examples

### 5.1 Genomic Classification

```python
from aethyro import AethyroClient
import numpy as np

# Connect to server
client = AethyroClient("http://localhost:8080")

# Load genomic variant data
variants = np.load("variants.npy")  # Shape: (1024,)

# Run inference
probabilities = client.infer(variants.tolist())

# Interpret results
class_names = ["Normal", "Disease A", "Disease B"]
for i, prob in enumerate(probabilities):
    print(f"{class_names[i]}: {prob:.2%}")
```

### 5.2 Real-time Monitoring

```python
import time
from aethyro import AethyroClient
import matplotlib.pyplot as plt

client = AethyroClient("http://localhost:8080")

latencies = []
for i in range(100):
    start = time.time()
    client.infer([0.5] * 1024)
    latencies.append((time.time() - start) * 1000)

# Plot latency distribution
plt.hist(latencies, bins=20)
plt.xlabel("Latency (ms)")
plt.ylabel("Count")
plt.title("Aethyro NTG Latency Distribution")
plt.savefig("latency.png")
```

### 5.3 Batch Processing

```python
from aethyro import AethyroClient
import csv

client = AethyroClient("http://localhost:8080")

# Read samples from CSV
samples = []
with open("samples.csv") as f:
    reader = csv.reader(f)
    next(reader)  # Skip header
    for row in reader:
        samples.append([float(x) for x in row])

# Process in batches
batch_size = 32
results = []
for i in range(0, len(samples), batch_size):
    batch = samples[i:i+batch_size]
    outputs = client.batch_infer(batch)
    results.extend(outputs)

# Write results
with open("results.csv", "w") as f:
    writer = csv.writer(f)
    writer.writerow(["prediction"])
    for result in results:
        writer.writerow([result])
```

---

## 6. Performance Tips

### 6.1 Optimization Checklist

- [ ] Use TOBL acceleration (auto-detected)
- [ ] Batch requests when possible (max 32)
- [ ] Minimize network round trips
- [ ] Use persistent connections
- [ ] Enable input validation only in dev mode
- [ ] Cache model info after first load

### 6.2 Benchmarking

```bash
# Single inference
time curl -X POST http://localhost:8080/v1/infer \
  -d '{"input": [0.5, -0.3, 0.8]}'

# Batch inference (throughput)
ab -n 1000 -c 10 -p input.json \
  http://localhost:8080/v1/batch-infer

# Profile with perf (Linux)
perf record -g kernel_host --config aethyro.yaml
perf report
```

---

## 7. FAQ

**Q: What input size does the model expect?**

A: Check with `GET /v1/model/info` or run:

```bash
curl -s http://localhost:8080/v1/model/info | jq .input_shape
```

**Q: How do I enable ledger mode?**

A: Set environment variable before starting:

```bash
export AETHYRO_LEDGER=1
./aethyro-ntg --config aethyro.yaml
```

**Q: Can I run multiple models simultaneously?**

A: Currently no; run separate processes on different ports:

```bash
./aethyro-ntg --model model1.ntg --listen :8080 &
./aethyro-ntg --model model2.ntg --listen :8081 &
```

**Q: What's the maximum batch size?**

A: Default 32, configurable in `aethyro.yaml`:

```yaml
runtime:
  max_batch_size: 64
```

**Q: How do I check if TOBL acceleration is active?**

A: Monitor the metrics:

```bash
curl -s http://localhost:8080/metrics | grep compute_tobl_ratio
# aethyro_compute_tobl_ratio 0.85
```

---

## 8. Support & Resources

- **Documentation:** https://docs.aethyro.com
- **GitHub Issues:** https://github.com/yourorg/aethyro-ntg/issues
- **Email Support:** support@aethyro.com
- **Community Discord:** https://discord.gg/aethyro

---

## Version History

- **v1.0 (2026-07-16):** Production user guide
- **v0.9 (2026-07-09):** Beta API documentation

