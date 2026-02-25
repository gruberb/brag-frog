use axum::{
    Form,
    extract::{Query, State},
    http::header,
    response::{Html, IntoResponse, Redirect},
};
use tower_sessions::Session;

use crate::AppState;
use crate::identity::auth::{self, middleware as auth_mw};
use crate::identity::model::{ProfileUpdate, User};
use crate::cycle::model::BragPhase;
use crate::kernel::error::AppError;

// ── Auth routes ──

/// Session key for the OAuth CSRF state token.
const OAUTH_STATE_KEY: &str = "oauth_state";

/// Query parameters returned by Google OAuth redirect.
#[derive(serde::Deserialize)]
pub struct CallbackParams {
    code: String,
    state: Option<String>,
}

/// Renders the login page with a Google OAuth sign-in URL.
pub async fn login_page(
    State(state): State<AppState>,
    session: Session,
) -> Result<Html<String>, AppError> {
    // Generate random state token for CSRF protection (login: prefix for routing)
    let state_token = format!("login:{}", uuid::Uuid::new_v4());
    session
        .insert(OAUTH_STATE_KEY, &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let google_auth_url = crate::identity::auth::google_auth_url(&state.config, &state_token);
    let mut ctx = tera::Context::new();
    ctx.insert("google_auth_url", &google_auth_url);
    let html = state.templates.render("auth/login.html", &ctx)?;
    Ok(Html(html))
}

/// Google OAuth callback: validates state, exchanges code for token, creates/finds user, sets session.
/// Uses state prefix routing: `login:` for normal login, `drive:` for Google Drive OAuth.
pub async fn callback(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, AppError> {
    // Validate OAuth state parameter
    let expected_state: Option<String> = session
        .get(OAUTH_STATE_KEY)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read OAuth state: {}", e)))?;

    // Remove state from session (one-time use)
    session.remove::<String>(OAUTH_STATE_KEY).await.ok();

    match (&expected_state, &params.state) {
        (Some(expected), Some(received)) if expected == received => {}
        _ => {
            tracing::warn!("OAuth state mismatch or missing");
            return Err(AppError::BadRequest("Invalid OAuth state".to_string()));
        }
    }

    // Determine flow from state prefix
    let received_state = params.state.as_deref().unwrap_or("");
    let is_drive_flow = received_state.starts_with("drive:");
    let is_calendar_flow = received_state.starts_with("calendar:");

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
        return Ok(Redirect::to("/integrations"));
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

    tracing::info!("Session set, redirecting to /");

    Ok(Redirect::to("/"))
}

/// Initiates Google Drive OAuth consent flow. Requires an authenticated session.
pub async fn connect_google_drive(
    auth: auth::middleware::AuthUser,
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, AppError> {
    let _ = auth; // ensure user is logged in
    let state_token = format!("drive:{}", uuid::Uuid::new_v4());
    session
        .insert(OAUTH_STATE_KEY, &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let url = crate::identity::auth::google_drive_auth_url(&state.config, &state_token);
    Ok(Redirect::to(&url))
}

/// Initiates Google Calendar OAuth consent flow. Requires an authenticated session.
pub async fn connect_google_calendar(
    auth: auth::middleware::AuthUser,
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, AppError> {
    let _ = auth; // ensure user is logged in
    let state_token = format!("calendar:{}", uuid::Uuid::new_v4());
    session
        .insert(OAUTH_STATE_KEY, &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let url = crate::identity::auth::google_calendar_auth_url(&state.config, &state_token);
    Ok(Redirect::to(&url))
}

/// Clears the session and redirects to the landing page.
pub async fn logout(session: Session) -> Result<Redirect, AppError> {
    auth_mw::clear_session(&session).await?;
    Ok(Redirect::to("/"))
}

// ── Settings routes ──

/// Renders the settings page: profile, role, calendar preferences.
pub async fn settings_page(
    auth: auth_mw::AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    let clg_levels: Vec<serde_json::Value> = crate::identity::clg::all_levels()
        .iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "title": l.title,
            })
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("clg_levels", &clg_levels);
    ctx.insert("current_page", "settings");

    let html = state.templates.render("pages/settings.html", &ctx)?;
    Ok(Html(html))
}

/// Form payload for user settings (CLG role, promotion flag, and profile fields).
#[derive(serde::Deserialize)]
pub struct SaveSettingsForm {
    pub role: Option<String>,
    #[serde(default)]
    pub wants_promotion: Option<String>, // "on" or absent
    pub display_name: Option<String>,
    pub team: Option<String>,
    pub manager_name: Option<String>,
    pub skip_level_name: Option<String>,
    pub direct_reports: Option<String>,
    pub timezone: Option<String>,
    pub week_start: Option<String>,
    pub work_start_time: Option<String>,
    pub work_end_time: Option<String>,
}

/// HTMX handler: persists the user's settings (role, promotion, profile), then redirects.
pub async fn save_settings(
    auth: auth_mw::AuthUser,
    State(state): State<AppState>,
    Form(input): Form<SaveSettingsForm>,
) -> Result<impl IntoResponse, AppError> {
    let role = input.role.as_deref().filter(|s| !s.is_empty());
    let wants_promotion = input.wants_promotion.is_some();

    User::update_settings(&state.db, auth.user_id, role, wants_promotion).await?;
    User::update_profile(
        &state.db,
        auth.user_id,
        &ProfileUpdate {
            display_name: input.display_name.as_deref().filter(|s| !s.is_empty()),
            team: input.team.as_deref().filter(|s| !s.is_empty()),
            manager_name: input.manager_name.as_deref().filter(|s| !s.is_empty()),
            skip_level_name: input.skip_level_name.as_deref().filter(|s| !s.is_empty()),
            direct_reports: input.direct_reports.as_deref().filter(|s| !s.is_empty()),
            timezone: input.timezone.as_deref().filter(|s| !s.is_empty()),
            week_start: input.week_start.as_deref().filter(|s| !s.is_empty()),
            work_start_time: input.work_start_time.as_deref().filter(|s| !s.is_empty()),
            work_end_time: input.work_end_time.as_deref().filter(|s| !s.is_empty()),
        },
    )
    .await?;

    Ok((
        [(header::HeaderName::from_static("hx-redirect"), "/settings")],
        "",
    ))
}

/// Renders the CLG (Career Level Guide) reference page with current and next level details.
pub async fn clg_guide_page(
    auth: auth_mw::AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    let current_level = user
        .role
        .as_deref()
        .and_then(|r| crate::identity::clg::get_level(r));
    let next_level = user
        .role
        .as_deref()
        .and_then(|r| crate::identity::clg::get_next_level(r));

    let all_levels: Vec<serde_json::Value> = crate::identity::clg::all_levels()
        .iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id,
                "title": l.title,
            })
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("current_level", &current_level);
    ctx.insert("next_level", &next_level);
    ctx.insert("all_levels", &all_levels);
    ctx.insert("current_page", "clg_guide");

    let html = state.templates.render("pages/clg_guide.html", &ctx)?;
    Ok(Html(html))
}
