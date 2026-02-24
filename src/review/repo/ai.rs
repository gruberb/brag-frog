use sqlx::SqlitePool;

use crate::review::model::{AiDocument, AiDocumentRow};
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

impl AiDocument {
    /// Recent documents of a given type for a user.
    pub async fn list_for_user(
        pool: &SqlitePool,
        user_id: i64,
        doc_type: &str,
        limit: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, AiDocumentRow>(
            "SELECT * FROM ai_documents WHERE user_id = ? AND doc_type = ? ORDER BY generated_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(doc_type)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Find previous preps for the same recurring meeting series.
    pub async fn list_for_recurring_group(
        pool: &SqlitePool,
        user_id: i64,
        recurring_group: &str,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, AiDocumentRow>(
            "SELECT * FROM ai_documents WHERE user_id = ? AND doc_type = 'meeting_prep' AND recurring_group = ? ORDER BY generated_at DESC",
        )
        .bind(user_id)
        .bind(recurring_group)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Create a new AI document.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        phase_id: i64,
        doc_type: &str,
        title: &str,
        content: &str,
        prompt_used: Option<&str>,
        model_used: Option<&str>,
        context_week_id: Option<i64>,
        meeting_entry_id: Option<i64>,
        meeting_role: Option<&str>,
        recurring_group: Option<&str>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_content = crypto.encrypt(content)?;
        let enc_prompt = crypto.encrypt_opt(&prompt_used.map(String::from))?;

        let row = sqlx::query_as::<_, AiDocumentRow>(
            r#"
            INSERT INTO ai_documents (user_id, phase_id, doc_type, title, content, prompt_used, model_used,
                context_week_id, meeting_entry_id, meeting_role, recurring_group)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(phase_id)
        .bind(doc_type)
        .bind(title)
        .bind(&enc_content)
        .bind(&enc_prompt)
        .bind(model_used)
        .bind(context_week_id)
        .bind(meeting_entry_id)
        .bind(meeting_role)
        .bind(recurring_group)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, AiDocumentRow>(
            "SELECT * FROM ai_documents WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM ai_documents WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
