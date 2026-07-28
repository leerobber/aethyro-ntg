//! Telemetry payload — mirrored from kernel hostframe_bridge.rs
//! Maps to BigQuery telemetry_events table schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub tick: u64,
    pub timestamp_ms: u64,
    pub mean_utility: f32,
    pub min_safety: f32,
    pub quarantine_fraction: f32,
    pub bus_drop_rate: f32,
    pub awareness_coverage: f32,
    pub phage_events_this_tick: usize,
    pub phage_events_total: usize,
    pub deterministic_alarm: bool,
    pub health_score: f32,
    pub needs_healing: bool,
}

impl TelemetryPayload {
    /// Convert to BigQuery row format.
    pub fn to_bq_row(&self, batch_id: &str, ingested_at: DateTime<Utc>) -> serde_json::Value {
        serde_json::json!({
            "batch_id": batch_id,
            "tick": self.tick,
            "timestamp_ms": self.timestamp_ms,
            "ingested_at": ingested_at.to_rfc3339(),
            "mean_utility": self.mean_utility,
            "min_safety": self.min_safety,
            "quarantine_fraction": self.quarantine_fraction,
            "bus_drop_rate": self.bus_drop_rate,
            "awareness_coverage": self.awareness_coverage,
            "phage_events_this_tick": self.phage_events_this_tick,
            "phage_events_total": self.phage_events_total,
            "deterministic_alarm": self.deterministic_alarm,
            "health_score": self.health_score,
            "needs_healing": self.needs_healing,
        })
    }
}

/// Aggregated metrics snapshot (for /metrics endpoint).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_batches_ingested: u64,
    pub total_records_ingested: u64,
    pub latest_tick: Option<u64>,
    pub mean_health_score: Option<f32>,
    pub timestamp: String,
}
