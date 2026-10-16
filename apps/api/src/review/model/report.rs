use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

/// The Logbook filter set a report was built from. Keys mirror the entries listing so the
/// SPA can send the filters it already holds. Comma-separated fields hold several values.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportFilters {
    /// Inclusive `YYYY-MM-DD`. Defaults to the cycle start.
    #[serde(default)]
    #[ts(optional)]
    pub from: Option<String>,
    /// Inclusive `YYYY-MM-DD`. Defaults to the cycle end.
    #[serde(default)]
    #[ts(optional)]
    pub to: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub priority_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub goal_id: Option<i64>,
    /// Comma-separated sync sources, for example `github,jira`.
    #[serde(default)]
    #[ts(optional)]
    pub source: Option<String>,
    /// Comma-separated entry type slugs.
    #[serde(default, rename = "type")]
    #[ts(optional, rename = "type")]
    pub entry_type: Option<String>,
    /// Case-insensitive title search.
    #[serde(default)]
    #[ts(optional)]
    pub q: Option<String>,
    /// Comma-separated team names.
    #[serde(default)]
    #[ts(optional)]
    pub team: Option<String>,
    /// Comma-separated collaborator names (after alias resolution).
    #[serde(default)]
    #[ts(optional)]
    pub person: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reach: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub complexity: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub role: Option<String>,
}

/// Raw database row. `title`, `content` and `stats_json` are encrypted.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportRow {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub filters_json: String,
    pub content: Vec<u8>,
    pub content_format: String,
    pub stats_json: Vec<u8>,
    pub model_used: Option<String>,
    pub generated_at: String,
    pub updated_at: String,
}

impl ReportRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Report, AppError> {
        Ok(Report {
            id: self.id,
            user_id: self.user_id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            filters_json: self.filters_json,
            content: crypto.decrypt(&self.content)?,
            content_format: self.content_format,
            stats_json: crypto.decrypt(&self.stats_json)?,
            model_used: self.model_used,
            generated_at: self.generated_at,
            updated_at: self.updated_at,
        })
    }
}

/// A saved report. `stats_json` is the snapshot (stats plus key entries) taken at generation
/// time, kept as text here because its shape belongs to the API layer.
#[derive(Debug, Clone)]
pub struct Report {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    pub title: String,
    pub filters_json: String,
    pub content: String,
    /// `markdown` for AI narratives, `blocknote` once edited in the SPA.
    pub content_format: String,
    pub stats_json: String,
    pub model_used: Option<String>,
    pub generated_at: String,
    pub updated_at: String,
}

/// Values for inserting a report.
pub struct NewReport<'a> {
    pub title: &'a str,
    pub filters_json: &'a str,
    pub content: &'a str,
    pub content_format: &'a str,
    pub stats_json: &'a str,
    pub model_used: Option<&'a str>,
}
