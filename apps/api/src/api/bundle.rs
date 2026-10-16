//! Cycle download bundle: everything the client needs to render a Markdown, PDF or DOCX
//! export of a whole cycle (written review, priorities with updates, entries by week).

use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use ts_rs::TS;

use super::auth::ApiAuth;
use super::dto::CycleDto;
use super::entries::EntryDto;
use super::error::ApiResult;
use super::priorities::{PriorityUpdateDto, PrioritiesResponse, load_priorities};
use super::review::{ReviewResponse, load_review};
use super::scope::owned_cycle;
use crate::AppState;
use crate::cycle::model::Week;
use crate::identity::model::PeopleAlias;
use crate::objectives::model::PriorityUpdate;
use crate::worklog::model::BragEntry;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BundleWeek {
    pub iso_week: i64,
    pub year: i64,
    pub start_date: String,
    pub end_date: String,
    pub entries: Vec<EntryDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PriorityTimeline {
    pub priority_id: i64,
    pub updates: Vec<PriorityUpdateDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct CycleBundle {
    pub cycle: CycleDto,
    pub exported_at: String,
    pub review: ReviewResponse,
    pub priorities: PrioritiesResponse,
    pub timelines: Vec<PriorityTimeline>,
    /// Newest week first; weeks without entries are left out.
    pub weeks: Vec<BundleWeek>,
}

/// `GET /api/v1/cycles/{id}/bundle`. One decrypt pass over the cycle's entries.
pub async fn get_bundle(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(cycle_id): Path<i64>,
) -> ApiResult<CycleBundle> {
    let cycle = owned_cycle(&state, auth.user_id, cycle_id).await?;
    let review = load_review(&state, &auth.crypto, cycle.id).await?;
    let priorities = load_priorities(&state, &auth.crypto, cycle.id).await?;

    let mut timelines = Vec::new();
    let all = priorities
        .goals
        .iter()
        .flat_map(|g| g.priorities.iter())
        .chain(priorities.unassigned.iter());
    for p in all {
        let updates = PriorityUpdate::list_for_priority(&state.db, p.id, &auth.crypto).await?;
        if !updates.is_empty() {
            timelines.push(PriorityTimeline {
                priority_id: p.id,
                updates: updates.into_iter().map(Into::into).collect(),
            });
        }
    }

    let mut entries = BragEntry::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    let alias_map = PeopleAlias::alias_map(&state.db, auth.user_id).await?;
    let team_map = PeopleAlias::team_map(&state.db, auth.user_id).await?;
    PeopleAlias::apply_to_entries(&mut entries, &alias_map, &team_map);

    let mut weeks = Week::list_for_phase(&state.db, cycle.id).await?;
    weeks.sort_by(|a, b| b.start_date.cmp(&a.start_date));
    let weeks = weeks
        .into_iter()
        .filter_map(|w| {
            let mut in_week: Vec<EntryDto> = entries
                .iter()
                .filter(|e| e.week_id == w.id)
                .cloned()
                .map(Into::into)
                .collect();
            in_week.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
            (!in_week.is_empty()).then_some(BundleWeek {
                iso_week: w.iso_week,
                year: w.year,
                start_date: w.start_date,
                end_date: w.end_date,
                entries: in_week,
            })
        })
        .collect();

    Ok(Json(CycleBundle {
        cycle: cycle.into(),
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        review,
        priorities,
        timelines,
        weeks,
    }))
}
