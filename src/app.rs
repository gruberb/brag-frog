//! Composition root: builds the application state and router.

use std::sync::Arc;

use axum::{
    Router, middleware,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post, put},
};
use tera::Tera;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, Session, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::SqliteStore;

use crate::AppState;
use crate::kernel::config::Config;
use crate::kernel::crypto::Crypto;
use crate::kernel::error::AppError;
use crate::kernel::middleware::{csrf_protection, security_headers};
use crate::kernel::render::markdown_filter;

use crate::identity::auth::middleware::require_auth;
use crate::identity::routes as identity_routes;
use crate::worklog::routes as worklog_routes;
use crate::objectives::routes as objectives_routes;
use crate::cycle::routes as cycle_routes;
use crate::reflections::routes as reflections_routes;
use crate::review::routes as review_routes;
use crate::integrations::integrations_routes;
use crate::integrations::sync_routes;

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
    crate::reflections::model::load_checkin_config(&config_path("checkin_sections.toml"));
    crate::review::model::load_assessment_config(&config_path("assessment_templates.toml"));
    crate::review::model::load_rating_scale(&config_path("rating_scale.toml"));
    crate::integrations::services_config::load(&config_path("services.toml"));

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
        sync_status: crate::integrations::sync_status::new_sync_status_map(),
    };

    (state, session_store)
}

// ---------------------------------------------------------------------------
// Static page handlers
// ---------------------------------------------------------------------------

/// Renders the privacy policy page.
pub async fn static_page_privacy(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.templates.render("pages/privacy.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the terms of service page.
pub async fn static_page_terms(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.templates.render("pages/terms.html", &ctx)?;
    Ok(Html(html))
}

/// Landing page: redirects authenticated users to `/dashboard`, shows login page otherwise.
async fn landing_page(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    let user_id: Option<i64> = session.get("user_id").await.unwrap_or(None);

    if user_id.is_some() {
        return Ok(Redirect::to("/dashboard").into_response());
    }

    let mut ctx = tera::Context::new();

    let state_token = format!("login:{}", uuid::Uuid::new_v4());
    session
        .insert("oauth_state", &state_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to set OAuth state: {}", e)))?;

    let google_auth_url = crate::identity::auth::google_auth_url(&state.config, &state_token);
    ctx.insert("google_auth_url", &google_auth_url);
    ctx.insert("instance_name", &state.config.instance_name);

    let html = state.templates.render("pages/landing.html", &ctx)?;
    Ok(Html(html).into_response())
}

// ---------------------------------------------------------------------------
// Router assembly
// ---------------------------------------------------------------------------

/// Assembles the full application router: public, auth, and protected route groups.
pub fn create_router() -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/auth/login", get(identity_routes::login_page))
        .route("/auth/callback", get(identity_routes::callback))
        .route("/auth/logout", post(identity_routes::logout));

    let public_routes = Router::new()
        .route("/", get(landing_page))
        .route("/privacy", get(static_page_privacy))
        .route("/terms", get(static_page_terms));

    let protected_routes = Router::new()
        // Dashboard
        .route("/dashboard", get(cycle_routes::dashboard::dashboard))
        .route(
            "/focus/week/{week_id}",
            post(cycle_routes::dashboard::create_focus),
        )
        .route(
            "/focus/{focus_id}",
            put(cycle_routes::dashboard::update_focus)
                .delete(cycle_routes::dashboard::delete_focus),
        )
        // Logbook
        .route("/logbook", get(cycle_routes::logbook::logbook))
        // Entries
        .route("/entries/quick", post(worklog_routes::quick_create_entry))
        .route("/entries/bulk-update", post(worklog_routes::bulk_update_entries))
        .route("/entries/{id}", put(worklog_routes::update_entry))
        .route("/entries/{id}", delete(worklog_routes::delete_entry))
        .route("/entries/{id}/view", get(worklog_routes::view_entry))
        .route("/entries/{id}/panel", get(worklog_routes::entry_panel))
        .route(
            "/entries/{id}/classify",
            post(worklog_routes::classify_entry),
        )
        .route(
            "/entries/{id}/exclude-file",
            post(worklog_routes::exclude_drive_file),
        )
        .route(
            "/entries/{id}/exclude-event",
            post(worklog_routes::exclude_calendar_event),
        )
        // Trends
        .route("/trends", get(cycle_routes::trends::trends_page))
        // Meeting Prep (panel only)
        .route(
            "/meeting-prep/panel/{entry_id}",
            get(cycle_routes::meeting_prep::meeting_prep_panel)
                .post(cycle_routes::meeting_prep::save_meeting_prep_panel),
        )
        .route(
            "/meeting-prep/panel/{entry_id}/ai-draft",
            post(cycle_routes::meeting_prep::ai_draft_meeting_prep),
        )
        // Check-ins
        .route("/checkins", get(reflections_routes::checkins::checkins_list))
        .route(
            "/checkin/{week_id}",
            get(reflections_routes::checkins::checkin_page)
                .post(reflections_routes::checkins::save_checkin)
                .delete(reflections_routes::checkins::delete_checkin),
        )
        // Contribution Examples
        .route(
            "/contribution-examples",
            get(review_routes::contribution_examples::contribution_examples_page)
                .post(review_routes::contribution_examples::create_contribution_example),
        )
        .route(
            "/contribution-examples/{id}",
            put(review_routes::contribution_examples::update_contribution_example)
                .delete(review_routes::contribution_examples::delete_contribution_example),
        )
        .route(
            "/contribution-examples/{example_id}/entries/{entry_id}",
            post(review_routes::contribution_examples::link_entry_to_example)
                .delete(review_routes::contribution_examples::unlink_entry_from_example),
        )
        // Quarterly Check-ins
        .route(
            "/quarterly-checkin/{quarter}/{year}",
            get(reflections_routes::checkins::quarterly_checkin_page)
                .post(reflections_routes::checkins::save_quarterly_checkin),
        )
        .route(
            "/quarterly-checkin/{quarter}/{year}/panel",
            get(reflections_routes::checkins::quarterly_checkin_panel),
        )
        // Logbook filtered entries (HTMX)
        .route(
            "/logbook/entries",
            get(cycle_routes::logbook::logbook_filtered_entries),
        )
        // Priorities
        .route("/priorities", get(objectives_routes::priorities_page).post(objectives_routes::create_priority))
        .route("/priorities/new-panel", get(objectives_routes::priority_form_panel))
        .route("/priorities/import-panel", get(objectives_routes::import_panel))
        .route("/priorities/{id}/edit-panel", get(objectives_routes::priority_edit_panel))
        .route("/priorities/goals/new-panel", get(objectives_routes::department_goal_form_panel))
        .route("/priorities/goals/{id}/edit-panel", get(objectives_routes::department_goal_edit_panel))
        .route("/priorities/import", post(objectives_routes::import_lattice_csv))
        .route(
            "/priorities/goals",
            post(objectives_routes::create_department_goal),
        )
        .route(
            "/priorities/goals/{id}",
            put(objectives_routes::update_department_goal)
                .delete(objectives_routes::delete_department_goal),
        )
        .route(
            "/priorities/{id}",
            put(objectives_routes::update_priority).delete(objectives_routes::delete_priority),
        )
        .route(
            "/priorities/{id}/updates",
            post(objectives_routes::post_priority_update),
        )
        // Phases (Performance Cycle)
        .route("/phases", post(cycle_routes::phases::create_phase))
        .route("/phases/{id}", delete(cycle_routes::phases::delete_phase))
        .route(
            "/phases/{id}/activate",
            post(cycle_routes::phases::activate_phase),
        )
        // Settings
        .route("/settings", get(identity_routes::settings_page))
        .route("/settings", post(identity_routes::save_settings))
        .route("/settings/people-alias", post(identity_routes::upsert_people_alias))
        .route("/settings/people-alias/{id}", delete(identity_routes::delete_people_alias))
        // Level Guide
        .route("/level-guide", get(identity_routes::clg_guide_page))
        // Review Guide
        .route("/review-guide", get(review_routes::summaries::review_guide_page))
        // Export
        .route("/export", get(review_routes::export::export_page))
        .route(
            "/export/download",
            get(review_routes::export::export_download),
        )
        // Integrations page + API routes
        .route("/integrations", get(integrations_routes::integrations_page))
        .route(
            "/integrations/{service}/test",
            post(integrations_routes::test_connection),
        )
        .route(
            "/integrations/{service}",
            post(integrations_routes::save_integration),
        )
        .route(
            "/integrations/{service}/reset",
            delete(integrations_routes::reset_integration),
        )
        .route(
            "/integrations/google_drive/excluded/{file_id}",
            delete(integrations_routes::restore_excluded_file),
        )
        .route(
            "/integrations/google_calendar/excluded/{event_id}",
            delete(integrations_routes::restore_excluded_event),
        )
        // Google Drive OAuth connect
        .route(
            "/integrations/google_drive/connect",
            get(identity_routes::connect_google_drive),
        )
        // Google Calendar OAuth connect
        .route(
            "/integrations/google_calendar/connect",
            get(identity_routes::connect_google_calendar),
        )
        // Sync
        .route("/sync/status", get(sync_routes::sync_status))
        .route("/sync/status/activity", get(sync_routes::sync_status_activity))
        .route("/sync/{service}", post(sync_routes::sync_service))
        .route("/sync/{service}/hard", post(sync_routes::hard_sync_service))
        .route("/sync/all", post(sync_routes::sync_all))
        .route("/sync/logs", delete(sync_routes::clear_sync_logs))
        // Self Review
        .route(
            "/review/{phase_id}",
            get(review_routes::summaries::summary_page),
        )
        .route(
            "/review/{phase_id}/generate",
            post(review_routes::summaries::generate_all),
        )
        .route(
            "/review/{phase_id}/ai-draft/{section}",
            post(review_routes::summaries::ai_draft_section),
        )
        .route(
            "/review/{phase_id}/save/{section}",
            post(review_routes::summaries::save_section),
        )
        .layer(middleware::from_fn(require_auth))
        .layer(middleware::from_fn(csrf_protection));

    Router::new()
        .merge(auth_routes)
        .merge(public_routes)
        .merge(protected_routes)
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
        .merge(create_router())
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/custom",
            ServeDir::new("custom").append_index_html_on_directories(false),
        )
        .layer(session_layer)
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}
