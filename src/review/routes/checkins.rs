use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::okr::model::KeyResult;
use crate::review::model::{SaveCheckin, Week, WeeklyCheckin};
use crate::shared::error::AppError;
use crate::shared::render::hx_redirect;

/// Renders the weekly check-in page for a specific week.
pub async fn checkin_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase_id = Week::phase_id(&state.db, week_id).await?;
    let phase = crate::review::model::BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    let week = sqlx::query_as::<_, Week>("SELECT * FROM weeks WHERE id = ?")
        .bind(week_id)
        .fetch_one(&state.db)
        .await?;

    let existing =
        WeeklyCheckin::find_for_week(&state.db, week_id, auth.user_id, &auth.crypto).await?;
    let key_results = KeyResult::list_active_for_user(&state.db, auth.user_id).await?;

    let kr_snapshots = if let Some(ref checkin) = existing {
        crate::review::model::KrCheckinSnapshot::list_for_checkin(
            &state.db,
            checkin.id,
            &auth.crypto,
        )
        .await?
    } else {
        vec![]
    };

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("week", &week);
    ctx.insert("checkin", &existing);
    ctx.insert("key_results", &key_results);
    ctx.insert("kr_snapshots", &kr_snapshots);
    ctx.insert("current_page", "dashboard");

    let html = state.templates.render("pages/checkin.html", &ctx)?;
    Ok(Html(html))
}

/// Check-in history page — lists all past check-ins.
pub async fn checkins_list(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = crate::review::model::BragPhase::get_active(&state.db, auth.user_id).await?;

    let checkins = WeeklyCheckin::list_with_weeks(&state.db, auth.user_id, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("checkins", &checkins);
    ctx.insert("current_page", "dashboard");

    let html = state.templates.render("pages/checkins_list.html", &ctx)?;
    Ok(Html(html))
}

/// Deletes a weekly check-in and redirects to checkin history.
pub async fn delete_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let phase_id = Week::phase_id(&state.db, week_id).await?;
    let _phase = crate::review::model::BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    WeeklyCheckin::delete(&state.db, week_id, auth.user_id).await?;

    Ok(hx_redirect("/checkins"))
}

/// Saves a weekly check-in (upsert) and redirects to dashboard.
pub async fn save_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
    Form(input): Form<SaveCheckin>,
) -> Result<impl IntoResponse, AppError> {
    let phase_id = Week::phase_id(&state.db, week_id).await?;
    let _phase = crate::review::model::BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    let _checkin =
        WeeklyCheckin::upsert(&state.db, week_id, auth.user_id, &input, &auth.crypto).await?;

    Ok(hx_redirect("/dashboard"))
}
