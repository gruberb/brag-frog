use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::{deserialize_optional_i64, deserialize_optional_string};

/// Raw database row with encrypted title/description/impact_narrative.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriorityRow {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub title: Vec<u8>,
    pub description: Option<Vec<u8>>,
    pub status: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub scope: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub impact_narrative: Option<Vec<u8>>,
    pub department_goal_id: Option<i64>,
    pub created_at: String,
}

impl PriorityRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Priority, AppError> {
        Ok(Priority {
            id: self.id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            title: crypto.decrypt(&self.title)?,
            description: crypto.decrypt_opt(&self.description)?,
            status: self.status,
            color: self.color,
            sort_order: self.sort_order,
            scope: self.scope,
            started_at: self.started_at,
            completed_at: self.completed_at,
            impact_narrative: crypto.decrypt_opt(&self.impact_narrative)?,
            department_goal_id: self.department_goal_id,
            created_at: self.created_at,
        })
    }
}

/// A user priority — qualitative focus area for a performance cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: Option<String>,
    /// One of: `not_started`, `active`, `on_hold`, `completed`, `cancelled`.
    pub status: String,
    /// Hex color for UI badges.
    pub color: Option<String>,
    pub sort_order: i64,
    /// One of: `small`, `medium`, `large`, `xlarge`.
    pub scope: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    /// Narrative describing the outcome and significance of this body of work.
    pub impact_narrative: Option<String>,
    pub department_goal_id: Option<i64>,
    pub created_at: String,
}

/// Form payload for creating a priority.
#[derive(Debug, Deserialize)]
pub struct CreatePriority {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub scope: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub department_goal_id: Option<i64>,
}

/// Form payload for updating a priority.
#[derive(Debug, Deserialize)]
pub struct UpdatePriority {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub scope: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub impact_narrative: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub department_goal_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub started_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub completed_at: Option<String>,
}
