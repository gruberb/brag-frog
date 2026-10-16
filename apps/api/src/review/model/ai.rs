use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

/// Raw DB row with encrypted content/prompt fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiDocumentRow {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    pub doc_type: String,
    pub title: String,
    pub content: Vec<u8>,
    pub prompt_used: Option<Vec<u8>>,
    pub model_used: Option<String>,
    pub context_week_id: Option<i64>,
    pub meeting_entry_id: Option<i64>,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub generated_at: String,
}

impl AiDocumentRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<AiDocument, AppError> {
        Ok(AiDocument {
            id: self.id,
            user_id: self.user_id,
            phase_id: self.phase_id,
            doc_type: self.doc_type,
            title: self.title,
            content: crypto.decrypt(&self.content)?,
            prompt_used: crypto.decrypt_opt(&self.prompt_used)?,
            model_used: self.model_used,
            context_week_id: self.context_week_id,
            meeting_entry_id: self.meeting_entry_id,
            meeting_role: self.meeting_role,
            recurring_group: self.recurring_group,
            generated_at: self.generated_at,
        })
    }
}

/// An AI-generated document (meeting prep, weekly digest, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDocument {
    pub id: i64,
    pub user_id: i64,
    pub phase_id: i64,
    /// One of: `meeting_prep`, `weekly_digest`.
    pub doc_type: String,
    pub title: String,
    pub content: String,
    pub prompt_used: Option<String>,
    pub model_used: Option<String>,
    pub context_week_id: Option<i64>,
    pub meeting_entry_id: Option<i64>,
    pub meeting_role: Option<String>,
    pub recurring_group: Option<String>,
    pub generated_at: String,
}
