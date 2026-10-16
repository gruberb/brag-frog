use crate::AppState;
use crate::integrations::model::IntegrationConfig;
use crate::integrations::sync_status::UserSyncStatus;

/// Spawns a non-blocking background sync of all enabled integrations for a user.
/// Skips if the user is already syncing. Safe to call multiple times. Returns whether a
/// sync was started.
///
/// The status entry is marked as syncing before the task is spawned, so a status poll
/// right after this call never reports idle for a sync that was just accepted.
pub fn spawn_sync_all(state: AppState, user_id: i64) -> bool {
    if !mark_syncing(&state, user_id, None) {
        tracing::debug!(user_id, "Skipping spawn_sync_all: already syncing");
        return false;
    }

    tokio::spawn(async move {
        if let Err(e) = run_sync_all_for_user(&state, user_id).await {
            tracing::warn!(user_id, error = %e, "Background sync failed");
        }
        mark_idle(&state, user_id);
    });
    true
}

/// Spawns a background sync of one service (`hard` also removes entries the sync no longer
/// returns). Returns false without spawning when the user is already syncing.
pub fn spawn_sync_service(state: AppState, user_id: i64, service: String, hard: bool) -> bool {
    if !mark_syncing(&state, user_id, Some(&service)) {
        return false;
    }

    tokio::spawn(async move {
        let result = if hard {
            crate::integrations::hard_sync::run_hard_sync(&state, user_id, &service).await
        } else {
            match crate::integrations::http_client() {
                Ok(client) => crate::integrations::run_sync(
                    &state.db,
                    &state.crypto,
                    &client,
                    user_id,
                    &service,
                    Some(&state.config),
                )
                .await
                .map(|_| ()),
                Err(e) => Err(crate::kernel::error::AppError::HttpClient(e)),
            }
        };
        let last_error = result.err().map(|e| {
            tracing::warn!(service = %service, error = %e, "Background service sync failed");
            format!("{service}: {e}")
        });
        state.sync_status.insert(
            user_id,
            UserSyncStatus {
                is_syncing: false,
                current_service: None,
                last_synced_at: Some(
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                ),
                last_error,
                services_remaining: 0,
                services_total: 1,
            },
        );
    });
    true
}

/// Atomically claims the "syncing" flag. False when a sync is already running.
fn mark_syncing(state: &AppState, user_id: i64, service: Option<&str>) -> bool {
    use dashmap::mapref::entry::Entry;
    match state.sync_status.entry(user_id) {
        Entry::Occupied(e) if e.get().is_syncing => false,
        Entry::Occupied(mut e) => {
            e.insert(syncing_status(service));
            true
        }
        Entry::Vacant(e) => {
            e.insert(syncing_status(service));
            true
        }
    }
}

fn syncing_status(service: Option<&str>) -> UserSyncStatus {
    UserSyncStatus {
        is_syncing: true,
        current_service: service.map(str::to_string),
        last_synced_at: None,
        last_error: None,
        services_remaining: usize::from(service.is_some()),
        services_total: usize::from(service.is_some()),
    }
}

/// Clears a flag left set by an early return (no enabled services, HTTP client failure).
/// A completed run has already written its own final status.
fn mark_idle(state: &AppState, user_id: i64) {
    if let Some(mut status) = state.sync_status.get_mut(&user_id) {
        status.is_syncing = false;
        status.current_service = None;
        status.services_remaining = 0;
    }
}

/// Runs sync for all enabled integrations sequentially, updating the status map.
async fn run_sync_all_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<(), crate::kernel::error::AppError> {
    let all_configs = IntegrationConfig::list_enabled_for_user(&state.db, user_id).await?;
    let configs: Vec<_> = all_configs
        .into_iter()
        .filter(|c| {
            crate::integrations::get_sync_service(&c.service, Some(&state.config)).is_some()
        })
        .collect();

    if configs.is_empty() {
        return Ok(());
    }

    let total = configs.len();

    // Mark as syncing
    state.sync_status.insert(
        user_id,
        UserSyncStatus {
            is_syncing: true,
            current_service: None,
            last_synced_at: None,
            last_error: None,
            services_remaining: total,
            services_total: total,
        },
    );

    let http_client = match crate::integrations::http_client() {
        Ok(c) => c,
        Err(e) => {
            state.sync_status.insert(
                user_id,
                UserSyncStatus {
                    is_syncing: false,
                    current_service: None,
                    last_synced_at: None,
                    last_error: Some(e.to_string()),
                    services_remaining: 0,
                    services_total: total,
                },
            );
            return Err(crate::kernel::error::AppError::HttpClient(e));
        }
    };

    let mut last_error: Option<String> = None;

    for (i, config) in configs.iter().enumerate() {
        // Update status with current service
        state.sync_status.insert(
            user_id,
            UserSyncStatus {
                is_syncing: true,
                current_service: Some(config.service.clone()),
                last_synced_at: None,
                last_error: last_error.clone(),
                services_remaining: total - i,
                services_total: total,
            },
        );

        if let Err(e) = crate::integrations::run_sync(
            &state.db,
            &state.crypto,
            &http_client,
            user_id,
            &config.service,
            Some(&state.config),
        )
        .await
        {
            tracing::warn!(
                service = %config.service,
                error = %e,
                "Background sync failed for service"
            );
            last_error = Some(format!("{}: {}", config.service, e));
        }
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    state.sync_status.insert(
        user_id,
        UserSyncStatus {
            is_syncing: false,
            current_service: None,
            last_synced_at: Some(now),
            last_error,
            services_remaining: 0,
            services_total: total,
        },
    );

    Ok(())
}

/// Background loop that syncs all users with enabled integrations every hour.
/// First tick is delayed by 1 hour to avoid thundering herd on boot.
pub async fn hourly_sync_loop(state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    // Skip the first immediate tick
    interval.tick().await;

    loop {
        interval.tick().await;
        tracing::info!("Hourly background sync: starting");

        let user_ids = match sqlx::query_scalar::<_, i64>(
            "SELECT DISTINCT user_id FROM integration_configs WHERE is_enabled = 1",
        )
        .fetch_all(&state.db)
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!(error = %e, "Hourly sync: failed to query users");
                continue;
            }
        };

        for user_id in user_ids {
            if let Err(e) = run_sync_all_for_user(&state, user_id).await {
                tracing::warn!(user_id, error = %e, "Hourly sync failed for user");
            }
        }

        tracing::info!("Hourly background sync: complete");
    }
}
