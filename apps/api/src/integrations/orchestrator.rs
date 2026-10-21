use chrono::{Datelike, NaiveDate};
use sqlx::{SqliteConnection, SqlitePool};

use crate::cycle::model::{BragPhase, Week};
use crate::identity::model::PeopleAlias;
use crate::kernel::crypto::Crypto;
use crate::kernel::error::AppError;
use crate::worklog::model::{BragEntry, EntryType};

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

/// Derives a deduped, comma-separated team string from collaborator emails using the team map.
fn derive_teams(collaborators: Option<&str>, team_map: &std::collections::HashMap<String, String>) -> Option<String> {
    let collabs = collaborators?;
    if collabs.is_empty() || team_map.is_empty() {
        return None;
    }
    let mut teams = Vec::new();
    for c in collabs.split(',') {
        let email = c.trim().to_lowercase();
        if let Some(team) = team_map.get(&email)
            && !teams.contains(team)
        {
            teams.push(team.clone());
        }
    }
    if teams.is_empty() {
        None
    } else {
        Some(teams.join(", "))
    }
}

/// True when upserting `entry` would leave `ex` as it is. Mirrors the `ON CONFLICT` clause of
/// [`BragEntry::create_from_sync`]: fields it merges with `COALESCE` only count as changed when
/// the merge would actually write a new value.
fn sync_would_change(
    ex: &BragEntry,
    entry: &SyncedEntry,
    week_id: i64,
    derived_teams: Option<&str>,
) -> bool {
    // `COALESCE(new, old)`: a value from the sync replaces the stored one.
    let replaces = |new: Option<&str>, old: Option<&str>| new.is_some() && new != old;
    // `COALESCE(old, new)`: the sync only fills a gap.
    let fills = |old: Option<&str>, new: Option<&str>| old.is_none() && new.is_some();
    ex.title != entry.title
        || ex.entry_type != entry.entry_type
        || ex.status.as_deref() != entry.status.as_deref()
        || ex.occurred_at != entry.occurred_at
        || ex.week_id != week_id
        || ex.start_time.as_deref() != entry.start_time.as_deref()
        || ex.end_time.as_deref() != entry.end_time.as_deref()
        || fills(ex.description.as_deref(), entry.description.as_deref())
        || fills(ex.teams.as_deref(), derived_teams)
        || replaces(entry.source_url.as_deref(), ex.source_url.as_deref())
        || replaces(entry.repository.as_deref(), ex.repository.as_deref())
        || replaces(entry.recurring_group.as_deref(), ex.recurring_group.as_deref())
        || replaces(entry.collaborators.as_deref(), ex.collaborators.as_deref())
}

/// Upserts `entries` on `conn`, which the caller runs as one transaction. Unchanged entries
/// are not written at all.
#[allow(clippy::too_many_arguments)]
async fn persist_synced_entries(
    conn: &mut SqliteConnection,
    user_id: i64,
    phase: &BragPhase,
    entries: &[SyncedEntry],
    start_date: NaiveDate,
    team_map: &std::collections::HashMap<String, String>,
    user_crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<EntrySyncStats, AppError> {
    let mut stats = EntrySyncStats::default();

    for entry in entries {
        // Refresh-only items exist only to update a stored row (see `SyncedEntry`). Keeping
        // them out of `synced_source_ids` lets a hard sync still drop rows the settings exclude.
        if !entry.source_id.is_empty() && !entry.refresh_only {
            stats
                .synced_source_ids
                .entry(entry.source.to_string())
                .or_default()
                .push(entry.source_id.clone());
        }

        let existing = if entry.source_id.is_empty() {
            None
        } else {
            BragEntry::find_by_source(
                &mut *conn,
                entry.source,
                &entry.source_id,
                user_id,
                user_crypto,
            )
            .await?
        };

        if let Some(ref ex) = existing
            && ex.deleted_at.is_some()
        {
            stats.skipped += 1;
            continue;
        }
        if entry.refresh_only && existing.is_none() {
            continue;
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

        let week = Week::find_or_create_on(
            &mut *conn,
            phase.id,
            iso_week,
            year,
            &week_start.format("%Y-%m-%d").to_string(),
            &week_end.format("%Y-%m-%d").to_string(),
        )
        .await?;

        // Derive teams from collaborator emails via the team map
        let derived_teams = derive_teams(entry.collaborators.as_deref(), team_map);

        match existing {
            Some(ref ex) if !sync_would_change(ex, entry, week.id, derived_teams.as_deref()) => {
                stats.unchanged += 1;
                continue;
            }
            Some(_) => stats.updated += 1,
            None => stats.created += 1,
        }

        BragEntry::create_from_sync(
            &mut *conn,
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
            entry.collaborators.as_deref(),
            derived_teams.as_deref(),
            user_crypto,
        )
        .await?;
    }

    Ok(stats)
}

/// Removes pending requests (review requests) of this service's sources that the sync no
/// longer returns. Only runs after a successful fetch, so a failed sync never clears them.
async fn delete_stale_pending_requests(
    conn: &mut SqliteConnection,
    user_id: i64,
    service_name: &str,
    entries: &[SyncedEntry],
) -> Result<u64, AppError> {
    let mut deleted = 0;
    for source in super::catalog::service_to_sources(service_name) {
        let pending_types: Vec<&str> = EntryType::ALL
            .iter()
            .filter(|t| t.is_pending_request() && t.source() == source)
            .map(|t| t.slug())
            .collect();
        if pending_types.is_empty() {
            continue;
        }
        let still_pending: Vec<&str> = entries
            .iter()
            .filter(|e| e.source == source && pending_types.contains(&e.entry_type))
            .map(|e| e.source_id.as_str())
            .collect();
        deleted += BragEntry::delete_stale_pending_requests(
            &mut *conn,
            user_id,
            source,
            &pending_types,
            &still_pending,
        )
        .await?;
    }
    Ok(deleted)
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

    // Fetch and save fail the same way: a database error after the fetch must still log
    // "Sync failed" and mark the integration, like an API error does. Dropping `tx` on an
    // error rolls the whole sync back.
    let outcome = async {
        let entries = sync_service
            .sync(http_client, token, service_config, start_date, end_date)
            .await?;
        // One transaction per sync: on a FUSE-mounted database every commit rewrites the
        // whole file in object storage, so per-entry commits made a sync take minutes.
        // Read before `begin`: with the single-connection pool used for FUSE, the
        // transaction holds the only connection until it commits.
        let team_map = PeopleAlias::team_map(pool, user_id).await?;
        let mut tx = pool.begin().await?;
        let stats = persist_synced_entries(
            &mut tx,
            user_id,
            &phase_window.phase,
            &entries,
            start_date,
            &team_map,
            user_crypto,
        )
        .await?;
        let deleted =
            delete_stale_pending_requests(&mut tx, user_id, service_name, &entries).await?;
        tx.commit().await?;
        Ok::<_, AppError>((entries, stats, deleted))
    }
    .await;

    match outcome {
        Ok((entries, stats, deleted)) => {
            tracing::info!(
                service = service_name,
                fetched = entries.len(),
                created = stats.created,
                updated = stats.updated,
                unchanged = stats.unchanged,
                skipped = stats.skipped,
                deleted,
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
                i64::try_from(deleted)
                    .map_err(|_| AppError::Internal("Deleted entries count overflow".to_string()))?,
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
