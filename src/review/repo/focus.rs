use sqlx::SqlitePool;

use crate::review::model::{
    CreateFocusParams, UpdateFocusParams, WeeklyFocus, WeeklyFocusEntry, WeeklyFocusRow,
};
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

// ---------------------------------------------------------------------------
// WeeklyFocus
// ---------------------------------------------------------------------------

impl WeeklyFocus {
    /// List all focus items for a week.
    pub async fn list_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, WeeklyFocusRow>(
            "SELECT * FROM weekly_focus WHERE week_id = ? AND user_id = ? ORDER BY sort_order",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Create a new focus item.
    pub async fn create(
        pool: &SqlitePool,
        p: &CreateFocusParams<'_>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(p.title)?;

        let row = sqlx::query_as::<_, WeeklyFocusRow>(
            r#"
            INSERT INTO weekly_focus (week_id, user_id, sort_order, title, linked_type, linked_id, link_1, link_2, link_3)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(p.week_id)
        .bind(p.user_id)
        .bind(p.sort_order)
        .bind(&enc_title)
        .bind(p.linked_type)
        .bind(p.linked_id)
        .bind(p.link_1)
        .bind(p.link_2)
        .bind(p.link_3)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Update a focus item.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        p: &UpdateFocusParams<'_>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(p.title)?;

        let row = sqlx::query_as::<_, WeeklyFocusRow>(
            r#"
            UPDATE weekly_focus SET
                title = ?, linked_type = ?, linked_id = ?,
                link_1 = ?, link_2 = ?, link_3 = ?,
                updated_at = datetime('now')
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&enc_title)
        .bind(p.linked_type)
        .bind(p.linked_id)
        .bind(p.link_1)
        .bind(p.link_2)
        .bind(p.link_3)
        .bind(id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Delete a focus item.
    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM weekly_focus WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Count focus items for a week.
    pub async fn count_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM weekly_focus WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// WeeklyFocusEntry
// ---------------------------------------------------------------------------

impl WeeklyFocusEntry {
    /// List entry IDs for a focus item.
    pub async fn list_for_focus(pool: &SqlitePool, focus_id: i64) -> Result<Vec<Self>, AppError> {
        let entries = sqlx::query_as::<_, WeeklyFocusEntry>(
            "SELECT * FROM weekly_focus_entries WHERE focus_id = ?",
        )
        .bind(focus_id)
        .fetch_all(pool)
        .await?;
        Ok(entries)
    }

    /// Replace all entries for a focus item.
    pub async fn set_entries(
        pool: &SqlitePool,
        focus_id: i64,
        entry_ids: &[i64],
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM weekly_focus_entries WHERE focus_id = ?")
            .bind(focus_id)
            .execute(pool)
            .await?;

        for &entry_id in entry_ids {
            sqlx::query("INSERT INTO weekly_focus_entries (focus_id, entry_id) VALUES (?, ?)")
                .bind(focus_id)
                .bind(entry_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }
}
