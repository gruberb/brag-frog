use sqlx::SqlitePool;

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

use super::model::{BragEntry, BragEntryRow, CreateEntry, UpdateEntry};

impl BragEntry {
    /// All non-deleted entries in a phase, newest first.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, BragEntryRow>(
            r#"
            SELECT e.* FROM brag_entries e
            JOIN weeks w ON e.week_id = w.id
            WHERE w.phase_id = ? AND e.deleted_at IS NULL
            ORDER BY e.occurred_at DESC
            "#,
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Non-deleted entries in a phase within `[start_date, end_date]`.
    pub async fn list_for_phase_in_range(
        pool: &SqlitePool,
        phase_id: i64,
        start_date: &str,
        end_date: &str,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, BragEntryRow>(
            r#"
            SELECT e.* FROM brag_entries e
            JOIN weeks w ON e.week_id = w.id
            WHERE w.phase_id = ? AND e.occurred_at >= ? AND e.occurred_at <= ? AND e.deleted_at IS NULL
            ORDER BY e.occurred_at DESC
            "#,
        )
        .bind(phase_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Calendar meetings in a date range, regardless of soft-delete status.
    /// Used by the dashboard to show all synced meetings (even excluded ones)
    /// without including manually-created meeting entries.
    pub async fn list_calendar_meetings_in_range(
        pool: &SqlitePool,
        phase_id: i64,
        start_date: &str,
        end_date: &str,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, BragEntryRow>(
            r#"
            SELECT e.* FROM brag_entries e
            JOIN weeks w ON e.week_id = w.id
            WHERE w.phase_id = ?
              AND e.occurred_at >= ? AND e.occurred_at <= ?
              AND e.entry_type = 'meeting'
              AND e.source = 'google_calendar'
            ORDER BY e.occurred_at ASC, e.start_time ASC
            "#,
        )
        .bind(phase_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Filtered entry listing with optional key_result, goal, type, date range, and source filters.
    /// Builds the WHERE clause dynamically.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_for_phase_filtered(
        pool: &SqlitePool,
        phase_id: i64,
        priority_id: Option<i64>,
        department_goal_id: Option<i64>,
        entry_types: &[String],
        start_date: Option<&str>,
        end_date: Option<&str>,
        sources: &[String],
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let mut sql = String::from(
            "SELECT DISTINCT e.* FROM brag_entries e JOIN weeks w ON e.week_id = w.id",
        );

        sql.push_str(" WHERE w.phase_id = ? AND e.deleted_at IS NULL");

        if start_date.is_some() {
            sql.push_str(" AND e.occurred_at >= ?");
        }
        if end_date.is_some() {
            sql.push_str(" AND e.occurred_at <= ?");
        }

        if priority_id.is_some() {
            sql.push_str(" AND e.priority_id = ?");
        }
        if department_goal_id.is_some() {
            sql.push_str(" AND e.priority_id IN (SELECT id FROM priorities WHERE department_goal_id = ?)");
        }
        if !entry_types.is_empty() {
            let placeholders: Vec<&str> = entry_types.iter().map(|_| "?").collect();
            sql.push_str(&format!(
                " AND e.entry_type IN ({})",
                placeholders.join(",")
            ));
        }
        if !sources.is_empty() {
            let placeholders: Vec<&str> = sources.iter().map(|_| "?").collect();
            sql.push_str(&format!(" AND e.source IN ({})", placeholders.join(",")));
        }

        sql.push_str(" ORDER BY e.occurred_at DESC");

        let mut query = sqlx::query_as::<_, BragEntryRow>(&sql).bind(phase_id);

        if let Some(sd) = start_date {
            query = query.bind(sd);
        }
        if let Some(ed) = end_date {
            query = query.bind(ed);
        }
        if let Some(pri_id) = priority_id {
            query = query.bind(pri_id);
        }
        if let Some(gid) = department_goal_id {
            query = query.bind(gid);
        }
        for et in entry_types {
            query = query.bind(et);
        }
        for src in sources {
            query = query.bind(src);
        }

        let rows = query.fetch_all(pool).await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Fetches a single entry by ID, scoped to the owning user via phase join.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, BragEntryRow>(
            r#"
            SELECT e.* FROM brag_entries e
            JOIN weeks w ON e.week_id = w.id
            JOIN brag_phases p ON w.phase_id = p.id
            WHERE e.id = ? AND p.user_id = ?
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Looks up an entry by `(source, source_id)` pair, scoped to the user. Used during sync dedup.
    pub async fn find_by_source(
        pool: &SqlitePool,
        source: &str,
        source_id: &str,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, BragEntryRow>(
            r#"SELECT e.* FROM brag_entries e
            JOIN weeks w ON e.week_id = w.id
            JOIN brag_phases p ON w.phase_id = p.id
            WHERE e.source = ? AND e.source_id = ? AND p.user_id = ?"#,
        )
        .bind(source)
        .bind(source_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Creates a manual entry. Verifies week ownership before insert.
    pub async fn create(
        pool: &SqlitePool,
        input: &CreateEntry,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        // Verify the week belongs to the user
        let owns_week: Option<i64> = sqlx::query_scalar(
            "SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE w.id = ? AND p.user_id = ?",
        )
        .bind(input.week_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        if owns_week.is_none() {
            return Err(AppError::NotFound("Week not found".to_string()));
        }

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;
        let enc_teams = crypto.encrypt_opt(&input.teams)?;
        let enc_collaborators = crypto.encrypt_opt(&input.collaborators)?;

        let row = sqlx::query_as::<_, BragEntryRow>(
            r#"
            INSERT INTO brag_entries (week_id, priority_id, source, source_url, title, description, entry_type, occurred_at, teams, collaborators, reach, complexity, role)
            VALUES (?, ?, 'manual', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(input.week_id)
        .bind(input.priority_id)
        .bind(&input.source_url)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(&input.entry_type)
        .bind(&input.occurred_at)
        .bind(&enc_teams)
        .bind(&enc_collaborators)
        .bind(&input.reach)
        .bind(&input.complexity)
        .bind(&input.role)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Upserts an entry from an external sync source. Deduplicates on `(source, source_id)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_from_sync(
        pool: &SqlitePool,
        week_id: i64,
        source: &str,
        source_id: &str,
        source_url: Option<&str>,
        title: &str,
        description: Option<&str>,
        entry_type: &str,
        status: Option<&str>,
        repository: Option<&str>,
        occurred_at: &str,
        meeting_role: Option<&str>,
        recurring_group: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(title)?;
        let enc_description = crypto.encrypt_opt(&description.map(String::from))?;

        let row = sqlx::query_as::<_, BragEntryRow>(
            r#"
            INSERT INTO brag_entries (week_id, source, source_id, source_url, title, description, entry_type, status, repository, occurred_at, meeting_role, recurring_group, start_time, end_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(source, source_id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                entry_type = excluded.entry_type,
                status = excluded.status,
                source_url = COALESCE(excluded.source_url, brag_entries.source_url),
                repository = COALESCE(excluded.repository, brag_entries.repository),
                recurring_group = COALESCE(excluded.recurring_group, brag_entries.recurring_group),
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                updated_at = datetime('now'),
                deleted_at = NULL
            RETURNING *
            "#,
        )
        .bind(week_id)
        .bind(source)
        .bind(source_id)
        .bind(source_url)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(entry_type)
        .bind(status)
        .bind(repository)
        .bind(occurred_at)
        .bind(meeting_role)
        .bind(recurring_group)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Updates an entry's fields. Optionally moves it to a different week.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateEntry,
        new_week_id: Option<i64>,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;
        let enc_teams = crypto.encrypt_opt(&input.teams)?;
        let enc_collaborators = crypto.encrypt_opt(&input.collaborators)?;

        let row = sqlx::query_as::<_, BragEntryRow>(
            r#"
            UPDATE brag_entries
            SET priority_id = ?, title = ?, description = ?, entry_type = ?,
                occurred_at = ?, teams = ?, collaborators = ?, source_url = ?,
                reach = ?, complexity = ?, role = ?,
                week_id = COALESCE(?, week_id),
                updated_at = datetime('now')
            WHERE id = ? AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            RETURNING *
            "#,
        )
        .bind(input.priority_id)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(&input.entry_type)
        .bind(&input.occurred_at)
        .bind(&enc_teams)
        .bind(&enc_collaborators)
        .bind(&input.source_url)
        .bind(&input.reach)
        .bind(&input.complexity)
        .bind(&input.role)
        .bind(new_week_id)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Hard-deletes an entry, scoped to the owning user.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM brag_entries WHERE id = ? AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Sets `deleted_at` without removing the row. Used during sync to hide stale entries.
    pub async fn soft_delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE brag_entries SET deleted_at = datetime('now')
            WHERE id = ? AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Permanently removes ALL entries for a given source/user (used when disconnecting a service).
    pub async fn hard_delete_all_for_service(
        pool: &SqlitePool,
        user_id: i64,
        source: &str,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM brag_entries
            WHERE source = ? AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(source)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Permanently removes all soft-deleted entries for a given source/user.
    pub async fn clear_soft_deletes_for_service(
        pool: &SqlitePool,
        user_id: i64,
        source: &str,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM brag_entries
            WHERE source = ? AND deleted_at IS NOT NULL AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(source)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Get distinct team names from all entries in a phase
    pub async fn distinct_teams_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<String>, AppError> {
        let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
            "SELECT e.teams FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ? AND e.teams IS NOT NULL AND e.deleted_at IS NULL",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;

        let mut set = std::collections::BTreeSet::new();
        for (encrypted,) in rows {
            let raw = crypto.decrypt(&encrypted)?;
            for part in raw.split(',') {
                let t = part.trim();
                if !t.is_empty() {
                    set.insert(t.to_string());
                }
            }
        }
        Ok(set.into_iter().collect())
    }

    /// Get distinct collaborator names from all entries in a phase
    pub async fn distinct_collaborators_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<String>, AppError> {
        let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
            "SELECT e.collaborators FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ? AND e.collaborators IS NOT NULL AND e.deleted_at IS NULL",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;

        let mut set = std::collections::BTreeSet::new();
        for (encrypted,) in rows {
            let raw = crypto.decrypt(&encrypted)?;
            for part in raw.split(',') {
                let c = part.trim();
                if !c.is_empty() {
                    set.insert(c.to_string());
                }
            }
        }
        Ok(set.into_iter().collect())
    }

    /// Counts non-deleted, non-manual entries per source within a phase.
    pub async fn source_counts_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let counts = sqlx::query_as::<_, (String, i64)>(
            "SELECT e.source, COUNT(*) FROM brag_entries e JOIN weeks w ON e.week_id = w.id WHERE w.phase_id = ? AND e.deleted_at IS NULL AND e.source != 'manual' GROUP BY e.source",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        Ok(counts)
    }

    /// Soft-deletes all entries whose source_id starts with `drive:{file_id}:`.
    pub async fn soft_delete_by_file_id(
        pool: &SqlitePool,
        user_id: i64,
        file_id: &str,
    ) -> Result<u64, AppError> {
        let pattern = format!("drive:{}:%", file_id);
        let result = sqlx::query(
            r#"
            UPDATE brag_entries SET deleted_at = datetime('now')
            WHERE source = 'google_drive' AND source_id LIKE ? AND deleted_at IS NULL
            AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(&pattern)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Clears `deleted_at` on all entries whose source_id starts with `drive:{file_id}:`.
    pub async fn clear_soft_deletes_by_file_id(
        pool: &SqlitePool,
        user_id: i64,
        file_id: &str,
    ) -> Result<u64, AppError> {
        let pattern = format!("drive:{}:%", file_id);
        let result = sqlx::query(
            r#"
            UPDATE brag_entries SET deleted_at = NULL
            WHERE source = 'google_drive' AND source_id LIKE ? AND deleted_at IS NOT NULL
            AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(&pattern)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Soft-deletes all entries whose source_id starts with `calendar:{event_id}`.
    pub async fn soft_delete_by_event_id(
        pool: &SqlitePool,
        user_id: i64,
        event_id: &str,
    ) -> Result<u64, AppError> {
        let pattern = format!("calendar:{}%", event_id);
        let result = sqlx::query(
            r#"
            UPDATE brag_entries SET deleted_at = datetime('now')
            WHERE source = 'google_calendar' AND source_id LIKE ? AND deleted_at IS NULL
            AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(&pattern)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Clears `deleted_at` on all entries whose source_id starts with `calendar:{event_id}`.
    pub async fn clear_soft_deletes_by_event_id(
        pool: &SqlitePool,
        user_id: i64,
        event_id: &str,
    ) -> Result<u64, AppError> {
        let pattern = format!("calendar:{}%", event_id);
        let result = sqlx::query(
            r#"
            UPDATE brag_entries SET deleted_at = NULL
            WHERE source = 'google_calendar' AND source_id LIKE ? AND deleted_at IS NOT NULL
            AND week_id IN (
                SELECT w.id FROM weeks w JOIN brag_phases p ON w.phase_id = p.id WHERE p.user_id = ?
            )
            "#,
        )
        .bind(&pattern)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Soft-delete entries from a service that were NOT returned by the latest sync,
    /// but only if the user hasn't edited them (no key_result, highlight, teams, or collaborators).
    pub async fn soft_delete_unmatched_for_service(
        pool: &SqlitePool,
        user_id: i64,
        source: &str,
        matched_source_ids: &[String],
    ) -> Result<u64, AppError> {
        if matched_source_ids.is_empty() {
            // No entries came back from sync — soft-delete all non-edited entries for this source
            let result = sqlx::query(
                r#"
                UPDATE brag_entries SET deleted_at = datetime('now')
                WHERE source = ? AND deleted_at IS NULL
                AND key_result_id IS NULL AND teams IS NULL AND collaborators IS NULL
                AND week_id IN (
                    SELECT w.id FROM weeks w
                    JOIN brag_phases p ON w.phase_id = p.id
                    WHERE p.user_id = ? AND p.is_active = 1
                )
                "#,
            )
            .bind(source)
            .bind(user_id)
            .execute(pool)
            .await?;
            return Ok(result.rows_affected());
        }

        // Build placeholders for the NOT IN clause
        let placeholders: Vec<&str> = matched_source_ids.iter().map(|_| "?").collect();
        let sql = format!(
            r#"
            UPDATE brag_entries SET deleted_at = datetime('now')
            WHERE source = ? AND deleted_at IS NULL
            AND source_id NOT IN ({})
            AND key_result_id IS NULL AND teams IS NULL AND collaborators IS NULL
            AND week_id IN (
                SELECT w.id FROM weeks w
                JOIN brag_phases p ON w.phase_id = p.id
                WHERE p.user_id = ? AND p.is_active = 1
            )
            "#,
            placeholders.join(",")
        );

        let mut query = sqlx::query(&sql).bind(source);
        for sid in matched_source_ids {
            query = query.bind(sid);
        }
        query = query.bind(user_id);

        let result = query.execute(pool).await?;
        Ok(result.rows_affected())
    }

    /// Hard-delete entries from a service that were NOT returned by the latest sync,
    /// but only if the user hasn't edited them (no key_result, teams, or collaborators).
    /// Unlike `soft_delete_unmatched_for_service`, this permanently removes entries
    /// so they can be re-created if they re-enter the sync window later (e.g., calendar
    /// events that move from future to past).
    pub async fn hard_delete_unmatched_for_service(
        pool: &SqlitePool,
        user_id: i64,
        source: &str,
        matched_source_ids: &[String],
    ) -> Result<u64, AppError> {
        if matched_source_ids.is_empty() {
            let result = sqlx::query(
                r#"
                DELETE FROM brag_entries
                WHERE source = ? AND key_result_id IS NULL AND teams IS NULL AND collaborators IS NULL
                AND week_id IN (
                    SELECT w.id FROM weeks w
                    JOIN brag_phases p ON w.phase_id = p.id
                    WHERE p.user_id = ? AND p.is_active = 1
                )
                "#,
            )
            .bind(source)
            .bind(user_id)
            .execute(pool)
            .await?;
            return Ok(result.rows_affected());
        }

        let placeholders: Vec<&str> = matched_source_ids.iter().map(|_| "?").collect();
        let sql = format!(
            r#"
            DELETE FROM brag_entries
            WHERE source = ? AND source_id NOT IN ({})
            AND key_result_id IS NULL AND teams IS NULL AND collaborators IS NULL
            AND week_id IN (
                SELECT w.id FROM weeks w
                JOIN brag_phases p ON w.phase_id = p.id
                WHERE p.user_id = ? AND p.is_active = 1
            )
            "#,
            placeholders.join(",")
        );

        let mut query = sqlx::query(&sql).bind(source);
        for sid in matched_source_ids {
            query = query.bind(sid);
        }
        query = query.bind(user_id);

        let result = query.execute(pool).await?;
        Ok(result.rows_affected())
    }
}
