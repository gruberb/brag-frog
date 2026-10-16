//! Priorities, department goals, progress updates and the Lattice CSV import.
//!
//! Handlers reuse the `objectives` repo and service layer. `PUT` bodies are patches: a field
//! that is absent keeps its stored value, `null` or a blank string clears it. The repo `update`
//! functions overwrite every column, so the handlers merge with the stored row first.

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use super::scope::resolve_cycle;
use crate::AppState;
use crate::objectives::import;
use crate::objectives::model::{
    CreateDepartmentGoal, CreatePriority, DepartmentGoal, PostPriorityUpdate, Priority,
    PriorityUpdate, UpdateDepartmentGoal, UpdatePriority,
};
use crate::objectives::service::{group_by_department_goal, import_lattice_rows, sort_priorities};

const PRIORITY_STATUSES: &[&str] = &["not_started", "active", "on_hold", "completed", "cancelled"];
const GOAL_STATUSES: &[&str] = &["not_started", "in_progress", "on_hold", "completed"];
const TRACKING_STATUSES: &[&str] = &[
    "on_track",
    "progressing",
    "off_track",
    "complete",
    "incomplete",
    "no_update",
];
const TIERS: &[&str] = &["department", "team", "individual"];
const LEVELS: &[&str] = &["p0", "p1", "p2", "p3", "p4"];
const SCOPES: &[&str] = &["small", "medium", "large", "xlarge"];
const MEASURE_TYPES: &[&str] = &["percent", "binary", "number"];

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Serialize, TS)]
#[ts(export)]
pub struct GoalDto {
    pub id: i64,
    pub cycle_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i64,
    pub source: String,
    pub created_at: String,
}

impl From<DepartmentGoal> for GoalDto {
    fn from(g: DepartmentGoal) -> Self {
        Self {
            id: g.id,
            cycle_id: g.phase_id,
            title: g.title,
            description: g.description,
            status: g.status,
            sort_order: g.sort_order,
            source: g.source,
            created_at: g.created_at,
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PriorityUpdateDto {
    pub id: i64,
    pub priority_id: i64,
    pub tracking_status: Option<String>,
    pub measure_value: Option<f64>,
    pub comment: Option<String>,
    pub is_blocker: bool,
    pub tradeoff_text: Option<String>,
    pub created_at: String,
}

impl From<PriorityUpdate> for PriorityUpdateDto {
    fn from(u: PriorityUpdate) -> Self {
        Self {
            id: u.id,
            priority_id: u.priority_id,
            tracking_status: u.tracking_status,
            measure_value: u.measure_value,
            comment: u.comment,
            is_blocker: u.is_blocker != 0,
            tradeoff_text: u.tradeoff_text,
            created_at: u.created_at,
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PriorityDto {
    pub id: i64,
    pub cycle_id: i64,
    pub title: String,
    pub status: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub scope: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub impact_narrative: Option<String>,
    pub department_goal_id: Option<i64>,
    pub created_at: String,
    pub priority_level: Option<String>,
    pub measure_type: Option<String>,
    pub measure_start: Option<f64>,
    pub measure_target: Option<f64>,
    pub measure_current: Option<f64>,
    pub description: Option<String>,
    pub tracking_status: Option<String>,
    pub due_date: Option<String>,
    pub tier: Option<String>,
    pub entry_count: i64,
    pub latest_update: Option<PriorityUpdateDto>,
}

impl PriorityDto {
    fn new(p: Priority, entry_count: i64, latest_update: Option<PriorityUpdate>) -> Self {
        Self {
            id: p.id,
            cycle_id: p.phase_id,
            title: p.title,
            status: p.status,
            color: p.color,
            sort_order: p.sort_order,
            scope: p.scope,
            started_at: p.started_at,
            completed_at: p.completed_at,
            impact_narrative: p.impact_narrative,
            department_goal_id: p.department_goal_id,
            created_at: p.created_at,
            priority_level: p.priority_level,
            measure_type: p.measure_type,
            measure_start: p.measure_start,
            measure_target: p.measure_target,
            measure_current: p.measure_current,
            description: p.description,
            tracking_status: p.tracking_status,
            due_date: p.due_date,
            tier: p.tier,
            entry_count,
            latest_update: latest_update.map(Into::into),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct GoalGroup {
    pub goal: GoalDto,
    pub priorities: Vec<PriorityDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PriorityCounts {
    pub total: usize,
    pub active: usize,
    pub not_started: usize,
    pub on_hold: usize,
    pub completed: usize,
    pub cancelled: usize,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PrioritiesResponse {
    pub cycle_id: i64,
    pub goals: Vec<GoalGroup>,
    pub unassigned: Vec<PriorityDto>,
    pub counts: PriorityCounts,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CycleQuery {
    #[ts(optional)]
    pub cycle_id: Option<i64>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreateGoalRequest {
    #[ts(optional)]
    pub cycle_id: Option<i64>,
    pub title: String,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub status: Option<String>,
}

/// A patch field: absent keeps the stored value, `null` clears it, a value replaces it.
/// Plain `Option<Option<T>>` cannot tell absent from `null`, so the wire type is its own
/// newtype and ts-rs is pointed at `Option<T>` through `#[ts(as = ...)]`.
#[derive(Debug)]
pub struct Patch<T>(Option<Option<T>>);

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(de).map(|v| Self(Some(v)))
    }
}

impl<T> Patch<T> {
    /// The patched value, or `current` when the field was absent.
    fn apply(self, current: Option<T>) -> Option<T> {
        self.0.unwrap_or(current)
    }
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdateGoalRequest {
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub description: Patch<String>,
    #[serde(default)]
    #[ts(optional)]
    pub status: Option<String>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreatePriorityRequest {
    #[ts(optional)]
    pub cycle_id: Option<i64>,
    pub title: String,
    #[serde(default)]
    #[ts(optional)]
    pub status: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub scope: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub impact_narrative: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub department_goal_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_level: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub measure_type: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub measure_start: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub measure_target: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub tracking_status: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub due_date: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub tier: Option<String>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdatePriorityRequest {
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub status: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub scope: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub impact_narrative: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<i64>")]
    pub department_goal_id: Patch<i64>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub started_at: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub completed_at: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub priority_level: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub measure_type: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<f64>")]
    pub measure_start: Patch<f64>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<f64>")]
    pub measure_target: Patch<f64>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<f64>")]
    pub measure_current: Patch<f64>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub description: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub tracking_status: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub due_date: Patch<String>,
    #[serde(default)]
    #[ts(optional = nullable, as = "Option<String>")]
    pub tier: Patch<String>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct PostUpdateRequest {
    #[serde(default)]
    #[ts(optional)]
    pub tracking_status: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub measure_value: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub comment: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub is_blocker: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub tradeoff_text: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ImportResult {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Blank strings mean "not set" on the wire, matching the form behaviour of the legacy UI.
fn non_blank(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.trim().is_empty())
}

fn require_title(title: &str) -> Result<(), ApiError> {
    if title.trim().is_empty() {
        return Err(ApiError::invalid("title must not be empty"));
    }
    Ok(())
}

fn check_enum(field: &str, value: Option<&str>, allowed: &[&str]) -> Result<(), ApiError> {
    match value {
        Some(v) if !allowed.contains(&v) => Err(ApiError::invalid(format!(
            "{field} must be one of: {}",
            allowed.join(", ")
        ))),
        _ => Ok(()),
    }
}

/// Blank strings clear a text field, like `null`.
fn merge_str(patch: Patch<String>, current: Option<String>) -> Option<String> {
    non_blank(patch.apply(current))
}

/// A department goal the user owns, as a target for `department_goal_id`.
async fn require_goal(
    state: &AppState,
    user_id: i64,
    goal_id: i64,
    crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<DepartmentGoal, ApiError> {
    DepartmentGoal::find_by_id(&state.db, goal_id, user_id, crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Department goal not found"))
}

async fn require_priority(
    state: &AppState,
    user_id: i64,
    id: i64,
    crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<Priority, ApiError> {
    Priority::find_by_id(&state.db, id, user_id, crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Priority not found"))
}

/// A `department_goal_id` must belong to the same cycle as the priority it is attached to.
async fn check_goal_in_cycle(
    state: &AppState,
    user_id: i64,
    goal_id: i64,
    cycle_id: i64,
    crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<(), ApiError> {
    let goal = require_goal(state, user_id, goal_id, crypto).await?;
    if goal.phase_id != cycle_id {
        return Err(ApiError::invalid(
            "department_goal_id belongs to another cycle",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers: listing
// ---------------------------------------------------------------------------

/// `GET /api/v1/priorities?cycle_id`
pub async fn list(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Query(q): Query<CycleQuery>,
) -> ApiResult<PrioritiesResponse> {
    let cycle = resolve_cycle(&state, auth.user_id, q.cycle_id).await?;
    Ok(Json(load_priorities(&state, &auth.crypto, cycle.id).await?))
}

/// Goals with their priorities for one cycle. Shared with the cycle bundle export.
pub async fn load_priorities(
    state: &AppState,
    crypto: &crate::kernel::crypto::UserCrypto,
    cycle_id: i64,
) -> Result<PrioritiesResponse, ApiError> {
    let mut goals = DepartmentGoal::list_for_phase(&state.db, cycle_id, crypto).await?;
    // Completed goals sink to the bottom; `sort_by_key` is stable so manual order survives.
    goals.sort_by_key(|g| g.status == "completed");
    let mut priorities = Priority::list_for_phase(&state.db, cycle_id, crypto).await?;
    sort_priorities(&mut priorities);

    let counts = priority_counts(&priorities);
    let entry_counts = Priority::entry_counts_for_phase(&state.db, cycle_id).await?;
    let mut latest = PriorityUpdate::latest_for_phase(&state.db, cycle_id, crypto).await?;

    let (mut by_goal, unassigned) = group_by_department_goal(&priorities);
    let mut to_dto = |p: &Priority| {
        let count = entry_counts.get(&p.id).copied().unwrap_or(0);
        PriorityDto::new(p.clone(), count, latest.remove(&p.id))
    };

    let goals = goals
        .into_iter()
        .map(|goal| {
            let members = by_goal.remove(&goal.id.to_string()).unwrap_or_default();
            GoalGroup {
                priorities: members.into_iter().map(&mut to_dto).collect(),
                goal: goal.into(),
            }
        })
        .collect();
    let unassigned = unassigned.into_iter().map(&mut to_dto).collect();

    Ok(PrioritiesResponse {
        cycle_id,
        goals,
        unassigned,
        counts,
    })
}

fn priority_counts(priorities: &[Priority]) -> PriorityCounts {
    let n = |status: &str| priorities.iter().filter(|p| p.status == status).count();
    PriorityCounts {
        total: priorities.len(),
        active: n("active"),
        not_started: n("not_started"),
        on_hold: n("on_hold"),
        completed: n("completed"),
        cancelled: n("cancelled"),
    }
}

/// Re-reads one priority with its derived fields, for create/update responses.
async fn priority_response(
    state: &AppState,
    priority: Priority,
    crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<PriorityDto, ApiError> {
    let counts = Priority::entry_counts_for_phase(&state.db, priority.phase_id).await?;
    let mut latest = PriorityUpdate::latest_for_phase(&state.db, priority.phase_id, crypto).await?;
    let count = counts.get(&priority.id).copied().unwrap_or(0);
    let update = latest.remove(&priority.id);
    Ok(PriorityDto::new(priority, count, update))
}

// ---------------------------------------------------------------------------
// Handlers: goals
// ---------------------------------------------------------------------------

/// `POST /api/v1/goals`
pub async fn create_goal(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<(StatusCode, Json<GoalDto>), ApiError> {
    require_title(&req.title)?;
    let status = non_blank(req.status);
    check_enum("status", status.as_deref(), GOAL_STATUSES)?;
    let cycle = resolve_cycle(&state, auth.user_id, req.cycle_id).await?;

    let goal = DepartmentGoal::create(
        &state.db,
        cycle.id,
        auth.user_id,
        &CreateDepartmentGoal {
            title: req.title.trim().to_string(),
            description: non_blank(req.description),
            status,
        },
        None,
        &auth.crypto,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(goal.into())))
}

/// `PUT /api/v1/goals/{id}`. Completing a goal completes its open priorities.
pub async fn update_goal(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGoalRequest>,
) -> ApiResult<GoalDto> {
    let current = require_goal(&state, auth.user_id, id, &auth.crypto).await?;
    let title = req.title.unwrap_or(current.title);
    require_title(&title)?;
    check_enum("status", req.status.as_deref(), GOAL_STATUSES)?;

    let input = UpdateDepartmentGoal {
        title: title.trim().to_string(),
        description: merge_str(req.description, current.description),
        status: Some(req.status.unwrap_or(current.status)),
    };
    let goal = DepartmentGoal::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;
    if input.status.as_deref() == Some("completed") {
        Priority::complete_all_for_department_goal(&state.db, id, auth.user_id).await?;
    }
    Ok(Json(goal.into()))
}

/// `DELETE /api/v1/goals/{id}`. Child priorities become unassigned.
pub async fn delete_goal(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require_goal(&state, auth.user_id, id, &auth.crypto).await?;
    DepartmentGoal::delete(&state.db, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Handlers: priorities
// ---------------------------------------------------------------------------

fn check_priority_enums(
    status: Option<&str>,
    scope: Option<&str>,
    level: Option<&str>,
    measure_type: Option<&str>,
    tracking: Option<&str>,
    tier: Option<&str>,
) -> Result<(), ApiError> {
    check_enum("status", status, PRIORITY_STATUSES)?;
    check_enum("scope", scope, SCOPES)?;
    check_enum("priority_level", level, LEVELS)?;
    check_enum("measure_type", measure_type, MEASURE_TYPES)?;
    check_enum("tracking_status", tracking, TRACKING_STATUSES)?;
    check_enum("tier", tier, TIERS)
}

/// `POST /api/v1/priorities`
pub async fn create_priority(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(req): Json<CreatePriorityRequest>,
) -> Result<(StatusCode, Json<PriorityDto>), ApiError> {
    require_title(&req.title)?;
    let input = CreatePriority {
        title: req.title.trim().to_string(),
        status: non_blank(req.status),
        scope: non_blank(req.scope),
        impact_narrative: non_blank(req.impact_narrative),
        department_goal_id: req.department_goal_id,
        priority_level: non_blank(req.priority_level),
        measure_type: non_blank(req.measure_type),
        measure_start: req.measure_start,
        measure_target: req.measure_target,
        description: non_blank(req.description),
        tracking_status: non_blank(req.tracking_status),
        due_date: non_blank(req.due_date),
        tier: non_blank(req.tier),
    };
    check_priority_enums(
        input.status.as_deref(),
        input.scope.as_deref(),
        input.priority_level.as_deref(),
        input.measure_type.as_deref(),
        input.tracking_status.as_deref(),
        input.tier.as_deref(),
    )?;
    let cycle = resolve_cycle(&state, auth.user_id, req.cycle_id).await?;
    if let Some(goal_id) = input.department_goal_id {
        check_goal_in_cycle(&state, auth.user_id, goal_id, cycle.id, &auth.crypto).await?;
    }

    let priority =
        Priority::create(&state.db, cycle.id, auth.user_id, &input, &auth.crypto).await?;
    let dto = priority_response(&state, priority, &auth.crypto).await?;
    Ok((StatusCode::CREATED, Json(dto)))
}

/// `PUT /api/v1/priorities/{id}`
pub async fn update_priority(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePriorityRequest>,
) -> ApiResult<PriorityDto> {
    let cur = require_priority(&state, auth.user_id, id, &auth.crypto).await?;
    let title = req.title.unwrap_or(cur.title);
    require_title(&title)?;

    let input = UpdatePriority {
        title: title.trim().to_string(),
        status: Some(req.status.unwrap_or(cur.status)),
        scope: merge_str(req.scope, cur.scope),
        impact_narrative: merge_str(req.impact_narrative, cur.impact_narrative),
        department_goal_id: req.department_goal_id.apply(cur.department_goal_id),
        started_at: merge_str(req.started_at, cur.started_at),
        completed_at: merge_str(req.completed_at, cur.completed_at),
        priority_level: merge_str(req.priority_level, cur.priority_level),
        measure_type: merge_str(req.measure_type, cur.measure_type),
        measure_start: req.measure_start.apply(cur.measure_start),
        measure_target: req.measure_target.apply(cur.measure_target),
        measure_current: req.measure_current.apply(cur.measure_current),
        description: merge_str(req.description, cur.description),
        tracking_status: merge_str(req.tracking_status, cur.tracking_status),
        due_date: merge_str(req.due_date, cur.due_date),
        tier: merge_str(req.tier, cur.tier),
    };
    check_priority_enums(
        input.status.as_deref(),
        input.scope.as_deref(),
        input.priority_level.as_deref(),
        input.measure_type.as_deref(),
        input.tracking_status.as_deref(),
        input.tier.as_deref(),
    )?;
    if let Some(goal_id) = input.department_goal_id {
        check_goal_in_cycle(&state, auth.user_id, goal_id, cur.phase_id, &auth.crypto).await?;
    }

    let priority = Priority::update(&state.db, id, auth.user_id, &input, &auth.crypto).await?;
    Ok(Json(
        priority_response(&state, priority, &auth.crypto).await?,
    ))
}

/// `DELETE /api/v1/priorities/{id}`. Linked entries become unassigned.
pub async fn delete_priority(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require_priority(&state, auth.user_id, id, &auth.crypto).await?;
    Priority::delete(&state.db, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Handlers: updates
// ---------------------------------------------------------------------------

/// `GET /api/v1/priorities/{id}/updates`, newest first.
pub async fn list_updates(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Vec<PriorityUpdateDto>> {
    require_priority(&state, auth.user_id, id, &auth.crypto).await?;
    let updates = PriorityUpdate::list_for_priority(&state.db, id, &auth.crypto).await?;
    Ok(Json(updates.into_iter().map(Into::into).collect()))
}

/// `POST /api/v1/priorities/{id}/updates`. Also syncs tracking status and the current
/// measure value onto the priority (done by the repo).
pub async fn post_update(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<PostUpdateRequest>,
) -> Result<(StatusCode, Json<PriorityUpdateDto>), ApiError> {
    require_priority(&state, auth.user_id, id, &auth.crypto).await?;
    let input = PostPriorityUpdate {
        tracking_status: non_blank(req.tracking_status),
        measure_value: req.measure_value,
        comment: non_blank(req.comment),
        is_blocker: req.is_blocker.map(i64::from),
        tradeoff_text: non_blank(req.tradeoff_text),
    };
    check_enum(
        "tracking_status",
        input.tracking_status.as_deref(),
        TRACKING_STATUSES,
    )?;
    if input.tracking_status.is_none()
        && input.measure_value.is_none()
        && input.comment.is_none()
        && input.is_blocker.is_none()
        && input.tradeoff_text.is_none()
    {
        return Err(ApiError::invalid("an update needs at least one field"));
    }
    let update = PriorityUpdate::create(&state.db, id, auth.user_id, &input, &auth.crypto).await?;
    Ok((StatusCode::CREATED, Json(update.into())))
}

// ---------------------------------------------------------------------------
// Handler: Lattice import
// ---------------------------------------------------------------------------

/// `POST /api/v1/priorities/import?cycle_id` with multipart field `csv_file`.
///
/// The service does not report per-row outcomes, so `created` is the growth in goals plus
/// priorities (parent goals auto-created from `Parent goal` columns count) and `updated` is
/// the remaining importable rows, which matched an existing `external_id`.
pub async fn import_csv(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Query(q): Query<CycleQuery>,
    mut multipart: Multipart,
) -> ApiResult<ImportResult> {
    let cycle = resolve_cycle(&state, auth.user_id, q.cycle_id).await?;

    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::invalid(format!("Invalid multipart body: {e}")))?
    {
        if field.name() == Some("csv_file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::invalid(format!("Could not read csv_file: {e}")))?;
            bytes = Some(data);
            break;
        }
    }
    let bytes = bytes
        .filter(|b| !b.is_empty())
        .ok_or_else(|| ApiError::invalid("No CSV file provided in field csv_file"))?;

    let rows = import::parse_lattice_csv(&bytes)
        .map_err(|e| ApiError::invalid(format!("CSV parse error: {e}")))?;

    let mut errors = Vec::new();
    let mut importable = Vec::with_capacity(rows.len());
    for (i, row) in rows.into_iter().enumerate() {
        if row.goal_name.trim().is_empty() {
            errors.push(format!("Row {}: missing goal name", i + 2));
        } else {
            importable.push(row);
        }
    }
    let skipped = errors.len();

    let before = row_total(&state, cycle.id).await?;
    import_lattice_rows(&state.db, cycle.id, auth.user_id, &importable, &auth.crypto).await?;
    let created = (row_total(&state, cycle.id).await? - before).max(0) as usize;

    Ok(Json(ImportResult {
        created,
        updated: importable.len().saturating_sub(created),
        skipped,
        errors,
    }))
}

async fn row_total(state: &AppState, cycle_id: i64) -> Result<i64, ApiError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM department_goals WHERE phase_id = ?)
              + (SELECT COUNT(*) FROM priorities WHERE phase_id = ?)",
    )
    .bind(cycle_id)
    .bind(cycle_id)
    .fetch_one(&state.db)
    .await?;
    Ok(n)
}
