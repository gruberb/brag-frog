//! Authentication for `/api/v1`: a JSON 401 middleware and an extractor with a JSON rejection.

use axum::{
    extract::{FromRequestParts, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_sessions::Session;

use super::error::ApiError;
use crate::AppState;
use crate::identity::auth::middleware::{AuthUser, get_user_id_from_session};
use crate::kernel::error::AppError;

/// Rejects unauthenticated API requests with a JSON 401 (never a redirect).
pub async fn require_auth_api(session: Session, request: Request, next: Next) -> Response {
    match get_user_id_from_session(&session).await {
        Ok(_) => next.run(request).await,
        Err(err) => ApiError::from(err).into_response(),
    }
}

/// `AuthUser` for API handlers: same session and per-user crypto, JSON error on failure.
pub struct ApiAuth(pub AuthUser);

impl FromRequestParts<AppState> for ApiAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        AuthUser::from_request_parts(parts, state)
            .await
            .map(ApiAuth)
            .map_err(|e: AppError| e.into())
    }
}
