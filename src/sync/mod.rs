pub mod atlassian;
pub mod bugzilla;
pub mod confluence;
pub mod github;
pub mod google_calendar;
pub mod google_drive;
pub mod integrations_routes;
pub mod jira;
pub mod model;
pub mod phabricator;
pub mod repo;
pub mod services_config;
pub mod sync_routes;

use std::net::IpAddr;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::entries::model::BragEntry;
use crate::review::model::{BragPhase, Week};
use crate::shared::crypto::Crypto;
use crate::shared::error::AppError;
use crate::sync::model::{IntegrationConfig, SyncLog};

/// Creates a shared HTTP client with a 30s timeout for all sync services.
pub fn http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
}

/// Validates a user-provided base URL against SSRF.
/// Rejects non-HTTPS, localhost, and private/reserved IP ranges.
pub fn validate_base_url(url_str: &str) -> Result<(), AppError> {
    let url = url::Url::parse(url_str)
        .map_err(|_| AppError::BadRequest("Invalid base URL".to_string()))?;

    if url.scheme() != "https" {
        return Err(AppError::BadRequest("Base URL must use HTTPS".to_string()));
    }

    // Resolve hostname and check for private IPs
    if let Some(host) = url.host_str() {
        // Block obvious localhost patterns
        if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]" {
            return Err(AppError::BadRequest(
                "Base URL must not point to localhost".to_string(),
            ));
        }

        // Check if it's a direct IP address and validate
        if let Ok(ip) = host.parse::<IpAddr>()
            && is_private_ip(&ip)
        {
            return Err(AppError::BadRequest(
                "Base URL must not point to a private IP address".to_string(),
            ));
        }
    }

    Ok(())
}

// Returns true for loopback, RFC-1918, link-local, CGNAT, and unspecified addresses.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()              // 127.0.0.0/8
            || v4.is_private()            // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || v4.is_link_local()         // 169.254.0.0/16
            || v4.is_unspecified()        // 0.0.0.0
            || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64 // 100.64.0.0/10 (CGNAT)
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

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
    /// Date the work item occurred, formatted as `YYYY-MM-DD`.
    pub occurred_at: String,
    /// Meeting role classification. `None` for non-meeting entries.
    pub meeting_role: Option<String>,
    /// Google Calendar recurring event base ID. `None` for non-recurring meetings.
    pub recurring_group: Option<String>,
    /// Meeting start time as HH:MM (24h). `None` for non-timed entries.
    pub start_time: Option<String>,
    /// Meeting end time as HH:MM (24h). `None` for non-timed entries.
    pub end_time: Option<String>,
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
    /// Maps source name (e.g. "jira", "confluence") to the list of source_ids returned by sync.
    pub synced_source_ids: std::collections::HashMap<String, Vec<String>>,
}

struct PhaseWindow {
    phase: BragPhase,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Default)]
struct EntrySyncStats {
    created: i64,
    updated: i64,
    unchanged: i64,
    skipped: i64,
    synced_source_ids: std::collections::HashMap<String, Vec<String>>,
}

/// Returns `true` for services that can sync without an API token.
pub fn service_requires_token(service: &str) -> bool {
    !matches!(service, "bugzilla")
}

/// Maps a service name to its `SyncService` implementation.
/// Google Drive requires OAuth credentials from `Config`; pass `None` to skip it.
pub fn get_sync_service(
    service: &str,
    config: Option<&crate::shared::config::Config>,
) -> Option<Box<dyn SyncService>> {
    match service {
        "github" => Some(Box::new(github::GitHubSync)),
        "phabricator" => Some(Box::new(phabricator::PhabricatorSync)),
        "bugzilla" => Some(Box::new(bugzilla::BugzillaSync)),
        "atlassian" => Some(Box::new(atlassian::AtlassianSync)),
        "google_drive" => config.map(|c| {
            Box::new(google_drive::GoogleDriveSync {
                client_id: c.google_client_id.clone(),
                client_secret: c.google_client_secret.clone(),
            }) as Box<dyn SyncService>
        }),
        "google_calendar" => config.map(|c| {
            Box::new(google_calendar::GoogleCalendarSync {
                client_id: c.google_client_id.clone(),
                client_secret: c.google_client_secret.clone(),
            }) as Box<dyn SyncService>
        }),
        _ => None,
    }
}

/// Top-level sync orchestrator: decrypts token, resolves active phase date range,
/// calls the service's `sync()`, upserts entries into the correct weeks, and
/// records a `SyncLog`. Skips soft-deleted entries to respect user removals.
pub async fn run_sync(
    pool: &SqlitePool,
    crypto: &Crypto,
    http_client: &reqwest::Client,
    user_id: i64,
    service_name: &str,
    app_config: Option<&crate::shared::config::Config>,
) -> Result<SyncResult, AppError> {
    let user_crypto = crypto.for_user(user_id)?;
    let config = load_enabled_integration(pool, user_id, service_name).await?;
    let token = decrypt_token_for_sync(&config, service_name, user_id, &user_crypto)?;
    let service_config = build_service_config(&config, service_name, app_config);

    let sync_service = get_sync_service(service_name, app_config)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown service: {service_name}")))?;

    run_sync_core(
        pool,
        user_id,
        service_name,
        sync_service,
        http_client,
        &token,
        &service_config,
        &user_crypto,
    )
    .await
}

/// Like `run_sync`, but accepts a pre-built `SyncService` — used by tests with mock services.
pub async fn run_sync_with_service(
    pool: &SqlitePool,
    crypto: &Crypto,
    http_client: &reqwest::Client,
    user_id: i64,
    service_name: &str,
    service: Box<dyn SyncService>,
) -> Result<SyncResult, AppError> {
    let user_crypto = crypto.for_user(user_id)?;
    let config = load_enabled_integration(pool, user_id, service_name).await?;
    let token = decrypt_token_for_sync(&config, service_name, user_id, &user_crypto)?;
    let service_config = build_service_config(&config, service_name, None);

    run_sync_core(
        pool,
        user_id,
        service_name,
        service,
        http_client,
        &token,
        &service_config,
        &user_crypto,
    )
    .await
}

async fn load_enabled_integration(
    pool: &SqlitePool,
    user_id: i64,
    service_name: &str,
) -> Result<IntegrationConfig, AppError> {
    let config = IntegrationConfig::find_by_service(pool, user_id, service_name)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Integration {service_name} not configured")))?;

    if !config.is_enabled {
        return Err(AppError::BadRequest(format!(
            "Integration {service_name} is not enabled"
        )));
    }

    Ok(config)
}

fn decrypt_token_for_sync(
    config: &IntegrationConfig,
    service_name: &str,
    user_id: i64,
    user_crypto: &crate::shared::crypto::UserCrypto,
) -> Result<String, AppError> {
    if let Some(encrypted) = &config.encrypted_token {
        let token = user_crypto.decrypt(encrypted)?;
        tracing::info!(
            user_id,
            service = service_name,
            action = "sync",
            "Token decrypted for sync"
        );
        return Ok(token);
    }

    if service_requires_token(service_name) {
        return Err(AppError::BadRequest(format!(
            "No token configured for {service_name}"
        )));
    }

    Ok(String::new())
}

fn build_service_config(
    config: &IntegrationConfig,
    service_name: &str,
    app_config: Option<&crate::shared::config::Config>,
) -> serde_json::Value {
    let mut service_config: serde_json::Value = config
        .config_json
        .as_deref()
        .map(|s| serde_json::from_str(s).unwrap_or_default())
        .unwrap_or_default();

    if let Some(cfg) = app_config
        && cfg.public_only
    {
        service_config["public_only"] = serde_json::Value::Bool(true);
    }

    let svc_config = crate::sync::services_config::get();
    match service_name {
        "github" => {
            if !svc_config.github.allowed_orgs.is_empty() {
                service_config["allowed_orgs"] =
                    serde_json::to_value(&svc_config.github.allowed_orgs).unwrap_or_default();
            }
        }
        "phabricator" => {
            if !svc_config.phabricator.allowed_projects.is_empty() {
                service_config["allowed_projects"] =
                    serde_json::to_value(&svc_config.phabricator.allowed_projects)
                        .unwrap_or_default();
            }
        }
        "bugzilla" => {
            if !svc_config.bugzilla.allowed_products.is_empty() {
                service_config["allowed_products"] =
                    serde_json::to_value(&svc_config.bugzilla.allowed_products).unwrap_or_default();
            }
        }
        "atlassian" => {
            if !svc_config.atlassian.allowed_jira_projects.is_empty() {
                service_config["allowed_jira_projects"] =
                    serde_json::to_value(&svc_config.atlassian.allowed_jira_projects)
                        .unwrap_or_default();
            }
            if !svc_config.atlassian.allowed_confluence_spaces.is_empty() {
                service_config["allowed_confluence_spaces"] =
                    serde_json::to_value(&svc_config.atlassian.allowed_confluence_spaces)
                        .unwrap_or_default();
            }
            if !svc_config.atlassian.excluded_jira_projects.is_empty() {
                service_config["excluded_jira_projects"] =
                    serde_json::to_value(&svc_config.atlassian.excluded_jira_projects)
                        .unwrap_or_default();
            }
        }
        _ => {}
    }

    service_config
}

async fn resolve_phase_window(pool: &SqlitePool, user_id: i64) -> Result<PhaseWindow, AppError> {
    let phase = BragPhase::get_active(pool, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active brag phase".to_string()))?;

    let start_date = NaiveDate::parse_from_str(&phase.start_date, "%Y-%m-%d")
        .map_err(|e| AppError::Internal(format!("Invalid phase start date: {e}")))?;
    let end_date = NaiveDate::parse_from_str(&phase.end_date, "%Y-%m-%d")
        .map_err(|e| AppError::Internal(format!("Invalid phase end date: {e}")))?;

    Ok(PhaseWindow {
        phase,
        start_date,
        end_date,
    })
}

async fn persist_synced_entries(
    pool: &SqlitePool,
    user_id: i64,
    phase: &BragPhase,
    entries: &[SyncedEntry],
    start_date: NaiveDate,
    user_crypto: &crate::shared::crypto::UserCrypto,
) -> Result<EntrySyncStats, AppError> {
    let mut stats = EntrySyncStats::default();

    for entry in entries {
        if !entry.source_id.is_empty() {
            stats
                .synced_source_ids
                .entry(entry.source.to_string())
                .or_default()
                .push(entry.source_id.clone());
        }

        let occurred =
            NaiveDate::parse_from_str(&entry.occurred_at, "%Y-%m-%d").unwrap_or(start_date);
        let iso_week = occurred
            .format("%V")
            .to_string()
            .parse::<i64>()
            .unwrap_or(1);
        let year = occurred
            .format("%G")
            .to_string()
            .parse::<i64>()
            .unwrap_or(2026);

        let week_start =
            occurred - chrono::Duration::days(i64::from(occurred.weekday().num_days_from_monday()));
        let week_end = week_start + chrono::Duration::days(6);

        let week = Week::find_or_create(
            pool,
            phase.id,
            iso_week,
            year,
            &week_start.format("%Y-%m-%d").to_string(),
            &week_end.format("%Y-%m-%d").to_string(),
        )
        .await?;

        let existing = if entry.source_id.is_empty() {
            None
        } else {
            BragEntry::find_by_source(pool, entry.source, &entry.source_id, user_id, user_crypto)
                .await?
        };

        if let Some(ref ex) = existing
            && ex.deleted_at.is_some()
        {
            stats.skipped += 1;
            continue;
        }

        if let Some(ref ex) = existing {
            if ex.title != entry.title
                || ex.description.as_deref() != entry.description.as_deref()
                || ex.entry_type != entry.entry_type
                || ex.status.as_deref() != entry.status.as_deref()
            {
                stats.updated += 1;
            } else {
                stats.unchanged += 1;
            }
        } else {
            stats.created += 1;
        }

        BragEntry::create_from_sync(
            pool,
            week.id,
            entry.source,
            &entry.source_id,
            entry.source_url.as_deref(),
            &entry.title,
            entry.description.as_deref(),
            entry.entry_type,
            entry.status.as_deref(),
            entry.repository.as_deref(),
            &entry.occurred_at,
            entry.meeting_role.as_deref(),
            entry.recurring_group.as_deref(),
            entry.start_time.as_deref(),
            entry.end_time.as_deref(),
            user_crypto,
        )
        .await?;
    }

    Ok(stats)
}

#[allow(clippy::too_many_arguments)]
async fn run_sync_core(
    pool: &SqlitePool,
    user_id: i64,
    service_name: &str,
    sync_service: Box<dyn SyncService>,
    http_client: &reqwest::Client,
    token: &str,
    service_config: &serde_json::Value,
    user_crypto: &crate::shared::crypto::UserCrypto,
) -> Result<SyncResult, AppError> {
    let phase_window = resolve_phase_window(pool, user_id).await?;
    let start_date = phase_window.start_date;
    let end_date = phase_window.end_date;

    let sync_started_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    tracing::info!(
        service = service_name,
        %start_date,
        %end_date,
        phase = phase_window.phase.name,
        "Starting sync for service"
    );

    match sync_service
        .sync(http_client, token, service_config, start_date, end_date)
        .await
    {
        Ok(entries) => {
            let stats = persist_synced_entries(
                pool,
                user_id,
                &phase_window.phase,
                &entries,
                start_date,
                user_crypto,
            )
            .await?;

            tracing::info!(
                service = service_name,
                fetched = entries.len(),
                created = stats.created,
                updated = stats.updated,
                unchanged = stats.unchanged,
                skipped = stats.skipped,
                "Sync completed"
            );

            let log = SyncLog::create(
                pool,
                user_id,
                service_name,
                &sync_started_at,
                "success",
                stats.created,
                stats.updated,
                0,
                i64::try_from(entries.len())
                    .map_err(|_| AppError::Internal("Synced entries count overflow".to_string()))?,
                stats.skipped,
                None,
            )
            .await?;
            IntegrationConfig::update_sync_status(pool, user_id, service_name, "success", None)
                .await?;

            Ok(SyncResult {
                log,
                synced_source_ids: stats.synced_source_ids,
            })
        }
        Err(e) => {
            tracing::error!(service = service_name, error = %e, "Sync failed");
            let error_msg = e.to_string();
            let _ = SyncLog::create(
                pool,
                user_id,
                service_name,
                &sync_started_at,
                "error",
                0,
                0,
                0,
                0,
                0,
                Some(&error_msg),
            )
            .await;
            IntegrationConfig::update_sync_status(
                pool,
                user_id,
                service_name,
                "error",
                Some(&error_msg),
            )
            .await?;

            Err(e)
        }
    }
}

use chrono::Datelike;
