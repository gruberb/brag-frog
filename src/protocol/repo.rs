use sqlx::SqlitePool;

use crate::kernel::error::AppError;
use crate::protocol::model::ProtocolCheck;

impl ProtocolCheck {
    /// List all checks for a week.
    pub async fn list_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, ProtocolCheck>(
            "SELECT * FROM protocol_checks WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Toggle a checklist item. Inserts if not exists, flips if exists.
    pub async fn toggle(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        slug: &str,
    ) -> Result<(), AppError> {
        // Try to find existing
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT checked FROM protocol_checks WHERE week_id = ? AND user_id = ? AND slug = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .bind(slug)
        .fetch_optional(pool)
        .await?;

        match existing {
            Some(val) => {
                let new_val = if val == 0 { 1 } else { 0 };
                sqlx::query(
                    "UPDATE protocol_checks SET checked = ? WHERE week_id = ? AND user_id = ? AND slug = ?",
                )
                .bind(new_val)
                .bind(week_id)
                .bind(user_id)
                .bind(slug)
                .execute(pool)
                .await?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO protocol_checks (week_id, user_id, slug, checked) VALUES (?, ?, ?, 1)",
                )
                .bind(week_id)
                .bind(user_id)
                .bind(slug)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Clear all checks for a week (reset checklist).
    pub async fn clear_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM protocol_checks WHERE week_id = ? AND user_id = ?")
            .bind(week_id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
