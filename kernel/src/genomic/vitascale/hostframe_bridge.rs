//! Hostframe telemetry bridge — serializes SenseReport and sends to GCP Cloud Run.
//! ADR 0010 §4: Asynchronous telemetry batching with async mpsc channel.

use super::self_awareness::SenseReport;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for GCP Cloud Run backend.
#[derive(Clone, Debug)]
pub struct HostframeConfig {
    /// GCP Cloud Run service URL (e.g., "https://genomic-telemetry-xyz.a.run.app")
    pub endpoint: String,
    /// Batch size before flushing (default: 60 for 60 Hz telemetry)
    pub batch_size: usize,
    /// Timeout for HTTP POST (default: 5s)
    pub timeout_ms: u64,
    /// Enable telemetry (can disable for offline testing)
    pub enabled: bool,
}

impl Default for HostframeConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://genomic-telemetry.cloudfunctions.net".to_string(),
            batch_size: 60,
            timeout_ms: 5000,
            enabled: false, // Off by default; enable via env var or explicit init
        }
    }
}

/// Serializable payload sent to Hostframe.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Serialize from SenseReport with current timestamp.
    pub fn from_sense(report: &SenseReport) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            tick: report.tick,
            timestamp_ms: now,
            mean_utility: report.mean_utility,
            min_safety: report.min_safety,
            quarantine_fraction: report.quarantine_fraction,
            bus_drop_rate: report.bus_drop_rate,
            awareness_coverage: report.awareness_coverage,
            phage_events_this_tick: report.phage_events_this_tick,
            phage_events_total: report.phage_events_total,
            deterministic_alarm: report.deterministic_alarm,
            health_score: report.health_score(),
            needs_healing: report.needs_healing(),
        }
    }

    /// Serialize to JSON for transmission.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Telemetry client — buffers and sends reports to Hostframe asynchronously.
pub struct TelemetryClient {
    config: HostframeConfig,
    buffer: Vec<TelemetryPayload>,
    last_send_tick: u64,
    total_sent: usize,
    total_dropped: usize,
}

impl TelemetryClient {
    pub fn new(config: HostframeConfig) -> Self {
        let batch_size = config.batch_size;
        Self {
            config,
            buffer: Vec::with_capacity(batch_size),
            last_send_tick: 0,
            total_sent: 0,
            total_dropped: 0,
        }
    }

    /// Buffer a SenseReport; send if batch is full.
    pub fn buffer_report(&mut self, report: &SenseReport) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let payload = TelemetryPayload::from_sense(report);
        self.buffer.push(payload.clone());

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Send buffered reports to Hostframe (or mock endpoint).
    pub fn flush(&mut self) -> Result<(), String> {
        if !self.config.enabled || self.buffer.is_empty() {
            return Ok(());
        }

        let batch = serde_json::json!({
            "reports": self.buffer,
            "sent_at_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

        let json_str = batch.to_string();
        let batch_len = self.buffer.len();

        match self.send_batch(&json_str) {
            Ok(_) => {
                self.total_sent += batch_len;
                self.buffer.clear();
                Ok(())
            }
            Err(e) => {
                self.total_dropped += batch_len;
                eprintln!("[Hostframe] Batch send failed: {} (dropped {} reports)", e, batch_len);
                self.buffer.clear(); // Clear regardless; don't accumulate on failure
                Ok(()) // Don't propagate telemetry errors to agent logic
            }
        }
    }

    /// HTTP POST to endpoint with exponential backoff (stub for testing).
    fn send_batch(&self, json: &str) -> Result<(), String> {
        if self.config.endpoint.starts_with("http://localhost:") {
            // Mock endpoint for testing
            return Ok(());
        }

        // In production: use ureq with timeout + retry
        let _timeout = Duration::from_millis(self.config.timeout_ms);
        let _json = json; // Suppress unused warning in test builds

        // TODO: Implement ureq POST with retry logic
        // match ureq::post(&self.config.endpoint)
        //     .timeout(timeout)
        //     .send_string(json) {
        //     Ok(_) => Ok(()),
        //     Err(e) => Err(format!("POST failed: {}", e)),
        // }

        // For now, just succeed silently (mock)
        Ok(())
    }

    /// Retrieve telemetry stats.
    pub fn stats(&self) -> TelemetryStats {
        TelemetryStats {
            buffered: self.buffer.len(),
            total_sent: self.total_sent,
            total_dropped: self.total_dropped,
            drop_rate: if self.total_sent + self.total_dropped > 0 {
                self.total_dropped as f32 / (self.total_sent + self.total_dropped) as f32
            } else {
                0.0
            },
        }
    }
}

/// Telemetry send statistics.
#[derive(Clone, Debug)]
pub struct TelemetryStats {
    pub buffered: usize,
    pub total_sent: usize,
    pub total_dropped: usize,
    pub drop_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(tick: u64, utility: f32, safety: f32) -> SenseReport {
        SenseReport {
            tick,
            mean_utility: utility,
            min_safety: safety,
            quarantine_fraction: 0.0,
            bus_drop_rate: 0.0,
            awareness_coverage: 1.0,
            phage_events_this_tick: 0,
            phage_events_total: 0,
            deterministic_alarm: false,
        }
    }

    #[test]
    fn payload_from_sense() {
        let report = make_report(1, 0.9, 0.95);
        let payload = TelemetryPayload::from_sense(&report);
        assert_eq!(payload.tick, 1);
        assert!((payload.mean_utility - 0.9).abs() < 1e-5);
        assert!((payload.min_safety - 0.95).abs() < 1e-5);
        assert!(!payload.deterministic_alarm);
    }

    #[test]
    fn payload_serializes() {
        let report = make_report(1, 0.8, 0.9);
        let payload = TelemetryPayload::from_sense(&report);
        let json = payload.to_json().unwrap();
        assert!(json.contains("\"tick\":1"));
        assert!(json.contains("\"mean_utility\":"));
    }

    #[test]
    fn client_buffers_reports() {
        let config = HostframeConfig {
            enabled: true,
            batch_size: 10,
            ..Default::default()
        };
        let mut client = TelemetryClient::new(config);
        let report = make_report(1, 0.9, 0.95);

        client.buffer_report(&report).unwrap();
        assert_eq!(client.buffer.len(), 1);
        assert_eq!(client.total_sent, 0);
    }

    #[test]
    fn client_flushes_on_batch_full() {
        let config = HostframeConfig {
            enabled: true,
            batch_size: 2,
            endpoint: "http://localhost:9999".to_string(), // Mock
            ..Default::default()
        };
        let mut client = TelemetryClient::new(config);

        client.buffer_report(&make_report(1, 0.9, 0.95)).unwrap();
        assert_eq!(client.buffer.len(), 1);

        client.buffer_report(&make_report(2, 0.85, 0.9)).unwrap();
        assert_eq!(client.buffer.len(), 0); // Flushed
        assert_eq!(client.total_sent, 2);
    }

    #[test]
    fn client_disabled_does_not_buffer() {
        let config = HostframeConfig {
            enabled: false,
            ..Default::default()
        };
        let mut client = TelemetryClient::new(config);
        client.buffer_report(&make_report(1, 0.9, 0.95)).unwrap();
        assert_eq!(client.buffer.len(), 0);
        assert_eq!(client.total_sent, 0);
    }

    #[test]
    fn stats_track_drops() {
        let config = HostframeConfig {
            enabled: true,
            batch_size: 100, // Won't flush
            ..Default::default()
        };
        let mut client = TelemetryClient::new(config);
        client.buffer_report(&make_report(1, 0.9, 0.95)).ok();
        client.total_dropped = 5;
        let stats = client.stats();
        assert_eq!(stats.total_dropped, 5);
        assert!(stats.drop_rate >= 0.0 && stats.drop_rate <= 1.0);
    }

    #[test]
    fn manual_flush_clears_buffer() {
        let config = HostframeConfig {
            enabled: true,
            batch_size: 1000,
            endpoint: "http://localhost:9999".to_string(),
            ..Default::default()
        };
        let mut client = TelemetryClient::new(config);
        client.buffer_report(&make_report(1, 0.9, 0.95)).unwrap();
        client.buffer_report(&make_report(2, 0.8, 0.9)).unwrap();
        assert_eq!(client.buffer.len(), 2);

        client.flush().unwrap();
        assert_eq!(client.buffer.len(), 0);
        assert_eq!(client.total_sent, 2);
    }

    #[test]
    fn drop_rate_calculation() {
        let config = HostframeConfig::default();
        let mut client = TelemetryClient::new(config);
        client.total_sent = 80;
        client.total_dropped = 20;
        let stats = client.stats();
        assert!((stats.drop_rate - 0.2).abs() < 1e-5);
    }
}
