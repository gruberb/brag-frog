//! The Today overview and meeting prep.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{Duration, Local};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::dto::CycleDto;
use super::entries::EntryDto;
use super::scope::resolve_cycle;
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::ai::get_ai_client;
use crate::ai::prompts::build_meeting_prep_prompt;
use crate::cycle::model::{MeetingPrepNote, Week};
use crate::cycle::service::dashboard::{OPEN_WORK_TYPES, compute_focus_days, filter_active_work};
use crate::identity::model::{PeopleAlias, User};
use crate::objectives::model::{DepartmentGoal, Priority, PriorityUpdate};
use crate::review::model::AiDocument;
use crate::worklog::model::BragEntry;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct WeekDto {
    pub id: i64,
    pub iso_week: i64,
    pub year: i64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct MeetingDto {
    #[serde(flatten)]
    #[ts(flatten)]
    pub entry: EntryDto,
    pub has_prep: bool,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TodayDay {
    pub date: String,
    /// `Monday`, `Tuesday`, ...
    pub label: String,
    pub is_today: bool,
    pub meetings: Vec<MeetingDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TodayPriority {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub tracking_status: Option<String>,
    pub due_date: Option<String>,
    pub goal_title: Option<String>,
    pub measure_type: Option<String>,
    pub measure_start: Option<f64>,
    pub measure_target: Option<f64>,
    pub measure_current: Option<f64>,
    pub has_blocker: bool,
    pub last_update_at: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TodayResponse {
    pub cycle: CycleDto,
    pub week: WeekDto,
    pub today: String,
    pub days: Vec<TodayDay>,
    pub active_work: Vec<EntryDto>,
    pub priorities: Vec<TodayPriority>,
    pub meeting_count: usize,
    pub focus_minutes: i64,
}

/// `GET /api/v1/today`: everything the Today page shows in one request.
pub async fn today(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
) -> ApiResult<TodayResponse> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    let phase = resolve_cycle(&state, auth.user_id, None).await?;
    let now = Local::now().naive_local().date();
    let today_str = now.format("%Y-%m-%d").to_string();
    let week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    let alias_map = PeopleAlias::alias_map(&state.db, auth.user_id).await?;
    let team_map = PeopleAlias::team_map(&state.db, auth.user_id).await?;

    // Whole cycle, not this week: entries are dated when the item was created (or last
    // updated), so a ticket opened weeks ago and still in progress is dated weeks ago too.
    let mut work_entries =
        BragEntry::list_for_phase_by_types(&state.db, phase.id, OPEN_WORK_TYPES, &auth.crypto)
            .await?;
    PeopleAlias::apply_to_entries(&mut work_entries, &alias_map, &team_map);

    let mut meetings = BragEntry::list_calendar_meetings_in_range(
        &state.db,
        phase.id,
        &week.start_date,
        &week.end_date,
        &auth.crypto,
    )
    .await?;
    PeopleAlias::apply_to_entries(&mut meetings, &alias_map, &team_map);

    let prep: HashMap<i64, bool> = MeetingPrepNote::list_for_week(
        &state.db,
        week.id,
        auth.user_id,
        &auth.crypto,
    )
    .await?
    .into_iter()
    .filter_map(|n| n.entry_id.map(|id| (id, n.notes.is_some() || n.meeting_goal.is_some())))
    .collect();

    let meeting_refs: Vec<&BragEntry> = meetings.iter().collect();
    let focus = compute_focus_days(
        &week.start_date,
        &meeting_refs,
        user.work_start_time.as_deref().unwrap_or("09:00"),
        user.work_end_time.as_deref().unwrap_or("17:00"),
    );
    let focus_minutes: i64 = focus.iter().map(|d| d.focus_minutes as i64).sum();
    // One column per weekday (Mon to Fri), including days without meetings.
    let start = chrono::NaiveDate::parse_from_str(&week.start_date, "%Y-%m-%d")
        .map_err(|_| ApiError::from(crate::kernel::error::AppError::Internal("bad week".into())))?;
    let days: Vec<TodayDay> = (0..5)
        .map(|i| {
            let d = start + Duration::days(i);
            let date = d.format("%Y-%m-%d").to_string();
            let day_meetings = meetings
                .iter()
                .filter(|m| m.occurred_at == date && m.deleted_at.is_none())
                .cloned()
                .map(|m| {
                    let has_prep = prep.get(&m.id).copied().unwrap_or(false);
                    MeetingDto {
                        entry: m.into(),
                        has_prep,
                    }
                })
                .collect();
            TodayDay {
                is_today: date == today_str,
                label: d.format("%A").to_string(),
                date,
                meetings: day_meetings,
            }
        })
        .collect();
    let meeting_count = days.iter().map(|d| d.meetings.len()).sum();

    let active_work: Vec<EntryDto> = filter_active_work(&work_entries)
        .into_iter()
        .cloned()
        .map(Into::into)
        .collect();

    let goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let mut out = Vec::new();
    for p in priorities.into_iter().filter(|p| p.status == "active") {
        let updates = PriorityUpdate::list_for_priority(&state.db, p.id, &auth.crypto).await?;
        // A blocker counts until a later non-blocker update resolves it.
        let has_blocker = updates.first().is_some_and(|u| u.is_blocker == 1);
        out.push(TodayPriority {
            id: p.id,
            title: p.title,
            status: p.status,
            tracking_status: p.tracking_status,
            due_date: p.due_date,
            goal_title: p
                .department_goal_id
                .and_then(|gid| goals.iter().find(|g| g.id == gid))
                .map(|g| g.title.clone()),
            measure_type: p.measure_type,
            measure_start: p.measure_start,
            measure_target: p.measure_target,
            measure_current: p.measure_current,
            has_blocker,
            last_update_at: updates.first().map(|u| u.created_at.clone()),
        });
    }

    Ok(Json(TodayResponse {
        cycle: phase.into(),
        week: WeekDto {
            id: week.id,
            iso_week: week.iso_week,
            year: week.year,
            start_date: week.start_date,
            end_date: week.end_date,
        },
        today: today_str,
        days,
        active_work,
        priorities: out,
        meeting_count,
        focus_minutes,
    }))
}

// ── Meeting prep ──

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PrepNoteDto {
    pub notes: Option<String>,
    pub meeting_goal: Option<String>,
    pub doc_urls: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PrepResponse {
    pub entry: EntryDto,
    pub note: Option<PrepNoteDto>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct SavePrepRequest {
    #[serde(default)]
    #[ts(optional)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub doc_urls: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub meeting_goal: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    /// Detach the meeting from its priority.
    #[serde(default)]
    pub clear_priority: bool,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct DraftPrepRequest {
    #[serde(default)]
    #[ts(optional)]
    pub context_snippets: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub meeting_goal: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PrepDraftResponse {
    pub content: String,
}

async fn owned_entry(
    state: &AppState,
    auth: &crate::identity::auth::middleware::AuthUser,
    entry_id: i64,
) -> Result<BragEntry, ApiError> {
    BragEntry::find_by_id(&state.db, entry_id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Meeting not found"))
}

fn blank(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// `GET /api/v1/meetings/{entry_id}/prep`
pub async fn get_prep(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> ApiResult<PrepResponse> {
    let entry = owned_entry(&state, &auth, entry_id).await?;
    let notes =
        MeetingPrepNote::list_for_week(&state.db, entry.week_id, auth.user_id, &auth.crypto)
            .await?;
    let note = notes
        .into_iter()
        .find(|n| n.entry_id == Some(entry_id))
        .map(|n| PrepNoteDto {
            notes: n.notes,
            meeting_goal: n.meeting_goal,
            doc_urls: n.doc_urls,
            updated_at: n.updated_at,
        });
    Ok(Json(PrepResponse {
        entry: entry.into(),
        note,
    }))
}

/// `PUT /api/v1/meetings/{entry_id}/prep`. The week comes from the entry, never from the client.
pub async fn put_prep(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(input): Json<SavePrepRequest>,
) -> ApiResult<PrepResponse> {
    let entry = owned_entry(&state, &auth, entry_id).await?;
    if let Some(pid) = input.priority_id
        && Priority::find_by_id(&state.db, pid, auth.user_id, &auth.crypto)
            .await?
            .is_none()
    {
        return Err(ApiError::invalid("Unknown priority"));
    }
    MeetingPrepNote::upsert(
        &state.db,
        auth.user_id,
        entry.week_id,
        entry_id,
        blank(&input.notes),
        blank(&input.doc_urls),
        blank(&input.meeting_goal),
        &auth.crypto,
    )
    .await?;
    // Prep notes also show up as the meeting's description in the logbook.
    if let Some(notes) = blank(&input.notes) {
        let enc = auth.crypto.encrypt(notes)?;
        sqlx::query(
            "UPDATE brag_entries SET description = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&enc)
        .bind(entry_id)
        .execute(&state.db)
        .await?;
    }
    if input.clear_priority || input.priority_id.is_some() {
        sqlx::query("UPDATE brag_entries SET priority_id = ? WHERE id = ?")
            .bind(if input.clear_priority { None } else { input.priority_id })
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    }
    get_prep(ApiAuth(auth), State(state), Path(entry_id)).await
}

/// `POST /api/v1/meetings/{entry_id}/prep/draft`: AI draft, not saved.
pub async fn draft_prep(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(input): Json<DraftPrepRequest>,
) -> ApiResult<PrepDraftResponse> {
    let ai = get_ai_client(&state, auth.user_id)
        .await
        .map_err(|e| ApiError::invalid(e.to_string().replace("Bad request: ", "")))?;
    let entry = owned_entry(&state, &auth, entry_id).await?;
    let phase = resolve_cycle(&state, auth.user_id, None).await?;
    let now = Local::now().naive_local().date();

    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;
    let linked_priority = entry
        .priority_id
        .and_then(|id| priorities.iter().find(|p| p.id == id));
    let linked_goal = linked_priority
        .and_then(|p| p.department_goal_id)
        .and_then(|gid| dept_goals.iter().find(|g| g.id == gid));

    let from = (now - Duration::days(21)).format("%Y-%m-%d").to_string();
    let to = now.format("%Y-%m-%d").to_string();
    let recent = BragEntry::list_for_phase_in_range(&state.db, phase.id, &from, &to, &auth.crypto)
        .await?
        .into_iter()
        .filter(|e| e.id != entry_id);
    let (recent_entries, other_recent): (Vec<BragEntry>, Vec<BragEntry>) = match linked_priority {
        Some(p) => {
            let (mut matched, rest): (Vec<_>, Vec<_>) =
                recent.partition(|e| e.priority_id == Some(p.id));
            matched.truncate(10);
            (matched, rest)
        }
        None => (Vec::new(), recent.collect()),
    };

    let existing = MeetingPrepNote::list_for_week(&state.db, entry.week_id, auth.user_id, &auth.crypto)
        .await?
        .into_iter()
        .find(|n| n.entry_id == Some(entry_id));
    let prior_preps = match entry.recurring_group.as_deref() {
        Some(rg) => AiDocument::list_for_recurring_group(&state.db, auth.user_id, rg, &auth.crypto)
            .await
            .unwrap_or_default()
            .into_iter()
            .take(2)
            .collect(),
        None => Vec::new(),
    };

    let prompt = build_meeting_prep_prompt(
        &entry,
        linked_goal,
        linked_priority,
        &recent_entries,
        &other_recent,
        input.context_snippets.as_deref().unwrap_or("").trim(),
        existing.as_ref(),
        blank(&input.meeting_goal),
        &prior_preps,
    );
    let content = ai.generate(&prompt).await?;
    Ok(Json(PrepDraftResponse { content }))
}
