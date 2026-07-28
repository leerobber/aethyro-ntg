use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseReport {
    pub timestamp_ms: u64,
    pub cycle: u64,
    pub hormone_adrenaline: f32,
    pub hormone_cortisol: f32,
    pub hormone_serotonin: f32,
    pub active_nodes: usize,
    pub mutations_queued: usize,
    pub safety_score: f32,
    pub behavioral_drift: f32,
    pub energy_consumed_uj: f32,
    pub coherence: f32,
}

impl Default for SenseReport {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            cycle: 0,
            hormone_adrenaline: 0.5,
            hormone_cortisol: 0.3,
            hormone_serotonin: 0.7,
            active_nodes: 0,
            mutations_queued: 0,
            safety_score: 1.0,
            behavioral_drift: 0.0,
            energy_consumed_uj: 0.0,
            coherence: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct TelemetryStream {
    reports: Arc<RwLock<Vec<SenseReport>>>,
    hz_60_ticker: Duration,
    cycle_counter: u64,
    start_time: Instant,
}

impl TelemetryStream {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(Vec::new())),
            hz_60_ticker: Duration::from_millis(16), // 60 Hz = 16.67ms per tick
            cycle_counter: 0,
            start_time: Instant::now(),
        }
    }

    pub async fn emit_report(&mut self, report: SenseReport) {
        let mut reports = self.reports.write().await;
        reports.push(report);
        if reports.len() > 3600 { // Keep 60 seconds of data (60 Hz × 60 sec)
            reports.remove(0);
        }
        self.cycle_counter += 1;
    }

    pub async fn get_latest_report(&self) -> Option<SenseReport> {
        let reports = self.reports.read().await;
        reports.last().cloned()
    }

    pub async fn get_all_reports(&self) -> Vec<SenseReport> {
        self.reports.read().await.clone()
    }

    pub async fn clear_reports(&self) {
        self.reports.write().await.clear();
    }

    pub fn hz_60_tick_duration(&self) -> Duration {
        self.hz_60_ticker
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

impl Default for TelemetryStream {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryMessage {
    pub message_type: String,
    pub data: serde_json::Value,
}

impl TelemetryMessage {
    pub fn sense_report(report: &SenseReport) -> Self {
        Self {
            message_type: "sense_report".to_string(),
            data: serde_json::to_value(report).unwrap_or(serde_json::json!({})),
        }
    }

    pub fn heartbeat(cycle: u64) -> Self {
        Self {
            message_type: "heartbeat".to_string(),
            data: serde_json::json!({ "cycle": cycle }),
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sense_report_default() {
        let report = SenseReport::default();
        assert_eq!(report.timestamp_ms, 0);
        assert_eq!(report.hormone_adrenaline, 0.5);
        assert_eq!(report.safety_score, 1.0);
    }

    #[test]
    fn test_telemetry_stream_new() {
        let stream = TelemetryStream::new();
        assert_eq!(stream.cycle_counter, 0);
        assert_eq!(stream.hz_60_tick_duration(), Duration::from_millis(16));
    }

    #[tokio::test]
    async fn test_emit_and_get_report() {
        let mut stream = TelemetryStream::new();
        let report = SenseReport {
            timestamp_ms: 100,
            cycle: 1,
            hormone_adrenaline: 0.6,
            ..Default::default()
        };

        stream.emit_report(report.clone()).await;
        let latest = stream.get_latest_report().await;

        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.timestamp_ms, 100);
        assert_eq!(latest.cycle, 1);
    }

    #[tokio::test]
    async fn test_telemetry_message_serialization() {
        let report = SenseReport {
            timestamp_ms: 200,
            cycle: 5,
            ..Default::default()
        };

        let msg = TelemetryMessage::sense_report(&report);
        assert_eq!(msg.message_type, "sense_report");

        let json = msg.to_json_string();
        assert!(!json.is_empty());
        assert!(json.contains("sense_report"));
    }

    #[tokio::test]
    async fn test_report_buffer_limit() {
        let mut stream = TelemetryStream::new();

        for i in 0..4000 {
            let report = SenseReport {
                cycle: i,
                ..Default::default()
            };
            stream.emit_report(report).await;
        }

        let reports = stream.get_all_reports().await;
        assert!(reports.len() <= 3600);
    }

    #[test]
    fn test_elapsed_ms() {
        let stream = TelemetryStream::new();
        let elapsed = stream.elapsed_ms();
        assert!(elapsed >= 0);
    }

    #[test]
    fn test_heartbeat_message() {
        let msg = TelemetryMessage::heartbeat(42);
        assert_eq!(msg.message_type, "heartbeat");

        let json = msg.to_json_string();
        assert!(json.contains("heartbeat"));
        assert!(json.contains("42"));
    }
}
