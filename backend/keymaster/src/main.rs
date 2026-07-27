//! NanoKeymaster Cloud Run backend — sovereign routing arbiter.
//!
//! Accepts POST /route from kernel_host (ureq) and returns a routing decision.
//! Stateless: routing policy is embedded in the request context; decisions are
//! logged server-side for audit but no persistent state is held.
//!
//! Endpoints:
//!   POST /route   — routing decision request
//!   GET  /health  — liveness probe
//!   GET  /ready   — readiness probe

mod router;
mod types;
mod audit;

use axum::{Router, routing::{get, post}};
use std::net::SocketAddr;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let app = Router::new()
        .route("/route",  post(router::handle_route))
        .route("/health", get(|| async { "ok" }))
        .route("/ready",  get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(10)));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(port, "keymaster-backend listening");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("failed to install CTRL+C handler");
    tracing::info!("shutdown signal received");
}
