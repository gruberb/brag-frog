use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

/// Raw DB row with encrypted notes.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MeetingPrepNoteRow {
    pub id: i64,
    pub user_id: i64,
    pub week_id: i64,
    pub entry_id: Option<i64>,
    pub notes: Option<Vec<u8>>,
    pub meeting_goal: Option<Vec<u8>>,
    pub doc_urls: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Meeting prep note — per meeting entry per week.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingPrepNote {
    pub id: i64,
    pub user_id: i64,
    pub week_id: i64,
    pub entry_id: Option<i64>,
    pub notes: Option<String>,
    pub meeting_goal: Option<String>,
    pub doc_urls: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MeetingPrepNoteRow {
    /// Decrypt an encrypted note row into a plaintext meeting prep note struct.
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<MeetingPrepNote, AppError> {
        Ok(MeetingPrepNote {
            id: self.id,
            user_id: self.user_id,
            week_id: self.week_id,
            entry_id: self.entry_id,
            notes: crypto.decrypt_opt(&self.notes)?,
            meeting_goal: crypto.decrypt_opt(&self.meeting_goal)?,
            doc_urls: self.doc_urls,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// A rule for auto-classifying recurring meetings by role.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MeetingRule {
    pub id: i64,
    pub user_id: i64,
    /// One of: `recurring_group`, `title_contains`.
    pub match_type: String,
    pub match_value: String,
    /// One of: `manager`, `tech_lead`, `skip_level`, `peer`, `stakeholder`.
    pub meeting_role: String,
    pub person_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRule {
    pub match_type: String,
    pub match_value: String,
    pub meeting_role: String,
    pub person_name: Option<String>,
}
