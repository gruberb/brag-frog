//! Sync endpoints. Syncs talk to slow external APIs, so requests only start a background task
//! and return 202; clients poll `GET /sync/status`.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::integrations::background;
use crate::integrations::catalog::service_display_name;
use crate::integrations::model::{IntegrationConfig, SERVICES, SyncLog};
use crate::kernel::error::AppError;

/// How many log rows `GET /sync/status` returns.
const RECENT_LOGS: i64 = 10;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SyncAccepted {
    /// False when a sync was already running, so nothing new was started.
    pub started: bool,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SyncLogDto {
    pub id: i64,
    pub service: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub entries_created: i64,
    pub entries_updated: i64,
    pub entries_deleted: i64,
    pub entries_fetched: i64,
    pub entries_skipped: i64,
    pub error_message: Option<String>,
}

impl From<SyncLog> for SyncLogDto {
    fn from(l: SyncLog) -> Self {
        Self {
            id: l.id,
            service: l.service,
            started_at: l.started_at,
            completed_at: l.completed_at,
            status: l.status,
            entries_created: l.entries_created,
            entries_updated: l.entries_updated,
            entries_deleted: l.entries_deleted,
            entries_fetched: l.entries_fetched,
            entries_skipped: l.entries_skipped,
            error_message: l.error_message,
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ServiceSyncState {
    pub service: String,
    pub display_name: String,
    pub is_enabled: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_sync_error: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SyncStatus {
    pub running: bool,
    pub current_service: Option<String>,
    pub services_remaining: u32,
    pub services_total: u32,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
    pub services: Vec<ServiceSyncState>,
    pub recent_logs: Vec<SyncLogDto>,
}

#[derive(Deserialize, Default)]
pub struct SyncQuery {
    #[serde(default)]
    pub hard: bool,
}

fn accepted(started: bool) -> (StatusCode, Json<SyncAccepted>) {
    (StatusCode::ACCEPTED, Json(SyncAccepted { started }))
}

/// `POST /api/v1/sync`: all enabled services.
pub async fn sync_all(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> (StatusCode, Json<SyncAccepted>) {
    accepted(background::spawn_sync_all(state, auth.user_id))
}

/// `POST /api/v1/sync/{service}?hard=true`. Validates up front so a bad request is a 4xx
/// instead of a 202 for a task that fails immediately.
pub async fn sync_service(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(q): Query<SyncQuery>,
) -> Result<(StatusCode, Json<SyncAccepted>), ApiError> {
    if !SERVICES.contains(&service.as_str())
        || crate::integrations::get_sync_service(&service, Some(&state.config)).is_none()
    {
        return Err(ApiError::not_found(format!(
            "No sync for service: {service}"
        )));
    }
    let config = IntegrationConfig::find_by_service(&state.db, auth.user_id, &service)
        .await?
        .filter(|c| c.is_enabled)
        .ok_or_else(|| ApiError::invalid(format!("{service} is not connected")))?;
    if config.encrypted_token.is_none() && crate::integrations::service_requires_token(&service) {
        return Err(ApiError::invalid(format!(
            "No token configured for {service}"
        )));
    }
    Ok(accepted(background::spawn_sync_service(
        state,
        auth.user_id,
        service,
        q.hard,
    )))
}

/// `GET /api/v1/sync/status`
pub async fn status(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<SyncStatus> {
    let configs = IntegrationConfig::list_for_user(&state.db, auth.user_id).await?;
    let logs = SyncLog::recent_for_user(&state.db, auth.user_id, RECENT_LOGS)
        .await
        .map_err(|e: AppError| ApiError::from(e))?;

    let live = state.sync_status.get(&auth.user_id);
    let last_db_sync = configs.iter().filter_map(|c| c.last_sync_at.clone()).max();

    Ok(Json(SyncStatus {
        running: live.as_ref().is_some_and(|s| s.is_syncing),
        current_service: live.as_ref().and_then(|s| s.current_service.clone()),
        services_remaining: live.as_ref().map_or(0, |s| s.services_remaining as u32),
        services_total: live.as_ref().map_or(0, |s| s.services_total as u32),
        last_synced_at: live
            .as_ref()
            .and_then(|s| s.last_synced_at.clone())
            .or(last_db_sync),
        last_error: live.as_ref().and_then(|s| s.last_error.clone()),
        services: configs
            .into_iter()
            .map(|c| ServiceSyncState {
                display_name: service_display_name(&c.service).to_string(),
                service: c.service,
                is_enabled: c.is_enabled,
                last_sync_at: c.last_sync_at,
                last_sync_status: c.last_sync_status,
                last_sync_error: c.last_sync_error,
            })
            .collect(),
        recent_logs: logs.into_iter().map(Into::into).collect(),
    }))
}
