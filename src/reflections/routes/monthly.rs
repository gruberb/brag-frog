use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;

use crate::AppState;
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::{PeopleAlias, User};
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;
use crate::reflections::model::{MonthlyCheckin, SaveMonthlyCheckin, WeeklyCheckin};

/// GET /monthly-checkin/{month}/{year} — monthly growth check-in page.
pub async fn monthly_checkin_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((month, year)): Path<(i64, i64)>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let checkin = MonthlyCheckin::find(
        &state.db,
        phase.id,
        auth.user_id,
        month,
        year,
        &auth.crypto,
    )
    .await?;

    // Last 4-5 weekly energy levels for trend display
    let weekly_checkins =
        WeeklyCheckin::list_with_weeks(&state.db, auth.user_id, &auth.crypto).await?;
    let energy_trend: Vec<_> = weekly_checkins
        .iter()
        .take(5)
        .map(|c| {
            serde_json::json!({
                "week": c.iso_week,
                "energy": c.energy_level,
                "productivity": c.productivity_rating,
            })
        })
        .collect();

    // Stale relationships (not interacted in 30+ days)
    let stale_relationships =
        PeopleAlias::list_stale_relationships(&state.db, auth.user_id, 30).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("checkin", &checkin);
    ctx.insert("month", &month);
    ctx.insert("year", &year);
    ctx.insert("energy_trend", &energy_trend);
    ctx.insert("stale_relationships", &stale_relationships);
    ctx.insert("current_page", "checkins");

    let html = state
        .templates
        .render("pages/monthly_checkin.html", &ctx)?;
    Ok(Html(html))
}

/// POST /monthly-checkin/{month}/{year} — save monthly check-in.
pub async fn save_monthly_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((month, year)): Path<(i64, i64)>,
    Form(mut input): Form<SaveMonthlyCheckin>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    input.month = month;
    input.year = year;
    MonthlyCheckin::upsert(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    Ok((
        [(
            axum::http::header::HeaderName::from_static("hx-redirect"),
            "/checkins?tab=monthly".to_string(),
        )],
        String::new(),
    ))
}

/// DELETE /monthly-checkin/{month}/{year} — delete a monthly check-in.
pub async fn delete_monthly_checkin(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((month, year)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    MonthlyCheckin::delete(&state.db, phase.id, auth.user_id, month, year).await?;
    Ok(hx_redirect("/checkins?tab=monthly"))
}
