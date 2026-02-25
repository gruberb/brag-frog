use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::AppState;
use crate::shared::error::AppError;

/// Landing page: redirects authenticated users to `/dashboard`, shows login page otherwise.
pub async fn landing_page(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    // Check if user is already logged in (non-error pattern)
    let user_id: Option<i64> = session.get("user_id").await.unwrap_or(None);

    if user_id.is_some() {
        return Ok(Redirect::to("/dashboard").into_response());
    }

    let mut ctx = tera::Context::new();

    let state_token = format!("login:{}", uuid::Uuid::new_v4());
    session
        .insert("oauth_state", &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let google_auth_url = crate::identity::auth::google_auth_url(&state.config, &state_token);
    ctx.insert("google_auth_url", &google_auth_url);
    ctx.insert("instance_name", &state.config.instance_name);

    let html = state.templates.render("pages/landing.html", &ctx)?;
    Ok(Html(html).into_response())
}
