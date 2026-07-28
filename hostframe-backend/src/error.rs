use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostframeError {
    #[error("BigQuery error: {0}")]
    BigQueryError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for HostframeError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            HostframeError::BigQueryError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            HostframeError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            HostframeError::AuthError(msg) => {
                (StatusCode::UNAUTHORIZED, msg)
            }
            HostframeError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
