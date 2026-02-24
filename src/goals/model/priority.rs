use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::{deserialize_optional_i64, deserialize_optional_string};

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
    pub kr_type: Option<String>,
    pub direction: Option<String>,
    pub unit: Option<String>,
    pub baseline: Option<f64>,
    pub target: Option<f64>,
    pub current_value: Option<f64>,
    pub target_date: Option<String>,
    pub score: Option<f64>,
    pub progress: i64,
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
            kr_type: self.kr_type,
            direction: self.direction,
            unit: self.unit,
            baseline: self.baseline,
            target: self.target,
            current_value: self.current_value,
            target_date: self.target_date,
            score: self.score,
            progress: self.progress,
            created_at: self.created_at,
        })
    }
}

/// A user priority — workstream, project, or measurable outcome.
/// Merges what was previously key_results + initiatives.
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
    // Optional measurement fields (preserved from KRs)
    pub kr_type: Option<String>,
    pub direction: Option<String>,
    pub unit: Option<String>,
    pub baseline: Option<f64>,
    pub target: Option<f64>,
    pub current_value: Option<f64>,
    pub target_date: Option<String>,
    pub score: Option<f64>,
    pub progress: i64,
    pub created_at: String,
}

impl Priority {
    /// Calculates the score (0.0-1.0) based on kr_type and measurement fields.
    pub fn recalculate_score(&self) -> Option<f64> {
        match self.kr_type.as_deref() {
            Some("numeric") => {
                let baseline = self.baseline.unwrap_or(0.0);
                let target = self.target.unwrap_or(0.0);
                let current = self.current_value.unwrap_or(baseline);
                let range = target - baseline;
                if range.abs() < f64::EPSILON {
                    return Some(if (current - target).abs() < f64::EPSILON {
                        1.0
                    } else {
                        0.0
                    });
                }
                let raw = match self.direction.as_deref() {
                    Some("decrease") => (baseline - current) / (baseline - target),
                    _ => (current - baseline) / (target - baseline),
                };
                Some(raw.clamp(0.0, 1.0))
            }
            Some("boolean") => Some(if self.current_value.unwrap_or(0.0) >= 1.0 {
                1.0
            } else {
                0.0
            }),
            Some("milestone") => Some(if self.status == "completed" { 1.0 } else { 0.0 }),
            Some("manual") => Some(self.progress as f64 / 100.0),
            _ => None,
        }
    }
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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub kr_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub direction: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub unit: Option<String>,
    #[serde(default)]
    pub baseline: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub target_date: Option<String>,
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
    #[serde(default)]
    pub progress: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub kr_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub direction: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub unit: Option<String>,
    #[serde(default)]
    pub baseline: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default)]
    pub current_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub target_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub started_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub completed_at: Option<String>,
}
