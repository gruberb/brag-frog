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

    /// Replaces the saved text of a section and records how `content` is encoded
    /// (`markdown` or `blocknote`).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_with_format(
        pool: &SqlitePool,
        phase_id: i64,
        section: &str,
        content: &str,
        content_format: &str,
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
            INSERT INTO summaries (phase_id, section, content, prompt_used, model_used, content_format)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(section)
        .bind(&enc_content)
        .bind(&enc_prompt)
        .bind(model_used)
        .bind(content_format)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Number of live entries in a phase, for the review header. Counts in SQL so no entry
    /// is decrypted.
    pub async fn phase_entry_count(pool: &SqlitePool, phase_id: i64) -> Result<i64, AppError> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ? AND e.deleted_at IS NULL",
        )
        .bind(phase_id)
        .fetch_one(pool)
        .await?;
        Ok(n)
    }
}
