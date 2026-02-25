use sqlx::SqlitePool;

use crate::cycle::model::{
    ContributionExample, ContributionExampleRow, CreateContributionExample,
    UpdateContributionExample,
};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

// ---------------------------------------------------------------------------
// ContributionExample
// ---------------------------------------------------------------------------

impl ContributionExample {
    /// All examples for a phase, ordered by sort_order.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, ContributionExampleRow>(
            "SELECT * FROM contribution_examples WHERE phase_id = ? ORDER BY sort_order, id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Find by ID, scoped to user.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, ContributionExampleRow>(
            r#"
            SELECT ce.* FROM contribution_examples ce
            JOIN brag_phases p ON ce.phase_id = p.id
            WHERE ce.id = ? AND p.user_id = ?
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Create a new contribution example.
    pub async fn create(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &CreateContributionExample,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let owns: Option<i64> =
            sqlx::query_scalar("SELECT id FROM brag_phases WHERE id = ? AND user_id = ?")
                .bind(phase_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        if owns.is_none() {
            return Err(AppError::NotFound("Phase not found".to_string()));
        }

        let max_order: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(sort_order) FROM contribution_examples WHERE phase_id = ?",
        )
        .bind(phase_id)
        .fetch_one(pool)
        .await?;

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_outcome = crypto.encrypt_opt(&input.outcome)?;
        let enc_behaviors = crypto.encrypt_opt(&input.behaviors)?;
        let enc_learnings = crypto.encrypt_opt(&input.learnings)?;

        let row = sqlx::query_as::<_, ContributionExampleRow>(
            r#"
            INSERT INTO contribution_examples (phase_id, title, outcome, behaviors, impact_level, learnings, assessment_type, sort_order)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(&enc_title)
        .bind(&enc_outcome)
        .bind(&enc_behaviors)
        .bind(&input.impact_level)
        .bind(&enc_learnings)
        .bind(&input.assessment_type)
        .bind(max_order.unwrap_or(0) + 1)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Update a contribution example.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateContributionExample,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_outcome = crypto.encrypt_opt(&input.outcome)?;
        let enc_behaviors = crypto.encrypt_opt(&input.behaviors)?;
        let enc_learnings = crypto.encrypt_opt(&input.learnings)?;

        let row = sqlx::query_as::<_, ContributionExampleRow>(
            r#"
            UPDATE contribution_examples SET title = ?, outcome = ?, behaviors = ?, impact_level = ?,
                learnings = ?, assessment_type = ?, status = ?, updated_at = datetime('now')
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING *
            "#,
        )
        .bind(&enc_title)
        .bind(&enc_outcome)
        .bind(&enc_behaviors)
        .bind(&input.impact_level)
        .bind(&enc_learnings)
        .bind(&input.assessment_type)
        .bind(input.status.as_deref().unwrap_or("draft"))
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Delete a contribution example and its entry links.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM contribution_example_entries WHERE example_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM contribution_examples WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Link an entry to this contribution example.
    pub async fn link_entry(
        pool: &SqlitePool,
        example_id: i64,
        entry_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT OR IGNORE INTO contribution_example_entries (example_id, entry_id) VALUES (?, ?)",
        )
        .bind(example_id)
        .bind(entry_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Unlink an entry from this contribution example.
    pub async fn unlink_entry(
        pool: &SqlitePool,
        example_id: i64,
        entry_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM contribution_example_entries WHERE example_id = ? AND entry_id = ?",
        )
        .bind(example_id)
        .bind(entry_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get entry IDs linked to a contribution example.
    pub async fn linked_entry_ids(
        pool: &SqlitePool,
        example_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT entry_id FROM contribution_example_entries WHERE example_id = ?",
        )
        .bind(example_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }
}
