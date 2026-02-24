use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

/// Raw DB row with encrypted title.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeeklyFocusRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: Vec<u8>,
    pub linked_type: Option<String>,
    pub linked_id: Option<i64>,
    pub link_1: Option<String>,
    pub link_2: Option<String>,
    pub link_3: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Weekly focus item — up to 3 per week per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyFocus {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: String,
    pub linked_type: Option<String>,
    pub linked_id: Option<i64>,
    pub link_1: Option<String>,
    pub link_2: Option<String>,
    pub link_3: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl WeeklyFocusRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<WeeklyFocus, AppError> {
        Ok(WeeklyFocus {
            id: self.id,
            week_id: self.week_id,
            user_id: self.user_id,
            sort_order: self.sort_order,
            title: crypto.decrypt(&self.title)?,
            linked_type: self.linked_type,
            linked_id: self.linked_id,
            link_1: self.link_1,
            link_2: self.link_2,
            link_3: self.link_3,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Parameters for creating a weekly focus item.
pub struct CreateFocusParams<'a> {
    pub week_id: i64,
    pub user_id: i64,
    pub sort_order: i64,
    pub title: &'a str,
    pub linked_type: Option<&'a str>,
    pub linked_id: Option<i64>,
    pub link_1: Option<&'a str>,
    pub link_2: Option<&'a str>,
    pub link_3: Option<&'a str>,
}

/// Parameters for updating a weekly focus item.
pub struct UpdateFocusParams<'a> {
    pub title: &'a str,
    pub linked_type: Option<&'a str>,
    pub linked_id: Option<i64>,
    pub link_1: Option<&'a str>,
    pub link_2: Option<&'a str>,
    pub link_3: Option<&'a str>,
}

/// Join table linking a focus item to brag entries.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WeeklyFocusEntry {
    pub focus_id: i64,
    pub entry_id: i64,
}
