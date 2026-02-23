use sqlx::SqlitePool;

use crate::shared::error::AppError;

use super::model::{ProfileUpdate, User};

impl User {
    /// Looks up a user by primary key.
    pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(user)
    }

    /// Inserts a new user or updates profile fields on conflict (by `google_id`).
    /// Bumps `last_login_at` on each conflict update.
    pub async fn upsert(
        pool: &SqlitePool,
        google_id: &str,
        email: &str,
        name: &str,
        avatar_url: Option<&str>,
    ) -> Result<Self, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (google_id, email, name, avatar_url)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(google_id) DO UPDATE SET
                email = excluded.email,
                name = excluded.name,
                avatar_url = excluded.avatar_url,
                last_login_at = datetime('now')
            RETURNING *
            "#,
        )
        .bind(google_id)
        .bind(email)
        .bind(name)
        .bind(avatar_url)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// Persists user-editable settings (CLG role, promotion flag, profile fields).
    pub async fn update_settings(
        pool: &SqlitePool,
        id: i64,
        role: Option<&str>,
        wants_promotion: bool,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET role = ?, wants_promotion = ? WHERE id = ?")
            .bind(role)
            .bind(wants_promotion)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Persists profile fields (display_name, team, manager, etc.).
    pub async fn update_profile(
        pool: &SqlitePool,
        id: i64,
        p: &ProfileUpdate<'_>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET display_name = ?, team = ?, manager_name = ?, skip_level_name = ?, direct_reports = ?, timezone = ?, week_start = ?, work_start_time = ?, work_end_time = ? WHERE id = ?",
        )
        .bind(p.display_name)
        .bind(p.team)
        .bind(p.manager_name)
        .bind(p.skip_level_name)
        .bind(p.direct_reports)
        .bind(p.timezone)
        .bind(p.week_start)
        .bind(p.work_start_time)
        .bind(p.work_end_time)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
