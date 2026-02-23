//! Session-based authentication middleware, helpers, and the `AuthUser` extractor.

use axum::{
    extract::{FromRequestParts, Request},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::AppState;
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

// Session key for the stored user ID.
const USER_ID_KEY: &str = "user_id";

/// Axum middleware that rejects unauthenticated requests.
/// HTMX requests get a 401 + `hx-redirect` header; normal requests get a 302 to `/`.
pub async fn require_auth(session: Session, request: Request, next: Next) -> Response {
    let user_id: Option<i64> = session.get(USER_ID_KEY).await.unwrap_or(None);

    if user_id.is_none() {
        let is_htmx = request.headers().get("HX-Request").is_some();

        if is_htmx {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::HeaderName::from_static("hx-redirect"), "/")],
                "Unauthorized",
            )
                .into_response();
        }

        return Redirect::to("/").into_response();
    }

    next.run(request).await
}

/// Stores the user ID in the session after successful login.
pub async fn set_user_session(session: &Session, user_id: i64) -> Result<(), AppError> {
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set session: {}", e)))?;
    Ok(())
}

/// Extracts the authenticated user ID from the session, or returns `Unauthorized`.
pub async fn get_user_id_from_session(session: &Session) -> Result<i64, AppError> {
    session
        .get::<i64>(USER_ID_KEY)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read session: {}", e)))?
        .ok_or(AppError::Unauthorized)
}

/// Flushes the entire session (logout).
pub async fn clear_session(session: &Session) -> Result<(), AppError> {
    session
        .flush()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to clear session: {}", e)))?;
    Ok(())
}

/// Authenticated user extractor. Combines session-based user ID with per-user encryption.
///
/// Use as a handler parameter to replace the manual `get_user_id_from_session` + `crypto.for_user` pattern:
/// ```ignore
/// pub async fn handler(auth: AuthUser, State(state): State<AppState>) -> Result<..., AppError> {
///     let entries = BragEntry::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
/// }
/// ```
pub struct AuthUser {
    pub user_id: i64,
    pub crypto: UserCrypto,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        let user_id = get_user_id_from_session(&session).await?;
        let crypto = state.crypto.for_user(user_id)?;

        Ok(AuthUser { user_id, crypto })
    }
}
