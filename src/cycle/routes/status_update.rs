use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;

use crate::cycle::model::{BragPhase, SaveStatusUpdate, StatusUpdate, Week, WeeklyFocus};
use crate::identity::auth::middleware::AuthUser;
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;
use crate::objectives::model::{Priority, PriorityUpdate};
use crate::worklog::model::BragEntry;
use crate::AppState;

/// GET /status-update/{week_id} — show status update panel.
pub async fn status_update_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let existing =
        StatusUpdate::find_for_week(&state.db, week_id, auth.user_id, &auth.crypto).await?;
    let has_ai = crate::ai::helpers::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("week_id", &week_id);
    ctx.insert("status_update", &existing);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("panels/status_update.html", &ctx)?;
    Ok(Html(html))
}

/// POST /status-update/{week_id}/generate — AI-generate a status update.
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

    let focus_items =
        WeeklyFocus::list_for_week(&state.db, week_id, auth.user_id, &auth.crypto).await?;
    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let blocker_updates =
        PriorityUpdate::list_active_blockers(&state.db, auth.user_id, &auth.crypto).await?;

    let prompt = crate::ai::prompts::build_status_update_prompt(
        &entries,
        &focus_items,
        &priorities,
        &blocker_updates,
        &week.start_date,
        &week.end_date,
    );

    let ai = crate::ai::helpers::get_ai_client(&state, auth.user_id).await?;
    let generated = ai.generate(&prompt).await?;

    // Auto-save the generated content
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

    let mut ctx = tera::Context::new();
    ctx.insert("week_id", &week_id);
    ctx.insert(
        "status_update",
        &StatusUpdate {
            id: 0,
            week_id,
            phase_id: phase.id,
            user_id: auth.user_id,
            content: Some(generated),
            created_at: String::new(),
            updated_at: String::new(),
        },
    );
    ctx.insert("has_ai", &true);

    let html = state
        .templates
        .render("panels/status_update.html", &ctx)?;
    Ok(Html(html))
}

/// POST /status-update/{week_id}/save — save edited status update.
pub async fn save_status_update(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
    Form(input): Form<SaveStatusUpdate>,
) -> Result<impl IntoResponse, AppError> {
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

    Ok(hx_redirect("/dashboard"))
}
