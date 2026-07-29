//! Self-awareness instrumentation and telemetry for Phase F.L0
//! Handles serialization of TelemetryPayload and HTTP ingestion to Hostframe backend.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub event_id: String,
    pub timestamp: u64,  // Unix timestamp in milliseconds
    pub event_type: String,
    pub ld_compute: Option<LdComputeStats>,
    pub safety_score: Option<SafetyScoreSnapshot>,
    pub system_stats: Option<SystemStats>,
    pub kernel_version: String,
    pub ledger_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdComputeStats {
    pub snps_count: u64,
    pub samples_count: u64,
    pub pairs_computed: u64,
    pub throughput_pairs_per_sec: f64,
    pub wall_clock_seconds: f64,
    pub high_ld_pairs_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyScoreSnapshot {
    pub constraint_score: f64,
    pub alignment_score: f64,
    pub confidence: f64,
    pub overall: f64,
    pub rollback_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub gpu_utilization_percent: Option<f64>,
}

impl TelemetryPayload {
    pub fn new(
        event_id: String,
        event_type: String,
        kernel_version: String,
    ) -> Self {
        Self {
            event_id,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            event_type,
            ld_compute: None,
            safety_score: None,
            system_stats: None,
            kernel_version,
            ledger_hash: None,
        }
    }

    pub fn with_ld_stats(mut self, stats: LdComputeStats) -> Self {
        self.ld_compute = Some(stats);
        self
    }

    pub fn with_safety_score(mut self, score: SafetyScoreSnapshot) -> Self {
        self.safety_score = Some(score);
        self
    }

    pub fn with_system_stats(mut self, stats: SystemStats) -> Self {
        self.system_stats = Some(stats);
        self
    }

    pub fn with_ledger_hash(mut self, hash: String) -> Self {
        self.ledger_hash = Some(hash);
        self
    }

    /// Send telemetry to Hostframe backend via HTTP POST.
    /// Non-blocking async; returns immediately if backend is unreachable.
    pub async fn send_to_hostframe(&self, backend_url: &str) -> Result<(), TelemetryError> {
        let client = reqwest::Client::new();
        let url = format!("{}/ingest/telemetry", backend_url);

        let response = client
            .post(&url)
            .json(self)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| TelemetryError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TelemetryError::BackendError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        Ok(())
    }

    /// Serialize to JSON for testing/logging.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug, Clone)]
pub enum TelemetryError {
    NetworkError(String),
    BackendError(String),
    SerializationError(String),
}

impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryError::NetworkError(e) => write!(f, "Network error: {}", e),
            TelemetryError::BackendError(e) => write!(f, "Backend error: {}", e),
            TelemetryError::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for TelemetryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_payload_creation() {
        let payload = TelemetryPayload::new(
            "test-event-1".to_string(),
            "ld_computation".to_string(),
            "0.1.0".to_string(),
        );

        assert_eq!(payload.event_type, "ld_computation");
        assert_eq!(payload.kernel_version, "0.1.0");
        assert!(payload.timestamp > 0);
    }

    #[test]
    fn test_ld_compute_stats_serialization() {
        let payload = TelemetryPayload::new(
            "test-1".to_string(),
            "ld_computation".to_string(),
            "0.1.0".to_string(),
        )
        .with_ld_stats(LdComputeStats {
            snps_count: 48494,
            samples_count: 389,
            pairs_computed: 1175809771,
            throughput_pairs_per_sec: 1460272.0,
            wall_clock_seconds: 805.7,
            high_ld_pairs_count: 0,
        });

        let json = payload.to_json().unwrap();
        assert!(json.contains("\"snps_count\":48494"));
        assert!(json.contains("\"pairs_computed\":1175809771"));

        // Deserialize to verify round-trip
        let deserialized: TelemetryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.ld_compute.unwrap().snps_count,
            48494
        );
    }

    #[test]
    fn test_safety_score_serialization() {
        let payload = TelemetryPayload::new(
            "test-2".to_string(),
            "safety_check".to_string(),
            "0.1.0".to_string(),
        )
        .with_safety_score(SafetyScoreSnapshot {
            constraint_score: 0.95,
            alignment_score: 0.88,
            confidence: 0.92,
            overall: 0.92,
            rollback_triggered: false,
        });

        let json = payload.to_json().unwrap();
        let deserialized: TelemetryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.safety_score.unwrap().overall, 0.92);
    }
}
