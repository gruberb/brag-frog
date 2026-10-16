use sqlx::SqlitePool;

use crate::review::model::{ContributionExample, ContributionExampleRow};
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

    /// `(example_id, entry_id, entry_title)` for every entry linked to an example in the
    /// phase, in one query. Replaces calling `linked_entry_ids` once per example.
    pub async fn linked_entries_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<(i64, i64, String)>, AppError> {
        let rows: Vec<(i64, i64, Vec<u8>)> = sqlx::query_as(
            r#"
            SELECT ce.example_id, ce.entry_id, e.title
            FROM contribution_example_entries ce
            JOIN contribution_examples c ON c.id = ce.example_id
            JOIN brag_entries e ON e.id = ce.entry_id
            WHERE c.phase_id = ? AND e.deleted_at IS NULL
            ORDER BY ce.example_id, ce.entry_id
            "#,
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|(ex, entry, title)| Ok((ex, entry, crypto.decrypt(&title)?)))
            .collect()
    }
}
