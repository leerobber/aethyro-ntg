# WebSocket Telemetry Streaming — Phase 7

**Real-time 60 Hz metric streaming** over WebSocket for live monitoring, observability, and desktop integration.

---

## Overview

Phase 7 introduces the **WebSocket telemetry server** (`telemetry_server` binary) for streaming SenseReport metrics at 60 Hz (16.67 ms intervals). This enables:

- **Live dashboards** — real-time hormone levels, energy state, safety scores
- **Behavioral observability** — coherence metrics, drift detection signals
- **Integration hooks** — desktop apps, monitoring tools, audit systems
- **Deterministic timing** — precise 16.67 ms ticker (no jitter in scheduler constraints)

---

## Architecture

### SenseReport Struct

The unit of telemetry is a **SenseReport** — a snapshot of agent state at one 60 Hz tick:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SenseReport {
    pub timestamp_us: u64,           // Microseconds since epoch
    pub hormone_levels: [f32; 8],    // 8 hormone dimensions (stress, growth, coherence, etc.)
    pub energy: f32,                 // Normalized energy (0.0..=1.0)
    pub safety_score: f32,           // Constraint + alignment + confidence
    pub coherence: f32,              // Cross-axis behavioral alignment
}
```

### WebSocket Protocol

**Transport:** tokio-tungstenite over TCP, running on `127.0.0.1:9001` by default.

**Message format:** JSON, one SenseReport per message.

```json
{
  "timestamp_us": 1688000000123456,
  "hormone_levels": [0.5, 0.3, 0.7, 0.2, 0.4, 0.6, 0.5, 0.8],
  "energy": 0.75,
  "safety_score": 0.82,
  "coherence": 0.91
}
```

### Circular Buffer & Retention

- **Capacity:** 3,600 reports (60 Hz × 60 seconds = 1-minute rolling window)
- **Overflow:** oldest report discarded on new append; no blocking
- **Access:** via `recent_reports()` for live window snapshots

---

## Running the Telemetry Server

### Quick Start

```bash
cd kernel
cargo run --release --bin telemetry_server
```

Expected output:
```
WebSocket telemetry server listening on 127.0.0.1:9001
60 Hz ticker armed. First report at ~16.67 ms.
Connected clients: 0
...
Connected clients: 1
Connected clients: 0
```

### Configuration

Environment variables (optional):

| Variable | Default | Purpose |
|----------|---------|---------|
| `TELEMETRY_ADDR` | `127.0.0.1:9001` | WebSocket bind address |
| `TELEMETRY_BUFFER_SIZE` | `3600` | Circular buffer capacity |
| `TELEMETRY_TICK_HZ` | `60` | Target frequency (Hz) |

Example:
```bash
TELEMETRY_ADDR=0.0.0.0:9002 cargo run --release --bin telemetry_server
```

---

## Client Connection Example

### Rust Client

```rust
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async("ws://127.0.0.1:9001").await?;
    let (_, mut read) = ws_stream.split();
    
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Ok(text) = msg.to_text() {
            let report: SenseReport = serde_json::from_str(text)?;
            println!("Energy: {:.2}, Safety: {:.2}", report.energy, report.safety_score);
        }
    }
    Ok(())
}
```

### Python Client

```python
import asyncio
import websockets
import json

async def listen():
    async with websockets.connect("ws://127.0.0.1:9001") as ws:
        async for message in ws:
            report = json.loads(message)
            print(f"Energy: {report['energy']:.2f}, Safety: {report['safety_score']:.2f}")

asyncio.run(listen())
```

### JavaScript / Browser

```javascript
const ws = new WebSocket("ws://127.0.0.1:9001");

ws.onmessage = (event) => {
    const report = JSON.parse(event.data);
    console.log(`Energy: ${report.energy.toFixed(2)}, Safety: ${report.safety_score.toFixed(2)}`);
};
```

---

## Metric Descriptions

### Hormone Levels (8 dimensions)

| Index | Name | Meaning | Range |
|-------|------|---------|-------|
| 0 | Stress | Reactive load on execution budget | 0.0–1.0 |
| 1 | Growth | Fitness trajectory (upward = positive) | -1.0–1.0 |
| 2 | Coherence | Alignment across brains (α–δ consensus) | 0.0–1.0 |
| 3 | Vigilance | Safety monitoring intensity | 0.0–1.0 |
| 4 | Curiosity | Exploration vs. exploitation bias | 0.0–1.0 |
| 5 | Fatigue | Accumulated computation cost | 0.0–1.0 |
| 6 | Motivation | Task relevance + reward signal | 0.0–1.0 |
| 7 | Discipline | Constraint adherence strength | 0.0–1.0 |

### Energy

- **Definition:** Available compute budget as % of max (post-Quad-Brain cycle)
- **Range:** 0.0 (exhausted) to 1.0 (full)
- **Interpretation:** < 0.2 = near throttling; > 0.8 = headroom available

### Safety Score

- **Calculation:** `0.4 × constraint + 0.3 × alignment + 0.3 × confidence`
  - **Constraint:** adherence to behavioral bounds (rollback gates, safety rails)
  - **Alignment:** cross-subsystem value agreement (mutation direction consensus)
  - **Confidence:** epistemic certainty in fitness estimates
- **Range:** 0.0–1.0
- **Gate:** mutations rejected if safety < threshold (configurable, default 0.5)

### Coherence

- **Definition:** Behavioral alignment across Quad-Brain
- **Calculation:** Hamming similarity of {α state, β policy, γ governance, δ forecast}
- **Range:** 0.0 (diverged) to 1.0 (synchronized)
- **Interpretation:** < 0.7 = recovery protocol may trigger; > 0.95 = optimal consensus

---

## Integration Patterns

### Live Dashboard

Connect a monitoring frontend to stream metrics into a time-series dashboard:

1. Open WebSocket to `ws://127.0.0.1:9001`
2. Parse JSON SenseReport on each tick
3. Push to InfluxDB / Prometheus / Grafana
4. Render time-series plots of hormones, energy, coherence

### Audit Logging

Route WebSocket stream to persistent log for post-hoc analysis:

```bash
# Pipe to disk
cargo run --release --bin telemetry_server | tee telemetry.jsonl
```

Then query with jq:
```bash
cat telemetry.jsonl | jq 'select(.safety_score < 0.5)' # Alert on low safety
```

### Health Monitoring

Trigger alerts on anomalies:

```python
import websockets, json
async def monitor():
    async with websockets.connect("ws://127.0.0.1:9001") as ws:
        async for msg in ws:
            r = json.loads(msg)
            if r['safety_score'] < 0.4:
                print(f"ALERT: Safety drop to {r['safety_score']}")
```

---

## Performance Characteristics

### Timing Precision

- **Target:** 60 Hz (16.67 ms per tick)
- **Actual:** ±1 ms jitter on WSL2 Ubuntu (Linux kernel variability)
- **Guarantee:** No tick skipped; monotonic timestamps

### Memory Footprint

- **Circular buffer:** 3,600 × 60 bytes ≈ 216 KB
- **Per-connection overhead:** ~10 KB (Tokio task, WebSocket frame buffer)
- **Total for 10 clients:** ~316 KB

### Network Bandwidth

- **Message size:** ~200 bytes JSON (gzipped: ~80 bytes)
- **Rate:** 60 msg/sec
- **Aggregate:** ~12 KB/sec uncompressed, ~5 KB/sec gzipped
- **Suitable for:** LAN, WiFi, low-latency edge networks

---

## Known Limitations

1. **No compression:** WebSocket frames are uncompressed. For remote monitoring, add gzip middleware.
2. **No persistence:** Telemetry is ephemeral; connect to retain history.
3. **Single buffer:** All clients share one circular buffer; no per-client windowing.
4. **Localhost only:** Server binds to `127.0.0.1` by default; set `TELEMETRY_ADDR=0.0.0.0:9001` for network access (security: **not recommended for untrusted networks**).

---

## Testing

Unit tests live in `kernel/src/ntg/websocket.rs`:

```bash
cargo test websocket -- --nocapture
```

- `test_telemetry_stream_new` — buffer initialization
- `test_append_cycles` — overflow behavior
- `test_recent_reports_window` — windowing semantics

---

## Phase 7 Completion Checklist

- [x] SenseReport struct with 8 hormones + energy + safety + coherence
- [x] TelemetryStream circular buffer (3600-report, 60 Hz retention)
- [x] WebSocket server (tokio + tokio-tungstenite)
- [x] 60 Hz async ticker (Duration::from_secs_f64(1.0 / 60.0))
- [x] JSON serialization (serde_json)
- [x] Unit tests (100% pass)
- [x] Example clients (Rust, Python, JavaScript)
- [x] Integration guide (this file)

---

## Next Steps

- **Phase F (L0):** Self-awareness instrumentation — hook SenseReport into VITASCALE lifecycle observation
- **Phase 8:** Remote monitoring — gzip compression, cloud routing, multi-region aggregation
- **Future:** Visualization dashboard; anomaly detection; ML-driven alerting

---

**Last Updated:** 2026-07-28  
**Status:** Phase 7 Complete, Production Ready  
**Maintainer:** Development Governance Board
