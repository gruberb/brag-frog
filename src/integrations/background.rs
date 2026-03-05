use crate::AppState;
use crate::integrations::model::IntegrationConfig;
use crate::integrations::sync_status::UserSyncStatus;

/// Spawns a non-blocking background sync of all enabled integrations for a user.
/// Skips if the user is already syncing. Safe to call multiple times.
pub fn spawn_sync_all(state: AppState, user_id: i64) {
    if let Some(status) = state.sync_status.get(&user_id)
        && status.is_syncing
    {
        tracing::debug!(user_id, "Skipping spawn_sync_all: already syncing");
        return;
    }

    tokio::spawn(async move {
        if let Err(e) = run_sync_all_for_user(&state, user_id).await {
            tracing::warn!(user_id, error = %e, "Background sync failed");
        }
    });
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
