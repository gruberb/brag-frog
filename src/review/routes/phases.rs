use axum::{
    Form,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::AppState;
use crate::identity::auth::middleware::AuthUser;
use crate::review::model::BragPhase;
use crate::shared::error::AppError;
use crate::shared::render::hx_redirect;

/// HTMX handler: creates a new phase (performance cycle) and redirects to the goals page.
pub async fn create_phase(
    auth: AuthUser,
    State(state): State<AppState>,
    Form(input): Form<crate::review::model::CreatePhase>,
) -> Result<impl IntoResponse, AppError> {
    let _phase = BragPhase::create(&state.db, auth.user_id, &input).await?;

    Ok(hx_redirect("/goals"))
}

/// HTMX handler: sets a phase as active (deactivating the current one) and redirects.
pub async fn activate_phase(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    BragPhase::set_active(&state.db, id, auth.user_id).await?;

    Ok(hx_redirect("/goals"))
}

/// HTMX handler: deletes a phase after ownership check. Cascades to weeks, entries, goals, summaries.
pub async fn delete_phase(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // Verify ownership
    BragPhase::find_by_id(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Phase not found".to_string()))?;

    BragPhase::delete(&state.db, id, auth.user_id).await?;

    Ok(hx_redirect("/goals"))
}
