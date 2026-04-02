//! Persistence layer for the cycle bounded context.
//! All SQL queries live here — `model.rs` contains no SQL.

mod focus;
mod meeting;
pub mod status_update;

use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;

use crate::kernel::error::AppError;

use super::model::*;

// ---------------------------------------------------------------------------
// BragPhase
// ---------------------------------------------------------------------------

impl BragPhase {
    /// All phases for a user, most recent first.
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let phases = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE user_id = ? ORDER BY start_date DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(phases)
    }

    /// Fetches a phase by ID, scoped to the given user.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
    ) -> Result<Option<Self>, AppError> {
        let phase = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(phase)
    }

    /// Returns the single active phase for a user, if any.
    pub async fn get_active(pool: &SqlitePool, user_id: i64) -> Result<Option<Self>, AppError> {
        let phase = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE user_id = ? AND is_active = 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(phase)
    }

    /// Creates a new phase and makes it active, deactivating any current active phase.
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        input: &CreatePhase,
    ) -> Result<Self, AppError> {
        // Deactivate any currently active phase
        sqlx::query("UPDATE brag_phases SET is_active = 0 WHERE user_id = ? AND is_active = 1")
            .bind(user_id)
            .execute(pool)
            .await?;

        let phase = sqlx::query_as::<_, BragPhase>(
            r#"
            INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active)
            VALUES (?, ?, ?, ?, 1)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&input.name)
        .bind(&input.start_date)
        .bind(&input.end_date)
        .fetch_one(pool)
        .await?;

        Ok(phase)
    }

    /// Deletes a phase and all owned data in a transaction.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        // Delete contribution_example_entries for entries in this phase
        sqlx::query(
            "DELETE FROM contribution_example_entries WHERE entry_id IN (SELECT e.id FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete meeting prep notes for weeks in this phase
        sqlx::query(
            "DELETE FROM meeting_prep_notes WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete entries in weeks belonging to this phase
        sqlx::query(
            "DELETE FROM brag_entries WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete weekly_checkins for weeks in this phase
        sqlx::query(
            "DELETE FROM weekly_checkins WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete weekly focus entries and focus items
        sqlx::query(
            "DELETE FROM weekly_focus_entries WHERE focus_id IN (SELECT f.id FROM weekly_focus f JOIN weeks w ON f.week_id = w.id WHERE w.phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "DELETE FROM weekly_focus WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete status updates for weeks in this phase
        sqlx::query(
            "DELETE FROM status_updates WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete weeks
        sqlx::query("DELETE FROM weeks WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete priorities
        sqlx::query("DELETE FROM priorities WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete contribution examples
        sqlx::query(
            "DELETE FROM contribution_example_entries WHERE example_id IN (SELECT id FROM contribution_examples WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM contribution_examples WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete quarterly checkins and annual alignment
        sqlx::query("DELETE FROM quarterly_checkins WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM annual_alignment WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete AI documents
        sqlx::query("DELETE FROM ai_documents WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete department_goals
        sqlx::query("DELETE FROM department_goals WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete summaries
        sqlx::query("DELETE FROM summaries WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete the phase itself (scoped to user)
        sqlx::query("DELETE FROM brag_phases WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Activates this phase and deactivates all others for the user.
    pub async fn set_active(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE brag_phases SET is_active = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await?;

        sqlx::query("UPDATE brag_phases SET is_active = 1 WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Week
// ---------------------------------------------------------------------------

impl Week {
    /// Fetches a single week by ID.
    pub async fn find_by_id(pool: &SqlitePool, week_id: i64) -> Result<Option<Self>, AppError> {
        let week =
            sqlx::query_as::<_, Week>("SELECT * FROM weeks WHERE id = ?")
                .bind(week_id)
                .fetch_optional(pool)
                .await?;
        Ok(week)
    }

    /// All weeks in a phase, ordered chronologically.
    pub async fn list_for_phase(pool: &SqlitePool, phase_id: i64) -> Result<Vec<Self>, AppError> {
        let weeks = sqlx::query_as::<_, Week>(
            "SELECT * FROM weeks WHERE phase_id = ? ORDER BY year, iso_week",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        Ok(weeks)
    }

    /// Returns the `phase_id` for a given week.
    pub async fn phase_id(pool: &SqlitePool, week_id: i64) -> Result<i64, AppError> {
        let phase_id: i64 = sqlx::query_scalar("SELECT phase_id FROM weeks WHERE id = ?")
            .bind(week_id)
            .fetch_one(pool)
            .await?;
        Ok(phase_id)
    }

    /// Returns the existing week for `(phase_id, iso_week, year)` or creates one.
    /// New weeks get a sequential `week_number` within the phase.
    pub async fn find_or_create(
        pool: &SqlitePool,
        phase_id: i64,
        iso_week: i64,
        year: i64,
        start_date: &str,
        end_date: &str,
    ) -> Result<Self, AppError> {
        // Try to find existing
        if let Some(week) = sqlx::query_as::<_, Week>(
            "SELECT * FROM weeks WHERE phase_id = ? AND iso_week = ? AND year = ?",
        )
        .bind(phase_id)
        .bind(iso_week)
        .bind(year)
        .fetch_optional(pool)
        .await?
        {
            return Ok(week);
        }

        // Calculate week number within phase
        let week_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks WHERE phase_id = ?")
            .bind(phase_id)
            .fetch_one(pool)
            .await?;

        let week = sqlx::query_as::<_, Week>(
            r#"
            INSERT INTO weeks (phase_id, week_number, iso_week, year, start_date, end_date)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(week_count + 1)
        .bind(iso_week)
        .bind(year)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        Ok(week)
    }

    /// Find or create the week that contains the given date.
    pub async fn find_or_create_for_date(
        pool: &SqlitePool,
        phase_id: i64,
        date: NaiveDate,
    ) -> Result<Self, AppError> {
        let iso_week = date.iso_week().week();
        let year = date.iso_week().year();
        let week_start = iso_week_to_date(year, iso_week);
        let week_end = week_start + chrono::Duration::days(6);

        Self::find_or_create(
            pool,
            phase_id,
            iso_week as i64,
            year as i64,
            &week_start.format("%Y-%m-%d").to_string(),
            &week_end.format("%Y-%m-%d").to_string(),
        )
        .await
    }
}
