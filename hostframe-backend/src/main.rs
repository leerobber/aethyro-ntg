use axum::{
    extract::Json,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

mod bigquery;
mod telemetry;
mod config;
mod error;

use config::Config;
use error::HostframeError;
use telemetry::TelemetryPayload;

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    bq_client: Arc<bigquery::BigQueryClient>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IngestRequest {
    reports: Vec<TelemetryPayload>,
    sent_at_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IngestResponse {
    success: bool,
    batch_id: String,
    records_ingested: usize,
}

/// Landing document for `/`, so a browser hitting the bare URL sees what the service is and
/// which routes it has instead of an empty 404. Unauthenticated and stateless by design.
async fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "aethyro-hostframe",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "GET /health": "liveness",
            "POST /ingest": "telemetry batch ingest ({reports, sent_at_ms})",
            "GET /metrics": "in-memory ingest counters",
        },
    }))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn ingest_telemetry(
    state: axum::extract::State<AppState>,
    Json(request): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, HostframeError> {
    info!(
        "Received telemetry batch with {} reports at {}ms",
        request.reports.len(),
        request.sent_at_ms
    );

    let batch_id = uuid::Uuid::new_v4().to_string();
    let count = request.reports.len();

    // Insert into BigQuery
    state
        .bq_client
        .insert_telemetry_batch(&batch_id, request.reports, request.sent_at_ms)
        .await?;

    info!("Ingested {} telemetry records (batch_id={})", count, batch_id);

    Ok(Json(IngestResponse {
        success: true,
        batch_id,
        records_ingested: count,
    }))
}

async fn metrics(
    state: axum::extract::State<AppState>,
) -> Result<Json<bigquery::MetricsSnapshot>, HostframeError> {
    let metrics = state.bq_client.get_metrics_snapshot().await?;
    Ok(Json(metrics))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;
    info!("Hostframe backend starting (project={})", config.gcp_project);

    let bq_client = Arc::new(bigquery::BigQueryClient::new(&config).await?);

    let state = AppState {
        config: Arc::new(config),
        bq_client,
    };

    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/health", axum::routing::get(health))
        .route("/ingest", post(ingest_telemetry))
        .route("/metrics", axum::routing::get(metrics))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
