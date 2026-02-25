use axum::{
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::worklog::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::cycle::model::BragPhase;
use crate::kernel::error::AppError;
use crate::integrations::model::{IntegrationConfig, SyncLog};

/// Renders the recent sync log fragment plus an OOB swap for the synced entries card.
pub(crate) async fn render_sync_log(
    state: &AppState,
    user_id: i64,
) -> Result<Html<String>, AppError> {
    let sync_logs = SyncLog::recent_for_user(&state.db, user_id, 10)
        .await
        .unwrap_or_default();
    let mut ctx = tera::Context::new();
    ctx.insert("sync_logs", &sync_logs);
    let mut html = state
        .templates
        .render("components/settings_sync_log.html", &ctx)?;

    // Append OOB swap for the synced entries card so counts update after sync
    if let Ok(Some(phase)) = BragPhase::get_active(&state.db, user_id).await {
        let source_counts = BragEntry::source_counts_for_phase(&state.db, phase.id)
            .await
            .unwrap_or_default();
        let mut entries_ctx = tera::Context::new();
        entries_ctx.insert("source_counts", &source_counts);
        if let Ok(entries_html) = state
            .templates
            .render("components/settings_synced_entries.html", &entries_ctx)
        {
            html.push_str(&format!(
                "<div id=\"synced-entries\" hx-swap-oob=\"outerHTML:#synced-entries\">{}</div>",
                entries_html
            ));
        }
    }

    Ok(Html(html))
}

/// HTMX handler: triggers an incremental sync for a single service and returns the updated log.
pub async fn sync_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(service): Path<String>,
) -> Result<Html<String>, AppError> {
    let http_client = crate::integrations::http_client()?;
    let _ = crate::integrations::run_sync(
        &state.db,
        &state.crypto,
        &http_client,
        auth.user_id,
        &service,
        Some(&state.config),
    )
    .await;

    render_sync_log(&state, auth.user_id).await
}

/// HTMX handler: syncs all enabled services sequentially and returns the updated log.
pub async fn sync_all(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let all_configs = IntegrationConfig::list_enabled_for_user(&state.db, auth.user_id).await?;
    let configs: Vec<_> = all_configs
        .into_iter()
        .filter(|c| crate::integrations::get_sync_service(&c.service, Some(&state.config)).is_some())
        .collect();

    if !configs.is_empty() {
        let http_client = crate::integrations::http_client()?;
        for config in &configs {
            if let Err(e) = crate::integrations::run_sync(
                &state.db,
                &state.crypto,
                &http_client,
                auth.user_id,
                &config.service,
                Some(&state.config),
            )
            .await
            {
                tracing::warn!(service = %config.service, error = %e, "Sync failed for service");
            }
        }
    }

    render_sync_log(&state, auth.user_id).await
}

/// HTMX handler: performs a hard sync -- clears soft-deletes, re-syncs, then soft-deletes stale entries.
pub async fn hard_sync_service(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(service): Path<String>,
) -> Result<Html<String>, AppError> {
    // Map service to its source names used in brag_entries.source
    let sources: Vec<&str> = match service.as_str() {
        "atlassian" => vec!["jira", "confluence"],
        "google_drive" => vec!["google_drive"],
        "google_calendar" => vec!["google_calendar"],
        other => vec![other],
    };

    for source in &sources {
        let _ = BragEntry::clear_soft_deletes_for_service(&state.db, auth.user_id, source).await;
    }

    // Calendar uses hard-delete for unmatched entries because its sync window
    // moves forward daily — future events that leave the window today may
    // re-enter it tomorrow, and soft-deleted entries would be permanently skipped.
    let use_hard_delete = service == "google_calendar";

    let http_client = crate::integrations::http_client()?;
    match crate::integrations::run_sync(
        &state.db,
        &state.crypto,
        &http_client,
        auth.user_id,
        &service,
        Some(&state.config),
    )
    .await
    {
        Ok(result) => {
            // Remove entries that were NOT returned by the sync
            // (stale entries from changed settings or date range), preserving user-edited ones
            let mut total_deleted = 0u64;
            for source in &sources {
                let matched_ids = result
                    .synced_source_ids
                    .get(*source)
                    .cloned()
                    .unwrap_or_default();
                let deleted = if use_hard_delete {
                    BragEntry::hard_delete_unmatched_for_service(
                        &state.db,
                        auth.user_id,
                        source,
                        &matched_ids,
                    )
                    .await
                } else {
                    BragEntry::soft_delete_unmatched_for_service(
                        &state.db,
                        auth.user_id,
                        source,
                        &matched_ids,
                    )
                    .await
                };
                if let Ok(deleted) = deleted {
                    total_deleted += deleted;
                }
            }

            // If entries were cleaned up, update the sync log with the deleted count
            if total_deleted > 0 {
                let _ =
                    SyncLog::set_entries_deleted(&state.db, result.log.id, total_deleted as i64)
                        .await;
            }
        }
        Err(e) => {
            tracing::warn!(service = %service, error = %e, "Hard sync failed for service");
        }
    }

    render_sync_log(&state, auth.user_id).await
}

/// HTMX handler: deletes all sync logs for the user and returns the empty log fragment.
pub async fn clear_sync_logs(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    SyncLog::delete_all_for_user(&state.db, auth.user_id).await?;
    render_sync_log(&state, auth.user_id).await
}
