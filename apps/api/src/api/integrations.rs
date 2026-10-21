//! Integration settings: status, configuration, connection test, reset and exclusion restore.
//! Reuses the helpers of the legacy HTML handlers so both surfaces save identical config.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::identity::model::User;
use crate::integrations::get_sync_service;
use crate::integrations::catalog::{
    IntegrationForm, build_config_json, merge_form_into_config, service_config_fields,
    service_display_name, service_note, service_to_sources, service_token_url,
};
use crate::integrations::model::{IntegrationConfig, SERVICES};
use crate::kernel::error::AppError;
use crate::worklog::model::BragEntry;

/// One field of a service's settings form.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConfigField {
    pub name: String,
    pub label: String,
    /// `text` or `checkbox`.
    pub field_type: String,
    pub placeholder: Option<String>,
    #[ts(type = "string | boolean")]
    pub default_value: serde_json::Value,
    /// The saved value, or the default when nothing was saved.
    #[ts(type = "string | boolean")]
    pub value: serde_json::Value,
}

/// A Drive file or calendar event the user removed from syncing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExcludedItem {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IntegrationDto {
    pub service: String,
    pub display_name: String,
    pub is_enabled: bool,
    pub has_token: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_sync_error: Option<String>,
    pub token_only: bool,
    /// Connected through Google OAuth instead of a pasted token.
    pub oauth_connect: bool,
    pub token_url: String,
    pub note: String,
    pub fields: Vec<ConfigField>,
    pub excluded_files: Vec<ExcludedItem>,
    pub excluded_events: Vec<ExcludedItem>,
    /// Non-deleted entries in the active cycle that this service produced.
    pub entry_count: i64,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct IntegrationsResponse {
    pub services: Vec<IntegrationDto>,
}

/// Body of `PUT` and `POST .../test`. Every key is optional; checkboxes are real booleans.
#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct IntegrationInput {
    pub token: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub org: Option<String>,
    pub base_url: Option<String>,
    pub token_expires: Option<String>,
    pub sync_pr_authored: Option<bool>,
    pub sync_pr_merged: Option<bool>,
    pub sync_pr_reviewed: Option<bool>,
    pub sync_pr_review_requested: Option<bool>,
    pub sync_pr_development: Option<bool>,
    pub sync_jira_completed: Option<bool>,
    pub sync_jira_in_progress: Option<bool>,
    pub sync_jira_created: Option<bool>,
    pub sync_jira_progress: Option<bool>,
}

impl IntegrationInput {
    /// The legacy form encodes a checked box as `Some("on")` and an unchecked one as absent.
    fn into_form(self) -> IntegrationForm {
        let on = |b: Option<bool>| b.filter(|v| *v).map(|_| "on".to_string());
        IntegrationForm {
            token: self.token,
            username: self.username,
            email: self.email,
            org: self.org,
            base_url: self.base_url,
            token_expires: self.token_expires,
            sync_pr_authored: on(self.sync_pr_authored),
            sync_pr_merged: on(self.sync_pr_merged),
            sync_pr_reviewed: on(self.sync_pr_reviewed),
            sync_pr_review_requested: on(self.sync_pr_review_requested),
            sync_jira_completed: on(self.sync_jira_completed),
            sync_jira_in_progress: on(self.sync_jira_in_progress),
            sync_jira_created: on(self.sync_jira_created),
            sync_jira_progress: on(self.sync_jira_progress),
            sync_pr_development: on(self.sync_pr_development),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Deserialize, Default)]
pub struct ResetQuery {
    #[serde(default)]
    pub delete_entries: bool,
}

fn known_service(service: &str) -> Result<&'static str, ApiError> {
    SERVICES
        .iter()
        .copied()
        .find(|s| *s == service)
        .ok_or_else(|| ApiError::not_found(format!("Unknown service: {service}")))
}

/// Form field definitions with saved values merged in.
pub fn config_fields(
    service: &str,
    user_email: &str,
    saved: &serde_json::Map<String, serde_json::Value>,
) -> Vec<ConfigField> {
    service_config_fields(service, user_email)
        .into_iter()
        .map(|f| {
            let text = |k: &str| f[k].as_str().unwrap_or_default().to_string();
            let name = text("name");
            let default_value = f["default_value"].clone();
            ConfigField {
                value: saved
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| default_value.clone()),
                label: text("label"),
                field_type: text("field_type"),
                placeholder: f["placeholder"].as_str().map(str::to_string),
                default_value,
                name,
            }
        })
        .collect()
}

fn excluded(
    saved: &serde_json::Map<String, serde_json::Value>,
    list_key: &str,
    id_key: &str,
) -> Vec<ExcludedItem> {
    saved
        .get(list_key)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            Some(ExcludedItem {
                id: v[id_key].as_str()?.to_string(),
                title: v["title"].as_str().map(str::to_string),
            })
        })
        .collect()
}

fn to_dto(
    service: &str,
    config: Option<&IntegrationConfig>,
    user_email: &str,
    entry_count: i64,
) -> IntegrationDto {
    let saved: serde_json::Map<String, serde_json::Value> = config
        .and_then(|c| c.config_json.as_deref())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let oauth = service == "google_drive" || service == "google_calendar";
    IntegrationDto {
        service: service.to_string(),
        display_name: service_display_name(service).to_string(),
        is_enabled: config.is_some_and(|c| c.is_enabled),
        has_token: config.is_some_and(|c| c.encrypted_token.is_some()),
        last_sync_at: config.and_then(|c| c.last_sync_at.clone()),
        last_sync_status: config.and_then(|c| c.last_sync_status.clone()),
        last_sync_error: config.and_then(|c| c.last_sync_error.clone()),
        token_only: service == "claude",
        oauth_connect: oauth,
        token_url: service_token_url(service).to_string(),
        note: service_note(service).to_string(),
        fields: config_fields(service, user_email, &saved),
        excluded_files: if service == "google_drive" {
            excluded(&saved, "excluded_files", "file_id")
        } else {
            vec![]
        },
        excluded_events: if service == "google_calendar" {
            excluded(&saved, "excluded_events", "event_id")
        } else {
            vec![]
        },
        entry_count,
    }
}

/// Entries per service in the active cycle (Atlassian sums Jira and Confluence).
async fn entry_counts(state: &AppState, user_id: i64) -> Result<HashMap<String, i64>, AppError> {
    let Some(phase) = crate::cycle::model::BragPhase::get_active(&state.db, user_id).await? else {
        return Ok(HashMap::new());
    };
    let by_source: HashMap<String, i64> = BragEntry::source_counts_for_phase(&state.db, phase.id)
        .await?
        .into_iter()
        .collect();
    Ok(SERVICES
        .iter()
        .map(|s| {
            let n = service_to_sources(s)
                .iter()
                .map(|src| by_source.get(*src).copied().unwrap_or(0))
                .sum();
            (s.to_string(), n)
        })
        .collect())
}

async fn load_user(state: &AppState, user_id: i64) -> Result<User, ApiError> {
    User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))
}

async fn one_dto(
    state: &AppState,
    user_id: i64,
    service: &str,
) -> Result<IntegrationDto, ApiError> {
    let user = load_user(state, user_id).await?;
    let config = IntegrationConfig::find_by_service(&state.db, user_id, service).await?;
    let counts = entry_counts(state, user_id).await?;
    Ok(to_dto(
        service,
        config.as_ref(),
        &user.email,
        counts.get(service).copied().unwrap_or(0),
    ))
}

/// `GET /api/v1/integrations`: connected services first, like the legacy page.
pub async fn list(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<IntegrationsResponse> {
    let user = load_user(&state, auth.user_id).await?;
    let configs = IntegrationConfig::list_for_user(&state.db, auth.user_id).await?;
    let counts = entry_counts(&state, auth.user_id).await?;

    let mut services: Vec<IntegrationDto> = SERVICES
        .iter()
        .map(|&s| {
            let config = configs.iter().find(|c| c.service == s);
            to_dto(s, config, &user.email, counts.get(s).copied().unwrap_or(0))
        })
        .collect();
    // Stable sort keeps the declared order inside each group.
    services.sort_by_key(|s| !s.has_token);
    Ok(Json(IntegrationsResponse { services }))
}

/// Token from the request, else the saved one, else empty for services that need none.
async fn resolve_token(
    auth: &crate::identity::auth::middleware::AuthUser,
    service: &str,
    input_token: Option<&str>,
    saved: Option<&IntegrationConfig>,
) -> Result<String, ApiError> {
    if let Some(t) = input_token.filter(|t| !t.is_empty()) {
        return Ok(t.to_string());
    }
    match saved.and_then(|c| c.encrypted_token.as_ref()) {
        Some(enc) => Ok(auth.crypto.decrypt(enc)?),
        None if crate::integrations::service_requires_token(service) => {
            Err(ApiError::invalid("No token provided"))
        }
        None => Ok(String::new()),
    }
}

/// Runs the service's connection test with the request's values layered over the saved ones.
async fn check_connection(
    state: &AppState,
    auth: &crate::identity::auth::middleware::AuthUser,
    service: &str,
    form: &IntegrationForm,
) -> Result<crate::integrations::ConnectionStatus, ApiError> {
    let saved = IntegrationConfig::find_by_service(&state.db, auth.user_id, service).await?;
    let token = resolve_token(auth, service, form.token.as_deref(), saved.as_ref()).await?;
    let saved_json: serde_json::Value = saved
        .as_ref()
        .and_then(|c| c.config_json.as_deref())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let merged = merge_form_into_config(saved_json, form, service);

    let sync_service = get_sync_service(service, Some(&state.config))
        .ok_or_else(|| AppError::BadRequest(format!("Nothing to test for {service}")))?;
    let http = crate::integrations::http_client().map_err(AppError::HttpClient)?;
    Ok(sync_service.test_connection(&http, &token, &merged).await?)
}

/// `PUT /api/v1/integrations/{service}`: tests the connection (except token-only services),
/// then stores the encrypted token and config and enables the service. The client starts the
/// first sync with `POST /sync/{service}`.
pub async fn save(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Json(input): Json<IntegrationInput>,
) -> ApiResult<IntegrationDto> {
    let service = known_service(&service)?;
    let form = input.into_form();

    if service != "claude" {
        let status = check_connection(&state, &auth, service, &form).await?;
        if !status.connected {
            return Err(ApiError::invalid(format!(
                "Connection failed: {}",
                status.error.unwrap_or_else(|| "Unknown error".to_string())
            )));
        }
    }

    let encrypted_token = match form.token.as_deref().filter(|t| !t.is_empty()) {
        Some(t) => {
            tracing::info!(
                user_id = auth.user_id,
                service,
                action = "save",
                "Token encrypted for integration"
            );
            Some(auth.crypto.encrypt(t)?)
        }
        None => None,
    };
    IntegrationConfig::upsert(
        &state.db,
        auth.user_id,
        service,
        true,
        encrypted_token.as_deref(),
        build_config_json(&form, service).as_deref(),
    )
    .await?;

    Ok(Json(one_dto(&state, auth.user_id, service).await?))
}

/// `POST /api/v1/integrations/{service}/test`. A failed connection is a 200 with `ok: false`;
/// only a missing token or unknown service is an error.
pub async fn test(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Json(input): Json<IntegrationInput>,
) -> ApiResult<TestResult> {
    let service = known_service(&service)?;
    let status = check_connection(&state, &auth, service, &input.into_form()).await?;
    Ok(Json(if status.connected {
        TestResult {
            ok: true,
            message: format!("Connected as {}", status.username.unwrap_or_default()),
        }
    } else {
        TestResult {
            ok: false,
            message: status.error.unwrap_or_else(|| "Unknown error".to_string()),
        }
    }))
}

/// `DELETE /api/v1/integrations/{service}?delete_entries=true`
pub async fn reset(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(q): Query<ResetQuery>,
) -> Result<StatusCode, ApiError> {
    let service = known_service(&service)?;
    if q.delete_entries {
        for source in service_to_sources(service) {
            BragEntry::hard_delete_all_for_service(&state.db, auth.user_id, source).await?;
        }
    }
    IntegrationConfig::delete(&state.db, auth.user_id, service).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/v1/integrations/{service}/excluded/{id}`: stops excluding a Drive file or
/// calendar event and un-deletes its entries so the next sync picks them up.
pub async fn restore_excluded(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path((service, id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    match service.as_str() {
        "google_drive" => {
            IntegrationConfig::update_config_json(&state.db, auth.user_id, "google_drive", |j| {
                if let Some(arr) = j["excluded_files"].as_array_mut() {
                    arr.retain(|v| v["file_id"].as_str() != Some(&id));
                }
            })
            .await?;
            BragEntry::clear_soft_deletes_by_file_id(&state.db, auth.user_id, &id).await?;
        }
        "google_calendar" => {
            IntegrationConfig::update_config_json(
                &state.db,
                auth.user_id,
                "google_calendar",
                |j| {
                    if let Some(arr) = j["excluded_events"].as_array_mut() {
                        arr.retain(|v| v["event_id"].as_str() != Some(&id));
                    }
                },
            )
            .await?;
            BragEntry::clear_soft_deletes_by_event_id(&state.db, auth.user_id, &id).await?;
        }
        other => {
            return Err(ApiError::invalid(format!(
                "{other} has no excluded items to restore"
            )));
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
