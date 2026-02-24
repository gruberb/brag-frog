use sqlx::SqlitePool;

use crate::review::model::{
    AnnualAlignment, AnnualAlignmentRow, CheckinWithWeek, CheckinWithWeekRow,
    QuarterlyCheckin, QuarterlyCheckinRow, SaveAnnualAlignment,
    SaveCheckin, SaveQuarterlyCheckin, WeeklyCheckin, WeeklyCheckinRow,
};
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

// ---------------------------------------------------------------------------
// WeeklyCheckin
// ---------------------------------------------------------------------------

impl WeeklyCheckin {
    /// Find checkin for a specific week and user.
    pub async fn find_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, WeeklyCheckinRow>(
            "SELECT * FROM weekly_checkins WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Upsert a weekly check-in (insert or update on conflict).
    pub async fn upsert(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        input: &SaveCheckin,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_highlights = crypto.encrypt_opt(&input.highlights_impact)?;
        let enc_learnings = crypto.encrypt_opt(&input.learnings_adjustments)?;
        let enc_growth = crypto.encrypt_opt(&input.growth_development)?;
        let enc_support = crypto.encrypt_opt(&input.support_feedback)?;
        let enc_ahead = crypto.encrypt_opt(&input.looking_ahead)?;

        let row = sqlx::query_as::<_, WeeklyCheckinRow>(
            r#"
            INSERT INTO weekly_checkins (week_id, user_id, highlights_impact, learnings_adjustments, growth_development, support_feedback, looking_ahead, energy_level, productivity_rating)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(week_id, user_id) DO UPDATE SET
                highlights_impact = excluded.highlights_impact,
                learnings_adjustments = excluded.learnings_adjustments,
                growth_development = excluded.growth_development,
                support_feedback = excluded.support_feedback,
                looking_ahead = excluded.looking_ahead,
                energy_level = excluded.energy_level,
                productivity_rating = excluded.productivity_rating,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(week_id)
        .bind(user_id)
        .bind(&enc_highlights)
        .bind(&enc_learnings)
        .bind(&enc_growth)
        .bind(&enc_support)
        .bind(&enc_ahead)
        .bind(input.energy_level)
        .bind(input.productivity_rating)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// List all check-ins for a user with week info, ordered newest first.
    pub async fn list_with_weeks(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<CheckinWithWeek>, AppError> {
        let rows = sqlx::query_as::<_, CheckinWithWeekRow>(
            r#"
            SELECT wc.*, w.iso_week, w.year, w.start_date AS week_start, w.end_date AS week_end,
                   bp.name AS phase_name
            FROM weekly_checkins wc
            JOIN weeks w ON w.id = wc.week_id
            JOIN brag_phases bp ON bp.id = w.phase_id
            WHERE wc.user_id = ?
            ORDER BY w.year DESC, w.iso_week DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Delete a check-in and its KR snapshots (in a transaction).
    pub async fn delete(pool: &SqlitePool, week_id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        // Delete KR snapshots first
        sqlx::query(
            "DELETE FROM kr_checkin_snapshots WHERE checkin_id IN (SELECT id FROM weekly_checkins WHERE week_id = ? AND user_id = ?)",
        )
        .bind(week_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Delete the checkin itself
        sqlx::query("DELETE FROM weekly_checkins WHERE week_id = ? AND user_id = ?")
            .bind(week_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Count completed check-ins for a user (for streak calculation).
    pub async fn count_for_user(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM weekly_checkins WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }
}

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

    /// Load all weekly check-ins for a given quarter with week metadata.
    pub async fn weekly_reflections_for_quarter(
        pool: &SqlitePool,
        user_id: i64,
        quarter: &str,
        year: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<CheckinWithWeek>, AppError> {
        let (start_week, end_week) = match quarter {
            "Q1" => (1, 13),
            "Q2" => (14, 26),
            "Q3" => (27, 39),
            "Q4" => (40, 53),
            _ => (1, 53),
        };

        let rows = sqlx::query_as::<_, CheckinWithWeekRow>(
            r#"
            SELECT wc.*, w.iso_week, w.year, w.start_date AS week_start, w.end_date AS week_end,
                   bp.name AS phase_name
            FROM weekly_checkins wc
            JOIN weeks w ON w.id = wc.week_id
            JOIN brag_phases bp ON bp.id = w.phase_id
            WHERE wc.user_id = ? AND w.year = ? AND w.iso_week >= ? AND w.iso_week <= ?
            ORDER BY w.iso_week
            "#,
        )
        .bind(user_id)
        .bind(year)
        .bind(start_week)
        .bind(end_week)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }
}

// ---------------------------------------------------------------------------
// AnnualAlignment
// ---------------------------------------------------------------------------

impl AnnualAlignment {
    /// Upsert an annual alignment (insert or update on conflict).
    pub async fn upsert(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &SaveAnnualAlignment,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_outcomes = crypto.encrypt_opt(&input.top_outcomes)?;
        let enc_why = crypto.encrypt_opt(&input.why_it_matters)?;
        let enc_criteria = crypto.encrypt_opt(&input.success_criteria)?;
        let enc_goals = crypto.encrypt_opt(&input.learning_goals)?;
        let enc_support = crypto.encrypt_opt(&input.support_needed)?;

        let row = sqlx::query_as::<_, AnnualAlignmentRow>(
            r#"
            INSERT INTO annual_alignment (phase_id, user_id, year, top_outcomes, why_it_matters, success_criteria, learning_goals, support_needed)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(phase_id, user_id, year) DO UPDATE SET
                top_outcomes = excluded.top_outcomes,
                why_it_matters = excluded.why_it_matters,
                success_criteria = excluded.success_criteria,
                learning_goals = excluded.learning_goals,
                support_needed = excluded.support_needed,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(input.year)
        .bind(&enc_outcomes)
        .bind(&enc_why)
        .bind(&enc_criteria)
        .bind(&enc_goals)
        .bind(&enc_support)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Find an annual alignment for a specific year.
    pub async fn find(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        year: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, AnnualAlignmentRow>(
            "SELECT * FROM annual_alignment WHERE phase_id = ? AND user_id = ? AND year = ?",
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(year)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }
}
