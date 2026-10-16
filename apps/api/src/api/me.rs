//! Session, identity and public configuration endpoints.

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use tower_sessions::Session;
use ts_rs::TS;

use super::auth::ApiAuth;
use super::dto::{CycleDto, UserDto};
use super::error::{ApiError, ApiResult};
use crate::AppState;
use crate::ai::helpers::has_ai_for_user;
use crate::cycle::model::BragPhase;
use crate::identity::auth::middleware::clear_session;
use crate::identity::model::User;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct MeResponse {
    pub user: UserDto,
    pub active_cycle: Option<CycleDto>,
    pub has_ai: bool,
    pub public_only: bool,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PublicConfig {
    pub instance_name: Option<String>,
    pub allowed_domain: Option<String>,
    pub public_only: bool,
}

/// `GET /api/v1/me`: the one call the SPA makes on load. Computes `has_ai` once so no
/// other endpoint needs to.
pub async fn me(ApiAuth(auth): ApiAuth, State(state): State<AppState>) -> ApiResult<MeResponse> {
    let user = User::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    let active = BragPhase::get_active(&state.db, auth.user_id).await?;
    Ok(Json(MeResponse {
        user: user.into(),
        active_cycle: active.map(Into::into),
        has_ai: has_ai_for_user(&state, auth.user_id).await,
        public_only: state.config.public_only,
    }))
}

/// `GET /api/v1/public-config`: what the sign-in page needs before a session exists.
pub async fn public_config(State(state): State<AppState>) -> Json<PublicConfig> {
    Json(PublicConfig {
        instance_name: state.config.instance_name.clone(),
        allowed_domain: state.config.allowed_domain.clone(),
        public_only: state.config.public_only,
    })
}

/// `POST /api/v1/auth/logout`
pub async fn logout(session: Session) -> Result<StatusCode, ApiError> {
    clear_session(&session).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /healthz`: liveness for Cloud Run probes. Deliberately does not touch the database.
pub async fn healthz() -> &'static str {
    "ok"
}
