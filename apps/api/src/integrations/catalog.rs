//! Static facts about each integration service (labels, help text, settings fields) and
//! the helpers that turn a settings payload into the stored config JSON.

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
pub(crate) fn build_config_json(input: &IntegrationForm, service: &str) -> Option<String> {
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
pub(crate) fn merge_form_into_config(
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

/// Map a service/integration name to the source names used in `brag_entries.source`.
pub(crate) fn service_to_sources(service: &str) -> Vec<&str> {
    match service {
        "atlassian" => vec!["jira", "confluence"],
        "google_drive" => vec!["google_drive"],
        "google_calendar" => vec!["google_calendar"],
        other => vec![other],
    }
}

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
    let cfg = crate::integrations::services_config::get();
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
    let cfg = crate::integrations::services_config::get();
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
            let gh = &crate::integrations::services_config::get().github;
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
            let phab = &crate::integrations::services_config::get().phabricator;
            vec![
                serde_json::json!({"name": "base_url", "label": "Base URL", "placeholder": phab.base_url_placeholder, "default_value": phab.default_base_url, "field_type": "text"}),
            ]
        }
        "bugzilla" => {
            let bz = &crate::integrations::services_config::get().bugzilla;
            vec![
                serde_json::json!({"name": "email", "label": "Bugzilla Email", "placeholder": bz.email_placeholder, "default_value": user_email, "field_type": "text"}),
                serde_json::json!({"name": "base_url", "label": "Base URL", "placeholder": bz.base_url_placeholder, "default_value": bz.default_base_url, "field_type": "text"}),
            ]
        }
        "atlassian" => {
            let atl = &crate::integrations::services_config::get().atlassian;
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
