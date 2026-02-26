use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;

use super::{ConnectionStatus, SyncService, SyncedEntry};
use crate::kernel::error::AppError;

/// Syncs Google Workspace file activity (Docs, Sheets, Slides, Forms) via the
/// Drive Activity API. The stored token is a refresh token; each sync/test call
/// exchanges it for a short-lived access token first.
pub struct GoogleDriveSync {
    pub client_id: String,
    pub client_secret: String,
}

/// Top-level response from `POST driveactivity.googleapis.com/v2/activity:query`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityQueryResponse {
    #[serde(default)]
    activities: Vec<DriveActivity>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveActivity {
    #[serde(default)]
    targets: Vec<Target>,
    #[serde(default)]
    actions: Vec<Action>,
    #[serde(default)]
    actors: Vec<Actor>,
    timestamp: Option<String>,
    #[serde(default)]
    time_range: Option<TimeRange>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    drive_item: Option<DriveItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveItem {
    name: Option<String>,
    title: Option<String>,
    mime_type: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    drive_file: Option<serde_json::Value>,
    #[serde(default)]
    drive_folder: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Action {
    detail: Option<ActionDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionDetail {
    create: Option<serde_json::Value>,
    edit: Option<serde_json::Value>,
    comment: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Actor {
    user: Option<UserActor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserActor {
    known_user: Option<KnownUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnownUser {
    is_current_user: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeRange {
    end_time: Option<String>,
}

/// Google Workspace MIME types we care about (excludes folders, PDFs, images, etc.).
const WORKSPACE_MIME_TYPES: &[&str] = &[
    "application/vnd.google-apps.document",
    "application/vnd.google-apps.spreadsheet",
    "application/vnd.google-apps.presentation",
    "application/vnd.google-apps.form",
];

/// Determine entry type from action detail.
fn action_to_entry_type(action: &ActionDetail) -> Option<&'static str> {
    if action.create.is_some() {
        Some("drive_created")
    } else if action.edit.is_some() {
        Some("drive_edited")
    } else if action.comment.is_some() {
        Some("drive_commented")
    } else {
        None
    }
}

/// Priority for merging multiple actions on the same file+day.
/// Higher = wins when deduplicating. "edited" is most meaningful.
fn entry_type_priority(entry_type: &str) -> u8 {
    match entry_type {
        "drive_edited" => 3,
        "drive_commented" => 2,
        "drive_created" => 1,
        _ => 0,
    }
}

/// Extract the file ID from the Drive Activity API `name` field (format: `items/ABCDEF`).
fn extract_file_id(name: &str) -> &str {
    name.strip_prefix("items/").unwrap_or(name)
}

/// Extract a date string (YYYY-MM-DD) from an RFC 3339 timestamp.
fn timestamp_to_date(ts: &str) -> Option<String> {
    ts.split('T').next().map(|s| s.to_string())
}

#[async_trait]
impl SyncService for GoogleDriveSync {
    async fn sync(
        &self,
        _client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError> {
        // Build exclusion set from config
        let excluded_file_ids: std::collections::HashSet<String> =
            crate::integrations::model::IntegrationConfig::excluded_drive_file_ids(config)
                .into_iter()
                .collect();

        // token is the refresh token — exchange for access token
        let access_token = crate::identity::auth::refresh_access_token(
            &self.client_id,
            &self.client_secret,
            token,
        )
        .await?;

        let http = super::http_client()?;
        // Keyed by (file_id, date) — one entry per file per day
        let mut entry_map: std::collections::HashMap<(String, String), SyncedEntry> =
            std::collections::HashMap::new();
        let mut page_token: Option<String> = None;
        let mut total_fetched = 0usize;
        let max_activities = 1000;

        // RFC 3339 timestamps for the filter
        let start_rfc3339 = format!("{}T00:00:00Z", start_date);
        let end_rfc3339 = format!("{}T23:59:59Z", end_date);
        let filter = format!(
            "time >= \"{}\" AND time <= \"{}\"",
            start_rfc3339, end_rfc3339
        );

        loop {
            let mut body = serde_json::json!({
                "filter": filter,
                "consolidationStrategy": { "none": {} },
                "pageSize": 100,
            });
            if let Some(ref pt) = page_token {
                body["pageToken"] = serde_json::Value::String(pt.clone());
            }

            let resp = http
                .post("https://driveactivity.googleapis.com/v2/activity:query")
                .bearer_auth(&access_token)
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let message =
                    extract_api_error(&text).unwrap_or_else(|| format!("HTTP {}", status));
                return Err(AppError::Internal(format!(
                    "Drive Activity API: {}",
                    message
                )));
            }

            let result: ActivityQueryResponse = resp.json().await?;

            for activity in &result.activities {
                // Only include activities by the current user
                let is_current_user = activity.actors.iter().any(|a| {
                    a.user
                        .as_ref()
                        .and_then(|u| u.known_user.as_ref())
                        .and_then(|k| k.is_current_user)
                        .unwrap_or(false)
                });
                if !is_current_user {
                    continue;
                }

                // Get timestamp
                let ts = activity
                    .timestamp
                    .as_deref()
                    .or_else(|| {
                        activity
                            .time_range
                            .as_ref()
                            .and_then(|tr| tr.end_time.as_deref())
                    })
                    .unwrap_or("");
                let date = match timestamp_to_date(ts) {
                    Some(d) => d,
                    None => continue,
                };

                // Process each action
                for action in &activity.actions {
                    let detail = match &action.detail {
                        Some(d) => d,
                        None => continue,
                    };
                    let entry_type = match action_to_entry_type(detail) {
                        Some(et) => et,
                        None => continue,
                    };

                    // Process each target
                    for target in &activity.targets {
                        let item = match &target.drive_item {
                            Some(i) => i,
                            None => continue,
                        };

                        // Skip folders
                        if item.drive_folder.is_some() {
                            continue;
                        }

                        // Only include Google Workspace MIME types
                        let mime = item.mime_type.as_deref().unwrap_or("");
                        if !WORKSPACE_MIME_TYPES.contains(&mime) {
                            continue;
                        }

                        let file_id = item.name.as_deref().map(extract_file_id).unwrap_or("");
                        if file_id.is_empty() {
                            continue;
                        }

                        // Skip excluded files
                        if excluded_file_ids.contains(file_id) {
                            continue;
                        }

                        let title = item.title.as_deref().unwrap_or("Untitled");
                        let source_url = format!("https://drive.google.com/file/d/{}", file_id);

                        // Dedup key: one entry per file per day
                        let map_key = (file_id.to_string(), date.clone());
                        let source_id = format!("drive:{}:{}", file_id, date);

                        if let Some(existing) = entry_map.get_mut(&map_key) {
                            // Upgrade entry_type if this action has higher priority
                            if entry_type_priority(entry_type)
                                > entry_type_priority(existing.entry_type)
                            {
                                existing.entry_type = entry_type;
                            }
                            // Always take the latest non-"Untitled" title
                            if title != "Untitled" {
                                existing.title = title.to_string();
                            }
                        } else {
                            entry_map.insert(
                                map_key,
                                SyncedEntry {
                                    source: "google_drive",
                                    source_id,
                                    source_url: Some(source_url),
                                    title: title.to_string(),
                                    description: Some(mime_label(mime).to_string()),
                                    entry_type,
                                    status: None,
                                    repository: None,
                                    occurred_at: date.clone(),
                                    meeting_role: None,
                                    recurring_group: None,
                                    start_time: None,
                                    end_time: None,
                                    collaborators: None,
                                },
                            );
                        }
                    }
                }
            }

            total_fetched += result.activities.len();
            page_token = result.next_page_token;

            if page_token.is_none() || total_fetched >= max_activities {
                break;
            }
        }

        let entries: Vec<SyncedEntry> = entry_map.into_values().collect();

        Ok(entries)
    }

    async fn test_connection(
        &self,
        _client: &reqwest::Client,
        token: &str,
        _config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError> {
        let access_token = match crate::identity::auth::refresh_access_token(
            &self.client_id,
            &self.client_secret,
            token,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                return Ok(ConnectionStatus {
                    connected: false,
                    username: None,
                    error: Some(format!("Token refresh failed: {}", e)),
                });
            }
        };

        let http = super::http_client()?;

        // Minimal query to verify access
        let resp = http
            .post("https://driveactivity.googleapis.com/v2/activity:query")
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "pageSize": 1,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Ok(ConnectionStatus {
                connected: false,
                username: None,
                error: Some(format!("Drive Activity API returned {}", status)),
            });
        }

        Ok(ConnectionStatus {
            connected: true,
            username: Some("Google Drive".to_string()),
            error: None,
        })
    }
}

/// Extract the human-readable `message` from a Google API JSON error response.
/// Returns `None` if the body isn't valid JSON or doesn't have the expected shape.
fn extract_api_error(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v["error"]["message"].as_str().map(|s| s.to_string())
}

/// Human-readable label for a Google Workspace MIME type.
fn mime_label(mime: &str) -> &'static str {
    match mime {
        "application/vnd.google-apps.document" => "Google Doc",
        "application/vnd.google-apps.spreadsheet" => "Google Sheet",
        "application/vnd.google-apps.presentation" => "Google Slides",
        "application/vnd.google-apps.form" => "Google Form",
        _ => "Google Drive",
    }
}
