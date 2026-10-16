use crate::AppState;
use crate::integrations::model::SyncLog;
use crate::kernel::error::AppError;
use crate::worklog::model::BragEntry;

/// Hard sync: clears soft-deletes, re-syncs, then removes entries the sync no longer returns.
/// Shared by the JSON API and the background sync.
pub async fn run_hard_sync(
    state: &AppState,
    user_id: i64,
    service: &str,
) -> Result<(), AppError> {
    // Map service to its source names used in brag_entries.source
    let sources = super::catalog::service_to_sources(service);

    for source in &sources {
        let _ = BragEntry::clear_soft_deletes_for_service(&state.db, user_id, source).await;
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
        user_id,
        service,
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
                        user_id,
                        source,
                        &matched_ids,
                    )
                    .await
                } else {
                    BragEntry::soft_delete_unmatched_for_service(
                        &state.db,
                        user_id,
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
    Ok(())
}
