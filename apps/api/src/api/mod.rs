//! JSON API under `/api/v1`. Handlers are thin: they reuse the repo and service layers of the
//! bounded contexts and only translate to and from wire types.

pub mod auth;
pub mod config;
pub mod dto;
pub mod error;
pub mod integrations;
pub mod me;
pub mod sync;
pub mod trends;

use axum::{
    Router, middleware,
    routing::{get, post},
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
        .route("/api/v1/config", get(config::get_config))
        .route("/api/v1/trends", get(trends::get_trends))
        .route("/api/v1/integrations", get(integrations::list))
        .route(
            "/api/v1/integrations/{service}",
            axum::routing::put(integrations::save).delete(integrations::reset),
        )
        .route("/api/v1/integrations/{service}/test", post(integrations::test))
        .route(
            "/api/v1/integrations/{service}/excluded/{id}",
            axum::routing::delete(integrations::restore_excluded),
        )
        .route("/api/v1/sync", post(sync::sync_all))
        .route("/api/v1/sync/status", get(sync::status))
        .route("/api/v1/sync/{service}", post(sync::sync_service))
        .layer(middleware::from_fn(auth::require_auth_api))
        .layer(middleware::from_fn(csrf_protection))
}
