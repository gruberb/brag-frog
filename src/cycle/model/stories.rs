use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::deserialize_optional_string;

/// Raw DB row with encrypted contribution example fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContributionExampleRow {
    pub id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub outcome: Option<Vec<u8>>,
    pub behaviors: Option<Vec<u8>>,
    pub impact_level: Option<String>,
    pub learnings: Option<Vec<u8>>,
    pub assessment_type: Option<String>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl ContributionExampleRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<ContributionExample, AppError> {
        Ok(ContributionExample {
            id: self.id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            outcome: crypto.decrypt_opt(&self.outcome)?,
            behaviors: crypto.decrypt_opt(&self.behaviors)?,
            impact_level: self.impact_level,
            learnings: crypto.decrypt_opt(&self.learnings)?,
            assessment_type: self.assessment_type,
            status: self.status,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// A structured contribution example for self-assessments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionExample {
    pub id: i64,
    pub phase_id: i64,
    pub title: String,
    pub outcome: Option<String>,
    /// Key behaviors/skills demonstrated.
    pub behaviors: Option<String>,
    /// One of: `team`, `cross_team`, `org`, `company`.
    pub impact_level: Option<String>,
    pub learnings: Option<String>,
    /// One of: `mid_year`, `year_end`.
    pub assessment_type: Option<String>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Form payload for creating a contribution example.
#[derive(Debug, Deserialize)]
pub struct CreateContributionExample {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub outcome: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub behaviors: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub impact_level: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learnings: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub assessment_type: Option<String>,
}

/// Form payload for updating a contribution example.
#[derive(Debug, Deserialize)]
pub struct UpdateContributionExample {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub outcome: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub behaviors: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub impact_level: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learnings: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub assessment_type: Option<String>,
    pub status: Option<String>,
}
