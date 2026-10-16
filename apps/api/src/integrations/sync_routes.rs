use axum::{
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::worklog::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::cycle::model::BragPhase;
use crate::kernel::error::AppError;
use crate::integrations::model::SyncLog;
use crate::integrations::background;

/// Reads sync status for a user, returning (is_syncing, current_service).
fn read_sync_state(state: &AppState, user_id: i64) -> (bool, Option<String>) {
    state
        .sync_status
        .get(&user_id)
        .map(|s| (s.is_syncing, s.current_service.clone()))
        .unwrap_or((false, None))
}

/// Renders the sync log fragment, OOB swaps for synced-entries card and sidebar indicator.
/// `force_syncing` overrides the status map (used by `sync_all` to avoid race conditions).
async fn render_sync_log_inner(
    state: &AppState,
    user_id: i64,
    force_syncing: bool,
) -> Result<Html<String>, AppError> {
    let sync_logs = SyncLog::recent_for_user(&state.db, user_id, 10)
        .await
        .unwrap_or_default();

    let (map_syncing, current_service) = read_sync_state(state, user_id);
    let is_syncing = force_syncing || map_syncing;

    let mut ctx = tera::Context::new();
    ctx.insert("sync_logs", &sync_logs);
    ctx.insert("is_syncing", &is_syncing);
    ctx.insert("current_service", &current_service);
    let mut html = state
        .templates
        .render("components/settings_sync_log.html", &ctx)?;

    // OOB: update synced entries card
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

    // OOB: update sidebar sync indicator
    let mut indicator_ctx = tera::Context::new();
    if let Some(status) = state.sync_status.get(&user_id) {
        indicator_ctx.insert("is_syncing", &(force_syncing || status.is_syncing));
        indicator_ctx.insert("current_service", &status.current_service);
        indicator_ctx.insert("last_synced_at", &status.last_synced_at);
        indicator_ctx.insert("last_error", &status.last_error);
        indicator_ctx.insert("services_remaining", &status.services_remaining);
        indicator_ctx.insert("services_total", &status.services_total);
    } else {
        indicator_ctx.insert("is_syncing", &force_syncing);
        indicator_ctx.insert("current_service", &None::<String>);
        indicator_ctx.insert("last_synced_at", &None::<String>);
        indicator_ctx.insert("last_error", &None::<String>);
        indicator_ctx.insert("services_remaining", &0usize);
        indicator_ctx.insert("services_total", &0usize);
    }
    if let Ok(indicator_html) = state
        .templates
        .render("components/sync_status.html", &indicator_ctx)
    {
        // The template renders an <a id="sync-indicator" ...>. We need to add OOB attr.
        // Replace the opening tag to include hx-swap-oob.
        let oob_indicator = indicator_html.replacen(
            "id=\"sync-indicator\"",
            "id=\"sync-indicator\" hx-swap-oob=\"outerHTML:#sync-indicator\"",
            1,
        );
        html.push_str(&oob_indicator);
    }

    Ok(Html(html))
}

/// Standard render: reads sync state from the map.
pub(crate) async fn render_sync_log(
    state: &AppState,
    user_id: i64,
) -> Result<Html<String>, AppError> {
    render_sync_log_inner(state, user_id, false).await
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

/// HTMX handler: returns the current sync status indicator fragment.
pub async fn sync_status(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();

    if let Some(status) = state.sync_status.get(&auth.user_id) {
        ctx.insert("is_syncing", &status.is_syncing);
        ctx.insert("current_service", &status.current_service);
        ctx.insert("last_synced_at", &status.last_synced_at);
        ctx.insert("last_error", &status.last_error);
        ctx.insert("services_remaining", &status.services_remaining);
        ctx.insert("services_total", &status.services_total);
    } else {
        // Fall back to DB for last sync time
        ctx.insert("is_syncing", &false);
        ctx.insert("current_service", &None::<String>);
        let last_sync: Option<String> = sqlx::query_scalar(
            "SELECT MAX(last_sync_at) FROM integration_configs WHERE user_id = ? AND is_enabled = 1",
        )
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(None);
        ctx.insert("last_synced_at", &last_sync);
        ctx.insert("last_error", &None::<String>);
        ctx.insert("services_remaining", &0usize);
        ctx.insert("services_total", &0usize);
    }

    let html = state
        .templates
        .render("components/sync_status.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: spawns a non-blocking background sync of all enabled services
/// and returns the sync log with syncing state forced on (avoids race with status map).
pub async fn sync_all(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    background::spawn_sync_all(state.clone(), auth.user_id);
    // Force syncing=true — the background task may not have populated the map yet
    render_sync_log_inner(&state, auth.user_id, true).await
}

/// HTMX handler: returns the sync activity fragment with OOB button state swaps.
/// Polled by the integrations page while a background sync is active.
pub async fn sync_status_activity(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
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
