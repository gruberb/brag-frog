//! Performance cycles (stored as `brag_phases`): list, create, activate, delete.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::dto::CycleDto;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::cycle::model::{BragPhase, CreatePhase};

/// A cycle plus the counts the Settings list shows.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct CycleSummaryDto {
    #[serde(flatten)]
    #[ts(flatten)]
    pub cycle: CycleDto,
    pub entry_count: i64,
    pub priority_count: i64,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreateCycleRequest {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct DeleteCycleRequest {
    /// Must equal the cycle's name. Deleting a cycle removes all of its data.
    pub confirm_name: String,
}

/// Loads a cycle the user owns or fails with 404. Every cycle-scoped endpoint starts here
/// because `brag_entries` and the other tables only reach a user through their cycle.
pub async fn owned_cycle(
    state: &AppState,
    cycle_id: i64,
    user_id: i64,
) -> Result<BragPhase, ApiError> {
    BragPhase::find_by_id(&state.db, cycle_id, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Cycle not found"))
}

fn parse_date(label: &str, value: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| ApiError::invalid(format!("{label} must be a date in YYYY-MM-DD format")))
}

/// `GET /api/v1/cycles`
pub async fn list(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<Vec<CycleSummaryDto>> {
    let cycles = BragPhase::list_for_user(&state.db, auth.user_id).await?;
    let counts: Vec<(i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT p.id,
               (SELECT COUNT(*) FROM brag_entries e JOIN weeks w ON e.week_id = w.id
                 WHERE w.phase_id = p.id AND e.deleted_at IS NULL),
               (SELECT COUNT(*) FROM priorities WHERE phase_id = p.id)
        FROM brag_phases p WHERE p.user_id = ?
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let out = cycles
        .into_iter()
        .map(|c| {
            let (entry_count, priority_count) = counts
                .iter()
                .find(|(id, _, _)| *id == c.id)
                .map(|(_, e, p)| (*e, *p))
                .unwrap_or((0, 0));
            CycleSummaryDto {
                cycle: c.into(),
                entry_count,
                priority_count,
            }
        })
        .collect();
    Ok(Json(out))
}

/// `POST /api/v1/cycles`: creates a cycle and makes it the active one (one active per user).
pub async fn create(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(input): Json<CreateCycleRequest>,
) -> Result<(StatusCode, Json<CycleDto>), ApiError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(ApiError::invalid("Name is required"));
    }
    let start = parse_date("start_date", &input.start_date)?;
    let end = parse_date("end_date", &input.end_date)?;
    if end < start {
        return Err(ApiError::invalid("end_date must not be before start_date"));
    }
    let phase = BragPhase::create(
        &state.db,
        auth.user_id,
        &CreatePhase {
            name: name.to_string(),
            start_date: input.start_date,
            end_date: input.end_date,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(phase.into())))
}

/// `POST /api/v1/cycles/{id}/activate`
pub async fn activate(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<CycleDto> {
    // `set_active` deactivates every cycle first, so an unowned id must be rejected before it.
    owned_cycle(&state, id, auth.user_id).await?;
    BragPhase::set_active(&state.db, id, auth.user_id).await?;
    let phase = owned_cycle(&state, id, auth.user_id).await?;
    Ok(Json(phase.into()))
}

/// `DELETE /api/v1/cycles/{id}`: cascading delete, guarded by typing the cycle name.
pub async fn delete(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<DeleteCycleRequest>,
) -> Result<StatusCode, ApiError> {
    let phase = owned_cycle(&state, id, auth.user_id).await?;
    if input.confirm_name != phase.name {
        return Err(ApiError::invalid("confirm_name does not match the cycle name"));
    }
    BragPhase::delete(&state.db, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
