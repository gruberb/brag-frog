use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::deserialize_optional_string;

/// Raw DB row with encrypted quarterly reflection fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuarterlyCheckinRow {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub quarter: String,
    pub year: i64,
    pub highlights_impact: Option<Vec<u8>>,
    pub learnings_adjustments: Option<Vec<u8>>,
    pub growth_development: Option<Vec<u8>>,
    pub support_feedback: Option<Vec<u8>>,
    pub looking_ahead: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

impl QuarterlyCheckinRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<QuarterlyCheckin, AppError> {
        Ok(QuarterlyCheckin {
            id: self.id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            quarter: self.quarter,
            year: self.year,
            highlights_impact: crypto.decrypt_opt(&self.highlights_impact)?,
            learnings_adjustments: crypto.decrypt_opt(&self.learnings_adjustments)?,
            growth_development: crypto.decrypt_opt(&self.growth_development)?,
            support_feedback: crypto.decrypt_opt(&self.support_feedback)?,
            looking_ahead: crypto.decrypt_opt(&self.looking_ahead)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Review check-in for a non-review quarter in the performance cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarterlyCheckin {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    /// One of: `Q1`, `Q2`, `Q3`, `Q4`.
    pub quarter: String,
    pub year: i64,
    pub highlights_impact: Option<String>,
    pub learnings_adjustments: Option<String>,
    pub growth_development: Option<String>,
    pub support_feedback: Option<String>,
    pub looking_ahead: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Form payload for saving a quarterly check-in.
/// `quarter` and `year` are populated from the URL path by the handler,
/// so they default to empty/zero to avoid 422 when absent from the form body.
#[derive(Debug, Deserialize)]
pub struct SaveQuarterlyCheckin {
    #[serde(default)]
    pub quarter: String,
    #[serde(default)]
    pub year: i64,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub highlights_impact: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learnings_adjustments: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub growth_development: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub support_feedback: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub looking_ahead: Option<String>,
}
