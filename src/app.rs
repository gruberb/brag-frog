//! Composition root: builds the application state and router.

use std::sync::Arc;

use axum::Router;
use tera::Tera;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::SqliteStore;

use crate::AppState;
use crate::shared::config::Config;
use crate::shared::crypto::Crypto;
use crate::shared::middleware::security_headers;
use crate::shared::render::markdown_filter;

/// Resolves a config file path: checks `custom/` first, falls back to `config/`.
/// This allows organizations to override default config files by placing their
/// versions in a `custom/` directory without modifying the defaults.
pub fn config_path(filename: &str) -> String {
    let custom = format!("custom/{}", filename);
    if std::path::Path::new(&custom).exists() {
        tracing::info!(path = %custom, "Using custom config override");
        custom
    } else {
        format!("config/{}", filename)
    }
}

/// Creates the application state: database pool, session store, templates, crypto.
/// Returns `(AppState, SqliteStore)` so the caller can build the session layer.
pub async fn build_state(config: Config) -> (AppState, SqliteStore) {
    // Database
    let pool = crate::db::setup_pool(&config.database_path).await;
    crate::db::run_migrations(&pool).await;

    // Session store
    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to migrate session store");

    // Config files — check custom/ overlay first, then fall back to config/
    crate::identity::clg::load_levels(&config_path("clg_levels.toml"));
    crate::review::model::load_review_config(&config_path("review_sections.toml"));
    crate::review::model::load_checkin_config(&config_path("checkin_sections.toml"));
    crate::review::model::load_assessment_config(&config_path("assessment_templates.toml"));
    crate::review::model::load_rating_scale(&config_path("rating_scale.toml"));
    crate::sync::services_config::load(&config_path("services.toml"));

    // Templates
    let mut templates = Tera::new("templates/**/*.html").expect("Failed to load templates");
    templates.register_filter("markdown", markdown_filter);
    crate::register_tera_filters(&mut templates);

    // Crypto
    if config.encryption_key.is_empty() {
        panic!("BRAGFROG_ENCRYPTION_KEY must be set");
    }
    let crypto = Crypto::new(&config.encryption_key).expect("Failed to initialize crypto");

    // Post-SQL migrations that require encryption (e.g., KR name encryption)
    crate::db::run_post_migrations(&pool, &crypto).await;

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        templates: Arc::new(templates),
        crypto: Arc::new(crypto),
    };

    (state, session_store)
}

/// Assembles the full Axum application: router, static files, sessions, security headers.
pub fn build_app(state: AppState, session_store: SqliteStore) -> Router {
    let is_production = state.config.base_url.starts_with("https://");
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(is_production)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(12)));

    Router::new()
        .merge(crate::routes::create_router())
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/custom",
            ServeDir::new("custom").append_index_html_on_directories(false),
        )
        .layer(session_layer)
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}
