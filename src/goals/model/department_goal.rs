use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::deserialize_optional_string;

/// Raw database row with AES-256-GCM encrypted `title` and `description` BLOBs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DepartmentGoalRow {
    pub id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub description: Option<Vec<u8>>,
    pub status: String,
    pub sort_order: i64,
    pub source: String,
    pub created_at: String,
}

impl DepartmentGoalRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<DepartmentGoal, AppError> {
        Ok(DepartmentGoal {
            id: self.id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            description: crypto.decrypt_opt(&self.description)?,
            status: self.status,
            sort_order: self.sort_order,
            source: self.source,
            created_at: self.created_at,
        })
    }
}

/// A department-level goal given by leadership, scoped to a phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentGoal {
    pub id: i64,
    pub phase_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i64,
    /// One of: `manual`, `imported`.
    pub source: String,
    pub created_at: String,
}

/// Form payload for creating a department goal.
#[derive(Debug, Deserialize)]
pub struct CreateDepartmentGoal {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
}

/// Form payload for updating a department goal.
#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentGoal {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
}
