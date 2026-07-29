//! Phase F.1 Integration Tests: Hostframe Backend Communication
//! Tests TelemetryPayload serialization and mock HTTP backend ingestion.

use ntg_kernel::{
    TelemetryPayload, LdComputeStats, SafetyScoreSnapshot, SystemStats,
};
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_telemetry_json_serialization() {
    let payload = TelemetryPayload::new(
        "test-ld-001".to_string(),
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

    let json = payload.to_json().expect("Failed to serialize");
    assert!(json.contains("\"event_type\":\"ld_computation\""));
    assert!(json.contains("\"snps_count\":48494"));
    assert!(json.contains("\"pairs_computed\":1175809771"));
    assert!(json.contains("\"kernel_version\":\"0.1.0\""));
}

#[tokio::test]
async fn test_telemetry_deserialization_round_trip() {
    let original = TelemetryPayload::new(
        "test-roundtrip".to_string(),
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

    let json = original.to_json().expect("Serialization failed");
    let deserialized: TelemetryPayload =
        serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.event_id, original.event_id);
    assert_eq!(deserialized.event_type, "safety_check");
    assert_eq!(
        deserialized.safety_score.unwrap().overall,
        0.92
    );
}

#[tokio::test]
async fn test_hostframe_mock_backend_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest/telemetry"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let payload = TelemetryPayload::new(
        "test-backend-001".to_string(),
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

    let result = payload.send_to_hostframe(&mock_server.uri()).await;
    assert!(result.is_ok(), "Failed to send telemetry to mock backend");
}

#[tokio::test]
async fn test_hostframe_backend_error_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest/telemetry"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let payload = TelemetryPayload::new(
        "test-error-001".to_string(),
        "ld_computation".to_string(),
        "0.1.0".to_string(),
    );

    let result = payload.send_to_hostframe(&mock_server.uri()).await;
    assert!(
        result.is_err(),
        "Should fail with 500 error from backend"
    );
}

#[tokio::test]
async fn test_telemetry_with_system_stats() {
    let payload = TelemetryPayload::new(
        "test-sys-001".to_string(),
        "system_metrics".to_string(),
        "0.1.0".to_string(),
    )
    .with_system_stats(SystemStats {
        memory_mb: 256.5,
        cpu_percent: 45.2,
        gpu_utilization_percent: Some(87.3),
    });

    let json = payload.to_json().expect("Serialization failed");
    let deserialized: TelemetryPayload =
        serde_json::from_str(&json).expect("Deserialization failed");

    let stats = deserialized.system_stats.unwrap();
    assert_eq!(stats.memory_mb, 256.5);
    assert_eq!(stats.cpu_percent, 45.2);
    assert_eq!(stats.gpu_utilization_percent, Some(87.3));
}

#[tokio::test]
async fn test_telemetry_complete_payload() {
    let payload = TelemetryPayload::new(
        "test-complete-001".to_string(),
        "full_workload".to_string(),
        "0.1.0".to_string(),
    )
    .with_ld_stats(LdComputeStats {
        snps_count: 48494,
        samples_count: 389,
        pairs_computed: 1175809771,
        throughput_pairs_per_sec: 1460272.0,
        wall_clock_seconds: 805.7,
        high_ld_pairs_count: 42,
    })
    .with_safety_score(SafetyScoreSnapshot {
        constraint_score: 0.98,
        alignment_score: 0.94,
        confidence: 0.96,
        overall: 0.95,
        rollback_triggered: false,
    })
    .with_system_stats(SystemStats {
        memory_mb: 512.0,
        cpu_percent: 65.0,
        gpu_utilization_percent: Some(92.5),
    })
    .with_ledger_hash("abc123def456".to_string());

    let json = payload.to_json().expect("Serialization failed");
    assert!(json.contains("\"snps_count\":48494"));
    assert!(json.contains("\"overall\":0.95"));
    assert!(json.contains("\"memory_mb\":512"));
    assert!(json.contains("\"ledger_hash\":\"abc123def456\""));

    let deserialized: TelemetryPayload =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert!(deserialized.ld_compute.is_some());
    assert!(deserialized.safety_score.is_some());
    assert!(deserialized.system_stats.is_some());
    assert!(deserialized.ledger_hash.is_some());
}

#[tokio::test]
async fn test_hostframe_multiple_events() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest/telemetry"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    for i in 0..5 {
        let payload = TelemetryPayload::new(
            format!("test-batch-{:03}", i),
            "ld_computation".to_string(),
            "0.1.0".to_string(),
        )
        .with_ld_stats(LdComputeStats {
            snps_count: 10000 + i * 100,
            samples_count: 389,
            pairs_computed: 50000000 + i * 1000000,
            throughput_pairs_per_sec: 1460272.0,
            wall_clock_seconds: 34.2 + i as f64,
            high_ld_pairs_count: i * 5,
        });

        let result = payload.send_to_hostframe(&mock_server.uri()).await;
        assert!(
            result.is_ok(),
            "Batch event {} failed to send",
            i
        );
    }
}
