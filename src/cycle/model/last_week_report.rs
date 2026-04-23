use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

/// Encrypted row as stored in `last_week_reports`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LastWeekReportRow {
    pub id: i64,
    pub week_id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub content: Option<Vec<u8>>,
    pub window_start: String,
    pub window_end: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for `LastWeekReport::upsert`. Content and window travel together —
/// the narrative and its date range are meaningless in isolation.
#[derive(Debug, Clone)]
pub struct SaveLastWeekReport<'a> {
    pub content: Option<&'a str>,
    pub window_start: &'a str,
    pub window_end: &'a str,
}

/// Decrypted domain view of a saved Last Week report.
///
/// The report is anchored to a `week_id` (one report per current week per user).
/// `window_start`/`window_end` describe the range the narrative covers and are
/// captured at generation time, so revisiting the page later doesn't silently
/// shift what the stored text refers to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastWeekReport {
    pub id: i64,
    pub week_id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub content: Option<String>,
    pub window_start: String,
    pub window_end: String,
    pub created_at: String,
    pub updated_at: String,
}

impl LastWeekReportRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<LastWeekReport, AppError> {
        Ok(LastWeekReport {
            id: self.id,
            week_id: self.week_id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            content: crypto.decrypt_opt(&self.content)?,
            window_start: self.window_start,
            window_end: self.window_end,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
