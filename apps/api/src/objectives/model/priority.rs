use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::kernel::serde_helpers::{
    deserialize_optional_f64, deserialize_optional_i64, deserialize_optional_string,
};

/// Raw database row with encrypted title/impact_narrative/description.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriorityRow {
    pub id: i64,
    pub phase_id: i64,
    pub user_id: i64,
    pub title: Vec<u8>,
    pub status: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub scope: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub impact_narrative: Option<Vec<u8>>,
    pub department_goal_id: Option<i64>,
    pub created_at: String,
    pub priority_level: Option<String>,
    pub measure_type: Option<String>,
    pub measure_start: Option<f64>,
    pub measure_target: Option<f64>,
    pub measure_current: Option<f64>,
    pub description: Option<Vec<u8>>,
    /// Lattice-aligned trajectory status: on_track/progressing/off_track/complete/incomplete/no_update.
    pub tracking_status: Option<String>,
    pub due_date: Option<String>,
    /// Priority tier: department/team/individual.
    pub tier: Option<String>,
}

impl PriorityRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Priority, AppError> {
        Ok(Priority {
            id: self.id,
            phase_id: self.phase_id,
            user_id: self.user_id,
            title: crypto.decrypt(&self.title)?,
            status: self.status,
            color: self.color,
            sort_order: self.sort_order,
            scope: self.scope,
            started_at: self.started_at,
            completed_at: self.completed_at,
            impact_narrative: crypto.decrypt_opt(&self.impact_narrative)?,
            department_goal_id: self.department_goal_id,
            created_at: self.created_at,
            priority_level: self.priority_level,
            measure_type: self.measure_type,
            measure_start: self.measure_start,
            measure_target: self.measure_target,
            measure_current: self.measure_current,
            description: crypto.decrypt_opt(&self.description)?,
            tracking_status: self.tracking_status,
            due_date: self.due_date,
            tier: self.tier,
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
    /// Lifecycle status: `not_started`, `active`, `on_hold`, `completed`, `cancelled`.
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
    /// Priority level: `p0`–`p4`.
    pub priority_level: Option<String>,
    /// Measurement type: `percent`, `binary`, `number`.
    pub measure_type: Option<String>,
    pub measure_start: Option<f64>,
    pub measure_target: Option<f64>,
    pub measure_current: Option<f64>,
    pub description: Option<String>,
    /// Lattice-aligned trajectory: `on_track`, `progressing`, `off_track`, `complete`, `incomplete`, `no_update`.
    pub tracking_status: Option<String>,
    pub due_date: Option<String>,
    /// Priority tier: `department`, `team`, `individual`.
    pub tier: Option<String>,
}

/// Form payload for creating a priority.
#[derive(Debug, Deserialize)]
pub struct CreatePriority {
    pub title: String,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub scope: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub impact_narrative: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub department_goal_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub priority_level: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub measure_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_start: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_target: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tracking_status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub due_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tier: Option<String>,
}

/// Form payload for updating a priority.
#[derive(Debug, Deserialize)]
pub struct UpdatePriority {
    pub title: String,
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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub priority_level: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub measure_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_start: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_target: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_current: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tracking_status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub due_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tier: Option<String>,
}

/// Form payload for posting a priority update (progress log entry).
#[derive(Debug, Deserialize)]
pub struct PostPriorityUpdate {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tracking_status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub measure_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub comment: Option<String>,
    #[serde(default)]
    pub is_blocker: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub tradeoff_text: Option<String>,
}

/// A single priority update log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityUpdate {
    pub id: i64,
    pub priority_id: i64,
    pub user_id: i64,
    pub tracking_status: Option<String>,
    pub measure_value: Option<f64>,
    pub comment: Option<String>,
    pub is_blocker: i64,
    pub tradeoff_text: Option<String>,
    pub created_at: String,
}

/// Raw database row for priority updates (comment and tradeoff_text are encrypted BLOBs).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriorityUpdateRow {
    pub id: i64,
    pub priority_id: i64,
    pub user_id: i64,
    pub tracking_status: Option<String>,
    pub measure_value: Option<f64>,
    pub comment: Option<Vec<u8>>,
    pub is_blocker: i64,
    pub tradeoff_text: Option<Vec<u8>>,
    pub created_at: String,
}

impl PriorityUpdateRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<PriorityUpdate, AppError> {
        Ok(PriorityUpdate {
            id: self.id,
            priority_id: self.priority_id,
            user_id: self.user_id,
            tracking_status: self.tracking_status,
            measure_value: self.measure_value,
            comment: crypto.decrypt_opt(&self.comment)?,
            is_blocker: self.is_blocker,
            tradeoff_text: crypto.decrypt_opt(&self.tradeoff_text)?,
            created_at: self.created_at,
        })
    }
}
