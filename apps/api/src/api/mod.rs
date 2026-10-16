//! JSON API under `/api/v1`. Handlers are thin: they reuse the repo and service layers of the
//! bounded contexts and only translate to and from wire types.

pub mod auth;
pub mod dto;
pub mod error;
pub mod me;
pub mod priorities;
pub mod review;
pub mod scope;

use axum::{
    Router, middleware,
    routing::{get, post, put},
};

use crate::AppState;
use crate::kernel::middleware::csrf_protection;

/// Routes that need no session: liveness and sign-in page configuration.
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(me::healthz))
        .route("/api/v1/public-config", get(me::public_config))
}

/// Authenticated API routes. Layer order: CSRF header check first, then session check.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/me", get(me::me))
        .route("/api/v1/auth/logout", post(me::logout))
        .route(
            "/api/v1/priorities",
            get(priorities::list).post(priorities::create_priority),
        )
        .route("/api/v1/priorities/import", post(priorities::import_csv))
        .route(
            "/api/v1/priorities/{id}",
            put(priorities::update_priority).delete(priorities::delete_priority),
        )
        .route(
            "/api/v1/priorities/{id}/updates",
            get(priorities::list_updates).post(priorities::post_update),
        )
        .route("/api/v1/goals", post(priorities::create_goal))
        .route(
            "/api/v1/goals/{id}",
            put(priorities::update_goal).delete(priorities::delete_goal),
        )
        .route("/api/v1/cycles/{id}/review", get(review::get_review))
        .route("/api/v1/cycles/{id}/review/{slug}", put(review::save_section))
        .route("/api/v1/cycles/{id}/review/{slug}/draft", post(review::draft_section))
        .route("/api/v1/cycles/{id}/archive", get(review::get_archive))
        .layer(middleware::from_fn(auth::require_auth_api))
        .layer(middleware::from_fn(csrf_protection))
}
