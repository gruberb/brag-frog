use axum::{
    Form,
    extract::{Path, State},
    response::Html,
};

use crate::AppState;
use crate::ai::prompts::build_self_reflection_prompt;
use crate::entries::model::BragEntry;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::clg::ClgLevel;
use crate::identity::model::User;
use crate::goals::model::{DepartmentGoal, Priority};
use crate::review::model::{
    BragPhase, ContributionExample, Summary, assessment_config, rating_scale_config,
    section_question, section_title,
};
use crate::review::routes::{get_ai_client, has_ai_for_user};
use crate::shared::error::AppError;

// Aggregated data needed to build AI prompts for summary generation.
struct SummaryData {
    entries: Vec<BragEntry>,
    dept_goals: Vec<DepartmentGoal>,
    priorities: Vec<Priority>,
    clg_level: Option<&'static ClgLevel>,
    wants_promotion: bool,
}

// Loads entries, department goals, priorities, and CLG level for building AI prompts.
async fn load_summary_data(
    state: &AppState,
    phase_id: i64,
    user_id: i64,
) -> Result<SummaryData, AppError> {
    let user_crypto = state.crypto.for_user(user_id)?;
    let entries = BragEntry::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let dept_goals = DepartmentGoal::list_for_phase(&state.db, phase_id, &user_crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, user_id, &user_crypto).await?;

    let user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let clg_level = user
        .role
        .as_deref()
        .and_then(|r| crate::identity::clg::get_level(r));

    Ok(SummaryData {
        entries,
        dept_goals,
        priorities,
        clg_level,
        wants_promotion: user.wants_promotion,
    })
}

/// Renders the self-review summary page for a phase with all four CultureAmp sections.
pub async fn summary_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(phase_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let summaries = Summary::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let has_ai = has_ai_for_user(&state, auth.user_id).await;
    let examples =
        ContributionExample::list_for_phase(&state.db, phase_id, &auth.crypto).await?;
    let assessment = assessment_config();
    let rating_scale = rating_scale_config();

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("sections", &build_sections_json(&summaries));
    ctx.insert("has_ai", &has_ai);
    ctx.insert("current_page", "summary");
    ctx.insert("examples", &examples);
    ctx.insert("assessment_mid_year", &assessment.mid_year);
    ctx.insert("assessment_year_end", &assessment.year_end);
    ctx.insert("rating_scale", &rating_scale.ratings);

    let html = state.templates.render("pages/summary.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: AI-generates all four summary sections and re-renders the full page.
pub async fn generate_all(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(phase_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let data = load_summary_data(&state, phase_id, auth.user_id).await?;

    for section in &crate::review::model::section_slugs() {
        let prompt = build_self_reflection_prompt(
            section,
            &data.dept_goals,
            &data.entries,
            &data.priorities,
            &phase.name,
            data.clg_level,
            data.wants_promotion,
        );

        let content = ai_client.generate(&prompt).await?;

        Summary::upsert(
            &state.db,
            phase_id,
            section,
            &content,
            Some(&prompt),
            Some(&state.config.ai_model),
            &auth.crypto,
        )
        .await?;
    }

    // Re-render the summary sections
    let summaries = Summary::list_for_phase(&state.db, phase_id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("sections", &build_sections_json(&summaries));
    ctx.insert("phase", &phase);
    ctx.insert("has_ai", &true);

    let html = state.templates.render("pages/summary.html", &ctx)?;
    Ok(Html(html))
}

/// HTMX handler: generates an AI draft for a section and returns plain text (does NOT persist).
pub async fn ai_draft_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((phase_id, section)): Path<(i64, String)>,
) -> Result<String, AppError> {
    let ai_client = get_ai_client(&state, auth.user_id).await?;

    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    let data = load_summary_data(&state, phase_id, auth.user_id).await?;

    let prompt = build_self_reflection_prompt(
        &section,
        &data.dept_goals,
        &data.entries,
        &data.priorities,
        &phase.name,
        data.clg_level,
        data.wants_promotion,
    );

    let content = ai_client.generate(&prompt).await?;

    // Return plain text — do NOT save to database
    Ok(content)
}

/// Form payload for saving or updating a summary section's content.
#[derive(serde::Deserialize)]
pub struct UpdateSummaryForm {
    pub content: String,
}

/// HTMX handler: upserts a summary section's content (creates if new) and returns the section fragment.
pub async fn save_section(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((phase_id, section)): Path<(i64, String)>,
    Form(input): Form<UpdateSummaryForm>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    // Upsert: creates if not exists, updates if exists
    let summary = Summary::upsert(
        &state.db,
        phase_id,
        &section,
        &input.content,
        None,
        None,
        &auth.crypto,
    )
    .await?;

    let has_ai = has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert(
        "section",
        &serde_json::json!({
            "key": section,
            "title": section_title(&section),
            "question": section_question(&section),
            "content": Some(summary.content),
            "generated_at": Some(summary.generated_at),
            "id": Some(summary.id),
        }),
    );
    ctx.insert("phase", &phase);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("components/summary_section.html", &ctx)?;
    Ok(Html(html))
}

// Builds the template-ready JSON array for all four CultureAmp sections, merging saved content.
fn build_sections_json(summaries: &[Summary]) -> Vec<serde_json::Value> {
    crate::review::model::section_slugs()
        .iter()
        .map(|&section| {
            let summary = summaries.iter().find(|s| s.section == section);
            serde_json::json!({
                "key": section,
                "title": section_title(section),
                "question": section_question(section),
                "content": summary.map(|s| s.content.clone()),
                "generated_at": summary.map(|s| s.generated_at.clone()),
                "id": summary.map(|s| s.id),
            })
        })
        .collect()
}
