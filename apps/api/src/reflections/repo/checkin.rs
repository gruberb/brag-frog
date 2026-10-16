use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::reflections::model::{QuarterlyCheckin, QuarterlyCheckinRow, SaveQuarterlyCheckin};

// ---------------------------------------------------------------------------
// QuarterlyCheckin
// ---------------------------------------------------------------------------

impl QuarterlyCheckin {
    /// Upsert a quarterly check-in (insert or update on conflict).
    pub async fn upsert(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &SaveQuarterlyCheckin,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_highlights = crypto.encrypt_opt(&input.highlights_impact)?;
        let enc_learnings = crypto.encrypt_opt(&input.learnings_adjustments)?;
        let enc_growth = crypto.encrypt_opt(&input.growth_development)?;
        let enc_support = crypto.encrypt_opt(&input.support_feedback)?;
        let enc_ahead = crypto.encrypt_opt(&input.looking_ahead)?;

        let row = sqlx::query_as::<_, QuarterlyCheckinRow>(
            r#"
            INSERT INTO quarterly_checkins (phase_id, user_id, quarter, year, highlights_impact, learnings_adjustments, growth_development, support_feedback, looking_ahead)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(phase_id, user_id, quarter, year) DO UPDATE SET
                highlights_impact = excluded.highlights_impact,
                learnings_adjustments = excluded.learnings_adjustments,
                growth_development = excluded.growth_development,
                support_feedback = excluded.support_feedback,
                looking_ahead = excluded.looking_ahead,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(&input.quarter)
        .bind(input.year)
        .bind(&enc_highlights)
        .bind(&enc_learnings)
        .bind(&enc_growth)
        .bind(&enc_support)
        .bind(&enc_ahead)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Find a quarterly check-in for a specific quarter.
    pub async fn find(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        quarter: &str,
        year: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, QuarterlyCheckinRow>(
            "SELECT * FROM quarterly_checkins WHERE phase_id = ? AND user_id = ? AND quarter = ? AND year = ?",
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(quarter)
        .bind(year)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// List all quarterly check-ins for a phase.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, QuarterlyCheckinRow>(
            "SELECT * FROM quarterly_checkins WHERE phase_id = ? ORDER BY year, quarter",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }
}
