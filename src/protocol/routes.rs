use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use chrono::Local;

use crate::AppState;
use crate::cycle::model::{BragPhase, Week};
use crate::identity::auth::middleware::AuthUser;
use crate::identity::model::User;
use crate::kernel::error::AppError;
use crate::kernel::render::hx_redirect;
use crate::protocol::model::{ProtocolCheck, CHECKLIST_ITEMS};

/// GET /protocol -- 10x Protocol page with weekly checklist.
pub async fn protocol_page(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let phase = BragPhase::get_active(&state.db, auth.user_id).await?;

    let (checked_slugs, week_label) = if let Some(ref p) = phase {
        let now = Local::now().naive_local().date();
        let week = Week::find_or_create_for_date(&state.db, p.id, now).await?;
        let checks = ProtocolCheck::list_for_week(&state.db, week.id, auth.user_id).await?;
        let slugs: Vec<String> = checks
            .into_iter()
            .filter(|c| c.checked == 1)
            .map(|c| c.slug)
            .collect();
        let label = format!("Week {} · {} to {}", week.iso_week, week.start_date, week.end_date);
        (slugs, label)
    } else {
        (Vec::new(), String::from("No active phase"))
    };

    // Group checklist items
    let groups = vec!["Monday", "Daily", "Mid-week", "Friday", "Monthly"];

    let mut ctx = tera::Context::new();
    ctx.insert("user", &user);
    ctx.insert("phase", &phase);
    ctx.insert("checklist_items", &CHECKLIST_ITEMS);
    ctx.insert("checked_slugs", &checked_slugs);
    ctx.insert("groups", &groups);
    ctx.insert("week_label", &week_label);
    ctx.insert("current_page", "protocol");

    let html = state.templates.render("pages/protocol.html", &ctx)?;
    Ok(Html(html))
}

/// POST /protocol/{slug}/toggle -- toggle a checklist item.
pub async fn toggle_protocol_check(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let now = Local::now().naive_local().date();
    let week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    ProtocolCheck::toggle(&state.db, week.id, auth.user_id, &slug).await?;

    Ok(hx_redirect("/protocol"))
}

/// POST /protocol/clear — reset all checklist items for the current week.
pub async fn clear_protocol_checks(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let phase = BragPhase::get_active(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::BadRequest("No active phase".into()))?;

    let now = Local::now().naive_local().date();
    let week = Week::find_or_create_for_date(&state.db, phase.id, now).await?;

    ProtocolCheck::clear_for_week(&state.db, week.id, auth.user_id).await?;

    Ok(hx_redirect("/protocol"))
}
