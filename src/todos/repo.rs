use sqlx::SqlitePool;

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::todos::model::{CreateTodo, Todo, TodoRow};

impl Todo {
    /// List active (uncompleted) todos for a user, ordered by sort_order.
    pub async fn list_active(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos WHERE user_id = ? AND completed = 0 ORDER BY sort_order, created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// List completed todos for a user, newest first.
    pub async fn list_completed(
        pool: &SqlitePool,
        user_id: i64,
        crypto: &UserCrypto,
    ) -> Result<Vec<Self>, AppError> {
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos WHERE user_id = ? AND completed = 1 ORDER BY completed_at DESC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(|r| r.decrypt(crypto)).collect()
    }

    /// Create a new todo.
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        input: &CreateTodo,
        crypto: &UserCrypto,
    ) -> Result<Self, AppError> {
        let enc_title = crypto.encrypt(&input.title)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM todos WHERE user_id = ? AND completed = 0",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let row = sqlx::query_as::<_, TodoRow>(
            "INSERT INTO todos (user_id, title, sort_order) VALUES (?, ?, ?) RETURNING *",
        )
        .bind(user_id)
        .bind(&enc_title)
        .bind(count)
        .fetch_one(pool)
        .await?;

        row.decrypt(crypto)
    }

    /// Toggle completed status.
    pub async fn toggle(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE todos SET
                completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END,
                completed_at = CASE WHEN completed = 0 THEN datetime('now') ELSE NULL END
            WHERE id = ? AND user_id = ?"#,
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a todo.
    pub async fn delete(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM todos WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
