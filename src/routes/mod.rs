mod public;

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::AppState;
use crate::identity::auth::middleware::require_auth;
use crate::shared::middleware::csrf_protection;
use crate::shared::render::{static_page_privacy, static_page_terms};

// Identity context routes
use crate::identity::routes as identity_routes;
// Entries context routes
use crate::entries::routes as entries_routes;
// Goals context routes
use crate::goals::routes as goals_routes;
// Review context routes
use crate::review::routes as review_routes;
// Sync context routes
use crate::sync::integrations_routes;
use crate::sync::sync_routes;

/// Assembles the full application router: public, auth, and protected route groups.
/// Protected routes are gated by `require_auth` and CSRF middleware.
pub fn create_router() -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/auth/login", get(identity_routes::login_page))
        .route("/auth/callback", get(identity_routes::callback))
        .route("/auth/logout", post(identity_routes::logout));

    let public_routes = Router::new()
        .route("/", get(public::landing_page))
        .route("/privacy", get(static_page_privacy))
        .route("/terms", get(static_page_terms));

    let protected_routes = Router::new()
        // Dashboard
        .route("/dashboard", get(review_routes::dashboard::dashboard))
        .route(
            "/focus/week/{week_id}",
            post(review_routes::dashboard::create_focus),
        )
        .route(
            "/focus/{focus_id}",
            put(review_routes::dashboard::update_focus)
                .delete(review_routes::dashboard::delete_focus),
        )
        // Logbook
        .route("/logbook", get(review_routes::logbook::logbook))
        // Entries
        .route("/entries/quick", post(entries_routes::quick_create_entry))
        .route("/entries/{id}", put(entries_routes::update_entry))
        .route("/entries/{id}", delete(entries_routes::delete_entry))
        .route("/entries/{id}/view", get(entries_routes::view_entry))
        .route("/entries/{id}/panel", get(entries_routes::entry_panel))
        .route(
            "/entries/{id}/classify",
            post(entries_routes::classify_entry),
        )
        .route(
            "/entries/{id}/exclude-file",
            post(entries_routes::exclude_drive_file),
        )
        .route(
            "/entries/{id}/exclude-event",
            post(entries_routes::exclude_calendar_event),
        )
        // Trends
        .route("/trends", get(review_routes::trends::trends_page))
        // Meeting Prep (panel only)
        .route(
            "/meeting-prep/panel/{entry_id}",
            get(review_routes::meeting_prep::meeting_prep_panel)
                .post(review_routes::meeting_prep::save_meeting_prep_panel),
        )
        .route(
            "/meeting-prep/panel/{entry_id}/ai-draft",
            post(review_routes::meeting_prep::ai_draft_meeting_prep),
        )
        // Check-ins
        .route("/checkins", get(review_routes::checkins::checkins_list))
        .route(
            "/checkin/{week_id}",
            get(review_routes::checkins::checkin_page)
                .post(review_routes::checkins::save_checkin)
                .delete(review_routes::checkins::delete_checkin),
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
            get(review_routes::checkins::quarterly_checkin_page)
                .post(review_routes::checkins::save_quarterly_checkin),
        )
        // Logbook filtered entries (HTMX)
        .route(
            "/logbook/entries",
            get(review_routes::logbook::logbook_filtered_entries),
        )
        // Priorities
        .route("/priorities", get(goals_routes::priorities_page))
        .route(
            "/priorities/goals",
            post(goals_routes::create_department_goal),
        )
        .route(
            "/priorities/goals/{id}",
            put(goals_routes::update_department_goal)
                .delete(goals_routes::delete_department_goal),
        )
        .route("/priorities/create", post(goals_routes::create_priority))
        .route(
            "/priorities/{id}",
            put(goals_routes::update_priority).delete(goals_routes::delete_priority),
        )
        // Phases (Performance Cycle)
        .route("/phases", post(review_routes::phases::create_phase))
        .route("/phases/{id}", delete(review_routes::phases::delete_phase))
        .route(
            "/phases/{id}/activate",
            post(review_routes::phases::activate_phase),
        )
        // Settings
        .route("/settings", get(identity_routes::settings_page))
        .route("/settings", post(identity_routes::save_settings))
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
