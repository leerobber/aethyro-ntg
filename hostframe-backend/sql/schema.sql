-- BigQuery schema for VITASCALE telemetry events
-- ADR 0010 §4: Asynchronous telemetry ingestion with deterministic audit trail

-- Main telemetry events table
CREATE OR REPLACE TABLE `{project}.{dataset}.telemetry_events` (
    batch_id STRING NOT NULL OPTIONS(description="Unique batch identifier"),
    tick BIGINT NOT NULL OPTIONS(description="Agent tick counter"),
    timestamp_ms BIGINT NOT NULL OPTIONS(description="Client-side timestamp (milliseconds since epoch)"),
    ingested_at TIMESTAMP NOT NULL OPTIONS(description="Server-side ingestion timestamp"),
    mean_utility FLOAT64 NOT NULL OPTIONS(description="Mean fitness/utility [0, 1]"),
    min_safety FLOAT64 NOT NULL OPTIONS(description="Minimum safety score [0, 1]"),
    quarantine_fraction FLOAT64 NOT NULL OPTIONS(description="Fraction of agents in quarantine [0, 1]"),
    bus_drop_rate FLOAT64 NOT NULL OPTIONS(description="Message drop rate on bus [0, 1]"),
    awareness_coverage FLOAT64 NOT NULL OPTIONS(description="Self-awareness sensor coverage [0, 1]"),
    phage_events_this_tick INT64 NOT NULL OPTIONS(description="Mutations attempted this tick"),
    phage_events_total INT64 NOT NULL OPTIONS(description="Cumulative mutations accepted"),
    deterministic_alarm BOOL NOT NULL OPTIONS(description="Replay audit alarm triggered"),
    health_score FLOAT64 NOT NULL OPTIONS(description="Composite health [0, 1]"),
    needs_healing BOOL NOT NULL OPTIONS(description="Recovery action required")
)
PARTITION BY DATE(ingested_at)
CLUSTER BY batch_id, tick
OPTIONS(
    description="VITASCALE telemetry events — one record per agent tick per batch",
    require_partition_filter=false
);

-- Batch metadata table (for ledger auditing)
CREATE OR REPLACE TABLE `{project}.{dataset}.telemetry_batches` (
    batch_id STRING NOT NULL OPTIONS(description="Unique batch identifier"),
    sent_at_ms BIGINT NOT NULL OPTIONS(description="Client send timestamp"),
    ingested_at TIMESTAMP NOT NULL OPTIONS(description="Server ingestion timestamp"),
    record_count INT64 NOT NULL OPTIONS(description="Number of telemetry records in batch"),
    tick_min BIGINT NOT NULL OPTIONS(description="Minimum tick in batch"),
    tick_max BIGINT NOT NULL OPTIONS(description="Maximum tick in batch"),
    mean_utility_avg FLOAT64 NOT NULL OPTIONS(description="Average utility across batch"),
    health_score_min FLOAT64 NOT NULL OPTIONS(description="Minimum health score in batch"),
    health_score_max FLOAT64 NOT NULL OPTIONS(description="Maximum health score in batch"),
    quarantine_fraction_avg FLOAT64 NOT NULL OPTIONS(description="Average quarantine fraction")
)
PARTITION BY DATE(ingested_at)
OPTIONS(
    description="VITASCALE telemetry batch metadata — one record per ingest",
    require_partition_filter=false
);

-- Materialized view: hourly aggregates for dashboards
CREATE OR REPLACE MATERIALIZED VIEW `{project}.{dataset}.telemetry_hourly` AS
SELECT
    TIMESTAMP_TRUNC(ingested_at, HOUR) as hour,
    COUNT(DISTINCT batch_id) as batch_count,
    COUNT(*) as event_count,
    AVG(mean_utility) as avg_utility,
    MIN(mean_utility) as min_utility,
    MAX(mean_utility) as max_utility,
    AVG(health_score) as avg_health,
    COUNTIF(needs_healing) as healing_events,
    COUNTIF(deterministic_alarm) as alarm_events
FROM `{project}.{dataset}.telemetry_events`
GROUP BY hour
ORDER BY hour DESC;

-- Materialized view: regime transitions (high-level analytics)
CREATE OR REPLACE MATERIALIZED VIEW `{project}.{dataset}.regime_transitions` AS
WITH regimes AS (
    SELECT
        batch_id,
        tick,
        ingested_at,
        CASE
            WHEN mean_utility > 0.75 AND health_score > 0.8 THEN 'Thriving'
            WHEN mean_utility < 0.4 OR quarantine_fraction > 0.6 THEN 'Recovering'
            WHEN mean_utility < 0.2 AND health_score < 0.4 THEN 'Catastrophic'
            ELSE 'Stressed'
        END as regime,
        mean_utility,
        health_score
    FROM `{project}.{dataset}.telemetry_events`
)
SELECT
    batch_id,
    tick,
    ingested_at,
    regime,
    mean_utility,
    health_score,
    LAG(regime) OVER (PARTITION BY batch_id ORDER BY tick) as prev_regime
FROM regimes
WHERE regime != LAG(regime) OVER (PARTITION BY batch_id ORDER BY tick)
   OR LAG(regime) OVER (PARTITION BY batch_id ORDER BY tick) IS NULL;
