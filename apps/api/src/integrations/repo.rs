//! Persistence layer for the sync bounded context.
//! All SQL queries live here — `model.rs` contains no SQL.

use sqlx::SqlitePool;

use crate::kernel::error::AppError;

use super::model::*;

// ---------------------------------------------------------------------------
// IntegrationConfig
// ---------------------------------------------------------------------------

impl IntegrationConfig {
    /// All integration configs for a user, ordered by service name.
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let configs = sqlx::query_as::<_, IntegrationConfig>(
            "SELECT * FROM integration_configs WHERE user_id = ? ORDER BY service",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(configs)
    }

    /// Looks up a single integration by `(user_id, service)`.
    pub async fn find_by_service(
        pool: &SqlitePool,
        user_id: i64,
        service: &str,
    ) -> Result<Option<Self>, AppError> {
        let config = sqlx::query_as::<_, IntegrationConfig>(
            "SELECT * FROM integration_configs WHERE user_id = ? AND service = ?",
        )
        .bind(user_id)
        .bind(service)
        .fetch_optional(pool)
        .await?;
        Ok(config)
    }

    /// Only enabled integrations for a user.
    pub async fn list_enabled_for_user(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<Self>, AppError> {
        let configs = sqlx::query_as::<_, IntegrationConfig>(
            "SELECT * FROM integration_configs WHERE user_id = ? AND is_enabled = 1",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(configs)
    }

    /// Insert or update on `(user_id, service)` conflict. Preserves existing token/config
    /// when the new values are `NULL` (via `COALESCE`).
    pub async fn upsert(
        pool: &SqlitePool,
        user_id: i64,
        service: &str,
        is_enabled: bool,
        encrypted_token: Option<&[u8]>,
        config_json: Option<&str>,
    ) -> Result<Self, AppError> {
        let config = sqlx::query_as::<_, IntegrationConfig>(
            r#"
            INSERT INTO integration_configs (user_id, service, is_enabled, encrypted_token, config_json)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(user_id, service) DO UPDATE SET
                is_enabled = excluded.is_enabled,
                encrypted_token = COALESCE(excluded.encrypted_token, integration_configs.encrypted_token),
                config_json = COALESCE(excluded.config_json, integration_configs.config_json)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(service)
        .bind(is_enabled)
        .bind(encrypted_token)
        .bind(config_json)
        .fetch_one(pool)
        .await?;

        Ok(config)
    }

    /// Removes the integration config row (and its encrypted token) entirely.
    pub async fn delete(pool: &SqlitePool, user_id: i64, service: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM integration_configs WHERE user_id = ? AND service = ?")
            .bind(user_id)
            .bind(service)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Read-modify-write helper for the `config_json` field.
    /// Parses the current JSON (or starts with `{}`), applies `mutate_fn`, and writes back.
    pub async fn update_config_json<F>(
        pool: &SqlitePool,
        user_id: i64,
        service: &str,
        mutate_fn: F,
    ) -> Result<(), AppError>
    where
        F: FnOnce(&mut serde_json::Value),
    {
        let existing = Self::find_by_service(pool, user_id, service).await?;
        let mut json: serde_json::Value = existing
            .as_ref()
            .and_then(|c| c.config_json.as_deref())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        mutate_fn(&mut json);

        let json_str = serde_json::to_string(&json)
            .map_err(|e| AppError::Internal(format!("JSON serialize: {}", e)))?;

        sqlx::query(
            "UPDATE integration_configs SET config_json = ? WHERE user_id = ? AND service = ?",
        )
        .bind(&json_str)
        .bind(user_id)
        .bind(service)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Records the result of the most recent sync attempt.
    pub async fn update_sync_status(
        pool: &SqlitePool,
        user_id: i64,
        service: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE integration_configs
            SET last_sync_at = datetime('now'), last_sync_status = ?, last_sync_error = ?
            WHERE user_id = ? AND service = ?
            "#,
        )
        .bind(status)
        .bind(error)
        .bind(user_id)
        .bind(service)
        .execute(pool)
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SyncLog
// ---------------------------------------------------------------------------

impl SyncLog {
    /// Inserts a completed sync log record (sets `completed_at` to now).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &SqlitePool,
        user_id: i64,
        service: &str,
        started_at: &str,
        status: &str,
        entries_created: i64,
        entries_updated: i64,
        entries_deleted: i64,
        entries_fetched: i64,
        entries_skipped: i64,
        error_message: Option<&str>,
    ) -> Result<Self, AppError> {
        let log = sqlx::query_as::<_, SyncLog>(
            r#"
            INSERT INTO sync_logs (user_id, service, started_at, completed_at, status, entries_created, entries_updated, entries_deleted, entries_fetched, entries_skipped, error_message)
            VALUES (?, ?, ?, datetime('now'), ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(service)
        .bind(started_at)
        .bind(status)
        .bind(entries_created)
        .bind(entries_updated)
        .bind(entries_deleted)
        .bind(entries_fetched)
        .bind(entries_skipped)
        .bind(error_message)
        .fetch_one(pool)
        .await?;

        Ok(log)
    }

    /// Most recent sync logs for a user, up to `limit`.
    pub async fn recent_for_user(
        pool: &SqlitePool,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<Self>, AppError> {
        let logs = sqlx::query_as::<_, SyncLog>(
            "SELECT * FROM sync_logs WHERE user_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// Updates the deleted-entries counter on an existing sync log.
    pub async fn set_entries_deleted(
        pool: &SqlitePool,
        id: i64,
        entries_deleted: i64,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE sync_logs SET entries_deleted = ? WHERE id = ?")
            .bind(entries_deleted)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Purges all sync log history for a user.
    pub async fn delete_all_for_user(pool: &SqlitePool, user_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM sync_logs WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
