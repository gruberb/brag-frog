use rand::Rng;
use sqlx::SqlitePool;

use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;

use super::model::{
    CreateGoal, CreateInitiative, CreateKeyResult, Goal, GoalRow, Initiative, InitiativeRow,
    KeyResult, UpdateGoal, UpdateInitiative, UpdateKeyResult,
};

// Curated palette of distinct, accessible colors for key result badges.
const KEY_RESULT_COLORS: &[&str] = &[
    "#FF453F", "#E0439D", "#D94F04", "#0B8043", "#1A73E8", "#F4511E", "#8E24AA", "#00897B",
    "#C2185B", "#6D4C41", "#5C6BC0", "#00ACC1", "#7CB342", "#F9A825", "#546E7A",
];

// Picks a random color from the palette for a new key result.
fn random_key_result_color() -> String {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..KEY_RESULT_COLORS.len());
    KEY_RESULT_COLORS[idx].to_string()
}

impl Goal {
    /// Load goals for multiple phases in a single query, grouped by phase_id.
    pub async fn list_for_phases(
        pool: &SqlitePool,
        phase_ids: &[i64],
        crypto: &UserCrypto,
    ) -> Result<std::collections::HashMap<i64, Vec<Self>>, AppError> {
        let mut map: std::collections::HashMap<i64, Vec<Self>> = std::collections::HashMap::new();
        if phase_ids.is_empty() {
            return Ok(map);
        }
        let placeholders: String = phase_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT * FROM goals WHERE phase_id IN ({}) ORDER BY sort_order, id",
            placeholders
        );
        let mut query = sqlx::query_as::<_, GoalRow>(&sql);
        for id in phase_ids {
            query = query.bind(id);
        }
        let rows = query.fetch_all(pool).await?;
        for row in rows {
            let phase_id = row.phase_id;
            let goal = row.decrypt(crypto)?;
            map.entry(phase_id).or_default().push(goal);
        }
        Ok(map)
    }

    /// All goals for a single phase, ordered by `sort_order`.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, GoalRow>(
            "SELECT * FROM goals WHERE phase_id = ? ORDER BY sort_order, id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Creates a goal at the end of the sort order. Verifies phase ownership.
    pub async fn create(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &CreateGoal,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        // Verify phase belongs to user
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
            sqlx::query_scalar("SELECT MAX(sort_order) FROM goals WHERE phase_id = ?")
                .bind(phase_id)
                .fetch_one(pool)
                .await?;

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, GoalRow>(
            r#"
            INSERT INTO goals (phase_id, title, description, category, sort_order, status)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(&input.category)
        .bind(max_order.unwrap_or(0) + 1)
        .bind(input.status.as_deref().unwrap_or("in_progress"))
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Updates a goal's title, description, category, and status.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateGoal,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, GoalRow>(
            r#"
            UPDATE goals SET title = ?, description = ?, category = ?, status = ?
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING *
            "#,
        )
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(&input.category)
        .bind(input.status.as_deref().unwrap_or("in_progress"))
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Deletes a goal. Does not cascade to key results (they become unassigned).
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM goals WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all goals for a user's active phase
    pub async fn list_for_active_phase(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, GoalRow>(
            r#"
            SELECT g.* FROM goals g
            JOIN brag_phases p ON g.phase_id = p.id
            WHERE p.user_id = ? AND p.is_active = 1
            ORDER BY g.sort_order, g.id
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }
}

impl KeyResult {
    /// All key results for a user (archived last), ordered by name.
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let key_results = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE user_id = ? ORDER BY is_archived, name",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(key_results)
    }

    /// Non-archived key results for a user, ordered by name.
    pub async fn list_active_for_user(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<Self>, AppError> {
        let key_results = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE user_id = ? AND is_archived = 0 ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(key_results)
    }

    /// Creates a key result with a random badge color.
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        input: &CreateKeyResult,
    ) -> Result<Self, AppError> {
        let color = random_key_result_color();
        let kr_type = input.kr_type.as_deref().unwrap_or("manual");
        let key_result = sqlx::query_as::<_, KeyResult>(
            r#"
            INSERT INTO key_results (user_id, name, color, status, goal_id, kr_type, direction, unit, baseline, target, target_date)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&input.name)
        .bind(&color)
        .bind(input.status.as_deref().unwrap_or("not_started"))
        .bind(input.goal_id)
        .bind(kr_type)
        .bind(&input.direction)
        .bind(&input.unit)
        .bind(input.baseline)
        .bind(input.target)
        .bind(&input.target_date)
        .fetch_one(pool)
        .await?;

        Ok(key_result)
    }

    /// Updates name, status, goal assignment, progress, and measurement fields.
    /// Auto-recalculates score and syncs progress.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateKeyResult,
    ) -> Result<Self, AppError> {
        let progress = input.progress.unwrap_or(0);
        let status = if progress >= 100 {
            "completed"
        } else {
            input.status.as_deref().unwrap_or("not_started")
        };

        let kr_type = input.kr_type.as_deref().unwrap_or("manual");

        let key_result = sqlx::query_as::<_, KeyResult>(
            r#"
            UPDATE key_results SET name = ?, status = ?, goal_id = ?, progress = ?,
                kr_type = ?, direction = ?, unit = ?, baseline = ?, target = ?,
                current_value = ?, target_date = ?
            WHERE id = ? AND user_id = ?
            RETURNING *
            "#,
        )
        .bind(&input.name)
        .bind(status)
        .bind(input.goal_id)
        .bind(progress)
        .bind(kr_type)
        .bind(&input.direction)
        .bind(&input.unit)
        .bind(input.baseline)
        .bind(input.target)
        .bind(input.current_value)
        .bind(&input.target_date)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        // Recalculate and persist score + synced progress
        if let Some(score) = key_result.recalculate_score() {
            let synced_progress = (score * 100.0).round() as i64;
            let final_status = if synced_progress >= 100 {
                "completed"
            } else {
                status
            };
            let updated = sqlx::query_as::<_, KeyResult>(
                "UPDATE key_results SET score = ?, progress = ?, status = ? WHERE id = ? RETURNING *",
            )
            .bind(score)
            .bind(synced_progress)
            .bind(final_status)
            .bind(key_result.id)
            .fetch_one(pool)
            .await?;
            return Ok(updated);
        }

        Ok(key_result)
    }

    /// Marks all key results under a goal as completed at 100% progress.
    pub async fn complete_all_for_goal(
        pool: &SqlitePool,
        goal_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE key_results SET status = 'completed', progress = 100 WHERE goal_id = ? AND user_id = ?",
        )
        .bind(goal_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Deletes a key result and nullifies `key_result_id` on all linked entries (in a tx).
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("UPDATE brag_entries SET key_result_id = NULL WHERE key_result_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM key_results WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}

impl Initiative {
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, InitiativeRow>(
            "SELECT * FROM initiatives WHERE phase_id = ? ORDER BY sort_order, id",
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
        let row = sqlx::query_as::<_, InitiativeRow>(
            r#"
            SELECT i.* FROM initiatives i
            JOIN brag_phases p ON i.phase_id = p.id
            WHERE i.id = ? AND p.user_id = ?
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
        input: &CreateInitiative,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        // Verify phase belongs to user
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
            sqlx::query_scalar("SELECT MAX(sort_order) FROM initiatives WHERE phase_id = ?")
                .bind(phase_id)
                .fetch_one(pool)
                .await?;

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, InitiativeRow>(
            r#"
            INSERT INTO initiatives (phase_id, title, description, status, scope, is_planned, sort_order)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(phase_id)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(input.status.as_deref().unwrap_or("planned"))
        .bind(&input.scope)
        .bind(input.is_planned.unwrap_or(true))
        .bind(max_order.unwrap_or(0) + 1)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateInitiative,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, InitiativeRow>(
            r#"
            UPDATE initiatives SET title = ?, description = ?, status = ?, scope = ?,
                is_planned = ?, started_at = ?, completed_at = ?
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING *
            "#,
        )
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(input.status.as_deref().unwrap_or("planned"))
        .bind(&input.scope)
        .bind(input.is_planned.unwrap_or(true))
        .bind(&input.started_at)
        .bind(&input.completed_at)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        // Nullify initiative_id on entries
        sqlx::query("UPDATE brag_entries SET initiative_id = NULL WHERE initiative_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete junction records
        sqlx::query("DELETE FROM initiative_key_results WHERE initiative_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM initiatives WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Link a key result to this initiative.
    pub async fn link_key_result(
        pool: &SqlitePool,
        initiative_id: i64,
        key_result_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT OR IGNORE INTO initiative_key_results (initiative_id, key_result_id) VALUES (?, ?)",
        )
        .bind(initiative_id)
        .bind(key_result_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Unlink a key result from this initiative.
    pub async fn unlink_key_result(
        pool: &SqlitePool,
        initiative_id: i64,
        key_result_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM initiative_key_results WHERE initiative_id = ? AND key_result_id = ?",
        )
        .bind(initiative_id)
        .bind(key_result_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get key result IDs linked to an initiative.
    pub async fn linked_key_result_ids(
        pool: &SqlitePool,
        initiative_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT key_result_id FROM initiative_key_results WHERE initiative_id = ?",
        )
        .bind(initiative_id)
        .fetch_all(pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }
}
