//! JSON error type for `/api/v1` handlers. Wraps `AppError` so repo and service code is shared
//! with the legacy HTML handlers; only the wire format differs.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::kernel::error::AppError;

/// Error returned by API handlers. Serialised as `{"error":{"code":"...","message":"..."}}`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

pub type ApiResult<T> = Result<Json<T>, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    /// 422: the request was well formed but its content is not acceptable.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(msg) => Self::not_found(msg),
            AppError::Unauthorized => {
                Self::new(StatusCode::UNAUTHORIZED, "unauthorized", "Unauthorized")
            }
            AppError::BadRequest(msg) => Self::new(StatusCode::BAD_REQUEST, "bad_request", msg),
            other => {
                // Internal causes are logged, never sent to the client.
                tracing::error!("API internal error: {:?}", other);
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal server error",
                )
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        AppError::from(err).into()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message}})),
        )
            .into_response()
    }
}
