//! BigQuery client for telemetry ingestion and metrics.

use crate::config::Config;
use crate::error::HostframeError;
use crate::telemetry::TelemetryPayload;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// In-memory metrics tracker (for demo; production uses BigQuery queries).
#[derive(Debug, Clone)]
pub struct MetricsState {
    total_batches: Arc<AtomicU64>,
    total_records: Arc<AtomicU64>,
    latest_tick: Arc<std::sync::Mutex<Option<u64>>>,
    mean_health_sum: Arc<std::sync::Mutex<f32>>,
    mean_health_count: Arc<AtomicU64>,
}

impl MetricsState {
    fn new() -> Self {
        Self {
            total_batches: Arc::new(AtomicU64::new(0)),
            total_records: Arc::new(AtomicU64::new(0)),
            latest_tick: Arc::new(std::sync::Mutex::new(None)),
            mean_health_sum: Arc::new(std::sync::Mutex::new(0.0)),
            mean_health_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn record_batch(&self, records: &[TelemetryPayload]) {
        self.total_batches.fetch_add(1, Ordering::SeqCst);
        self.total_records
            .fetch_add(records.len() as u64, Ordering::SeqCst);

        if let Some(record) = records.last() {
            *self.latest_tick.lock().unwrap() = Some(record.tick);
            *self.mean_health_sum.lock().unwrap() += record.health_score;
            self.mean_health_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn get_metrics(&self) -> MetricsSnapshot {
        let count = self.mean_health_count.load(Ordering::SeqCst);
        let mean_health = if count > 0 {
            Some(*self.mean_health_sum.lock().unwrap() / count as f32)
        } else {
            None
        };

        MetricsSnapshot {
            total_batches_ingested: self.total_batches.load(Ordering::SeqCst),
            total_records_ingested: self.total_records.load(Ordering::SeqCst),
            latest_tick: *self.latest_tick.lock().unwrap(),
            mean_health_score: mean_health,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

pub struct BigQueryClient {
    config: Config,
    metrics: MetricsState,
}

impl BigQueryClient {
    pub async fn new(config: &Config) -> Result<Self, HostframeError> {
        // In a production setup, this would authenticate with BigQuery using
        // google-bigquery1 crate and GOOGLE_APPLICATION_CREDENTIALS
        tracing::info!(
            "BigQuery client initialized (project={}, dataset={}, table={})",
            config.gcp_project,
            config.gcp_dataset,
            config.bq_table
        );

        Ok(Self {
            config: config.clone(),
            metrics: MetricsState::new(),
        })
    }

    /// Insert a batch of telemetry records into BigQuery.
    pub async fn insert_telemetry_batch(
        &self,
        batch_id: &str,
        records: Vec<TelemetryPayload>,
        sent_at_ms: u64,
    ) -> Result<(), HostframeError> {
        if records.is_empty() {
            return Err(HostframeError::InvalidRequest(
                "Empty telemetry batch".to_string(),
            ));
        }

        // Record metrics (in production, this would also INSERT into BigQuery)
        self.metrics.record_batch(&records);

        tracing::info!(
            "BigQuery insert: batch_id={}, records={}, sent_at_ms={}",
            batch_id,
            records.len(),
            sent_at_ms
        );

        // In production, construct rows like:
        // for record in records {
        //     let row = record.to_bq_row(batch_id, Utc::now());
        //     // Insert into BigQuery via tabledata.insertAll API
        // }

        Ok(())
    }

    /// Get aggregated metrics snapshot.
    pub async fn get_metrics_snapshot(&self) -> Result<MetricsSnapshot, HostframeError> {
        Ok(self.metrics.get_metrics())
    }
}

// Export for metrics endpoint
pub use crate::telemetry::MetricsSnapshot;
