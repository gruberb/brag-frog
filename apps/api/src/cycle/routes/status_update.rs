use axum::Form;
use axum::extract::{Path, State};
use axum::response::Html;

use crate::AppState;
use crate::cycle::model::{BragPhase, SaveStatusUpdate, StatusUpdate, Week};
use crate::identity::auth::middleware::AuthUser;
use crate::kernel::error::AppError;
use crate::objectives::model::{Priority, PriorityUpdate};
use crate::worklog::model::BragEntry;

/// POST /status-update/{week_id}/generate — AI-generate a stakeholder-facing
/// status update for the week and auto-save the draft. Renders the Reports
/// "Latest Updates" section so HTMX can swap it in place.
pub async fn generate_status_update(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let week = Week::find_by_id(&state.db, week_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".into()))?;

    let entries = BragEntry::list_for_phase_in_range(
        &state.db,
        phase.id,
        &week.start_date,
        &week.end_date,
        &auth.crypto,
    )
    .await?;

    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let blocker_updates =
        PriorityUpdate::list_active_blockers(&state.db, auth.user_id, &auth.crypto).await?;

    let prompt = crate::ai::prompts::build_status_update_prompt(
        &entries,
        &priorities,
        &blocker_updates,
        &week.start_date,
        &week.end_date,
    );

    let ai = crate::ai::helpers::get_ai_client(&state, auth.user_id).await?;
    let generated = ai.generate(&prompt).await?;

    // Auto-save the generated content so the draft survives page reload.
    let input = SaveStatusUpdate {
        content: Some(generated.clone()),
    };
    StatusUpdate::upsert(
        &state.db,
        week_id,
        phase.id,
        auth.user_id,
        &input,
        &auth.crypto,
    )
    .await?;

    render_section(&state, week_id, phase.id, auth.user_id, Some(generated)).await
}

/// POST /status-update/{week_id}/save — persist the edited status update
/// and re-render the section in view mode.
pub async fn save_status_update(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
    Form(input): Form<SaveStatusUpdate>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    StatusUpdate::upsert(
        &state.db,
        week_id,
        phase.id,
        auth.user_id,
        &input,
        &auth.crypto,
    )
    .await?;

    render_section(&state, week_id, phase.id, auth.user_id, input.content).await
}

/// Renders the Latest Updates section partial with the current saved content
/// and AI availability. `content` is what was just generated/saved; when
/// present we use it directly to avoid a redundant DB round-trip.
async fn render_section(
    state: &AppState,
    week_id: i64,
    phase_id: i64,
    user_id: i64,
    content: Option<String>,
) -> Result<Html<String>, AppError> {
    let has_ai = crate::ai::helpers::has_ai_for_user(state, user_id).await;

    let status_update = content.map(|c| StatusUpdate {
        id: 0,
        week_id,
        phase_id,
        user_id,
        content: Some(c),
        created_at: String::new(),
        updated_at: String::new(),
    });

    let mut ctx = tera::Context::new();
    ctx.insert("week_id", &week_id);
    ctx.insert("status_update", &status_update);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("components/status_update_section.html", &ctx)?;
    Ok(Html(html))
}
