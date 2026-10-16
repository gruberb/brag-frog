//! Logbook entries: filtered listing with cursor paging, CRUD, bulk edit, meeting
//! classification, exclusion of synced items, facets and entry type metadata.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::cycles::owned_cycle;
use super::error::{ApiError, ApiResult};
use super::facets;
use crate::AppState;
use crate::cycle::model::{BragPhase, Week};
use crate::cycle::routes::logbook_insights::{
    AnalyzePageQuery, apply_in_memory_filters, category_to_entry_types,
};
use crate::identity::model::PeopleAlias;
use crate::kernel::error::AppError;
use crate::objectives::model::Priority;
use crate::worklog::model::{BragEntry, BulkUpdateEntries, CreateEntry, EntryType, UpdateEntry};

const FACET_TTL: Duration = Duration::from_secs(60);
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

// ── Wire types ──

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DecisionDto {
    pub alternatives: Option<String>,
    pub reasoning: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EntryDto {
    pub id: i64,
    pub week_id: i64,
    pub source: String,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub entry_type: String,
    pub status: Option<String>,
    pub repository: Option<String>,
    pub occurred_at: String,
    pub teams: Vec<String>,
    pub collaborators: Vec<String>,
    pub outcome_statement: Option<String>,
    pub role: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    pub priority_id: Option<i64>,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub evidence_urls: Option<String>,
    pub impact_tags: Option<String>,
    pub decision: Option<DecisionDto>,
    pub is_synced: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Splits the comma-separated storage format into trimmed, non-empty items.
pub fn split_csv(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn join_csv(items: &Option<Vec<String>>) -> Option<String> {
    items.as_ref().map(|v| {
        v.iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    })
}

impl From<BragEntry> for EntryDto {
    fn from(e: BragEntry) -> Self {
        let decision = if e.decision_alternatives.is_some()
            || e.decision_reasoning.is_some()
            || e.decision_outcome.is_some()
        {
            Some(DecisionDto {
                alternatives: e.decision_alternatives.clone(),
                reasoning: e.decision_reasoning.clone(),
                outcome: e.decision_outcome.clone(),
            })
        } else {
            None
        };
        let is_synced = e.is_synced();
        Self {
            id: e.id,
            week_id: e.week_id,
            source: e.source,
            source_id: e.source_id,
            source_url: e.source_url,
            title: e.title,
            description: e.description,
            entry_type: e.entry_type,
            status: e.status,
            repository: e.repository,
            occurred_at: e.occurred_at,
            teams: split_csv(&e.teams),
            collaborators: split_csv(&e.collaborators),
            outcome_statement: e.outcome_statement,
            role: e.role,
            reach: e.reach,
            complexity: e.complexity,
            priority_id: e.priority_id,
            meeting_role: e.meeting_role,
            recurring_group: e.recurring_group,
            start_time: e.start_time,
            end_time: e.end_time,
            evidence_urls: e.evidence_urls,
            impact_tags: e.impact_tags,
            decision,
            is_synced,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EntryPage {
    pub items: Vec<EntryDto>,
    pub next_cursor: Option<String>,
    pub total_matching: usize,
}

/// Filter keys shared with the report endpoint (`POST /reports` uses the same names).
#[derive(Debug, Default, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct EntryFilter {
    #[serde(default)]
    #[ts(optional)]
    pub from: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub to: Option<String>,
    /// Comma-separated sources (`github,phabricator`).
    #[serde(default)]
    #[ts(optional)]
    pub source: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub goal_id: Option<i64>,
    /// Entry type category (`code`, `reviews`, `docs`, `meetings`, `collaboration`, `learning`).
    #[serde(default)]
    #[ts(optional)]
    pub category: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub q: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub team: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub person: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reach: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub complexity: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub no_priority: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub no_team: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub no_person: Option<bool>,
}

/// Query-string form of the list endpoint. Fields are spelled out instead of flattening
/// `EntryFilter`, because serde cannot parse numbers and booleans through `#[serde(flatten)]`
/// from a query string.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub cycle_id: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub source: Option<String>,
    pub priority_id: Option<i64>,
    pub goal_id: Option<i64>,
    pub category: Option<String>,
    pub q: Option<String>,
    pub team: Option<String>,
    pub person: Option<String>,
    pub reach: Option<String>,
    pub complexity: Option<String>,
    pub role: Option<String>,
    pub no_priority: Option<bool>,
    pub no_team: Option<bool>,
    pub no_person: Option<bool>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

impl ListQuery {
    fn filter(&self) -> EntryFilter {
        EntryFilter {
            from: self.from.clone(),
            to: self.to.clone(),
            source: self.source.clone(),
            priority_id: self.priority_id,
            goal_id: self.goal_id,
            category: self.category.clone(),
            q: self.q.clone(),
            team: self.team.clone(),
            person: self.person.clone(),
            reach: self.reach.clone(),
            complexity: self.complexity.clone(),
            role: self.role.clone(),
            no_priority: self.no_priority,
            no_team: self.no_team,
            no_person: self.no_person,
        }
    }
}

fn flag(v: Option<bool>) -> Option<String> {
    v.filter(|b| *b).map(|_| "1".to_string())
}

impl EntryFilter {
    fn as_page_query(&self) -> AnalyzePageQuery {
        AnalyzePageQuery {
            department_goal_id: self.goal_id.map(|v| v.to_string()),
            priority_id: self.priority_id.map(|v| v.to_string()),
            category: self.category.clone(),
            source: self.source.clone(),
            team: self.team.clone(),
            collaborator: self.person.clone(),
            search: self.q.clone(),
            no_priority: flag(self.no_priority),
            no_team: flag(self.no_team),
            no_collaborator: flag(self.no_person),
            reach: self.reach.clone(),
            complexity: self.complexity.clone(),
            role: self.role.clone(),
        }
    }
}

/// Resolves the cycle for a request: the given id (ownership checked) or the active cycle.
pub async fn resolve_cycle(
    state: &AppState,
    user_id: i64,
    cycle_id: Option<i64>,
) -> Result<BragPhase, ApiError> {
    match cycle_id {
        Some(id) => owned_cycle(state, id, user_id).await,
        None => BragPhase::get_active(&state.db, user_id)
            .await?
            .ok_or_else(|| ApiError::invalid("No active cycle")),
    }
}

/// Runs the filter against a cycle and returns display-ready entries, newest first, with
/// people aliases applied. Future synced entries (upcoming meetings) are hidden; manual
/// entries with future dates stay. Shared by the entries list and by reports.
pub async fn filtered_entries(
    state: &AppState,
    user_id: i64,
    crypto: &crate::kernel::crypto::UserCrypto,
    phase: &BragPhase,
    filter: &EntryFilter,
) -> Result<Vec<BragEntry>, ApiError> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let sources: Vec<String> = filter
        .source
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let start = filter.from.clone().unwrap_or_else(|| phase.start_date.clone());
    let end = filter.to.clone().unwrap_or_else(|| phase.end_date.clone());

    let mut entries = BragEntry::list_for_phase_filtered(
        &state.db,
        phase.id,
        filter.priority_id,
        filter.goal_id,
        &[],
        Some(&start),
        Some(&end),
        &sources,
        crypto,
    )
    .await?;
    entries.retain(|e| e.source == "manual" || e.occurred_at.as_str() <= today.as_str());

    // Aliases first so name filters match what the user sees.
    let alias_map = PeopleAlias::alias_map(&state.db, user_id).await?;
    let team_map = PeopleAlias::team_map(&state.db, user_id).await?;
    PeopleAlias::apply_to_entries(&mut entries, &alias_map, &team_map);

    apply_in_memory_filters(&mut entries, &filter.as_page_query());
    if let Some(cat) = filter.category.as_deref().filter(|c| !c.is_empty()) {
        let allowed = category_to_entry_types(cat);
        entries.retain(|e| allowed.contains(&e.entry_type.as_str()));
    }
    // Stable order for cursor paging.
    entries.sort_by(|a, b| {
        b.occurred_at
            .cmp(&a.occurred_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(entries)
}

fn encode_cursor(e: &BragEntry) -> String {
    format!("{}|{}", e.occurred_at, e.id)
}

fn decode_cursor(c: &str) -> Result<(String, i64), ApiError> {
    let (date, id) = c
        .split_once('|')
        .ok_or_else(|| ApiError::invalid("Invalid cursor"))?;
    let id = id.parse().map_err(|_| ApiError::invalid("Invalid cursor"))?;
    Ok((date.to_string(), id))
}

/// `GET /api/v1/entries`
pub async fn list(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<EntryPage> {
    let phase = resolve_cycle(&state, auth.user_id, q.cycle_id).await?;
    let all = filtered_entries(&state, auth.user_id, &auth.crypto, &phase, &q.filter()).await?;
    let total_matching = all.len();
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let start_idx = match q.cursor.as_deref() {
        Some(c) => {
            let (date, id) = decode_cursor(c)?;
            // Entries are sorted (date desc, id desc): resume after the cursor position.
            all.iter()
                .position(|e| (e.occurred_at.as_str(), e.id) < (date.as_str(), id))
                .unwrap_or(all.len())
        }
        None => 0,
    };
    let page: Vec<BragEntry> = all.into_iter().skip(start_idx).take(limit + 1).collect();
    let has_more = page.len() > limit;
    let page: Vec<BragEntry> = page.into_iter().take(limit).collect();
    let next_cursor = if has_more {
        page.last().map(encode_cursor)
    } else {
        None
    };
    Ok(Json(EntryPage {
        items: page.into_iter().map(Into::into).collect(),
        next_cursor,
        total_matching,
    }))
}

// ── Facets and metadata ──

#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct SourceCount {
    pub source: String,
    pub count: i64,
}

#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct Facets {
    pub teams: Vec<String>,
    pub people: Vec<String>,
    pub sources: Vec<SourceCount>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct CycleQuery {
    pub cycle_id: Option<i64>,
}

/// `GET /api/v1/entries/facets`: values for filter dropdowns, cached for a minute per cycle.
pub async fn facets_handler(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Query(q): Query<CycleQuery>,
) -> ApiResult<Facets> {
    let phase = resolve_cycle(&state, auth.user_id, q.cycle_id).await?;
    if let Some(hit) = facets::get_fresh(&state.entry_facets, phase.id, FACET_TTL) {
        return Ok(Json((*hit).clone()));
    }
    let entries = filtered_entries(
        &state,
        auth.user_id,
        &auth.crypto,
        &phase,
        &EntryFilter::default(),
    )
    .await?;
    let mut teams = BTreeSet::new();
    let mut people = BTreeSet::new();
    for e in &entries {
        teams.extend(split_csv(&e.teams));
        people.extend(split_csv(&e.collaborators));
    }
    let counts = BragEntry::source_counts_for_phase(&state.db, phase.id).await?;
    let value = Arc::new(Facets {
        teams: teams.into_iter().collect(),
        people: people.into_iter().collect(),
        sources: counts
            .into_iter()
            .map(|(source, count)| SourceCount { source, count })
            .collect(),
        total: entries.len(),
    });
    facets::put(&state.entry_facets, phase.id, value.clone());
    Ok(Json((*value).clone()))
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EntryTypeDto {
    pub value: String,
    pub label: String,
    pub group: Option<String>,
    pub manual: bool,
}

/// `GET /api/v1/meta/entry-types`
pub async fn entry_types(_auth: ApiAuth) -> Json<Vec<EntryTypeDto>> {
    Json(
        EntryType::ALL
            .iter()
            .map(|t| EntryTypeDto {
                value: t.slug().to_string(),
                label: t.label().to_string(),
                group: EntryType::category_for_slug(t.slug()).map(String::from),
                manual: t.is_manual(),
            })
            .collect(),
    )
}

// ── Mutations ──

/// Body for creating and for replacing an entry (PUT is a full replace, like the old form).
#[derive(Deserialize, TS)]
#[ts(export)]
pub struct EntryInput {
    pub title: String,
    pub entry_type: String,
    /// Defaults to today when creating.
    #[serde(default)]
    #[ts(optional)]
    pub occurred_at: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub teams: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub collaborators: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub source_url: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reach: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub complexity: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
}

fn blank_to_none(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// A priority id sent by the client must belong to the user.
async fn check_priority(
    state: &AppState,
    user_id: i64,
    crypto: &crate::kernel::crypto::UserCrypto,
    priority_id: Option<i64>,
) -> Result<(), ApiError> {
    if let Some(pid) = priority_id
        && Priority::find_by_id(&state.db, pid, user_id, crypto)
            .await?
            .is_none()
    {
        return Err(ApiError::invalid("Unknown priority"));
    }
    Ok(())
}

fn validate_input(input: &EntryInput) -> Result<NaiveDate, ApiError> {
    if input.title.trim().is_empty() {
        return Err(ApiError::invalid("Title is required"));
    }
    if !EntryType::ALL.iter().any(|t| t.slug() == input.entry_type) {
        return Err(ApiError::invalid("Unknown entry type"));
    }
    let date = input
        .occurred_at
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| ApiError::invalid("occurred_at must be a date in YYYY-MM-DD format"))
}

/// `POST /api/v1/entries`: creates a manual entry in the active cycle.
pub async fn create(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(input): Json<EntryInput>,
) -> Result<(StatusCode, Json<EntryDto>), ApiError> {
    let date = validate_input(&input)?;
    let phase = resolve_cycle(&state, auth.user_id, None).await?;
    phase
        .validate_date_in_range(date)
        .map_err(|e| ApiError::invalid(e.to_string().replace("Bad request: ", "")))?;
    check_priority(&state, auth.user_id, &auth.crypto, input.priority_id).await?;

    let week = Week::find_or_create_for_date(&state.db, phase.id, date).await?;
    let entry = BragEntry::create(
        &state.db,
        &CreateEntry {
            week_id: week.id,
            priority_id: input.priority_id,
            title: input.title.trim().to_string(),
            description: blank_to_none(&input.description),
            entry_type: input.entry_type.clone(),
            occurred_at: date.format("%Y-%m-%d").to_string(),
            teams: join_csv(&input.teams).filter(|s| !s.is_empty()),
            collaborators: join_csv(&input.collaborators).filter(|s| !s.is_empty()),
            source_url: blank_to_none(&input.source_url),
            reach: blank_to_none(&input.reach),
            complexity: blank_to_none(&input.complexity),
            role: blank_to_none(&input.role),
        },
        auth.user_id,
        &auth.crypto,
    )
    .await?;
    facets::invalidate(&state.entry_facets, phase.id);
    Ok((StatusCode::CREATED, Json(entry.into())))
}

/// `GET /api/v1/entries/{id}`
pub async fn get(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<EntryDto> {
    let mut entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Entry not found"))?;
    let alias_map = PeopleAlias::alias_map(&state.db, auth.user_id).await?;
    let team_map = PeopleAlias::team_map(&state.db, auth.user_id).await?;
    PeopleAlias::apply_to_entries(std::slice::from_mut(&mut entry), &alias_map, &team_map);
    Ok(Json(entry.into()))
}

/// `PUT /api/v1/entries/{id}`: replaces editable fields; moves the entry to the right week
/// when the date changed.
pub async fn update(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<EntryInput>,
) -> ApiResult<EntryDto> {
    let date = validate_input(&input)?;
    check_priority(&state, auth.user_id, &auth.crypto, input.priority_id).await?;
    let entry = crate::worklog::service::update_with_week_reassignment(
        &state.db,
        id,
        auth.user_id,
        &UpdateEntry {
            priority_id: input.priority_id,
            title: input.title.trim().to_string(),
            description: blank_to_none(&input.description),
            entry_type: input.entry_type.clone(),
            occurred_at: date.format("%Y-%m-%d").to_string(),
            teams: join_csv(&input.teams).filter(|s| !s.is_empty()),
            collaborators: join_csv(&input.collaborators).filter(|s| !s.is_empty()),
            source_url: blank_to_none(&input.source_url),
            reach: blank_to_none(&input.reach),
            complexity: blank_to_none(&input.complexity),
            role: blank_to_none(&input.role),
        },
        &auth.crypto,
    )
    .await
    .map_err(|e| match e {
        AppError::BadRequest(m) => ApiError::invalid(m),
        other => other.into(),
    })?;
    invalidate_for_week(&state, entry.week_id).await;
    Ok(Json(entry.into()))
}

async fn invalidate_for_week(state: &AppState, week_id: i64) {
    if let Ok(phase_id) = Week::phase_id(&state.db, week_id).await {
        facets::invalidate(&state.entry_facets, phase_id);
    }
}

/// `DELETE /api/v1/entries/{id}`: manual entries are deleted, synced ones soft-deleted so the
/// next sync does not bring them back.
pub async fn delete(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Entry not found"))?;
    if entry.source == "manual" {
        BragEntry::delete(&state.db, id, auth.user_id).await?;
    } else {
        BragEntry::soft_delete(&state.db, id, auth.user_id).await?;
    }
    invalidate_for_week(&state, entry.week_id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct BulkRequest {
    pub ids: Vec<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub teams: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub collaborators: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub reach: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub complexity: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
    /// `append` (default) merges teams and collaborators; `replace` overwrites them.
    #[serde(default)]
    #[ts(optional)]
    pub merge_mode: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BulkResult {
    pub updated: usize,
}

/// `POST /api/v1/entries/bulk`
pub async fn bulk(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Json(input): Json<BulkRequest>,
) -> ApiResult<BulkResult> {
    if input.ids.is_empty() {
        return Err(ApiError::invalid("ids must not be empty"));
    }
    check_priority(&state, auth.user_id, &auth.crypto, input.priority_id).await?;
    let updated = crate::worklog::service::bulk_update_entries(
        &state.db,
        auth.user_id,
        &BulkUpdateEntries {
            entry_ids: input
                .ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(","),
            priority_id: input.priority_id,
            teams: join_csv(&input.teams).filter(|s| !s.is_empty()),
            collaborators: join_csv(&input.collaborators).filter(|s| !s.is_empty()),
            reach: blank_to_none(&input.reach),
            complexity: blank_to_none(&input.complexity),
            role: blank_to_none(&input.role),
            merge_mode: input.merge_mode.clone(),
        },
        &auth.crypto,
    )
    .await?;
    if let Some(phase) = BragPhase::get_active(&state.db, auth.user_id).await? {
        facets::invalidate(&state.entry_facets, phase.id);
    }
    Ok(Json(BulkResult { updated }))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ClassifyRequest {
    pub meeting_role: String,
    #[serde(default)]
    #[ts(optional)]
    pub teams: Option<Vec<String>>,
    /// Also remember the choice for every meeting in the same recurring series.
    #[serde(default)]
    pub save_rule: bool,
}

/// `POST /api/v1/entries/{id}/classify`
pub async fn classify(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ClassifyRequest>,
) -> ApiResult<EntryDto> {
    if input.meeting_role.trim().is_empty() {
        return Err(ApiError::invalid("meeting_role is required"));
    }
    let teams = join_csv(&input.teams);
    let result = crate::worklog::service::classify_entry(
        &state.db,
        id,
        auth.user_id,
        input.meeting_role.trim(),
        teams.as_deref(),
        input.save_rule,
        &auth.crypto,
    )
    .await?;
    // The worklog service must not reach into the cycle module, so the rule is saved here.
    if let Some(group) = result.save_rule_for_group.as_ref() {
        let rule = crate::cycle::model::CreateMeetingRule {
            match_type: "recurring_group".to_string(),
            match_value: group.clone(),
            meeting_role: result.meeting_role.clone(),
            person_name: None,
        };
        let _ = crate::cycle::model::MeetingRule::create(&state.db, auth.user_id, &rule).await;
        let _ =
            crate::cycle::model::MeetingRule::apply_meeting_rules(&state.db, auth.user_id).await;
    }
    Ok(Json(result.entry.into()))
}

/// `POST /api/v1/entries/{id}/exclude`: stops syncing this Drive file or Calendar event
/// series and hides its entries.
pub async fn exclude(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let entry = BragEntry::find_by_id(&state.db, id, auth.user_id, &auth.crypto)
        .await?
        .ok_or_else(|| ApiError::not_found("Entry not found"))?;
    match entry.source.as_str() {
        "google_drive" => {
            crate::worklog::service::exclude_drive_file(&state.db, auth.user_id, &entry).await?
        }
        "google_calendar" => {
            crate::worklog::service::exclude_calendar_event(&state.db, auth.user_id, &entry)
                .await?
        }
        _ => return Err(ApiError::invalid("Only Drive and Calendar entries can be excluded")),
    }
    invalidate_for_week(&state, entry.week_id).await;
    Ok(StatusCode::NO_CONTENT)
}
