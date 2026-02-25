use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;

use crate::cycle::model::{BragPhase, Week};
use crate::kernel::crypto::Crypto;
use crate::kernel::error::AppError;
use crate::worklog::model::BragEntry;

use super::factory::{get_sync_service, service_requires_token};
use super::model::{IntegrationConfig, SyncLog};
use super::{SyncResult, SyncService, SyncedEntry};

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

/// Top-level sync orchestrator: decrypts token, resolves active phase date range,
/// calls the service's `sync()`, upserts entries into the correct weeks, and
/// records a `SyncLog`. Skips soft-deleted entries to respect user removals.
pub async fn run_sync(
    pool: &SqlitePool,
    crypto: &Crypto,
    http_client: &reqwest::Client,
    user_id: i64,
    service_name: &str,
    app_config: Option<&crate::kernel::config::Config>,
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
    user_crypto: &crate::kernel::crypto::UserCrypto,
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
    app_config: Option<&crate::kernel::config::Config>,
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

    let svc_config = crate::integrations::services_config::get();
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
    user_crypto: &crate::kernel::crypto::UserCrypto,
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
    user_crypto: &crate::kernel::crypto::UserCrypto,
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
