//! Structured audit logging for routing decisions.
//!
//! Each decision is emitted as a JSON log line via `tracing`. In Cloud Run
//! these land in Cloud Logging automatically. The audit_seq provides a
//! monotonic ordering handle for post-hoc verification without a persistent
//! store (tamper-evident ordering is enforced in the kernel ledger, not here).

use tracing::info;
use crate::types::{RouteRequest, RoutingTarget};

pub fn log_decision(
    req: &RouteRequest,
    target: RoutingTarget,
    reason: &str,
    seq: u64,
) {
    info!(
        audit_seq     = seq,
        request_id    = %req.request_id,
        agent_id      = req.agent_id,
        context_hash  = %req.context_hash,
        policy_score  = req.local_policy_score,
        tokens        = req.estimated_tokens,
        ms_since_local = req.ms_since_last_local,
        target        = ?target,
        reason        = reason,
        "routing_decision"
    );
}
