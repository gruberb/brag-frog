//! Saved reports: the current Logbook filter set turned into a snapshot document.
//!
//! A report stores the filters, a stats snapshot and an optional AI narrative. There is no
//! edit endpoint: a snapshot must not drift when entries change, so regenerating means
//! creating a new report.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use super::scope::{owned_cycle, resolve_cycle};
use crate::AppState;
use crate::ai::get_ai_client;
use crate::ai::prompts::build_report_prompt;
use crate::cycle::service::logbook_insights::{AnalyzePageQuery, apply_in_memory_filters};
use crate::identity::model::PeopleAlias;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::review::model::{NewReport, Report, ReportFilters};
use crate::review::service::report_stats::{
    KEY_ENTRY_LIMIT, ReportKeyEntry, ReportStats, compute_report_stats, default_title,
    describe_filters, pick_key_entries,
};
use crate::worklog::model::BragEntry;

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreateReportRequest {
    pub cycle_id: i64,
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    pub filters: ReportFilters,
    #[serde(default)]
    pub with_ai: bool,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ReportListQuery {
    #[ts(optional)]
    pub cycle_id: Option<i64>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReportListItem {
    pub id: i64,
    pub cycle_id: i64,
    pub title: String,
    pub filters: ReportFilters,
    pub entry_count: usize,
    pub generated_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReportNarrative {
    /// Empty when no narrative was generated.
    pub content: String,
    pub content_format: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReportDto {
    pub id: i64,
    pub cycle_id: i64,
    pub title: String,
    pub filters: ReportFilters,
    pub entry_count: usize,
    pub generated_at: String,
    pub updated_at: String,
    pub model_used: Option<String>,
    pub stats: ReportStats,
    pub narrative: ReportNarrative,
    pub key_entries: Vec<ReportKeyEntry>,
    /// Why no narrative was produced although `with_ai` was set. Only present on creation.
    pub ai_error: Option<String>,
}

/// What `stats_json` stores: the numbers and the evidence as they were at generation time.
#[derive(Serialize, Deserialize)]
struct Snapshot {
    stats: ReportStats,
    key_entries: Vec<ReportKeyEntry>,
}

fn parse_filters(report: &Report) -> Result<ReportFilters, ApiError> {
    serde_json::from_str(&report.filters_json)
        .map_err(|e| internal(format!("stored report filters are unreadable: {e}")))
}

fn parse_snapshot(report: &Report) -> Result<Snapshot, ApiError> {
    serde_json::from_str(&report.stats_json)
        .map_err(|e| internal(format!("stored report stats are unreadable: {e}")))
}

fn internal(cause: String) -> ApiError {
    crate::kernel::error::AppError::Internal(cause).into()
}

fn to_dto(report: Report, ai_error: Option<String>) -> Result<ReportDto, ApiError> {
    let filters = parse_filters(&report)?;
    let snapshot = parse_snapshot(&report)?;
    Ok(ReportDto {
        id: report.id,
        cycle_id: report.phase_id,
        title: report.title,
        filters,
        entry_count: snapshot.stats.entries,
        generated_at: report.generated_at,
        updated_at: report.updated_at,
        model_used: report.model_used,
        stats: snapshot.stats,
        narrative: ReportNarrative {
            content: report.content,
            content_format: report.content_format,
        },
        key_entries: snapshot.key_entries,
        ai_error,
    })
}

fn check_date(field: &str, value: &Option<String>) -> Result<(), ApiError> {
    match value {
        Some(v) if NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() => Err(ApiError::invalid(
            format!("{field} must be a YYYY-MM-DD date"),
        )),
        _ => Ok(()),
    }
}

fn csv(value: &Option<String>) -> Vec<String> {
    value
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Runs the Logbook query for a filter set: SQL for dates, priority, goal, type and source,
/// then the in-memory filters for the encrypted fields. Future synced entries (upcoming
/// calendar events) are left out, as in the Logbook.
async fn filtered_entries(
    state: &AppState,
    user_id: i64,
    cycle_id: i64,
    from: &str,
    to: &str,
    f: &ReportFilters,
    crypto: &crate::kernel::crypto::UserCrypto,
) -> Result<Vec<BragEntry>, ApiError> {
    let mut entries = BragEntry::list_for_phase_filtered(
        &state.db,
        cycle_id,
        f.priority_id,
        f.goal_id,
        &csv(&f.entry_type),
        Some(from),
        Some(to),
        &csv(&f.source),
        crypto,
    )
    .await?;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today.as_str());

    // Aliases first so `person` and `team` match the names the Logbook shows.
    let alias_map = PeopleAlias::alias_map(&state.db, user_id).await?;
    let team_map = PeopleAlias::team_map(&state.db, user_id).await?;
    PeopleAlias::apply_to_entries(&mut entries, &alias_map, &team_map);

    let query = AnalyzePageQuery {
        department_goal_id: None,
        priority_id: None,
        category: None,
        source: None,
        team: f.team.clone(),
        collaborator: f.person.clone(),
        search: f.q.clone(),
        no_priority: None,
        no_team: None,
        no_collaborator: None,
        reach: f.reach.clone(),
        complexity: f.complexity.clone(),
        role: f.role.clone(),
    };
    apply_in_memory_filters(&mut entries, &query);
    Ok(entries)
}

/// `POST /api/v1/reports`
pub async fn create_report(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateReportRequest>,
) -> Result<(StatusCode, Json<ReportDto>), ApiError> {
    let cycle = owned_cycle(&state, auth.user_id, req.cycle_id).await?;
    let filters = req.filters;
    check_date("from", &filters.from)?;
    check_date("to", &filters.to)?;
    let from = filters
        .from
        .clone()
        .unwrap_or_else(|| cycle.start_date.clone());
    let to = filters.to.clone().unwrap_or_else(|| cycle.end_date.clone());
    if from > to {
        return Err(ApiError::invalid("from must not be after to"));
    }

    let priorities = Priority::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    // Ids from the filter must belong to this cycle, which also proves ownership.
    if let Some(id) = filters.priority_id
        && !priorities.iter().any(|p| p.id == id)
    {
        return Err(ApiError::not_found("Priority not found"));
    }
    if let Some(id) = filters.goal_id
        && !dept_goals.iter().any(|g| g.id == id)
    {
        return Err(ApiError::not_found("Department goal not found"));
    }

    let entries = filtered_entries(
        &state,
        auth.user_id,
        cycle.id,
        &from,
        &to,
        &filters,
        &auth.crypto,
    )
    .await?;
    if entries.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "empty_selection",
            "No entries match these filters",
        ));
    }

    let priority_names: HashMap<i64, String> =
        priorities.iter().map(|p| (p.id, p.title.clone())).collect();
    let snapshot = Snapshot {
        stats: compute_report_stats(&entries, &priority_names),
        key_entries: pick_key_entries(&entries, KEY_ENTRY_LIMIT),
    };

    let title = req
        .title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            let name = filters.priority_id.and_then(|id| priority_names.get(&id));
            default_title(name.map(String::as_str), &from, &to)
        });

    let (narrative, model_used, ai_error) = if req.with_ai {
        let stats_line = format!(
            "{} entries, {} people, {} repositories, {} teams",
            snapshot.stats.entries,
            snapshot.stats.people,
            snapshot.stats.repos,
            snapshot.stats.teams
        );
        let prompt = build_report_prompt(
            &title,
            &from,
            &to,
            &describe_filters(&filters),
            &stats_line,
            &entries,
            &priorities,
            &dept_goals,
        );
        generate_narrative(&state, auth.user_id, &prompt).await
    } else {
        (String::new(), None, None)
    };

    let report = Report::create(
        &state.db,
        auth.user_id,
        cycle.id,
        &NewReport {
            title: &title,
            filters_json: &serde_json::to_string(&filters)
                .map_err(|e| internal(format!("filters not serialisable: {e}")))?,
            content: &narrative,
            content_format: "markdown",
            stats_json: &serde_json::to_string(&snapshot)
                .map_err(|e| internal(format!("stats not serialisable: {e}")))?,
            model_used: model_used.as_deref(),
        },
        &auth.crypto,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(to_dto(report, ai_error)?)))
}

/// Never fails the request: a missing integration or a provider error becomes `ai_error`
/// and the report is saved without a narrative.
async fn generate_narrative(
    state: &AppState,
    user_id: i64,
    prompt: &str,
) -> (String, Option<String>, Option<String>) {
    let client = match get_ai_client(state, user_id).await {
        Ok(c) => c,
        Err(e) => {
            return (
                String::new(),
                None,
                Some(e.to_string().replace("Bad request: ", "")),
            );
        }
    };
    match client.generate(prompt).await {
        Ok(text) => (text, Some(state.config.ai_model.clone()), None),
        Err(e) => {
            tracing::error!("Report narrative failed: {e:?}");
            (
                String::new(),
                None,
                Some(
                    "The AI service did not return a narrative. The report was saved without one."
                        .to_string(),
                ),
            )
        }
    }
}

/// `GET /api/v1/reports?cycle_id`, newest first.
pub async fn list_reports(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Query(q): Query<ReportListQuery>,
) -> ApiResult<Vec<ReportListItem>> {
    let cycle = resolve_cycle(&state, auth.user_id, q.cycle_id).await?;
    let reports = Report::list_for_phase(&state.db, auth.user_id, cycle.id, &auth.crypto).await?;
    let items = reports
        .into_iter()
        .map(|r| {
            Ok(ReportListItem {
                filters: parse_filters(&r)?,
                entry_count: parse_snapshot(&r)?.stats.entries,
                id: r.id,
                cycle_id: r.phase_id,
                title: r.title,
                generated_at: r.generated_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(Json(items))
}

/// `GET /api/v1/reports/{id}`
pub async fn get_report(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<ReportDto> {
    let report = Report::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Report not found"))?;
    Ok(Json(to_dto(report, None)?))
}

/// `DELETE /api/v1/reports/{id}`
pub async fn delete_report(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if Report::delete(&state.db, id, auth.user_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Report not found"))
    }
}
