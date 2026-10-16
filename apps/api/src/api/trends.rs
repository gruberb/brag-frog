//! `GET /api/v1/trends`: the cycle aggregation as typed JSON.

use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::cycle::model::BragPhase;
use crate::cycle::service::trends::{self, EntryFilter, Trends};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TrendRange {
    /// The last 28 days up to today.
    #[serde(rename = "4w")]
    FourWeeks,
    /// The calendar quarter containing today.
    Quarter,
    /// The whole cycle.
    #[default]
    Cycle,
}

#[derive(Debug, Deserialize)]
pub struct TrendsQuery {
    pub cycle_id: Option<i64>,
    #[serde(default)]
    pub range: TrendRange,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TrendsResponse {
    pub cycle_id: i64,
    pub range: TrendRange,
    pub from: String,
    pub to: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub trends: Trends,
}

/// Resolves a range to inclusive dates, clamped to the cycle.
fn range_bounds(range: TrendRange, cycle: &BragPhase, today: NaiveDate) -> (String, String) {
    let (from, to) = match range {
        TrendRange::Cycle => (cycle.start_date.clone(), cycle.end_date.clone()),
        TrendRange::FourWeeks => (
            (today - Duration::days(27)).format("%Y-%m-%d").to_string(),
            today.format("%Y-%m-%d").to_string(),
        ),
        TrendRange::Quarter => {
            let first_month = (today.month0() / 3) * 3 + 1;
            let start = NaiveDate::from_ymd_opt(today.year(), first_month, 1).unwrap_or(today);
            let next = if first_month == 10 {
                NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(today.year(), first_month + 3, 1)
            };
            let end = next.map(|d| d - Duration::days(1)).unwrap_or(today);
            (
                start.format("%Y-%m-%d").to_string(),
                end.format("%Y-%m-%d").to_string(),
            )
        }
    };
    // ISO dates compare correctly as strings.
    (
        from.max(cycle.start_date.clone()),
        to.min(cycle.end_date.clone()),
    )
}

pub async fn get_trends(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    q: Result<Query<TrendsQuery>, QueryRejection>,
) -> ApiResult<TrendsResponse> {
    // Keep the JSON error shape for malformed query strings such as an unknown range.
    let Query(q) = q.map_err(|e| ApiError::invalid(e.body_text()))?;
    let cycle = match q.cycle_id {
        Some(id) => BragPhase::find_by_id(&state.db, id, auth.user_id).await?,
        None => BragPhase::get_active(&state.db, auth.user_id).await?,
    }
    .ok_or_else(|| ApiError::not_found("Cycle not found"))?;

    let today = chrono::Local::now().date_naive();
    let (from, to) = range_bounds(q.range, &cycle, today);
    let filter = EntryFilter {
        from: Some(from.clone()),
        to: Some(to.clone()),
        ..Default::default()
    };
    let trends = trends::compute(
        &state.db,
        &auth.crypto,
        auth.user_id,
        &cycle,
        &filter,
        today,
    )
    .await?;

    Ok(Json(TrendsResponse {
        cycle_id: cycle.id,
        range: q.range,
        from,
        to,
        trends,
    }))
}
