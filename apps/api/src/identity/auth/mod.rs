/// Google OAuth 2.0 authentication flow: login redirect URL, token exchange,
/// user-info fetch, and domain-restricted user upsert.
pub mod middleware;

use serde::{Deserialize, Serialize};

use crate::identity::model::User;
use crate::kernel::config::Config;
use crate::kernel::error::AppError;

/// Response from Google's `oauth2.googleapis.com/token` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
}

/// User profile returned by Google's `oauth2/v3/userinfo` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    /// Hosted domain (e.g., `your-company.com`). Used for domain restriction.
    pub hd: Option<String>,
}

/// Builds the Google OAuth consent-screen URL with CSRF `state_token`.
/// When `allowed_domain` is configured, appends `&hd=<domain>` so Google
/// prefers/requires the org account in the account picker.
pub fn google_auth_url(config: &Config, state_token: &str) -> String {
    let hd_param = config
        .allowed_domain
        .as_ref()
        .map(|d| format!("&hd={}", urlencoding::encode(d)))
        .unwrap_or_default();
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&access_type=offline&prompt=consent&state={}{}",
        urlencoding::encode(&config.google_client_id),
        urlencoding::encode(&config.google_redirect_uri),
        urlencoding::encode(state_token),
        hd_param,
    )
}

/// Exchanges an authorization code for Google OAuth tokens.
pub async fn exchange_code(config: &Config, code: &str) -> Result<GoogleTokenResponse, AppError> {
    let client = crate::kernel::http::http_client()?;
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", &config.google_client_id),
            ("client_secret", &config.google_client_secret),
            ("redirect_uri", &config.google_redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::debug!("Google token exchange failed: {}", body);
        return Err(AppError::Internal(
            "OAuth token exchange failed".to_string(),
        ));
    }

    let token_resp: GoogleTokenResponse = resp.json().await?;
    Ok(token_resp)
}

/// Fetches the authenticated user's profile from Google using the access token.
pub async fn get_user_info(access_token: &str) -> Result<GoogleUserInfo, AppError> {
    let client = crate::kernel::http::http_client()?;
    let resp = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::debug!("Google userinfo request failed: {}", body);
        return Err(AppError::Internal(
            "OAuth user info request failed".to_string(),
        ));
    }

    let user_info: GoogleUserInfo = resp.json().await?;
    Ok(user_info)
}

/// Validates domain restriction (if configured) and upserts the user row.
/// Returns `Unauthorized` if the user's hosted domain doesn't match `allowed_domain`.
pub async fn authenticate_user(
    pool: &sqlx::SqlitePool,
    config: &Config,
    user_info: &GoogleUserInfo,
) -> Result<User, AppError> {
    // Check domain restriction if configured
    if let Some(ref allowed_domain) = config.allowed_domain {
        match &user_info.hd {
            Some(hd) if hd == allowed_domain => {}
            _ => {
                return Err(AppError::Unauthorized);
            }
        }
    }

    let user = User::upsert(
        pool,
        &user_info.sub,
        &user_info.email,
        &user_info.name,
        user_info.picture.as_deref(),
    )
    .await?;

    Ok(user)
}

/// Builds a Google OAuth consent-screen URL requesting Drive Activity API read access
/// and Drive read-only access (for supplementary comments fetch).
/// Uses `access_type=offline&prompt=consent` to ensure a refresh token is returned.
pub fn google_drive_auth_url(config: &Config, state_token: &str) -> String {
    let hd_param = config
        .allowed_domain
        .as_ref()
        .map(|d| format!("&hd={}", urlencoding::encode(d)))
        .unwrap_or_default();
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.activity.readonly+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.readonly&access_type=offline&prompt=consent&state={}{}",
        urlencoding::encode(&config.google_client_id),
        urlencoding::encode(&config.google_redirect_uri),
        urlencoding::encode(state_token),
        hd_param,
    )
}

/// Builds a Google OAuth consent-screen URL requesting Calendar events read-only access.
/// Uses `access_type=offline&prompt=consent` to ensure a refresh token is returned.
pub fn google_calendar_auth_url(config: &Config, state_token: &str) -> String {
    let hd_param = config
        .allowed_domain
        .as_ref()
        .map(|d| format!("&hd={}", urlencoding::encode(d)))
        .unwrap_or_default();
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.events.readonly&access_type=offline&prompt=consent&state={}{}",
        urlencoding::encode(&config.google_client_id),
        urlencoding::encode(&config.google_redirect_uri),
        urlencoding::encode(state_token),
        hd_param,
    )
}

/// Exchanges a refresh token for a fresh access token via Google's token endpoint.
pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String, AppError> {
    let client = crate::kernel::http::http_client()?;
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::debug!("Google token refresh failed: {}", body);
        return Err(AppError::Internal(
            "Failed to refresh Google access token".to_string(),
        ));
    }

    let token_resp: GoogleTokenResponse = resp.json().await?;
    Ok(token_resp.access_token)
}
