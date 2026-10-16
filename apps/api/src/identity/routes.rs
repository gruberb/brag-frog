use axum::{
    extract::{Query, State},
    response::Redirect,
};
use tower_sessions::Session;

use crate::AppState;
use crate::identity::auth::{self, middleware as auth_mw};
use crate::identity::oauth_state::{self, OAuthFlow};
use crate::kernel::error::AppError;

// ── Auth routes ──

/// Query parameters returned by Google OAuth redirect.
#[derive(serde::Deserialize)]
pub struct CallbackParams {
    code: String,
    state: Option<String>,
}

/// Starts the Google sign-in: redirects the browser to the consent screen.
pub async fn login(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let state_token = oauth_state::mint(&state.crypto, OAuthFlow::Login)?;
    let url = crate::identity::auth::google_auth_url(&state.config, &state_token);
    Ok(Redirect::to(&url))
}

/// Google OAuth callback: validates state, exchanges code for token, creates/finds user, sets session.
///
/// The CSRF `state` parameter is a stateless HMAC-signed token (see
/// `identity::oauth_state`); the embedded flow tag selects which branch runs
/// here — login vs. Drive vs. Calendar consent.
pub async fn callback(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, AppError> {
    let received_state = params.state.as_deref().ok_or_else(|| {
        tracing::warn!("OAuth callback missing state parameter");
        AppError::BadRequest("Invalid OAuth state".to_string())
    })?;
    let flow = oauth_state::verify(&state.crypto, received_state).inspect_err(|_| {
        tracing::warn!("OAuth state verification failed");
    })?;

    let is_drive_flow = flow == OAuthFlow::Drive;
    let is_calendar_flow = flow == OAuthFlow::Calendar;

    tracing::info!("OAuth callback received, exchanging code...");

    let token_resp = auth::exchange_code(&state.config, &params.code)
        .await
        .inspect_err(|e| tracing::error!("Token exchange failed: {:?}", e))?;

    if is_drive_flow || is_calendar_flow {
        // Google service OAuth flow: user must already be logged in
        let user_id: Option<i64> = session.get("user_id").await.unwrap_or(None);
        let user_id = user_id.ok_or(AppError::Unauthorized)?;

        let service_name = if is_calendar_flow {
            "google_calendar"
        } else {
            "google_drive"
        };

        let refresh_token = token_resp.refresh_token.ok_or_else(|| {
            AppError::Internal(format!(
                "Google did not return a refresh token for {}",
                service_name
            ))
        })?;

        // Encrypt and store the refresh token as the integration token
        let user_crypto = state.crypto.for_user(user_id)?;
        let encrypted = user_crypto.encrypt(&refresh_token)?;

        crate::integrations::model::IntegrationConfig::upsert(
            &state.db,
            user_id,
            service_name,
            true,
            Some(&encrypted),
            None,
        )
        .await?;

        tracing::info!(
            user_id,
            service = service_name,
            "Google integration connected"
        );
        return Ok(Redirect::to("/settings/integrations"));
    }

    // Normal login flow
    tracing::info!("Token exchanged, fetching user info...");

    let user_info = auth::get_user_info(&token_resp.access_token)
        .await
        .inspect_err(|e| tracing::error!("User info fetch failed: {:?}", e))?;

    tracing::info!("User authenticated: sub={}", user_info.sub);

    let user = auth::authenticate_user(&state.db, &state.config, &user_info)
        .await
        .inspect_err(|e| tracing::error!("User auth failed: {:?}", e))?;

    tracing::info!("User authenticated: id={}, setting session...", user.id);

    auth_mw::set_user_session(&session, user.id).await?;

    crate::integrations::background::spawn_sync_all(state.clone(), user.id);

    tracing::info!("Session set, redirecting to /today");

    Ok(Redirect::to("/today"))
}

/// Initiates Google Drive OAuth consent flow. Requires an authenticated session.
pub async fn connect_google_drive(
    auth: auth::middleware::AuthUser,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let _ = auth; // ensure user is logged in
    let state_token = oauth_state::mint(&state.crypto, OAuthFlow::Drive)?;
    let url = crate::identity::auth::google_drive_auth_url(&state.config, &state_token);
    Ok(Redirect::to(&url))
}

/// Initiates Google Calendar OAuth consent flow. Requires an authenticated session.
pub async fn connect_google_calendar(
    auth: auth::middleware::AuthUser,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let _ = auth; // ensure user is logged in
    let state_token = oauth_state::mint(&state.crypto, OAuthFlow::Calendar)?;
    let url = crate::identity::auth::google_calendar_auth_url(&state.config, &state_token);
    Ok(Redirect::to(&url))
}
