use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0} not found")]
    NotFound(String),
    #[error("{}", .0.join("; "))]
    Invalid(Vec<String>),
    /// A feature that needs configuration the server doesn't have.
    #[error("{0}")]
    Unavailable(String),
    /// An upstream service failed.
    #[error("{0}")]
    Upstream(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(vec![msg.into()])
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.into())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal(e.into())
    }
}

impl From<srs_core::scheduler::SchedulerError> for ApiError {
    fn from(e: srs_core::scheduler::SchedulerError) -> Self {
        Self::Internal(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, json!({ "error": self.to_string() })),
            ApiError::Invalid(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "error": self.to_string(), "errors": errors }),
            ),
            ApiError::Unavailable(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "error": self.to_string() }),
            ),
            ApiError::Upstream(_) => (StatusCode::BAD_GATEWAY, json!({ "error": self.to_string() })),
            ApiError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal error" }),
                )
            }
        };
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
