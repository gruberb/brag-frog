use serde::{Deserialize, Serialize};

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::shared::serde_helpers::{deserialize_optional_i64, deserialize_optional_string};

// ---------------------------------------------------------------------------
// Goal
// ---------------------------------------------------------------------------

/// Raw database row with AES-256-GCM encrypted `title` and `description` BLOBs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GoalRow {
    pub id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub description: Option<Vec<u8>>,
    pub category: Option<String>,
    pub sort_order: i64,
    pub status: String,
    pub weight: Option<i64>,
    pub created_at: String,
}

impl GoalRow {
    /// Decrypts title and description into a [`Goal`].
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Goal, AppError> {
        Ok(Goal {
            id: self.id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            description: crypto.decrypt_opt(&self.description)?,
            category: self.category,
            sort_order: self.sort_order,
            status: self.status,
            weight: self.weight,
            created_at: self.created_at,
        })
    }
}

/// A high-level objective scoped to a phase (e.g., "Turn OHTTP into a reusable capability").
/// Owns key results, which in turn own entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: i64,
    pub phase_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sort_order: i64,
    pub status: String,
    /// Optional percentage weight 0-100 for goal weighting.
    pub weight: Option<i64>,
    pub created_at: String,
}

/// Form payload for creating a goal.
#[derive(Debug, Deserialize)]
pub struct CreateGoal {
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
}

/// Form payload for updating a goal.
#[derive(Debug, Deserialize)]
pub struct UpdateGoal {
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// KeyResult
// ---------------------------------------------------------------------------

/// A measurable outcome under a [`Goal`](super::Goal) (e.g., "Ship viaduct component").
/// Entries link to goals only through their key result's `goal_id`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct KeyResult {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    /// Hex color for UI badges. Randomly assigned on creation.
    pub color: Option<String>,
    pub is_archived: bool,
    /// One of: `not_started`, `in_progress`, `on_hold`, `completed`.
    pub status: String,
    /// Optional parent goal FK. `None` = unassigned.
    pub goal_id: Option<i64>,
    /// Completion percentage (0-100).
    pub progress: i64,
    // Measurement fields
    /// One of: `manual`, `numeric`, `boolean`, `milestone`.
    pub kr_type: String,
    /// For numeric KRs: `increase`, `decrease`, or `maintain`.
    pub direction: Option<String>,
    /// Free text unit label (e.g., "ms", "%", "tickets/week").
    pub unit: Option<String>,
    /// Starting value for numeric KRs.
    pub baseline: Option<f64>,
    /// Target value for numeric KRs.
    pub target: Option<f64>,
    /// Latest actual value.
    pub current_value: Option<f64>,
    /// Target completion date (YYYY-MM-DD) for milestone KRs.
    pub target_date: Option<String>,
    /// Auto-calculated score 0.0-1.0.
    pub score: Option<f64>,
    pub created_at: String,
}

impl KeyResult {
    /// Calculates the score (0.0-1.0) based on kr_type and measurement fields.
    pub fn recalculate_score(&self) -> Option<f64> {
        match self.kr_type.as_str() {
            "numeric" => {
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
            "boolean" => Some(if self.current_value.unwrap_or(0.0) >= 1.0 {
                1.0
            } else {
                0.0
            }),
            "milestone" => Some(if self.status == "completed" { 1.0 } else { 0.0 }),
            _ => {
                // manual: use progress field
                Some(self.progress as f64 / 100.0)
            }
        }
    }
}

/// Form payload for creating a key result.
#[derive(Debug, Deserialize)]
pub struct CreateKeyResult {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub goal_id: Option<i64>,
    pub status: Option<String>,
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

/// Form payload for updating a key result.
#[derive(Debug, Deserialize)]
pub struct UpdateKeyResult {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub goal_id: Option<i64>,
    pub status: Option<String>,
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
}

// ---------------------------------------------------------------------------
// Initiative
// ---------------------------------------------------------------------------

/// Raw database row with encrypted title/description.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InitiativeRow {
    pub id: i64,
    pub phase_id: i64,
    pub title: Vec<u8>,
    pub description: Option<Vec<u8>>,
    pub status: String,
    pub scope: Option<String>,
    pub is_planned: bool,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

impl InitiativeRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Initiative, AppError> {
        Ok(Initiative {
            id: self.id,
            phase_id: self.phase_id,
            title: crypto.decrypt(&self.title)?,
            description: crypto.decrypt_opt(&self.description)?,
            status: self.status,
            scope: self.scope,
            is_planned: self.is_planned,
            started_at: self.started_at,
            completed_at: self.completed_at,
            sort_order: self.sort_order,
            created_at: self.created_at,
        })
    }
}

/// A project or workstream that drives key results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Initiative {
    pub id: i64,
    pub phase_id: i64,
    pub title: String,
    pub description: Option<String>,
    /// One of: `planned`, `active`, `completed`, `paused`, `cancelled`.
    pub status: String,
    /// One of: `small`, `medium`, `large`, `xlarge`.
    pub scope: Option<String>,
    pub is_planned: bool,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateInitiative {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub scope: Option<String>,
    #[serde(default)]
    pub is_planned: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInitiative {
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub scope: Option<String>,
    #[serde(default)]
    pub is_planned: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub started_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub completed_at: Option<String>,
}
