//! Domain types for the sync bounded context.
//! No SQL lives here — all persistence is in `repo.rs`.

use serde::{Deserialize, Serialize};

/// Per-user configuration for an external service integration.
/// API tokens are stored as AES-256-GCM encrypted BLOBs.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IntegrationConfig {
    pub id: i64,
    pub user_id: i64,
    /// Service identifier (one of [`SERVICES`]).
    pub service: String,
    pub is_enabled: bool,
    /// AES-256-GCM ciphertext of the API token.
    pub encrypted_token: Option<Vec<u8>>,
    /// Service-specific JSON config (e.g., Jira project keys, GitHub org).
    pub config_json: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_sync_error: Option<String>,
    pub created_at: String,
}

/// All supported service identifiers. "atlassian" covers both Jira and Confluence.
pub const SERVICES: &[&str] = &[
    "github",
    "phabricator",
    "bugzilla",
    "atlassian",
    "google_drive",
    "google_calendar",
    "claude",
];

impl IntegrationConfig {
    /// Extract excluded Drive file IDs from a parsed config_json value.
    pub fn excluded_drive_file_ids(config: &serde_json::Value) -> Vec<String> {
        config["excluded_files"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["file_id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract excluded Calendar event IDs from a parsed config_json value.
    pub fn excluded_calendar_event_ids(config: &serde_json::Value) -> Vec<String> {
        config["excluded_events"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["event_id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Audit log entry for a single sync run against an external service.
/// Records timing, outcome, and entry-level counters.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncLog {
    pub id: i64,
    pub user_id: i64,
    pub service: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    /// `"success"` or `"error"`.
    pub status: String,
    pub entries_created: i64,
    pub entries_updated: i64,
    pub entries_deleted: i64,
    pub entries_fetched: i64,
    pub entries_skipped: i64,
    pub error_message: Option<String>,
}
