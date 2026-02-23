use axum::{
    Form,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::Html,
};

use crate::AppState;
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::review::model::BragPhase;
use crate::shared::error::AppError;
use crate::sync::get_sync_service;
use crate::sync::model::{IntegrationConfig, SyncLog};

/// Renders the integrations page: service connections, sync logs, entry source counts.
pub async fn integrations_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    let configs = IntegrationConfig::list_for_user(&state.db, auth.user_id).await?;

    let mut services: Vec<serde_json::Value> = crate::sync::model::SERVICES
        .iter()
        .map(|&service| {
            let config = configs.iter().find(|c| c.service == service);

            let config_values: serde_json::Map<String, serde_json::Value> = config
                .and_then(|c| c.config_json.as_deref())
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            let fields: Vec<serde_json::Value> = service_config_fields(service, &user.email)
                .into_iter()
                .map(|mut field| {
                    let name = field["name"].as_str().unwrap_or_default().to_string();
                    if let Some(saved) = config_values.get(&name) {
                        field["value"] = saved.clone();
                    } else {
                        field["value"] = field["default_value"].clone();
                    }
                    field
                })
                .collect();

            let has_token = config.map(|c| c.encrypted_token.is_some()).unwrap_or(false);

            let excluded_files: Vec<serde_json::Value> = if service == "google_drive" {
                config_values
                    .get("excluded_files")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| serde_json::Value::Object(v.as_object().cloned().unwrap_or_default()))
                    .collect()
            } else {
                vec![]
            };

            let excluded_events: Vec<serde_json::Value> = if service == "google_calendar" {
                config_values
                    .get("excluded_events")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| serde_json::Value::Object(v.as_object().cloned().unwrap_or_default()))
                    .collect()
            } else {
                vec![]
            };

            serde_json::json!({
                "name": service,
                "display_name": service_display_name(service),
                "is_enabled": config.map(|c| c.is_enabled).unwrap_or(false),
                "has_token": has_token,
                "last_sync_at": config.and_then(|c| c.last_sync_at.clone()),
                "last_sync_status": config.and_then(|c| c.last_sync_status.clone()),
                "last_sync_error": config.and_then(|c| c.last_sync_error.clone()),
                "config_fields": fields,
                "token_only": service == "claude",
                "oauth_connect": service == "google_drive" || service == "google_calendar",
                "token_url": service_token_url(service),
                "note": service_note(service),
                "excluded_files": excluded_files,
                "excluded_events": excluded_events,
            })
        })
        .collect();

    services.sort_by(|a, b| {
        let a_connected = a["has_token"].as_bool().unwrap_or(false);
        let b_connected = b["has_token"].as_bool().unwrap_or(false);
        b_connected.cmp(&a_connected)
    });

    let source_counts: Vec<(String, i64)> = if let Some(ref p) = phase {
        BragEntry::source_counts_for_phase(&state.db, p.id)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    let sync_logs = SyncLog::recent_for_user(&state.db, auth.user_id, 10)
        .await
        .unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("services", &services);
    ctx.insert("source_counts", &source_counts);
    ctx.insert("sync_logs", &sync_logs);
    ctx.insert("current_page", "integrations");
    ctx.insert("public_only", &state.config.public_only);

    let html = state.templates.render("pages/integrations.html", &ctx)?;
    Ok(Html(html))
}

// Minimal HTML entity escaping for dynamic strings injected into HTML responses.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Form payload for saving or testing an integration. Fields are a superset of all services.
#[derive(serde::Deserialize)]
pub struct IntegrationForm {
    pub token: Option<String>,
    // Dynamic config fields
    pub username: Option<String>,
    pub email: Option<String>,
    pub org: Option<String>,
    pub base_url: Option<String>,
    pub token_expires: Option<String>,
    // Sync setting checkboxes (HTML sends "on" or absent)
    pub sync_pr_authored: Option<String>,
    pub sync_pr_merged: Option<String>,
    pub sync_pr_reviewed: Option<String>,
    pub sync_jira_completed: Option<String>,
    pub sync_jira_in_progress: Option<String>,
    pub sync_jira_created: Option<String>,
    pub sync_pr_development: Option<String>,
}

/// Return the checkbox field names relevant to a given service.
fn checkbox_fields_for_service(service: &str) -> &[&str] {
    match service {
        "github" => &[
            "sync_pr_authored",
            "sync_pr_merged",
            "sync_pr_reviewed",
            "sync_pr_development",
        ],
        "atlassian" => &[
            "sync_jira_completed",
            "sync_jira_in_progress",
            "sync_jira_created",
        ],
        _ => &[],
    }
}

/// Look up a checkbox field value from the form by name.
fn checkbox_value<'a>(input: &'a IntegrationForm, name: &str) -> Option<&'a Option<String>> {
    match name {
        "sync_pr_authored" => Some(&input.sync_pr_authored),
        "sync_pr_merged" => Some(&input.sync_pr_merged),
        "sync_pr_reviewed" => Some(&input.sync_pr_reviewed),
        "sync_jira_completed" => Some(&input.sync_jira_completed),
        "sync_jira_in_progress" => Some(&input.sync_jira_in_progress),
        "sync_jira_created" => Some(&input.sync_jira_created),
        "sync_pr_development" => Some(&input.sync_pr_development),
        _ => None,
    }
}

/// Insert non-empty form fields into a fresh JSON config map.
fn build_config_json(input: &IntegrationForm, service: &str) -> Option<String> {
    let mut config = serde_json::Map::new();
    let fields: &[(&str, &Option<String>)] = &[
        ("username", &input.username),
        ("email", &input.email),
        ("org", &input.org),
        ("base_url", &input.base_url),
        ("token_expires", &input.token_expires),
    ];
    for &(key, val) in fields {
        if let Some(v) = val
            && !v.is_empty()
        {
            config.insert(key.to_string(), serde_json::Value::String(v.clone()));
        }
    }
    // Store checkbox fields as booleans, scoped to the service
    for &name in checkbox_fields_for_service(service) {
        if let Some(val) = checkbox_value(input, name) {
            let checked = val.is_some();
            config.insert(name.to_string(), serde_json::Value::Bool(checked));
        }
    }
    if config.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(config).to_string())
    }
}

/// Merge non-empty form fields into an existing JSON config (for test_connection).
fn merge_form_into_config(
    mut saved: serde_json::Value,
    input: &IntegrationForm,
    service: &str,
) -> serde_json::Value {
    let fields: &[(&str, &Option<String>)] = &[
        ("username", &input.username),
        ("email", &input.email),
        ("org", &input.org),
        ("base_url", &input.base_url),
    ];
    for &(key, val) in fields {
        if let Some(v) = val
            && !v.is_empty()
        {
            saved[key] = serde_json::Value::String(v.clone());
        }
    }
    for &name in checkbox_fields_for_service(service) {
        if let Some(val) = checkbox_value(input, name) {
            saved[name] = serde_json::Value::Bool(val.is_some());
        }
    }
    saved
}

/// HTMX handler: tests the connection first (for non-token-only services), then upserts
/// the integration config (encrypts token) and enables it. Returns inline feedback.
pub async fn save_integration(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Form(input): Form<IntegrationForm>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let is_token_only = service == "claude";

    // For non-token-only services, test the connection before saving
    if !is_token_only {
        let saved_config =
            IntegrationConfig::find_by_service(&state.db, auth.user_id, &service).await?;

        // Resolve token: form input > saved encrypted token > empty (if not required)
        let token = match &input.token {
            Some(t) if !t.is_empty() => t.clone(),
            _ => match saved_config
                .as_ref()
                .and_then(|c| c.encrypted_token.as_ref())
            {
                Some(encrypted) => auth.crypto.decrypt(encrypted)?,
                None => {
                    if crate::sync::service_requires_token(&service) {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            "HX-Retarget",
                            HeaderValue::from_str(&format!("#save-status-{}", service))
                                .unwrap_or(HeaderValue::from_static("#save-status")),
                        );
                        headers.insert("HX-Reswap", HeaderValue::from_static("innerHTML"));
                        return Ok((
                            headers,
                            Html(
                                r#"<span class="status-error">No token provided</span>"#
                                    .to_string(),
                            ),
                        ));
                    }
                    String::new()
                }
            },
        };

        // Build config from form fields, falling back to saved values
        let saved_json: serde_json::Value = saved_config
            .as_ref()
            .and_then(|c| c.config_json.as_deref())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let service_config = merge_form_into_config(saved_json, &input, &service);

        let sync_service = get_sync_service(&service, Some(&state.config))
            .ok_or_else(|| AppError::BadRequest(format!("Unknown service: {}", service)))?;

        let http_client = crate::sync::http_client()?;
        let status = sync_service
            .test_connection(&http_client, &token, &service_config)
            .await?;

        if !status.connected {
            let error = html_escape(&status.error.unwrap_or_else(|| "Unknown error".to_string()));
            let mut headers = HeaderMap::new();
            headers.insert(
                "HX-Retarget",
                HeaderValue::from_str(&format!("#save-status-{}", service))
                    .unwrap_or(HeaderValue::from_static("#save-status")),
            );
            headers.insert("HX-Reswap", HeaderValue::from_static("innerHTML"));
            return Ok((
                headers,
                Html(format!(
                    r#"<span class="status-error">Connection failed: {}</span>"#,
                    error
                )),
            ));
        }
    }

    // Connection test passed (or token-only) — proceed to save
    let is_enabled = true;

    let encrypted_token = match &input.token {
        Some(token) if !token.is_empty() => {
            tracing::info!(user_id = auth.user_id, service = %service, action = "save", "Token encrypted for integration");
            Some(auth.crypto.encrypt(token)?)
        }
        _ => None,
    };

    let config_json = build_config_json(&input, &service);

    IntegrationConfig::upsert(
        &state.db,
        auth.user_id,
        &service,
        is_enabled,
        encrypted_token.as_deref(),
        config_json.as_deref(),
    )
    .await?;

    // Return success feedback inline (no page reload).
    // The HX-Trigger header tells the client to auto-sync this service.
    let mut headers = HeaderMap::new();
    headers.insert(
        "HX-Retarget",
        HeaderValue::from_str(&format!("#save-status-{}", service))
            .unwrap_or(HeaderValue::from_static("#save-status")),
    );
    headers.insert("HX-Reswap", HeaderValue::from_static("innerHTML"));
    if !is_token_only {
        headers.insert(
            "HX-Trigger",
            HeaderValue::from_str(&format!(r#"{{"integrationSaved":"{}"}}"#, service))
                .unwrap_or(HeaderValue::from_static("{}")),
        );
    }
    let message = if is_token_only {
        r#"<span class="status-success">Saved</span>"#
    } else {
        r#"<span class="status-success">Saved — syncing now…</span>"#
    };
    Ok((headers, Html(message.to_string())))
}

/// HTMX handler: tests a service connection using form or saved token. Returns a status span.
pub async fn test_connection(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Form(input): Form<IntegrationForm>,
) -> Result<Html<String>, AppError> {
    let saved_config =
        IntegrationConfig::find_by_service(&state.db, auth.user_id, &service).await?;

    // Use token from form if provided, otherwise fall back to saved token.
    // Services that don't require a token (GitHub, Bugzilla) can proceed with empty string.
    let token = match &input.token {
        Some(t) if !t.is_empty() => t.clone(),
        _ => match saved_config
            .as_ref()
            .and_then(|c| c.encrypted_token.as_ref())
        {
            Some(encrypted) => {
                tracing::info!(user_id = auth.user_id, service = %service, action = "test_connection", "Token decrypted for test");
                auth.crypto.decrypt(encrypted)?
            }
            None => {
                if crate::sync::service_requires_token(&service) {
                    return Ok(Html(
                        r#"<span class="status-error">No token provided</span>"#.to_string(),
                    ));
                }
                String::new()
            }
        },
    };

    // Build config from form fields, falling back to saved values
    let saved_json: serde_json::Value = saved_config
        .as_ref()
        .and_then(|c| c.config_json.as_deref())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let service_config = merge_form_into_config(saved_json, &input, &service);

    let sync_service = get_sync_service(&service, Some(&state.config))
        .ok_or_else(|| AppError::BadRequest(format!("Unknown service: {}", service)))?;

    let http_client = crate::sync::http_client()?;
    let status = sync_service
        .test_connection(&http_client, &token, &service_config)
        .await?;

    if status.connected {
        let username = html_escape(&status.username.unwrap_or_default());
        // OOB swap to auto-fill username field if this is GitHub
        let oob = if service == "github" && !username.is_empty() {
            format!(
                r#"<input type="text" id="config-github-username" name="username" class="form-input" placeholder="gruberb" value="{}" hx-swap-oob="outerHTML:#config-github-username">"#,
                username
            )
        } else {
            String::new()
        };
        Ok(Html(format!(
            r#"<span class="status-success">Connected as {}</span>{}"#,
            username, oob
        )))
    } else {
        let error = html_escape(&status.error.unwrap_or_else(|| "Unknown error".to_string()));
        Ok(Html(format!(
            r#"<span class="status-error">Failed: {}</span>"#,
            error
        )))
    }
}

/// Query parameters for the reset endpoint.
#[derive(serde::Deserialize, Default)]
pub struct ResetQuery {
    #[serde(default)]
    pub delete_entries: Option<String>,
}

/// Map a service/integration name to the source names used in `brag_entries.source`.
fn service_to_sources(service: &str) -> Vec<&str> {
    match service {
        "atlassian" => vec!["jira", "confluence"],
        "google_drive" => vec!["google_drive"],
        "google_calendar" => vec!["google_calendar"],
        other => vec![other],
    }
}

/// HTMX handler: deletes an integration config (token + settings) and redirects to settings.
/// With `?delete_entries=true`, also hard-deletes all synced entries for that service.
pub async fn reset_integration(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(query): Query<ResetQuery>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let delete_entries = query
        .delete_entries
        .as_deref()
        .is_some_and(|v| v == "true" || v == "1");

    if delete_entries {
        for source in service_to_sources(&service) {
            let _ = BragEntry::hard_delete_all_for_service(&state.db, auth.user_id, source).await;
        }
    }

    IntegrationConfig::delete(&state.db, auth.user_id, &service).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", HeaderValue::from_static("/integrations"));

    Ok((headers, Html(String::new())))
}

/// HTMX handler: removes an event from the Google Calendar exclusion list and un-deletes its entries.
pub async fn restore_excluded_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(event_id): Path<String>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    // Remove from exclusion list
    IntegrationConfig::update_config_json(&state.db, auth.user_id, "google_calendar", |json| {
        if let Some(arr) = json["excluded_events"].as_array_mut() {
            arr.retain(|v| v["event_id"].as_str() != Some(&event_id));
        }
    })
    .await?;

    // Clear soft-deletes so next sync picks them up
    BragEntry::clear_soft_deletes_by_event_id(&state.db, auth.user_id, &event_id).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", HeaderValue::from_static("/integrations"));
    Ok((headers, Html(String::new())))
}

/// HTMX handler: removes a file from the Google Drive exclusion list and un-deletes its entries.
pub async fn restore_excluded_file(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    // Remove from exclusion list
    IntegrationConfig::update_config_json(&state.db, auth.user_id, "google_drive", |json| {
        if let Some(arr) = json["excluded_files"].as_array_mut() {
            arr.retain(|v| v["file_id"].as_str() != Some(&file_id));
        }
    })
    .await?;

    // Clear soft-deletes so next sync picks them up
    BragEntry::clear_soft_deletes_by_file_id(&state.db, auth.user_id, &file_id).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", HeaderValue::from_static("/integrations"));
    Ok((headers, Html(String::new())))
}

// --- Service helper functions (moved from routes/settings.rs) ---

/// Maps a service slug to its human-readable display name.
pub fn service_display_name(service: &str) -> &str {
    match service {
        "github" => "GitHub",
        "phabricator" => "Phabricator",
        "bugzilla" => "Bugzilla",
        "atlassian" => "Atlassian",
        "google_drive" => "Google Drive",
        "google_calendar" => "Google Calendar",
        "claude" => "Claude AI",
        _ => service,
    }
}

/// Returns the URL where users can create an API token for the given service.
pub fn service_token_url(service: &str) -> &str {
    let cfg = crate::sync::services_config::get();
    match service {
        "github" => &cfg.github.token_url,
        "phabricator" => &cfg.phabricator.token_url,
        "bugzilla" => &cfg.bugzilla.token_url,
        "atlassian" => &cfg.atlassian.token_url,
        "claude" => &cfg.claude.token_url,
        "google_drive" => "",
        "google_calendar" => "",
        _ => "",
    }
}

/// Returns a help note shown below the integration card for the given service.
pub fn service_note(service: &str) -> &str {
    let cfg = crate::sync::services_config::get();
    match service {
        "github" => &cfg.github.note,
        "phabricator" => &cfg.phabricator.note,
        "bugzilla" => &cfg.bugzilla.note,
        "atlassian" => &cfg.atlassian.note,
        "google_drive" => {
            "Syncs Google Docs, Sheets, and Slides you created, edited, or commented on"
        }
        "google_calendar" => {
            "Syncs accepted calendar events with 2+ attendees (excludes all-day events)"
        }
        _ => "",
    }
}

/// Returns the form field definitions (name, label, type, defaults) for a service's config UI.
pub fn service_config_fields(service: &str, user_email: &str) -> Vec<serde_json::Value> {
    match service {
        "github" => {
            let gh = &crate::sync::services_config::get().github;
            vec![
                serde_json::json!({"name": "username", "label": "GitHub Username", "placeholder": "auto-detected on Test", "default_value": "", "field_type": "text"}),
                serde_json::json!({"name": "org", "label": "Organizations", "placeholder": gh.org_placeholder, "default_value": gh.default_orgs, "field_type": "text"}),
                serde_json::json!({"name": "sync_pr_authored", "label": "PRs I opened", "default_value": true, "field_type": "checkbox"}),
                serde_json::json!({"name": "sync_pr_merged", "label": "PRs that got merged", "default_value": true, "field_type": "checkbox"}),
                serde_json::json!({"name": "sync_pr_reviewed", "label": "PRs I reviewed", "default_value": true, "field_type": "checkbox"}),
                serde_json::json!({"name": "sync_pr_development", "label": "Daily commit activity on my PRs", "default_value": true, "field_type": "checkbox"}),
            ]
        }
        "phabricator" => {
            let phab = &crate::sync::services_config::get().phabricator;
            vec![
                serde_json::json!({"name": "base_url", "label": "Base URL", "placeholder": phab.base_url_placeholder, "default_value": phab.default_base_url, "field_type": "text"}),
            ]
        }
        "bugzilla" => {
            let bz = &crate::sync::services_config::get().bugzilla;
            vec![
                serde_json::json!({"name": "email", "label": "Bugzilla Email", "placeholder": bz.email_placeholder, "default_value": user_email, "field_type": "text"}),
                serde_json::json!({"name": "base_url", "label": "Base URL", "placeholder": bz.base_url_placeholder, "default_value": bz.default_base_url, "field_type": "text"}),
            ]
        }
        "atlassian" => {
            let atl = &crate::sync::services_config::get().atlassian;
            vec![
                serde_json::json!({"name": "email", "label": "Atlassian Email", "placeholder": atl.email_placeholder, "default_value": user_email, "field_type": "text"}),
                serde_json::json!({"name": "base_url", "label": "Base URL", "placeholder": atl.base_url_placeholder, "default_value": atl.default_base_url, "field_type": "text"}),
                serde_json::json!({"name": "token_expires", "label": "Token expires (optional)", "placeholder": "2026-06-01", "default_value": "", "field_type": "text"}),
                serde_json::json!({"name": "sync_jira_completed", "label": "Completed issues", "default_value": true, "field_type": "checkbox"}),
                serde_json::json!({"name": "sync_jira_in_progress", "label": "In-progress issues", "default_value": false, "field_type": "checkbox"}),
                serde_json::json!({"name": "sync_jira_created", "label": "All created/assigned issues", "default_value": false, "field_type": "checkbox"}),
            ]
        }
        "claude" => vec![],
        "google_drive" => vec![],
        "google_calendar" => vec![],
        _ => vec![],
    }
}
