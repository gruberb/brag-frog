//! Brag Frog library crate. Declares all modules and the shared application state.

pub mod ai;
pub mod app;
pub mod db;
pub mod worklog;
pub mod identity;
pub mod objectives;
pub mod cycle;
pub mod reflections;
pub mod review;
pub mod kernel;
pub mod integrations;
pub mod protocol;
pub mod todos;

use std::collections::HashMap;
use std::sync::Arc;

use tera::Tera;

use kernel::config::Config;
use kernel::crypto::Crypto;
use integrations::sync_status::SyncStatusMap;

/// Registers custom Tera filters used by templates.
/// Called at startup from `app::build_state()` and in test helpers.
pub fn register_tera_filters(templates: &mut Tera) {
    templates.register_filter("entry_type_label", entry_type_label_filter);
    templates.register_filter("source_label", source_label_filter);
}

fn entry_type_label_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let slug = tera::try_get_value!("entry_type_label", "value", String, value);
    Ok(tera::Value::String(
        worklog::model::EntryType::display_name(&slug).to_string(),
    ))
}

fn source_label_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let slug = tera::try_get_value!("source_label", "value", String, value);
    Ok(tera::Value::String(
        worklog::model::source_display_name(&slug).to_string(),
    ))
}

/// Shared application state threaded through all Axum handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// SQLite connection pool (WAL mode, FK-enabled, max 5 connections).
    pub db: sqlx::SqlitePool,
    /// Server and OAuth configuration loaded from `BRAGFOX_*` env vars.
    pub config: Arc<Config>,
    /// Pre-compiled Tera template engine (loaded once at startup).
    pub templates: Arc<Tera>,
    /// AES-256-GCM encryption context for API token storage.
    pub crypto: Arc<Crypto>,
    /// In-memory sync status per user, updated by background sync tasks.
    pub sync_status: SyncStatusMap,
}
