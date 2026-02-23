//! Persistence layer for the review bounded context.
//! All SQL queries live here — `model.rs` contains no SQL.

use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

use super::model::*;

// ---------------------------------------------------------------------------
// BragPhase
// ---------------------------------------------------------------------------

impl BragPhase {
    /// All phases for a user, most recent first.
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let phases = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE user_id = ? ORDER BY start_date DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(phases)
    }

    /// Fetches a phase by ID, scoped to the given user.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
    ) -> Result<Option<Self>, AppError> {
        let phase = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(phase)
    }

    /// Returns the single active phase for a user, if any.
    pub async fn get_active(pool: &SqlitePool, user_id: i64) -> Result<Option<Self>, AppError> {
        let phase = sqlx::query_as::<_, BragPhase>(
            "SELECT * FROM brag_phases WHERE user_id = ? AND is_active = 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(phase)
    }

    /// Creates a new phase and makes it active, deactivating any current active phase.
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        input: &CreatePhase,
    ) -> Result<Self, AppError> {
        // Deactivate any currently active phase
        sqlx::query("UPDATE brag_phases SET is_active = 0 WHERE user_id = ? AND is_active = 1")
            .bind(user_id)
            .execute(pool)
            .await?;

        let phase = sqlx::query_as::<_, BragPhase>(
            r#"
            INSERT INTO brag_phases (user_id, name, start_date, end_date, is_active)
            VALUES (?, ?, ?, ?, 1)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&input.name)
        .bind(&input.start_date)
        .bind(&input.end_date)
        .fetch_one(pool)
        .await?;

        Ok(phase)
    }

    /// Deletes a phase and all owned data in a transaction.
    /// Cascades to: entries, weeks, goals, key_results, summaries, initiatives, checkins, impact_stories, ai_documents.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        // Delete entry_competencies and story_entries for entries in this phase
        sqlx::query(
            "DELETE FROM entry_competencies WHERE entry_id IN (SELECT e.id FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "DELETE FROM story_entries WHERE entry_id IN (SELECT e.id FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete entries in weeks belonging to this phase
        sqlx::query(
            "DELETE FROM brag_entries WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete kr_checkin_snapshots and weekly_checkins for weeks in this phase
        sqlx::query(
            "DELETE FROM kr_checkin_snapshots WHERE checkin_id IN (SELECT c.id FROM weekly_checkins c JOIN weeks w ON c.week_id = w.id WHERE w.phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "DELETE FROM weekly_checkins WHERE week_id IN (SELECT id FROM weeks WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete weeks
        sqlx::query("DELETE FROM weeks WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete initiative_key_results and initiatives
        sqlx::query(
            "DELETE FROM initiative_key_results WHERE initiative_id IN (SELECT id FROM initiatives WHERE phase_id = ?)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM initiatives WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete impact stories (story_entries already deleted above)
        sqlx::query("DELETE FROM impact_stories WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete AI documents
        sqlx::query("DELETE FROM ai_documents WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete goals
        sqlx::query("DELETE FROM goals WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete summaries
        sqlx::query("DELETE FROM summaries WHERE phase_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete the phase itself (scoped to user)
        sqlx::query("DELETE FROM brag_phases WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Activates this phase and deactivates all others for the user.
    pub async fn set_active(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE brag_phases SET is_active = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await?;

        sqlx::query("UPDATE brag_phases SET is_active = 1 WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Week
// ---------------------------------------------------------------------------

impl Week {
    /// All weeks in a phase, ordered chronologically.
    pub async fn list_for_phase(pool: &SqlitePool, phase_id: i64) -> Result<Vec<Self>, AppError> {
        let weeks = sqlx::query_as::<_, Week>(
            "SELECT * FROM weeks WHERE phase_id = ? ORDER BY year, iso_week",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        Ok(weeks)
    }

    /// Returns the `phase_id` for a given week.
    pub async fn phase_id(pool: &SqlitePool, week_id: i64) -> Result<i64, AppError> {
        let phase_id: i64 = sqlx::query_scalar("SELECT phase_id FROM weeks WHERE id = ?")
            .bind(week_id)
            .fetch_one(pool)
            .await?;
        Ok(phase_id)
    }

    /// Returns the existing week for `(phase_id, iso_week, year)` or creates one.
    /// New weeks get a sequential `week_number` within the phase.
    pub async fn find_or_create(
        pool: &SqlitePool,
        phase_id: i64,
        iso_week: i64,
        year: i64,
        start_date: &str,
        end_date: &str,
    ) -> Result<Self, AppError> {
        // Try to find existing
        if let Some(week) = sqlx::query_as::<_, Week>(
            "SELECT * FROM weeks WHERE phase_id = ? AND iso_week = ? AND year = ?",
        )
        .bind(phase_id)
        .bind(iso_week)
        .bind(year)
        .fetch_optional(pool)
        .await?
        {
            return Ok(week);
        }

        // Calculate week number within phase
        let week_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks WHERE phase_id = ?")
            .bind(phase_id)
            .fetch_one(pool)
            .await?;

        let week = sqlx::query_as::<_, Week>(
            r#"
            INSERT INTO weeks (phase_id, week_number, iso_week, year, start_date, end_date)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(week_count + 1)
        .bind(iso_week)
        .bind(year)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        Ok(week)
    }

    /// Find or create the week that contains the given date.
    pub async fn find_or_create_for_date(
        pool: &SqlitePool,
        phase_id: i64,
        date: NaiveDate,
    ) -> Result<Self, AppError> {
        let iso_week = date.iso_week().week();
        let year = date.iso_week().year();
        let week_start = iso_week_to_date(year, iso_week);
        let week_end = week_start + chrono::Duration::days(6);

        Self::find_or_create(
            pool,
            phase_id,
            iso_week as i64,
            year as i64,
            &week_start.format("%Y-%m-%d").to_string(),
            &week_end.format("%Y-%m-%d").to_string(),
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// WeeklyCheckin
// ---------------------------------------------------------------------------

impl WeeklyCheckin {
    /// Find checkin for a specific week and user.
    pub async fn find_for_week(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, WeeklyCheckinRow>(
            "SELECT * FROM weekly_checkins WHERE week_id = ? AND user_id = ?",
        )
        .bind(week_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Upsert a weekly check-in (insert or update on conflict).
    pub async fn upsert(
        pool: &SqlitePool,
        week_id: i64,
        user_id: i64,
        input: &SaveCheckin,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_proud_of = crypto.encrypt_opt(&input.proud_of)?;
        let enc_learned = crypto.encrypt_opt(&input.learned)?;
        let enc_wants_to_change = crypto.encrypt_opt(&input.wants_to_change)?;
        let enc_frustrations = crypto.encrypt_opt(&input.frustrations)?;
        let enc_notes = crypto.encrypt_opt(&input.notes)?;

        let row = sqlx::query_as::<_, WeeklyCheckinRow>(
            r#"
            INSERT INTO weekly_checkins (week_id, user_id, proud_of, learned, wants_to_change, frustrations, notes, energy_level, productivity_rating)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(week_id, user_id) DO UPDATE SET
                proud_of = excluded.proud_of,
                learned = excluded.learned,
                wants_to_change = excluded.wants_to_change,
                frustrations = excluded.frustrations,
                notes = excluded.notes,
                energy_level = excluded.energy_level,
                productivity_rating = excluded.productivity_rating,
                updated_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(week_id)
        .bind(user_id)
        .bind(&enc_proud_of)
        .bind(&enc_learned)
        .bind(&enc_wants_to_change)
        .bind(&enc_frustrations)
        .bind(&enc_notes)
        .bind(input.energy_level)
        .bind(input.productivity_rating)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// List all check-ins for a user with week info, ordered newest first.
    pub async fn list_with_weeks(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<CheckinWithWeek>, AppError> {
        let rows = sqlx::query_as::<_, CheckinWithWeekRow>(
            r#"
            SELECT wc.*, w.iso_week, w.year, w.start_date AS week_start, w.end_date AS week_end,
                   bp.name AS phase_name
            FROM weekly_checkins wc
            JOIN weeks w ON w.id = wc.week_id
            JOIN brag_phases bp ON bp.id = w.phase_id
            WHERE wc.user_id = ?
            ORDER BY w.year DESC, w.iso_week DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Delete a check-in and its KR snapshots (in a transaction).
    pub async fn delete(pool: &SqlitePool, week_id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        // Delete KR snapshots first
        sqlx::query(
            "DELETE FROM kr_checkin_snapshots WHERE checkin_id IN (SELECT id FROM weekly_checkins WHERE week_id = ? AND user_id = ?)",
        )
        .bind(week_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Delete the checkin itself
        sqlx::query("DELETE FROM weekly_checkins WHERE week_id = ? AND user_id = ?")
            .bind(week_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Count completed check-ins for a user (for streak calculation).
    pub async fn count_for_user(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM weekly_checkins WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// KrCheckinSnapshot
// ---------------------------------------------------------------------------

impl KrCheckinSnapshot {
    /// All snapshots for a given check-in.
    pub async fn list_for_checkin(
        pool: &SqlitePool,
        checkin_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, KrCheckinSnapshotRow>(
            "SELECT * FROM kr_checkin_snapshots WHERE checkin_id = ?",
        )
        .bind(checkin_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Upsert a KR snapshot within a check-in.
    pub async fn upsert(
        pool: &SqlitePool,
        p: &UpsertKrSnapshot<'_>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_blockers = crypto.encrypt_opt(&p.blockers.map(String::from))?;
        let enc_next_week_bet = crypto.encrypt_opt(&p.next_week_bet.map(String::from))?;

        let row = sqlx::query_as::<_, KrCheckinSnapshotRow>(
            r#"
            INSERT INTO kr_checkin_snapshots (checkin_id, key_result_id, current_value, confidence, blockers, next_week_bet)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(checkin_id, key_result_id) DO UPDATE SET
                current_value = excluded.current_value,
                confidence = excluded.confidence,
                blockers = excluded.blockers,
                next_week_bet = excluded.next_week_bet
            RETURNING *
            "#,
        )
        .bind(p.checkin_id)
        .bind(p.key_result_id)
        .bind(p.current_value)
        .bind(p.confidence)
        .bind(&enc_blockers)
        .bind(&enc_next_week_bet)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }
}

// ---------------------------------------------------------------------------
// ImpactStory
// ---------------------------------------------------------------------------

impl ImpactStory {
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, ImpactStoryRow>(
            "SELECT * FROM impact_stories WHERE phase_id = ? ORDER BY sort_order, id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, ImpactStoryRow>(
            r#"
            SELECT s.* FROM impact_stories s
            JOIN brag_phases p ON s.phase_id = p.id
            WHERE s.id = ? AND p.user_id = ?
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    pub async fn create(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &CreateImpactStory,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let owns: Option<i64> =
            sqlx::query_scalar("SELECT id FROM brag_phases WHERE id = ? AND user_id = ?")
                .bind(phase_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        if owns.is_none() {
            return Err(AppError::NotFound("Phase not found".to_string()));
        }

        let max_order: Option<i64> =
            sqlx::query_scalar("SELECT MAX(sort_order) FROM impact_stories WHERE phase_id = ?")
                .bind(phase_id)
                .fetch_one(pool)
                .await?;

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_situation = crypto.encrypt_opt(&input.situation)?;
        let enc_actions = crypto.encrypt_opt(&input.actions)?;
        let enc_result = crypto.encrypt_opt(&input.result)?;

        let row = sqlx::query_as::<_, ImpactStoryRow>(
            r#"
            INSERT INTO impact_stories (phase_id, title, situation, actions, result, sort_order)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(&enc_title)
        .bind(&enc_situation)
        .bind(&enc_actions)
        .bind(&enc_result)
        .bind(max_order.unwrap_or(0) + 1)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateImpactStory,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_situation = crypto.encrypt_opt(&input.situation)?;
        let enc_actions = crypto.encrypt_opt(&input.actions)?;
        let enc_result = crypto.encrypt_opt(&input.result)?;

        let row = sqlx::query_as::<_, ImpactStoryRow>(
            r#"
            UPDATE impact_stories SET title = ?, situation = ?, actions = ?, result = ?,
                status = ?, updated_at = datetime('now')
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING *
            "#,
        )
        .bind(&enc_title)
        .bind(&enc_situation)
        .bind(&enc_actions)
        .bind(&enc_result)
        .bind(input.status.as_deref().unwrap_or("draft"))
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM story_entries WHERE story_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM impact_stories WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Link an entry to this story.
    pub async fn link_entry(
        pool: &SqlitePool,
        story_id: i64,
        entry_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT OR IGNORE INTO story_entries (story_id, entry_id) VALUES (?, ?)")
            .bind(story_id)
            .bind(entry_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Unlink an entry from this story.
    pub async fn unlink_entry(
        pool: &SqlitePool,
        story_id: i64,
        entry_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM story_entries WHERE story_id = ? AND entry_id = ?")
            .bind(story_id)
            .bind(entry_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get entry IDs linked to a story.
    pub async fn linked_entry_ids(pool: &SqlitePool, story_id: i64) -> Result<Vec<i64>, AppError> {
        let ids: Vec<(i64,)> =
            sqlx::query_as("SELECT entry_id FROM story_entries WHERE story_id = ?")
                .bind(story_id)
                .fetch_all(pool)
                .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// AiDocument
// ---------------------------------------------------------------------------

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
