//! Read-only access to data written by features that no longer exist: quarterly check-ins,
//! contribution examples, status updates and last-week reports. The tables stay in the
//! database so past cycles remain viewable; nothing writes to them anymore.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct QuarterlyCheckin {
    pub id: i64,
    pub quarter: String,
    pub year: i64,
    pub highlights_impact: Option<String>,
    pub learnings_adjustments: Option<String>,
    pub growth_development: Option<String>,
    pub support_feedback: Option<String>,
    pub looking_ahead: Option<String>,
    pub updated_at: String,
}

/// A self-review example. Besides the archive view, these still feed the AI review prompt.
#[derive(Debug, Clone, Serialize)]
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
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusUpdate {
    pub id: i64,
    pub week_id: i64,
    pub content: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastWeekReport {
    pub id: i64,
    pub window_start: String,
    pub window_end: String,
    pub content: Option<String>,
    pub updated_at: String,
}

type Blob = Option<Vec<u8>>;

pub async fn checkins(
    pool: &SqlitePool,
    phase_id: i64,
    crypto: &UserCrypto,
) -> Result<Vec<QuarterlyCheckin>, AppError> {
    type Row = (i64, String, i64, Blob, Blob, Blob, Blob, Blob, String);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, quarter, year, highlights_impact, learnings_adjustments, growth_development, \
         support_feedback, looking_ahead, updated_at \
         FROM quarterly_checkins WHERE phase_id = ? ORDER BY year, quarter",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(id, quarter, year, a, b, c, d, e, updated_at)| {
            Ok(QuarterlyCheckin {
                id,
                quarter,
                year,
                highlights_impact: crypto.decrypt_opt(&a)?,
                learnings_adjustments: crypto.decrypt_opt(&b)?,
                growth_development: crypto.decrypt_opt(&c)?,
                support_feedback: crypto.decrypt_opt(&d)?,
                looking_ahead: crypto.decrypt_opt(&e)?,
                updated_at,
            })
        })
        .collect()
}

pub async fn contribution_examples(
    pool: &SqlitePool,
    phase_id: i64,
    crypto: &UserCrypto,
) -> Result<Vec<ContributionExample>, AppError> {
    type Row = (i64, i64, Vec<u8>, Blob, Blob, Option<String>, Blob, Option<String>, String);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, phase_id, title, outcome, behaviors, impact_level, learnings, assessment_type, status \
         FROM contribution_examples WHERE phase_id = ? ORDER BY sort_order, id",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(
            |(id, phase_id, title, outcome, behaviors, impact_level, learnings, assessment_type, status)| {
                Ok(ContributionExample {
                    id,
                    phase_id,
                    title: crypto.decrypt(&title)?,
                    outcome: crypto.decrypt_opt(&outcome)?,
                    behaviors: crypto.decrypt_opt(&behaviors)?,
                    impact_level,
                    learnings: crypto.decrypt_opt(&learnings)?,
                    assessment_type,
                    status,
                })
            },
        )
        .collect()
}

/// `(example_id, entry_id, entry_title)` for every entry linked to an example in the phase.
pub async fn example_entries(
    pool: &SqlitePool,
    phase_id: i64,
    crypto: &UserCrypto,
) -> Result<Vec<(i64, i64, String)>, AppError> {
    let rows: Vec<(i64, i64, Vec<u8>)> = sqlx::query_as(
        r#"
        SELECT ce.example_id, ce.entry_id, e.title
        FROM contribution_example_entries ce
        JOIN contribution_examples c ON c.id = ce.example_id
        JOIN brag_entries e ON e.id = ce.entry_id
        WHERE c.phase_id = ? AND e.deleted_at IS NULL
        ORDER BY ce.example_id, ce.entry_id
        "#,
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(ex, entry, title)| Ok((ex, entry, crypto.decrypt(&title)?)))
        .collect()
}

pub async fn status_updates(
    pool: &SqlitePool,
    phase_id: i64,
    crypto: &UserCrypto,
) -> Result<Vec<StatusUpdate>, AppError> {
    let rows: Vec<(i64, i64, Blob, String)> = sqlx::query_as(
        "SELECT id, week_id, content, updated_at FROM status_updates WHERE phase_id = ? ORDER BY week_id",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(id, week_id, content, updated_at)| {
            Ok(StatusUpdate {
                id,
                week_id,
                content: crypto.decrypt_opt(&content)?,
                updated_at,
            })
        })
        .collect()
}

pub async fn last_week_reports(
    pool: &SqlitePool,
    phase_id: i64,
    crypto: &UserCrypto,
) -> Result<Vec<LastWeekReport>, AppError> {
    let rows: Vec<(i64, String, String, Blob, String)> = sqlx::query_as(
        "SELECT id, window_start, window_end, content, updated_at FROM last_week_reports \
         WHERE phase_id = ? ORDER BY week_id",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(id, window_start, window_end, content, updated_at)| {
            Ok(LastWeekReport {
                id,
                window_start,
                window_end,
                content: crypto.decrypt_opt(&content)?,
                updated_at,
            })
        })
        .collect()
}
