use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

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
