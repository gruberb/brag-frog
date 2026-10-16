use sqlx::SqlitePool;

use crate::cycle::model::{SaveStatusUpdate, StatusUpdate, StatusUpdateRow};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl StatusUpdate {
    pub async fn find_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, StatusUpdateRow>(
            "SELECT * FROM status_updates WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    pub async fn upsert(
        pool: &SqlitePool,
        week_id: i64,
        phase_id: i64,
        user_id: i64,
        input: &SaveStatusUpdate,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_content = crypto.encrypt_opt(&input.content)?;

        let row = sqlx::query_as::<_, StatusUpdateRow>(
            r#"
            INSERT INTO status_updates (week_id, phase_id, user_id, content)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(week_id, user_id) DO UPDATE SET
                content = excluded.content,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(week_id)
        .bind(phase_id)
        .bind(user_id)
        .bind(&enc_content)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }
}
