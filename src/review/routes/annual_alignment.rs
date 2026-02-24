use axum::{
    Form,
    extract::State,
    response::{Html, IntoResponse},
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::review::model::{AnnualAlignment, BragPhase, SaveAnnualAlignment};
use crate::shared::error::AppError;
use crate::shared::render::hx_redirect;

/// Renders the annual alignment page for the active phase.
pub async fn annual_alignment_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = match BragPhase::get_active(&state.db, auth.user_id).await? {
        Some(p) => p,
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("user", &user);
            ctx.insert("current_page", "review");
            let html = state.templates.render("pages/no_phase.html", &ctx)?;
            return Ok(Html(html));
        }
    };

    // Extract year from phase start_date
    let year: i64 = phase.start_date[..4].parse().unwrap_or(2026);

    let existing =
        AnnualAlignment::find(&state.db, phase.id, auth.user_id, year, &auth.crypto).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("year", &year);
    ctx.insert("alignment", &existing);
    ctx.insert("current_page", "review");

    let html = state
        .templates
        .render("pages/annual_alignment.html", &ctx)?;
    Ok(Html(html))
}

/// Saves annual alignment (upsert) and redirects.
pub async fn save_annual_alignment(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<SaveAnnualAlignment>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active phase".to_string()))?;

    AnnualAlignment::upsert(&state.db, phase.id, auth.user_id, &input, &auth.crypto).await?;

    Ok(hx_redirect("/annual-alignment"))
}
