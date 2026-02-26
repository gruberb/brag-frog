pub mod atlassian;
pub mod bugzilla;
pub mod confluence;
pub mod factory;
pub mod github;
pub mod google_calendar;
pub mod google_drive;
pub mod integrations_routes;
pub mod jira;
pub mod model;
pub mod orchestrator;
pub mod phabricator;
pub mod repo;
pub mod services_config;
pub mod sync_routes;
pub mod validation;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;

use crate::kernel::error::AppError;
use crate::integrations::model::SyncLog;

/// Re-export from kernel for callers that historically used `sync::http_client()`.
pub use crate::kernel::http::http_client;

// Re-export key public items from submodules for backwards compatibility.
pub use factory::{get_sync_service, service_requires_token};
pub use orchestrator::{run_sync, run_sync_with_service};
pub use validation::validate_base_url;

/// Normalized work item returned by a SyncService, ready for upsert into `brag_entries`.
#[derive(Debug, Clone, Serialize)]
pub struct SyncedEntry {
    pub source: &'static str,
    pub source_id: String,
    pub source_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub entry_type: &'static str,
    pub status: Option<String>,
    pub repository: Option<String>,
    pub occurred_at: String,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub collaborators: Option<String>,
}

/// Result of a `test_connection` call -- reports auth success and the authenticated username.
#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub username: Option<String>,
    pub error: Option<String>,
}

/// Trait for services that sync external work items into `BragEntry` records.
/// Uses `async_trait` for object safety (`Box<dyn SyncService>`).
#[async_trait]
pub trait SyncService: Send + Sync {
    /// Fetches work items from the external service within `[start_date, end_date]`.
    async fn sync(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<SyncedEntry>, AppError>;

    /// Verifies the token/config are valid and returns the authenticated identity.
    async fn test_connection(
        &self,
        client: &reqwest::Client,
        token: &str,
        config: &serde_json::Value,
    ) -> Result<ConnectionStatus, AppError>;
}

/// Outcome of a full sync run: the persisted log plus source IDs for stale-entry cleanup.
pub struct SyncResult {
    pub log: SyncLog,
    pub synced_source_ids: std::collections::HashMap<String, Vec<String>>,
}
