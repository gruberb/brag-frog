use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::deserialize_optional_string;

/// Raw DB row with encrypted reflection fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WeeklyCheckinRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub highlights_impact: Option<Vec<u8>>,
    pub learnings_adjustments: Option<Vec<u8>>,
    pub growth_development: Option<Vec<u8>>,
    pub support_feedback: Option<Vec<u8>>,
    pub looking_ahead: Option<Vec<u8>>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl WeeklyCheckinRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<WeeklyCheckin, AppError> {
        Ok(WeeklyCheckin {
            id: self.id,
            week_id: self.week_id,
            user_id: self.user_id,
            highlights_impact: crypto.decrypt_opt(&self.highlights_impact)?,
            learnings_adjustments: crypto.decrypt_opt(&self.learnings_adjustments)?,
            growth_development: crypto.decrypt_opt(&self.growth_development)?,
            support_feedback: crypto.decrypt_opt(&self.support_feedback)?,
            looking_ahead: crypto.decrypt_opt(&self.looking_ahead)?,
            energy_level: self.energy_level,
            productivity_rating: self.productivity_rating,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Weekly structured reflection — one per week per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyCheckin {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub highlights_impact: Option<String>,
    pub learnings_adjustments: Option<String>,
    pub growth_development: Option<String>,
    pub support_feedback: Option<String>,
    pub looking_ahead: Option<String>,
    /// 1-5 energy rating.
    pub energy_level: Option<i64>,
    /// 1-5 productivity rating.
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Form payload for saving a weekly check-in.
#[derive(Debug, Deserialize)]
pub struct SaveCheckin {
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
    #[serde(default)]
    pub energy_level: Option<i64>,
    #[serde(default)]
    pub productivity_rating: Option<i64>,
}

/// Raw DB row for checkin + week join.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CheckinWithWeekRow {
    pub id: i64,
    pub week_id: i64,
    pub user_id: i64,
    pub highlights_impact: Option<Vec<u8>>,
    pub learnings_adjustments: Option<Vec<u8>>,
    pub growth_development: Option<Vec<u8>>,
    pub support_feedback: Option<Vec<u8>>,
    pub looking_ahead: Option<Vec<u8>>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub iso_week: i64,
    pub year: i64,
    pub week_start: String,
    pub week_end: String,
    pub phase_name: String,
}

/// Decrypted checkin with week metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinWithWeek {
    pub id: i64,
    pub week_id: i64,
    pub highlights_impact: Option<String>,
    pub learnings_adjustments: Option<String>,
    pub growth_development: Option<String>,
    pub support_feedback: Option<String>,
    pub looking_ahead: Option<String>,
    pub energy_level: Option<i64>,
    pub productivity_rating: Option<i64>,
    pub created_at: String,
    pub iso_week: i64,
    pub year: i64,
    pub week_start: String,
    pub week_end: String,
    pub phase_name: String,
}

impl CheckinWithWeekRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<CheckinWithWeek, AppError> {
        Ok(CheckinWithWeek {
            id: self.id,
            week_id: self.week_id,
            highlights_impact: crypto.decrypt_opt(&self.highlights_impact)?,
            learnings_adjustments: crypto.decrypt_opt(&self.learnings_adjustments)?,
            growth_development: crypto.decrypt_opt(&self.growth_development)?,
            support_feedback: crypto.decrypt_opt(&self.support_feedback)?,
            looking_ahead: crypto.decrypt_opt(&self.looking_ahead)?,
            energy_level: self.energy_level,
            productivity_rating: self.productivity_rating,
            created_at: self.created_at,
            iso_week: self.iso_week,
            year: self.year,
            week_start: self.week_start,
            week_end: self.week_end,
            phase_name: self.phase_name,
        })
    }
}

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

/// Quarterly check-in synthesized from weekly reflections.
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
#[derive(Debug, Deserialize)]
pub struct SaveQuarterlyCheckin {
    pub quarter: String,
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

/// Raw DB row with encrypted annual alignment fields.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AnnualAlignmentRow {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub year: i64,
    pub top_outcomes: Option<Vec<u8>>,
    pub why_it_matters: Option<Vec<u8>>,
    pub success_criteria: Option<Vec<u8>>,
    pub learning_goals: Option<Vec<u8>>,
    pub support_needed: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

impl AnnualAlignmentRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<AnnualAlignment, AppError> {
        Ok(AnnualAlignment {
            id: self.id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            year: self.year,
            top_outcomes: crypto.decrypt_opt(&self.top_outcomes)?,
            why_it_matters: crypto.decrypt_opt(&self.why_it_matters)?,
            success_criteria: crypto.decrypt_opt(&self.success_criteria)?,
            learning_goals: crypto.decrypt_opt(&self.learning_goals)?,
            support_needed: crypto.decrypt_opt(&self.support_needed)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Annual priorities and expectations alignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualAlignment {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub year: i64,
    pub top_outcomes: Option<String>,
    pub why_it_matters: Option<String>,
    pub success_criteria: Option<String>,
    pub learning_goals: Option<String>,
    pub support_needed: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Form payload for saving an annual alignment.
#[derive(Debug, Deserialize)]
pub struct SaveAnnualAlignment {
    pub year: i64,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub top_outcomes: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub why_it_matters: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub success_criteria: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub learning_goals: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub support_needed: Option<String>,
}
