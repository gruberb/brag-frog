use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

use super::super::model::{CreateDepartmentGoal, DepartmentGoal, DepartmentGoalRow, UpdateDepartmentGoal};

impl DepartmentGoal {
    /// All department goals for a phase, ordered by sort_order.
    pub async fn list_for_phase(
        pool: &SqlitePool,
        phase_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, DepartmentGoalRow>(
            "SELECT id, phase_id, title, description, status, sort_order, source, created_at FROM department_goals WHERE phase_id = ? ORDER BY sort_order, id",
        )
        .bind(phase_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Creates a department goal at the end of the sort order. Verifies phase ownership.
    pub async fn create(
        pool: &SqlitePool,
        phase_id: i64,
        user_id: i64,
        input: &CreateDepartmentGoal,
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
            sqlx::query_scalar("SELECT MAX(sort_order) FROM department_goals WHERE phase_id = ?")
                .bind(phase_id)
                .fetch_one(pool)
                .await?;

        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, DepartmentGoalRow>(
            r#"
            INSERT INTO department_goals (phase_id, title, description, sort_order, status)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id, phase_id, title, description, status, sort_order, source, created_at
            "#,
        )
        .bind(phase_id)
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(max_order.unwrap_or(0) + 1)
        .bind(input.status.as_deref().unwrap_or("in_progress"))
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Updates a department goal.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        input: &UpdateDepartmentGoal,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let enc_description = crypto.encrypt_opt(&input.description)?;

        let row = sqlx::query_as::<_, DepartmentGoalRow>(
            r#"
            UPDATE department_goals SET title = ?, description = ?, status = ?
            WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)
            RETURNING id, phase_id, title, description, status, sort_order, source, created_at
            "#,
        )
        .bind(&enc_title)
        .bind(&enc_description)
        .bind(input.status.as_deref().unwrap_or("in_progress"))
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Deletes a department goal. Nullifies department_goal_id on child priorities.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;

        sqlx::query("UPDATE priorities SET department_goal_id = NULL WHERE department_goal_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM department_goals WHERE id = ? AND phase_id IN (SELECT id FROM brag_phases WHERE user_id = ?)",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
