# 0013: Telemetry loop: declared ingest-only

**Status:** Proposed (2026-10-27). Describes what exists; the wiring below is not built.
**Phase:** Foundation Lock A5.2 ([plan](../FOUNDATION_LOCK_PLAN.md)).

## What exists

* `hostframe-backend` serves `POST /ingest` (validates a batch, updates in-memory counters),
  `GET /metrics` (reads those counters) and `GET /health`. State is lost on restart.
* Its BigQuery client (`src/bigquery.rs`) logs the batch and counts it. It does not authenticate or
  write to BigQuery; the code comments say "in production, this would ...". Startup needs only a
  non-empty `GCP_PROJECT`.
* `hostframe-backend/sql/schema.sql` and `terraform/` describe the intended BigQuery table; they
  have not been applied against a live project.
* `kernel/src/bin/telemetry_server.rs` is a separate 60 Hz WebSocket simulation that emits ten
  synthetic reports and exits. It does not talk to hostframe.

## What does not exist

**No producer.** Nothing in `kernel_host`, the dashboard or any other binary posts to `/ingest`.
The only callers are `scripts/smoke-whole.sh` and `scripts/verify-deploy.sh`, which send the repo's
sample payload. The dashboard has no view of `/metrics`.

## Declaration

Until a producer and a real BigQuery write exist, the telemetry path is **ingest-only and
demonstrative**: it proves the service accepts and counts batches, nothing more. Do not describe it
as a working telemetry-to-dashboard loop in any document. `docs/STATUS.md` already carries this
statement; this record is its evidence.

## To make it a loop (capability work, after the Lock)

1. Pick a producer (`kernel_host` per-call stats are the obvious source) and send batches.
2. Implement the BigQuery insert, or choose a store that does not need a GCP account.
3. Add one visible surface, for example a `/api/metrics` proxy and a dashboard panel.
