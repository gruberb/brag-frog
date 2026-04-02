//! Domain types and pure logic for the cycle bounded context.
//! Phases, weeks, focus items, meetings.

mod focus;
mod meeting;
pub mod status_update;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use focus::*;
pub use meeting::*;
pub use status_update::*;

// ---------------------------------------------------------------------------
// Phase
// ---------------------------------------------------------------------------

/// A performance review cycle (e.g., "H1 2025"). At most one is active per user.
/// Owns weeks, goals, key results, and summaries; deletion cascades aggressively.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BragPhase {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    /// ISO 8601 date string (`YYYY-MM-DD`).
    pub start_date: String,
    /// ISO 8601 date string (`YYYY-MM-DD`).
    pub end_date: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Form input for creating a new phase.
#[derive(Debug, Deserialize)]
pub struct CreatePhase {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
}

// ---------------------------------------------------------------------------
// Week
// ---------------------------------------------------------------------------

/// An ISO week within a phase. Created implicitly when the logbook is visited
/// or when a sync produces entries for a new week.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Week {
    pub id: i64,
    pub phase_id: i64,
    /// 1-based ordinal within this phase (not ISO week number).
    pub week_number: i64,
    pub iso_week: i64,
    pub year: i64,
    pub start_date: String,
    pub end_date: String,
}

impl Week {
    /// Builds a `week_id -> JSON` lookup map for template consumption.
    pub fn to_json_map(weeks: &[Week]) -> HashMap<i64, serde_json::Value> {
        let mut map = HashMap::new();
        for w in weeks {
            map.insert(
                w.id,
                serde_json::json!({
                    "id": w.id,
                    "iso_week": w.iso_week,
                    "year": w.year,
                    "start_date": w.start_date,
                    "end_date": w.end_date,
                }),
            );
        }
        map
    }
}

// Converts ISO year + week number to the Monday of that week.
pub(crate) fn iso_week_to_date(year: i32, week: u32) -> chrono::NaiveDate {
    use chrono::{Datelike, NaiveDate};
    // ISO week 1 contains January 4th
    let jan4 = NaiveDate::from_ymd_opt(year, 1, 4).unwrap();
    let jan4_weekday = jan4.weekday().num_days_from_monday();
    let week1_monday = jan4 - chrono::Duration::days(jan4_weekday as i64);
    week1_monday + chrono::Duration::weeks((week - 1) as i64)
}
