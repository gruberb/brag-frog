use sqlx::SqlitePool;

use crate::cycle::model::{LastWeekReport, LastWeekReportRow, SaveLastWeekReport};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl LastWeekReport {
    /// Loads the saved Last Week report anchored to `week_id` for a user, if any.
    pub async fn find_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, LastWeekReportRow>(
            "SELECT * FROM last_week_reports WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Creates or overwrites the Last Week report for `(week_id, user_id)`.
    /// Content and window are refreshed together on regenerate.
    pub async fn upsert(
        pool: &SqlitePool,
        week_id: i64,
        phase_id: i64,
        user_id: i64,
        input: SaveLastWeekReport<'_>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_content = crypto.encrypt_opt(&input.content.map(str::to_string))?;

        let row = sqlx::query_as::<_, LastWeekReportRow>(
            r#"
            INSERT INTO last_week_reports
                (week_id, phase_id, user_id, content, window_start, window_end)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(week_id, user_id) DO UPDATE SET
                content = excluded.content,
                window_start = excluded.window_start,
                window_end = excluded.window_end,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(week_id)
        .bind(phase_id)
        .bind(user_id)
        .bind(&enc_content)
        .bind(input.window_start)
        .bind(input.window_end)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }
}
