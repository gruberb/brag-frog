use sqlx::SqlitePool;

use crate::review::model::{
    CreateMeetingRule, MeetingPrepNote, MeetingPrepNoteRow, MeetingRule,
};
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

// ---------------------------------------------------------------------------
// MeetingPrepNote
// ---------------------------------------------------------------------------

impl MeetingPrepNote {
    /// List all prep notes for a given week.
    pub async fn list_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, MeetingPrepNoteRow>(
            "SELECT * FROM meeting_prep_notes WHERE week_id = ? AND user_id = ? ORDER BY created_at",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Upsert a meeting prep note for a specific entry.
    pub async fn upsert(
        pool: &SqlitePool,
        user_id: i64,
        week_id: i64,
        entry_id: i64,
        notes: Option<&str>,
        doc_urls: Option<&str>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_notes = crypto.encrypt_opt(&notes.map(String::from))?;

        let row = sqlx::query_as::<_, MeetingPrepNoteRow>(
            r#"
            INSERT INTO meeting_prep_notes (user_id, week_id, entry_id, notes, doc_urls)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(user_id, week_id, entry_id) DO UPDATE SET
                notes = excluded.notes,
                doc_urls = excluded.doc_urls,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(week_id)
        .bind(entry_id)
        .bind(&enc_notes)
        .bind(doc_urls)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Delete a meeting prep note.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM meeting_prep_notes WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MeetingRule
// ---------------------------------------------------------------------------

impl MeetingRule {
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let rules = sqlx::query_as::<_, MeetingRule>(
            "SELECT * FROM meeting_rules WHERE user_id = ? ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(rules)
    }

    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        input: &CreateMeetingRule,
    ) -> Result<Self, AppError> {
        let rule = sqlx::query_as::<_, MeetingRule>(
            r#"
            INSERT INTO meeting_rules (user_id, match_type, match_value, meeting_role, person_name)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(user_id, match_type, match_value) DO UPDATE SET
                meeting_role = excluded.meeting_role,
                person_name = excluded.person_name
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&input.match_type)
        .bind(&input.match_value)
        .bind(&input.meeting_role)
        .bind(&input.person_name)
        .fetch_one(pool)
        .await?;
        Ok(rule)
    }

    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM meeting_rules WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Apply all meeting rules to unclassified meeting entries for a user.
    /// Sets `meeting_role` on entries that match a rule.
    pub async fn apply_meeting_rules(pool: &SqlitePool, user_id: i64) -> Result<u64, AppError> {
        let rules = Self::list_for_user(pool, user_id).await?;
        let mut total_updated = 0u64;

        for rule in &rules {
            let result = match rule.match_type.as_str() {
                "recurring_group" => {
                    sqlx::query(
                        r#"
                        UPDATE brag_entries SET meeting_role = ?
                        WHERE meeting_role IS NULL AND recurring_group = ?
                        AND entry_type = 'meeting' AND deleted_at IS NULL
                        AND week_id IN (
                            SELECT w.id FROM weeks w
                            JOIN brag_phases p ON w.phase_id = p.id
                            WHERE p.user_id = ?
                        )
                        "#,
                    )
                    .bind(&rule.meeting_role)
                    .bind(&rule.match_value)
                    .bind(user_id)
                    .execute(pool)
                    .await?
                }
                "title_contains" => {
                    let pattern = format!("%{}%", rule.match_value);
                    // Note: title is encrypted so we can't do LIKE on it directly.
                    // This rule type must be applied post-decryption in application code.
                    // For now, skip it — the classify endpoint handles this case.
                    let _ = pattern;
                    continue;
                }
                _ => continue,
            };
            total_updated += result.rows_affected();
        }

        Ok(total_updated)
    }
}
