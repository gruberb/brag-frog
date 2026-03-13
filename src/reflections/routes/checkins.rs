use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::objectives::model::Priority;
use crate::cycle::model::{BragPhase, Week};
use crate::reflections::model::{
    QuarterlyCheckin, SaveCheckin, SaveQuarterlyCheckin, WeeklyCheckin,
};
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;

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
    let phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    let week = sqlx::query_as::<_, Week>("SELECT * FROM weeks WHERE id = ?")
        .bind(week_id)
        .fetch_one(&state.db)
        .await?;

    let existing =
        WeeklyCheckin::find_for_week(&state.db, week_id, auth.user_id, &auth.crypto).await?;
    let priorities = Priority::list_active_for_user(&state.db, auth.user_id, &auth.crypto).await?;
    let checkin_sections = &crate::reflections::model::checkin_config().sections;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("week", &week);
    ctx.insert("checkin", &existing);
    ctx.insert("priorities", &priorities);
    ctx.insert("checkin_sections", &checkin_sections);
    ctx.insert("current_page", "checkins");

    let html = state.templates.render("pages/checkin.html", &ctx)?;
    Ok(Html(html))
}

/// Weekly reflections history page.
pub async fn checkins_list(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    let checkins = WeeklyCheckin::list_with_weeks(&state.db, auth.user_id, &auth.crypto).await?;

    let current_week = if let Some(ref p) = phase {
        let now = chrono::Local::now().naive_local().date();
        Some(Week::find_or_create_for_date(&state.db, p.id, now).await?)
    } else {
        None
    };

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("checkins", &checkins);
    ctx.insert("current_week", &current_week);
    ctx.insert("current_page", "checkins");

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
    let _phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    WeeklyCheckin::delete(&state.db, week_id, auth.user_id).await?;

    Ok(hx_redirect("/checkins"))
}

/// Saves a weekly check-in (upsert) and redirects to checkins list.
pub async fn save_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(week_id): Path<i64>,
    Form(input): Form<SaveCheckin>,
) -> Result<impl IntoResponse, AppError> {
    let phase_id = Week::phase_id(&state.db, week_id).await?;
    let _phase = BragPhase::find_by_id(&state.db, phase_id, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Week not found".to_string()))?;

    let _checkin =
        WeeklyCheckin::upsert(&state.db, week_id, auth.user_id, &input, &auth.crypto).await?;

    Ok(hx_redirect("/checkins"))
}

/// Renders the quarterly conversation prep as a slide-over panel.
pub async fn quarterly_checkin_panel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((quarter, year)): Path<(String, i64)>,
) -> Result<Html<String>, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    let existing =
        QuarterlyCheckin::find(&state.db, phase.id, auth.user_id, &quarter, year, &auth.crypto)
            .await?;

    let weekly_reflections = QuarterlyCheckin::weekly_reflections_for_quarter(
        &state.db,
        auth.user_id,
        &quarter,
        year,
        &auth.crypto,
    )
    .await?;

    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let checkin_sections = &crate::reflections::model::checkin_config().sections;
    let has_ai = crate::ai::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("phase", &phase);
    ctx.insert("quarter", &quarter);
    ctx.insert("year", &year);
    ctx.insert("checkin", &existing);
    ctx.insert("weekly_reflections", &weekly_reflections);
    ctx.insert("priorities", &priorities);
    ctx.insert("checkin_sections", &checkin_sections);
    ctx.insert("has_ai", &has_ai);

    let html = state
        .templates
        .render("panels/quarterly_checkin_form.html", &ctx)?;
    Ok(Html(html))
}

// ---------------------------------------------------------------------------
// Quarterly Check-ins
// ---------------------------------------------------------------------------

/// Renders the quarterly check-in page for a specific quarter.
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

    let existing =
        QuarterlyCheckin::find(&state.db, phase.id, auth.user_id, &quarter, year, &auth.crypto)
            .await?;

    let weekly_reflections = QuarterlyCheckin::weekly_reflections_for_quarter(
        &state.db,
        auth.user_id,
        &quarter,
        year,
        &auth.crypto,
    )
    .await?;

    let priorities = Priority::list_for_phase(&state.db, phase.id, &auth.crypto).await?;
    let checkin_sections = &crate::reflections::model::checkin_config().sections;
    let has_ai = crate::ai::has_ai_for_user(&state, auth.user_id).await;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("quarter", &quarter);
    ctx.insert("year", &year);
    ctx.insert("checkin", &existing);
    ctx.insert("weekly_reflections", &weekly_reflections);
    ctx.insert("priorities", &priorities);
    ctx.insert("checkin_sections", &checkin_sections);
    ctx.insert("has_ai", &has_ai);
    ctx.insert("current_page", "summary");

    let html = state
        .templates
        .render("pages/quarterly_checkin.html", &ctx)?;
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

    let _checkin = QuarterlyCheckin::upsert(
        &state.db,
        phase.id,
        auth.user_id,
        &input,
        &auth.crypto,
    )
    .await?;

    let redirect_url = format!("/review/{}", phase.id);
    Ok((
        [(
            axum::http::header::HeaderName::from_static("hx-redirect"),
            axum::http::HeaderValue::from_str(&redirect_url)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("/checkins")),
        )],
        "",
    ))
}
