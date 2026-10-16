use sqlx::SqlitePool;

use crate::review::model::{AiDocument, AiDocumentRow};
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

impl AiDocument {
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

}
