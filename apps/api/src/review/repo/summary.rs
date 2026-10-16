use sqlx::SqlitePool;

use crate::review::model::{Summary, SummaryRow};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl Summary {
    /// All summaries for a phase, ordered by ID.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            "SELECT * FROM summaries WHERE phase_id = ? ORDER BY id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Replaces the summary for a `(phase_id, section)` pair (delete + insert).
    pub async fn upsert(
        pool: &SqlitePool,
        phase_id: i64,
        section: &str,
        content: &str,
        prompt_used: Option<&str>,
        model_used: Option<&str>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        // Delete old version if exists
        sqlx::query("DELETE FROM summaries WHERE phase_id = ? AND section = ?")
            .bind(phase_id)
            .bind(section)
            .execute(pool)
            .await?;

        let enc_content = crypto.encrypt(content)?;
        let enc_prompt = crypto.encrypt_opt(&prompt_used.map(String::from))?;

        let row = sqlx::query_as::<_, SummaryRow>(
            r#"
            INSERT INTO summaries (phase_id, section, content, prompt_used, model_used)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(section)
        .bind(&enc_content)
        .bind(&enc_prompt)
        .bind(model_used)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }
}
