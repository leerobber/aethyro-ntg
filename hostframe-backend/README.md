# Hostframe Backend — VITASCALE GCP Telemetry Service

Production-grade telemetry ingestion service for KAIROS agent lifecycle tracking. Handles batched telemetry from distributed agents, ingests into BigQuery, and provides metrics aggregation.

## Architecture

```
aethyro-ntg (kernel)
    ↓
    hostframe_bridge (TelemetryClient buffers + batches)
    ↓
HTTP POST (ureq)
    ↓
Cloud Run (hostframe-backend)
    ↓
BigQuery (telemetry_events + telemetry_batches tables)
    ↓
Cloud Storage (audit ledger archival)
```

## Features

- **Async batch ingestion**: 60 Hz telemetry, batched for efficiency
- **BigQuery integration**: Structured telemetry with partitioning & clustering
- **Audit trail**: Hash-chained mutation ledger in Cloud Storage
- **Observability**: /metrics endpoint, materialized views for dashboards
- **Scalability**: Auto-scaling Cloud Run, data retention policies
- **Monitoring**: Error rate alerts, health checks

## Project Structure

```
hostframe-backend/
├── src/
│   ├── main.rs              # HTTP server (axum)
│   ├── bigquery.rs          # BigQuery client
│   ├── telemetry.rs         # Payload schemas
│   ├── config.rs            # Configuration
│   └── error.rs             # Error handling
├── Cargo.toml               # Dependencies
├── Dockerfile               # Multi-stage build
├── sql/
│   └── schema.sql           # BigQuery schema + views
├── terraform/
│   ├── main.tf              # Cloud Run, BigQuery, Storage
│   ├── variables.tf         # Input variables
│   └── terraform.tfvars.example
└── README.md
```

## Deployment

### Prerequisites

- GCP project with Cloud Run, BigQuery, and Cloud Storage APIs enabled
- Terraform >= 1.0
- gcloud CLI configured
- Docker

### Quick Start

1. **Build and push container:**

```bash
export GCP_PROJECT=your-project-id
export IMAGE=gcr.io/$GCP_PROJECT/hostframe-backend:latest

docker build -t $IMAGE .
docker push $IMAGE
```

2. **Configure Terraform:**

```bash
cd terraform
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your GCP project and image
```

3. **Deploy infrastructure:**

```bash
terraform init
terraform plan
terraform apply
```

4. **Verify deployment:**

```bash
# Get Cloud Run URL from terraform output
HOSTFRAME_URL=$(terraform output -raw cloud_run_url)
curl $HOSTFRAME_URL/health

# Example output:
# {"status":"ok","version":"0.1.0"}
```

## API Endpoints

### GET /health

Health check endpoint.

```bash
curl https://hostframe-backend-xxxxx.run.app/health
```

Response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### POST /ingest

Ingest a batch of telemetry events.

Request:
```json
{
  "reports": [
    {
      "tick": 100,
      "timestamp_ms": 1690000000000,
      "mean_utility": 0.85,
      "min_safety": 0.92,
      "quarantine_fraction": 0.0,
      "bus_drop_rate": 0.001,
      "awareness_coverage": 1.0,
      "phage_events_this_tick": 0,
      "phage_events_total": 5,
      "deterministic_alarm": false,
      "health_score": 0.87,
      "needs_healing": false
    }
  ],
  "sent_at_ms": 1690000005000
}
```

Response:
```json
{
  "success": true,
  "batch_id": "550e8400-e29b-41d4-a716-446655440000",
  "records_ingested": 1
}
```

### GET /metrics

Aggregated metrics snapshot.

```bash
curl https://hostframe-backend-xxxxx.run.app/metrics
```

Response:
```json
{
  "total_batches_ingested": 42,
  "total_records_ingested": 2520,
  "latest_tick": 2520,
  "mean_health_score": 0.86,
  "timestamp": "2026-07-28T15:45:30Z"
}
```

## BigQuery Schema

Main tables:

- **telemetry_events**: One record per telemetry payload (partitioned by ingested_at, clustered by batch_id + tick)
- **telemetry_batches**: Batch metadata (sent_at, ingestion latency, min/max ticks)

Materialized views:

- **telemetry_hourly**: Hourly aggregates (for dashboards)
- **regime_transitions**: Regime changes (Thriving → Stressed → Recovering)

Query example:

```sql
SELECT
  TIMESTAMP_TRUNC(ingested_at, HOUR) as hour,
  COUNT(*) as event_count,
  AVG(mean_utility) as avg_utility,
  AVG(health_score) as avg_health
FROM `project.aethyro_telemetry.telemetry_events`
WHERE DATE(ingested_at) = CURRENT_DATE()
GROUP BY hour
ORDER BY hour DESC;
```

## Configuration

Environment variables:

- `GCP_PROJECT` (required): GCP project ID
- `GOOGLE_CLOUD_PROJECT` (optional): Fallback project ID
- `BQ_DATASET` (optional, default: `aethyro_telemetry`): BigQuery dataset
- `BQ_TABLE` (optional, default: `telemetry_events`): BigQuery table
- `GOOGLE_APPLICATION_CREDENTIALS` (optional): Path to service account key

## Integration with Kernel

Update kernel `hostframe_bridge.rs` to point to your Cloud Run URL:

```rust
pub struct HostframeConfig {
    pub endpoint: String,  // "https://hostframe-backend-xxxxx.run.app"
    // ...
}
```

Then enable telemetry:

```rust
let config = HostframeConfig {
    endpoint: std::env::var("HOSTFRAME_ENDPOINT").unwrap(),
    batch_size: 60,
    timeout_ms: 5000,
    enabled: true,  // Enable for live KAIROS runs
};
```

## Monitoring & Alerting

Terraform provisions:
- Cloud Run error rate alert (> 5% errors)
- Email notifications (if `alert_email` provided)

Manual dashboard in Looker Studio:
1. Connect to BigQuery dataset `aethyro_telemetry`
2. Use materialized view `telemetry_hourly` for time-series
3. Use `regime_transitions` for regime tracking

## Development

### Local Testing

```bash
# Set up fake GCP environment variables
export GCP_PROJECT=test-project
export BQ_DATASET=test_telemetry

# Run locally
cargo run

# In another terminal:
curl http://localhost:8080/health
curl -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d @test_payload.json
```

### Running Tests

```bash
cargo test --release
```

## Production Checklist

- [ ] Terraform applied to GCP project
- [ ] Cloud Run service deployed and tested
- [ ] BigQuery dataset and tables created
- [ ] Service account IAM roles granted
- [ ] Cloud Storage audit ledger bucket created
- [ ] Monitoring alerts configured
- [ ] kernel `hostframe_bridge.rs` updated with endpoint
- [ ] Telemetry enabled in KAIROS runs
- [ ] Data retention policy (90 days) verified
- [ ] Backup/archival plan for long-term audit trail

## Known Limitations

- BigQuery client uses mocked insertion (production would use real BigQuery API)
- No authentication/API key validation yet (next iteration)
- Audit ledger archival to Cloud Storage not yet wired
- Limited to 100 concurrent Cloud Run instances (configurable)

## Next Steps

1. **Wire full BigQuery API** (google-bigquery1 crate)
2. **Implement audit ledger archival** to Cloud Storage
3. **Add API key authentication** for security
4. **Implement mutation replay verification** against ledger
5. **Create Looker Studio dashboards** for real-time monitoring

## References

- [ADR 0010 §4](https://github.com/leerobber/aethyro-ntg/blob/main/docs/adr/0010-vitascale.md): VITASCALE Hostframe telemetry
- [Cloud Run Docs](https://cloud.google.com/run/docs)
- [BigQuery API](https://cloud.google.com/bigquery/docs/reference/rest)
- [Terraform Google Provider](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
