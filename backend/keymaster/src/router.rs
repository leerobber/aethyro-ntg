//! Routing decision logic — stateless, deterministic policy engine.
//!
//! Policy rules (applied in order — first match wins):
//!   1. Schema mismatch → 400 Bad Request
//!   2. policy_score ≥ 0.7 AND ms_since_last_local < 30_000 → Local
//!   3. policy_score ≥ 0.4 → LocalFallback
//!   4. otherwise → External

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    audit::log_decision,
    types::{
        ErrorResponse, RouteRequest, RouteResponse, RoutingTarget, SCHEMA_VERSION,
    },
};

static AUDIT_SEQ: AtomicU64 = AtomicU64::new(1);

pub async fn handle_route(
    Json(req): Json<RouteRequest>,
) -> impl IntoResponse {
    if req.schema_version != SCHEMA_VERSION {
        let body = ErrorResponse {
            error: format!(
                "schema_version mismatch: got {}, want {}",
                req.schema_version, SCHEMA_VERSION
            ),
            request_id: Some(req.request_id),
        };
        return (StatusCode::BAD_REQUEST, Json(serde_json::to_value(body).unwrap()))
            .into_response();
    }

    let (target, reason) = decide(&req);
    let seq = AUDIT_SEQ.fetch_add(1, Ordering::Relaxed);

    log_decision(&req, target, &reason, seq);

    let resp = RouteResponse {
        request_id: req.request_id,
        target,
        reason,
        audit_seq: seq,
    };
    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
}

fn decide(req: &RouteRequest) -> (RoutingTarget, String) {
    let score = req.local_policy_score;
    let ms = req.ms_since_last_local;

    if score >= 0.7 && ms < 30_000 {
        (
            RoutingTarget::Local,
            format!("policy_score={score:.3} ≥ 0.7, ms_since_local={ms} < 30s"),
        )
    } else if score >= 0.4 {
        (
            RoutingTarget::LocalFallback,
            format!("policy_score={score:.3} ≥ 0.4 (local-first, fallback enabled)"),
        )
    } else {
        (
            RoutingTarget::External,
            format!("policy_score={score:.3} < 0.4, routing to external backend"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SCHEMA_VERSION;

    fn req(score: f32, ms: u64) -> RouteRequest {
        RouteRequest {
            schema_version: SCHEMA_VERSION,
            request_id: "test-req-1".into(),
            agent_id: 1,
            context_hash: "abc123".into(),
            local_policy_score: score,
            estimated_tokens: 100,
            ms_since_last_local: ms,
        }
    }

    #[test]
    fn high_score_recent_local() {
        let (t, _) = decide(&req(0.9, 1_000));
        assert_eq!(t, RoutingTarget::Local);
    }

    #[test]
    fn high_score_but_stale_local() {
        let (t, _) = decide(&req(0.9, 60_000));
        assert_eq!(t, RoutingTarget::LocalFallback);
    }

    #[test]
    fn mid_score_local_fallback() {
        let (t, _) = decide(&req(0.5, 0));
        assert_eq!(t, RoutingTarget::LocalFallback);
    }

    #[test]
    fn low_score_external() {
        let (t, _) = decide(&req(0.2, 0));
        assert_eq!(t, RoutingTarget::External);
    }

    #[test]
    fn boundary_score_exactly_07_recent() {
        let (t, _) = decide(&req(0.7, 29_999));
        assert_eq!(t, RoutingTarget::Local);
    }

    #[test]
    fn boundary_score_exactly_04() {
        let (t, _) = decide(&req(0.4, 60_000));
        assert_eq!(t, RoutingTarget::LocalFallback);
    }
}
