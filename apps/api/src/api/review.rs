//! Self-review sections, AI drafts and the read-only archive of retired features.
//!
//! Section text lives in the encrypted `summaries.content` column. `content_format` says how
//! to read it: `markdown` (rows written before the SPA) or `blocknote` (editor JSON).

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::auth::ApiAuth;
use super::error::{ApiError, ApiResult};
use super::scope::owned_cycle;
use crate::AppState;
use crate::ai::get_ai_client;
use crate::ai::prompts::build_self_reflection_prompt;
use crate::cycle::model::Week;
use crate::identity::model::User;
use crate::objectives::model::{DepartmentGoal, Priority};
use crate::archive;
use crate::review::model::{
    ReviewSection, Summary, get_section, review_config,
};
use crate::worklog::model::BragEntry;

const CONTENT_FORMATS: &[&str] = &["markdown", "blocknote"];

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReviewSectionDto {
    pub slug: String,
    pub title: String,
    pub question: String,
    pub form_question_number: Option<String>,
    pub form_required: bool,
    pub form_question: Option<String>,
    pub form_guidance: Option<String>,
    pub form_bullets: Vec<String>,
    pub form_tip: Option<String>,
    pub form_placeholder: Option<String>,
    pub focus_priorities: bool,
    pub focus_priority_help: Option<String>,
    /// `null` until the section has been saved once.
    pub content: Option<String>,
    /// Format of `content`; `blocknote` for a section that has no content yet.
    pub content_format: String,
    pub generated_at: Option<String>,
}

impl ReviewSectionDto {
    fn new(config: &ReviewSection, saved: Option<&Summary>) -> Self {
        Self {
            slug: config.slug.clone(),
            title: config.title.clone(),
            question: config.question.clone(),
            form_question_number: config.form_question_number.clone(),
            form_required: config.form_required,
            form_question: config.form_question.clone(),
            form_guidance: config.form_guidance.clone(),
            form_bullets: config.form_bullets.clone(),
            form_tip: config.form_tip.clone(),
            form_placeholder: config.form_placeholder.clone(),
            focus_priorities: config.focus_priorities,
            focus_priority_help: config.focus_priority_help.clone(),
            content: saved.map(|s| s.content.clone()),
            content_format: saved
                .map_or_else(|| "blocknote".to_string(), |s| s.content_format.clone()),
            generated_at: saved.map(|s| s.generated_at.clone()),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ReviewResponse {
    pub cycle_id: i64,
    pub sections: Vec<ReviewSectionDto>,
    pub entry_count: i64,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct SaveSectionRequest {
    pub content: String,
    pub content_format: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct DraftRequest {
    /// Department goals to emphasise. Unknown ids are ignored.
    #[serde(default)]
    #[ts(optional)]
    pub goal_ids: Option<Vec<i64>>,
    /// Restrict the evidence to these entries. Omitted or empty means the whole cycle.
    #[serde(default)]
    #[ts(optional)]
    pub entry_ids: Option<Vec<i64>>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DraftResponse {
    pub markdown: String,
}

fn section_config(slug: &str) -> Result<&'static ReviewSection, ApiError> {
    get_section(slug).ok_or_else(|| ApiError::not_found("Review section not found"))
}

/// `GET /api/v1/cycles/{id}/review`: every configured section, saved or not.
pub async fn get_review(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(cycle_id): Path<i64>,
) -> ApiResult<ReviewResponse> {
    let cycle = owned_cycle(&state, auth.user_id, cycle_id).await?;
    Ok(Json(load_review(&state, &auth.crypto, cycle.id).await?))
}

/// All configured review sections of a cycle. Shared with the cycle bundle export.
pub async fn load_review(
    state: &AppState,
    crypto: &crate::kernel::crypto::UserCrypto,
    cycle_id: i64,
) -> Result<ReviewResponse, ApiError> {
    let saved = Summary::list_for_phase(&state.db, cycle_id, crypto).await?;
    let sections = review_config()
        .sections
        .iter()
        .map(|cfg| ReviewSectionDto::new(cfg, saved.iter().find(|s| s.section == cfg.slug)))
        .collect();
    Ok(ReviewResponse {
        cycle_id,
        sections,
        entry_count: Summary::phase_entry_count(&state.db, cycle_id).await?,
    })
}

/// `PUT /api/v1/cycles/{id}/review/{slug}`
pub async fn save_section(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path((cycle_id, slug)): Path<(i64, String)>,
    Json(req): Json<SaveSectionRequest>,
) -> ApiResult<ReviewSectionDto> {
    let cycle = owned_cycle(&state, auth.user_id, cycle_id).await?;
    let config = section_config(&slug)?;
    if !CONTENT_FORMATS.contains(&req.content_format.as_str()) {
        return Err(ApiError::invalid(format!(
            "content_format must be one of: {}",
            CONTENT_FORMATS.join(", ")
        )));
    }
    let saved = Summary::upsert_with_format(
        &state.db,
        cycle.id,
        &slug,
        &req.content,
        &req.content_format,
        None,
        None,
        &auth.crypto,
    )
    .await?;
    Ok(Json(ReviewSectionDto::new(config, Some(&saved))))
}

/// `POST /api/v1/cycles/{id}/review/{slug}/draft`: generates text and returns it without
/// saving. Without a configured AI integration this is a 400 `bad_request`.
pub async fn draft_section(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path((cycle_id, slug)): Path<(i64, String)>,
    Json(req): Json<DraftRequest>,
) -> ApiResult<DraftResponse> {
    let cycle = owned_cycle(&state, auth.user_id, cycle_id).await?;
    section_config(&slug)?;
    let ai = get_ai_client(&state, auth.user_id).await?;

    let mut entries = BragEntry::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    if let Some(ids) = req.entry_ids.as_ref().filter(|ids| !ids.is_empty()) {
        entries.retain(|e| ids.contains(&e.id));
        if entries.is_empty() {
            return Err(ApiError::invalid("None of entry_ids belong to this cycle"));
        }
    }
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, cycle.id, &auth.crypto).await?;
    let focus: Vec<i64> = req
        .goal_ids
        .unwrap_or_default()
        .into_iter()
        .filter(|id| dept_goals.iter().any(|g| g.id == *id))
        .collect();

    // Retired contribution examples still feed the prompt, as they did before the redesign.
    let examples = archive::contribution_examples(&state.db, cycle.id, &auth.crypto).await?;
    let mut example_entry_ids: HashMap<i64, Vec<i64>> = HashMap::new();
    for (example_id, entry_id, _) in
        archive::example_entries(&state.db, cycle.id, &auth.crypto).await?
    {
        example_entry_ids
            .entry(example_id)
            .or_default()
            .push(entry_id);
    }

    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    let clg_level = user
        .role
        .as_deref()
        .and_then(crate::identity::clg::get_level);

    let prompt = build_self_reflection_prompt(
        &slug,
        &dept_goals,
        &entries,
        &priorities,
        &examples,
        &example_entry_ids,
        &focus,
        &cycle.name,
        clg_level,
        user.wants_promotion,
    );
    let markdown = ai.generate(&prompt).await.map_err(|e| {
        tracing::error!("AI draft failed: {e:?}");
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "ai_failed",
            "The AI service did not return a draft. Try again.",
        )
    })?;
    Ok(Json(DraftResponse { markdown }))
}

// ---------------------------------------------------------------------------
// Archive: retired features, read only
// ---------------------------------------------------------------------------

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchivedCheckin {
    pub id: i64,
    pub quarter: String,
    pub year: i64,
    pub highlights_impact: Option<String>,
    pub learnings_adjustments: Option<String>,
    pub growth_development: Option<String>,
    pub support_feedback: Option<String>,
    pub looking_ahead: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchivedEntryRef {
    pub id: i64,
    pub title: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchivedExample {
    pub id: i64,
    pub title: String,
    pub outcome: Option<String>,
    pub behaviors: Option<String>,
    pub impact_level: Option<String>,
    pub learnings: Option<String>,
    pub assessment_type: Option<String>,
    pub status: String,
    pub entries: Vec<ArchivedEntryRef>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchivedStatusUpdate {
    pub id: i64,
    pub week_start: Option<String>,
    pub content: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchivedLastWeekReport {
    pub id: i64,
    pub window_start: String,
    pub window_end: String,
    pub content: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ArchiveResponse {
    pub cycle_id: i64,
    pub checkins: Vec<ArchivedCheckin>,
    pub contribution_examples: Vec<ArchivedExample>,
    pub status_updates: Vec<ArchivedStatusUpdate>,
    pub last_week_reports: Vec<ArchivedLastWeekReport>,
}

/// `GET /api/v1/cycles/{id}/archive`
pub async fn get_archive(
    ApiAuth(auth): ApiAuth,
    State(state): State<AppState>,
    Path(cycle_id): Path<i64>,
) -> ApiResult<ArchiveResponse> {
    let cycle = owned_cycle(&state, auth.user_id, cycle_id).await?;
    let crypto = &auth.crypto;

    let checkins = archive::checkins(&state.db, cycle.id, crypto).await?;
    let examples = archive::contribution_examples(&state.db, cycle.id, crypto).await?;
    let links = archive::example_entries(&state.db, cycle.id, crypto).await?;
    let status_updates = archive::status_updates(&state.db, cycle.id, crypto).await?;
    let reports = archive::last_week_reports(&state.db, cycle.id, crypto).await?;
    let week_starts: HashMap<i64, String> = Week::list_for_phase(&state.db, cycle.id)
        .await?
        .into_iter()
        .map(|w| (w.id, w.start_date))
        .collect();

    let mut linked: HashMap<i64, Vec<ArchivedEntryRef>> = HashMap::new();
    for (example_id, id, title) in links {
        linked
            .entry(example_id)
            .or_default()
            .push(ArchivedEntryRef { id, title });
    }

    Ok(Json(ArchiveResponse {
        cycle_id: cycle.id,
        checkins: checkins
            .into_iter()
            .map(|c| ArchivedCheckin {
                id: c.id,
                quarter: c.quarter,
                year: c.year,
                highlights_impact: c.highlights_impact,
                learnings_adjustments: c.learnings_adjustments,
                growth_development: c.growth_development,
                support_feedback: c.support_feedback,
                looking_ahead: c.looking_ahead,
                updated_at: c.updated_at,
            })
            .collect(),
        contribution_examples: examples
            .into_iter()
            .map(|e| ArchivedExample {
                entries: linked.remove(&e.id).unwrap_or_default(),
                id: e.id,
                title: e.title,
                outcome: e.outcome,
                behaviors: e.behaviors,
                impact_level: e.impact_level,
                learnings: e.learnings,
                assessment_type: e.assessment_type,
                status: e.status,
            })
            .collect(),
        status_updates: status_updates
            .into_iter()
            .map(|s| ArchivedStatusUpdate {
                week_start: week_starts.get(&s.week_id).cloned(),
                id: s.id,
                content: s.content,
                updated_at: s.updated_at,
            })
            .collect(),
        last_week_reports: reports
            .into_iter()
            .map(|r| ArchivedLastWeekReport {
                id: r.id,
                window_start: r.window_start,
                window_end: r.window_end,
                content: r.content,
                updated_at: r.updated_at,
            })
            .collect(),
    }))
}
