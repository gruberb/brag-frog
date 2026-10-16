use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::ai::get_ai_client;
use crate::ai::prompts::build_quarterly_checkin_prompt;
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::error::AppError;
use crate::reflections::model::{QuarterlyCheckin, SaveQuarterlyCheckin, checkin_config};
use crate::worklog::model::BragEntry;

#[derive(Debug, serde::Deserialize)]
pub struct UpdateQuarterlySectionForm {
    pub content: String,
}

fn quarterly_section_content(checkin: Option<&QuarterlyCheckin>, section: &str) -> Option<String> {
    let checkin = checkin?;

    match section {
        "highlights_impact" => checkin.highlights_impact.clone(),
        "learnings_adjustments" => checkin.learnings_adjustments.clone(),
        "growth_development" => checkin.growth_development.clone(),
        "support_feedback" => checkin.support_feedback.clone(),
        "looking_ahead" => checkin.looking_ahead.clone(),
        _ => None,
    }
}

fn build_quarterly_sections_json(checkin: &Option<QuarterlyCheckin>) -> Vec<serde_json::Value> {
    checkin_config()
        .sections
        .iter()
        .map(|section| {
            let content = quarterly_section_content(checkin.as_ref(), &section.slug);
            let updated_at = content
                .as_ref()
                .and_then(|_| checkin.as_ref().map(|c| c.updated_at.clone()));

            serde_json::json!({
                "key": section.slug.clone(),
                "title": section.quarterly_title.as_deref().unwrap_or(&section.title),
                "question": section.quarterly_question.clone(),
                "content": content,
                "updated_at": updated_at,
            })
        })
        .collect()
}

fn quarterly_input_with_section(
    existing: Option<&QuarterlyCheckin>,
    section: &str,
    quarter: String,
    year: i64,
    content: String,
) -> Result<SaveQuarterlyCheckin, AppError> {
    let content = if content.trim().is_empty() {
        None
    } else {
        Some(content)
    };

    let mut input = SaveQuarterlyCheckin {
        quarter,
        year,
        highlights_impact: existing.and_then(|c| c.highlights_impact.clone()),
        learnings_adjustments: existing.and_then(|c| c.learnings_adjustments.clone()),
        growth_development: existing.and_then(|c| c.growth_development.clone()),
        support_feedback: existing.and_then(|c| c.support_feedback.clone()),
        looking_ahead: existing.and_then(|c| c.looking_ahead.clone()),
    };

    match section {
        "highlights_impact" => input.highlights_impact = content,
        "learnings_adjustments" => input.learnings_adjustments = content,
        "growth_development" => input.growth_development = content,
        "support_feedback" => input.support_feedback = content,
        "looking_ahead" => input.looking_ahead = content,
        _ => {
            return Err(AppError::BadRequest(format!(
                "Unknown check-in section: {}",
                section
            )));
        }
    }

    Ok(input)
}

/// Renders the review check-in body for inline HTMX loading.
pub async fn quarterly_checkin_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year)): Path<(String, i64)>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let existing = QuarterlyCheckin::find(
        &state.db,
        phase.id,
        auth.user_id,
        &quarter,
        year,
        &auth.crypto,
    )
    .await?;

    let quarterly_sections = build_quarterly_sections_json(&existing);
    let has_ai = crate::ai::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("phase", &phase);
    ctx.insert("quarter", &quarter);
    ctx.insert("year", &year);
    ctx.insert("checkin", &existing);
    ctx.insert("quarterly_sections", &quarterly_sections);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("panels/quarterly_checkin_form.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the standalone review check-in page for a specific quarter.
pub async fn quarterly_checkin_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year)): Path<(String, i64)>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let existing = QuarterlyCheckin::find(
        &state.db,
        phase.id,
        auth.user_id,
        &quarter,
        year,
        &auth.crypto,
    )
    .await?;

    let quarterly_sections = build_quarterly_sections_json(&existing);
    let has_ai = crate::ai::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("quarter", &quarter);
    ctx.insert("year", &year);
    ctx.insert("checkin", &existing);
    ctx.insert("quarterly_sections", &quarterly_sections);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("current_page", "summary");

    let html = state
        .templates
        .render("pages/quarterly_checkin.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: AI-generates a draft for a single quarterly check-in section.
///
/// Returns plain text only — the caller's JS drops the result into the matching
/// textarea and the user explicitly saves via the form. The prompt is assembled
/// from the section's instruction in `checkin_sections.toml` and brag entries
/// logged during the quarter for concrete anchoring.
pub async fn ai_draft_quarterly_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year, section)): Path<(String, i64, String)>,
) -> Result<String, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    // Look up the section definition by slug. An unknown slug is a bug, not a
    // user error — fail loudly so we catch any drift between template and config.
    let section_cfg = checkin_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown check-in section: {}", section)))?;

    // Bound entries to the quarter's calendar dates so the model only sees work
    // logged inside the conversation window.
    let (start_date, end_date) = quarter_date_range(&quarter, year);
    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &start_date,
        &end_date,
        &auth.crypto,
    )
    .await?;

    let prompt = build_quarterly_checkin_prompt(
        &section_cfg.quarterly_question,
        &section_cfg.ai_prompt,
        &entries,
        &quarter,
        year,
    );

    let content = ai_client.generate(&prompt).await?;
    Ok(content)
}

/// Maps a quarter label to its inclusive `[start, end]` calendar date range
/// (`YYYY-MM-DD`). Used to slice phase data down to the quarterly window.
fn quarter_date_range(quarter: &str, year: i64) -> (String, String) {
    let (start_month, end_month, end_day) = match quarter {
        "Q1" => (1, 3, 31),
        "Q2" => (4, 6, 30),
        "Q3" => (7, 9, 30),
        "Q4" => (10, 12, 31),
        // Defensive: an unknown quarter falls back to the whole year so the AI
        // still has data to chew on instead of silently empty input.
        _ => (1, 12, 31),
    };
    (
        format!("{:04}-{:02}-01", year, start_month),
        format!("{:04}-{:02}-{:02}", year, end_month, end_day),
    )
}

/// Saves one quarterly section and returns the re-rendered markdown fragment.
pub async fn save_quarterly_checkin_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year, section)): Path<(String, i64, String)>,
    Form(input): Form<UpdateQuarterlySectionForm>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let section_cfg = checkin_config()
        .sections
        .iter()
        .find(|s| s.slug == section)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown check-in section: {}", section)))?;

    let existing = QuarterlyCheckin::find(
        &state.db,
        phase.id,
        auth.user_id,
        &quarter,
        year,
        &auth.crypto,
    )
    .await?;
    let save_input = quarterly_input_with_section(
        existing.as_ref(),
        &section,
        quarter.clone(),
        year,
        input.content,
    )?;

    let checkin =
        QuarterlyCheckin::upsert(&state.db, phase.id, auth.user_id, &save_input, &auth.crypto)
            .await?;
    let has_ai = crate::ai::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert(
        "section",
        &serde_json::json!({
            "key": section.clone(),
            "title": section_cfg.quarterly_title.as_deref().unwrap_or(&section_cfg.title),
            "question": section_cfg.quarterly_question.clone(),
            "content": quarterly_section_content(Some(&checkin), &section),
            "updated_at": checkin.updated_at,
        }),
    );
    ctx.insert("phase", &phase);
    ctx.insert("quarter", &quarter);
    ctx.insert("year", &year);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("components/quarterly_checkin_section.html", &ctx)?;
    Ok(Html(html))
}

/// Saves a quarterly check-in (upsert) and redirects to the review page.
pub async fn save_quarterly_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year)): Path<(String, i64)>,
    Form(mut input): Form<SaveQuarterlyCheckin>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    input.quarter = quarter;
    input.year = year;

    let _checkin =
        QuarterlyCheckin::upsert(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    let redirect_url = format!("/review/{}", phase.id);
    Ok((
        [(
            axum::http::header::HeaderName::from_static("hx-redirect"),
            axum::http::HeaderValue::from_str(&redirect_url)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("/dashboard")),
        )],
        "",
    ))
}
