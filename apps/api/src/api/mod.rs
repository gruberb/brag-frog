//! JSON API under `/api/v1`. Handlers are thin: they reuse the repo and service layers of the
//! bounded contexts and only translate to and from wire types.

pub mod auth;
pub mod cycles;
pub mod dto;
pub mod entries;
pub mod error;
pub mod facets;
pub mod me;
pub mod settings;
pub mod today;

use axum::{
    Router, middleware,
    routing::{delete, get, post},
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
        // cycles
        .route("/api/v1/cycles", get(cycles::list).post(cycles::create))
        .route("/api/v1/cycles/{id}", delete(cycles::delete))
        .route("/api/v1/cycles/{id}/activate", post(cycles::activate))
        // settings
        .route(
            "/api/v1/settings/profile",
            get(settings::get_profile).put(settings::put_profile),
        )
        .route(
            "/api/v1/settings/people",
            get(settings::list_people).post(settings::upsert_person),
        )
        .route("/api/v1/settings/people/{id}", delete(settings::delete_person))
        // entries
        .route("/api/v1/entries", get(entries::list).post(entries::create))
        .route("/api/v1/entries/facets", get(entries::facets_handler))
        .route("/api/v1/entries/bulk", post(entries::bulk))
        .route(
            "/api/v1/entries/{id}",
            get(entries::get).put(entries::update).delete(entries::delete),
        )
        .route("/api/v1/entries/{id}/classify", post(entries::classify))
        .route("/api/v1/entries/{id}/exclude", post(entries::exclude))
        .route("/api/v1/meta/entry-types", get(entries::entry_types))
        // today and meeting prep
        .route("/api/v1/today", get(today::today))
        .route(
            "/api/v1/meetings/{entry_id}/prep",
            get(today::get_prep).put(today::put_prep),
        )
        .route("/api/v1/meetings/{entry_id}/prep/draft", post(today::draft_prep))
        .layer(middleware::from_fn(auth::require_auth_api))
        .layer(middleware::from_fn(csrf_protection))
}
