use rand::Rng;
use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

use super::super::model::{
    CreatePriority, PostPriorityUpdate, Priority, PriorityRow, PriorityUpdate,
    PriorityUpdateRow, UpdatePriority,
};

const PRIORITY_COLORS: &[&str] = &[
    "#FF453F", "#E0439D", "#D94F04", "#0B8043", "#1A73E8", "#F4511E", "#8E24AA", "#00897B",
    "#C2185B", "#6D4C41", "#5C6BC0", "#00ACC1", "#7CB342", "#F9A825", "#546E7A",
];

pub fn random_color() -> String {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..PRIORITY_COLORS.len());
    PRIORITY_COLORS[idx].to_string()
}

const SELECT_COLS: &str = "id, phase_id, user_id, title, status, color, sort_order,
    scope, started_at, completed_at, impact_narrative, department_goal_id, created_at,
    priority_level, measure_type, measure_start, measure_target, measure_current, description,
    tracking_status, due_date, tier";

impl Priority {
    /// All priorities for a phase, ordered by sort_order.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let q = format!(
            "SELECT {} FROM priorities WHERE phase_id = ? ORDER BY sort_order, id",
            SELECT_COLS
        );
        let rows = sqlx::query_as::<_, PriorityRow>(&q)
            .bind(phase_id)
            .fetch_all(pool)
            .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Priorities linked to a specific department goal.
    pub async fn list_for_department_goal(
        pool: &SqlitePool,
        department_goal_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let q = format!(
            "SELECT {} FROM priorities WHERE department_goal_id = ? ORDER BY sort_order, id",
            SELECT_COLS
        );
        let rows = sqlx::query_as::<_, PriorityRow>(&q)
            .bind(department_goal_id)
            .fetch_all(pool)
            .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// All active (non-completed, non-cancelled) priorities for a user's active phase.
    pub async fn list_active_for_user(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, PriorityRow>(
            r#"
            SELECT pr.id, pr.phase_id, pr.user_id, pr.title, pr.status,
                pr.color, pr.sort_order, pr.scope, pr.started_at, pr.completed_at,
                pr.impact_narrative, pr.department_goal_id, pr.created_at,
                pr.priority_level, pr.measure_type, pr.measure_start, pr.measure_target,
                pr.measure_current, pr.description,
                pr.tracking_status, pr.due_date, pr.tier
            FROM priorities pr
            JOIN brag_phases p ON pr.phase_id = p.id
            WHERE p.user_id = ? AND p.is_active = 1
              AND pr.status NOT IN ('completed', 'cancelled')
            ORDER BY pr.sort_order, pr.id
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Find by ID, scoped to user.
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Option<Self>, AppError> {
        let row = sqlx::query_as::<_, PriorityRow>(
            r#"
            SELECT pr.id, pr.phase_id, pr.user_id, pr.title, pr.status,
                pr.color, pr.sort_order, pr.scope, pr.started_at, pr.completed_at,
                pr.impact_narrative, pr.department_goal_id, pr.created_at,
                pr.priority_level, pr.measure_type, pr.measure_start, pr.measure_target,
                pr.measure_current, pr.description,
                pr.tracking_status, pr.due_date, pr.tier
            FROM priorities pr
            JOIN brag_phases p ON pr.phase_id = p.id
            WHERE pr.id = ? AND p.user_id = ?
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        row.map(|r| r.decrypt(crypto)).transpose()
    }

    /// Creates a priority with a random badge color.
    pub async fn create(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &CreatePriority,
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
            sqlx::query_scalar("SELECT MAX(sort_order) FROM priorities WHERE phase_id = ?")
                .bind(phase_id)
                .fetch_one(pool)
                .await?;

        let color = random_color();
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_narrative = crypto.encrypt_opt(&input.impact_narrative)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        // measure_current defaults to measure_start on create
        let measure_current = input.measure_start;

        let row = sqlx::query_as::<_, PriorityRow>(
            r#"
            INSERT INTO priorities (phase_id, user_id, title, status, color, sort_order,
                scope, impact_narrative, department_goal_id,
                priority_level, measure_type, measure_start, measure_target, measure_current,
                description, tracking_status, due_date, tier)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at,
                priority_level, measure_type, measure_start, measure_target, measure_current,
                description, tracking_status, due_date, tier
            "#,
        )
        .bind(phase_id)
        .bind(user_id)
        .bind(&enc_title)
        .bind(input.status.as_deref().unwrap_or("active"))
        .bind(&color)
        .bind(max_order.unwrap_or(0) + 1)
        .bind(&input.scope)
        .bind(&enc_narrative)
        .bind(input.department_goal_id)
        .bind(&input.priority_level)
        .bind(&input.measure_type)
        .bind(input.measure_start)
        .bind(input.measure_target)
        .bind(measure_current)
        .bind(&enc_description)
        .bind(&input.tracking_status)
        .bind(&input.due_date)
        .bind(&input.tier)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Updates a priority's fields.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdatePriority,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let status = input.status.as_deref().unwrap_or("active");

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_narrative = crypto.encrypt_opt(&input.impact_narrative)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, PriorityRow>(
            r#"
            UPDATE priorities SET title = ?, status = ?, scope = ?,
                impact_narrative = ?, department_goal_id = ?,
                started_at = ?, completed_at = ?,
                priority_level = ?, measure_type = ?,
                measure_start = ?, measure_target = ?, measure_current = ?,
                description = ?, tracking_status = ?, due_date = ?, tier = ?
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at,
                priority_level, measure_type, measure_start, measure_target, measure_current,
                description, tracking_status, due_date, tier
            "#,
        )
        .bind(&enc_title)
        .bind(status)
        .bind(&input.scope)
        .bind(&enc_narrative)
        .bind(input.department_goal_id)
        .bind(&input.started_at)
        .bind(&input.completed_at)
        .bind(&input.priority_level)
        .bind(&input.measure_type)
        .bind(input.measure_start)
        .bind(input.measure_target)
        .bind(input.measure_current)
        .bind(&enc_description)
        .bind(&input.tracking_status)
        .bind(&input.due_date)
        .bind(&input.tier)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Marks all non-terminal priorities under a department goal as completed.
    pub async fn complete_all_for_department_goal(
        pool: &SqlitePool,
        department_goal_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE priorities SET status = 'completed', completed_at = date('now')
            WHERE department_goal_id = ?
              AND status NOT IN ('completed', 'cancelled')
              AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            "#,
        )
        .bind(department_goal_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Deletes a priority, its update log, and nullifies `priority_id` on linked entries.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("UPDATE brag_entries SET priority_id = NULL WHERE priority_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM priority_updates WHERE priority_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM priorities WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

impl PriorityUpdate {
    /// Lists updates for a priority, newest first.
    pub async fn list_for_priority(
        pool: &SqlitePool,
        priority_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, PriorityUpdateRow>(
            "SELECT id, priority_id, user_id, tracking_status, measure_value, comment, created_at
             FROM priority_updates WHERE priority_id = ? ORDER BY created_at DESC",
        )
        .bind(priority_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Posts a new update, also syncing tracking_status and measure_current
    /// on the parent priority.
    pub async fn create(
        pool: &SqlitePool,
        priority_id: i64,
        user_id: i64,
        input: &PostPriorityUpdate,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_comment = crypto.encrypt_opt(&input.comment)?;

        let row = sqlx::query_as::<_, PriorityUpdateRow>(
            r#"
            INSERT INTO priority_updates (priority_id, user_id, tracking_status, measure_value, comment)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id, priority_id, user_id, tracking_status, measure_value, comment, created_at
            "#,
        )
        .bind(priority_id)
        .bind(user_id)
        .bind(&input.tracking_status)
        .bind(input.measure_value)
        .bind(&enc_comment)
        .fetch_one(pool)
        .await?;

        // Sync tracking_status and measure_current back to the priority
        if input.tracking_status.is_some() || input.measure_value.is_some() {
            let mut set_parts = Vec::new();
            if input.tracking_status.is_some() {
                set_parts.push("tracking_status = ?");
            }
            if input.measure_value.is_some() {
                set_parts.push("measure_current = ?");
            }
            let sql = format!(
                "UPDATE priorities SET {} WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
                set_parts.join(", ")
            );
            let mut query = sqlx::query(&sql);
            if let Some(ref ts) = input.tracking_status {
                query = query.bind(ts);
            }
            if let Some(mv) = input.measure_value {
                query = query.bind(mv);
            }
            query = query.bind(priority_id).bind(user_id);
            query.execute(pool).await?;
        }

        row.decrypt(crypto)
    }
}
