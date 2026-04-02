use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::deserialize_optional_string;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StatusUpdateRow {
    pub id: i64,
    pub week_id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub content: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub id: i64,
    pub week_id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub content: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl StatusUpdateRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<StatusUpdate, AppError> {
        Ok(StatusUpdate {
            id: self.id,
            week_id: self.week_id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            content: crypto.decrypt_opt(&self.content)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveStatusUpdate {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub content: Option<String>,
}
