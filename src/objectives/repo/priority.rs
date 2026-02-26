use rand::Rng;
use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

use super::super::model::{CreatePriority, Priority, PriorityRow, UpdatePriority};

const PRIORITY_COLORS: &[&str] = &[
    "#FF453F", "#E0439D", "#D94F04", "#0B8043", "#1A73E8", "#F4511E", "#8E24AA", "#00897B",
    "#C2185B", "#6D4C41", "#5C6BC0", "#00ACC1", "#7CB342", "#F9A825", "#546E7A",
];

fn random_color() -> String {
    let mut rng = rand::rng();
    let idx = rng.random_range(0..PRIORITY_COLORS.len());
    PRIORITY_COLORS[idx].to_string()
}

impl Priority {
    /// All priorities for a phase, ordered by sort_order.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, PriorityRow>(
            "SELECT id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at
             FROM priorities WHERE phase_id = ? ORDER BY sort_order, id",
        )
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
        let rows = sqlx::query_as::<_, PriorityRow>(
            "SELECT id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at
             FROM priorities WHERE department_goal_id = ? ORDER BY sort_order, id",
        )
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
                pr.impact_narrative, pr.department_goal_id, pr.created_at
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
                pr.impact_narrative, pr.department_goal_id, pr.created_at
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

        let row = sqlx::query_as::<_, PriorityRow>(
            r#"
            INSERT INTO priorities (phase_id, user_id, title, status, color, sort_order,
                scope, impact_narrative, department_goal_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at
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

        let row = sqlx::query_as::<_, PriorityRow>(
            r#"
            UPDATE priorities SET title = ?, status = ?, scope = ?,
                impact_narrative = ?, department_goal_id = ?,
                started_at = ?, completed_at = ?
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING id, phase_id, user_id, title, status, color, sort_order,
                scope, started_at, completed_at, impact_narrative, department_goal_id, created_at
            "#,
        )
        .bind(&enc_title)
        .bind(status)
        .bind(&input.scope)
        .bind(&enc_narrative)
        .bind(input.department_goal_id)
        .bind(&input.started_at)
        .bind(&input.completed_at)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Deletes a priority and nullifies `priority_id` on all linked entries.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("UPDATE brag_entries SET priority_id = NULL WHERE priority_id = ?")
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
