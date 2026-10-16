//! Ownership checks shared by handlers that act on a cycle.

use crate::AppState;
use crate::cycle::model::BragPhase;

use super::error::ApiError;

/// A cycle the user owns. Another user's id is indistinguishable from a missing one (404),
/// so ids cannot be probed.
pub async fn owned_cycle(
    state: &AppState,
    user_id: i64,
    cycle_id: i64,
) -> Result<BragPhase, ApiError> {
    BragPhase::find_by_id(&state.db, cycle_id, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Cycle not found"))
}

/// The requested cycle, or the active one when the caller passes none.
pub async fn resolve_cycle(
    state: &AppState,
    user_id: i64,
    cycle_id: Option<i64>,
) -> Result<BragPhase, ApiError> {
    match cycle_id {
        Some(id) => owned_cycle(state, user_id, id).await,
        None => BragPhase::get_active(&state.db, user_id)
            .await?
            .ok_or_else(|| ApiError::not_found("No active cycle")),
    }
}
