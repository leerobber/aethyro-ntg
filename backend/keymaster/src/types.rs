//! Wire types for the NanoKeymaster routing protocol.
//!
//! These must stay wire-compatible with the ureq POST in kernel_host.
//! Schema version is carried in every request so the backend can reject
//! stale clients cleanly.

use serde::{Deserialize, Serialize};

/// Schema version for protocol compatibility checking.
pub const SCHEMA_VERSION: u8 = 1;

/// Routing target returned to the NanoKeymaster agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingTarget {
    /// Serve from the local ternary brain (air-gapped).
    Local,
    /// Try local first; fall back to external on miss.
    LocalFallback,
    /// Route to the configured external LLM backend.
    External,
}

/// Inbound routing decision request from kernel_host.
#[derive(Debug, Deserialize)]
pub struct RouteRequest {
    /// Schema version — reject if != SCHEMA_VERSION.
    pub schema_version: u8,
    /// Unique request ID (UUID v4) for idempotency and audit.
    pub request_id: String,
    /// Agent ID originating the request.
    pub agent_id: u32,
    /// Query context hash (SHA-256 hex of the query payload, not the payload itself).
    pub context_hash: String,
    /// Ternary policy score [0, 1] from the local brain.
    pub local_policy_score: f32,
    /// Estimated token count for the pending query.
    pub estimated_tokens: u32,
    /// Milliseconds since last successful local inference.
    pub ms_since_last_local: u64,
}

/// Outbound routing decision response.
#[derive(Debug, Serialize)]
pub struct RouteResponse {
    /// Echoed from request for correlation.
    pub request_id: String,
    /// The routing decision.
    pub target: RoutingTarget,
    /// Human-readable rationale (for audit log).
    pub reason: String,
    /// Server-assigned audit sequence number (monotonic per instance).
    pub audit_seq: u64,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub request_id: Option<String>,
}
