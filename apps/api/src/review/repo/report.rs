use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::review::model::{NewReport, Report, ReportRow};

impl Report {
    /// Inserts a report. The caller has already checked that the user owns `phase_id`.
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        phase_id: i64,
        input: &NewReport<'_>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            INSERT INTO reports
                (user_id, phase_id, title, filters_json, content, content_format, stats_json, model_used)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(phase_id)
        .bind(crypto.encrypt(input.title)?)
        .bind(input.filters_json)
        .bind(crypto.encrypt(input.content)?)
        .bind(input.content_format)
        .bind(crypto.encrypt(input.stats_json)?)
        .bind(input.model_used)
        .fetch_one(pool)
        .await?;
        row.decrypt(crypto)
    }

    /// A user's reports for a phase, newest first.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        user_id: i64,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, ReportRow>(
            "SELECT * FROM reports WHERE user_id = ? AND phase_id = ? ORDER BY generated_at DESC, id DESC",
        )
        .bind(user_id)
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Fetches a report scoped to its owner.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row =
            sqlx::query_as::<_, ReportRow>("SELECT * FROM reports WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Deletes a report scoped to its owner. Returns whether a row was removed.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM reports WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
