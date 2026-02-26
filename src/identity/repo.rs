use std::collections::HashMap;
use sqlx::SqlitePool;

use crate::kernel::error::AppError;
use crate::worklog::model::BragEntry;

use super::model::{PeopleAlias, ProfileUpdate, User};

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

impl PeopleAlias {
    /// Returns all aliases for a user, ordered by display name.
    pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, AppError> {
        let aliases = sqlx::query_as::<_, PeopleAlias>(
            "SELECT * FROM people_aliases WHERE user_id = ? ORDER BY display_name",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(aliases)
    }

    /// Inserts or updates an alias for (user_id, email). On conflict, updates display name and team.
    pub async fn upsert(
        pool: &SqlitePool,
        user_id: i64,
        email: &str,
        display_name: &str,
        team: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO people_aliases (user_id, email, display_name, team)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id, email) DO UPDATE SET
                display_name = excluded.display_name,
                team = excluded.team
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(display_name)
        .bind(team)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Deletes a single alias, scoped to the owning user.
    pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM people_aliases WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Builds a lookup map from lowercased email to display name.
    pub async fn alias_map(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<HashMap<String, String>, AppError> {
        let aliases = Self::list_for_user(pool, user_id).await?;
        let map = aliases
            .into_iter()
            .map(|a| (a.email.to_lowercase(), a.display_name))
            .collect();
        Ok(map)
    }

    /// Builds a lookup map from lowercased email to team name (only aliases with a team set).
    pub async fn team_map(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<HashMap<String, String>, AppError> {
        let aliases = Self::list_for_user(pool, user_id).await?;
        let map = aliases
            .into_iter()
            .filter_map(|a| a.team.map(|t| (a.email.to_lowercase(), t)))
            .collect();
        Ok(map)
    }

    /// Replaces raw collaborator emails/identifiers with aliased display names
    /// and enriches entry teams from the team map. Modifies entries in place so
    /// templates render friendly names and alias-derived team chips.
    pub fn apply_to_entries(
        entries: &mut [BragEntry],
        alias_map: &HashMap<String, String>,
        team_map: &HashMap<String, String>,
    ) {
        if alias_map.is_empty() && team_map.is_empty() {
            return;
        }
        for entry in entries.iter_mut() {
            // Collect collaborator emails before aliasing for team lookup
            let collab_emails: Vec<String> = entry
                .collaborators
                .as_deref()
                .map(|c| {
                    c.split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // Replace collaborator emails with display names
            if !alias_map.is_empty()
                && let Some(ref collabs) = entry.collaborators
            {
                let transformed: Vec<&str> = collabs
                    .split(',')
                    .map(|c| {
                        let trimmed = c.trim();
                        alias_map
                            .get(&trimmed.to_lowercase())
                            .map(|s| s.as_str())
                            .unwrap_or(trimmed)
                    })
                    .collect();
                entry.collaborators = Some(transformed.join(", "));
            }

            // Enrich teams from collaborator emails
            if !team_map.is_empty() {
                let mut existing: Vec<String> = entry
                    .teams
                    .as_deref()
                    .map(|t| {
                        t.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                for email in &collab_emails {
                    if let Some(team) = team_map.get(email)
                        && !existing.iter().any(|t| t == team)
                    {
                        existing.push(team.clone());
                    }
                }

                if !existing.is_empty() {
                    entry.teams = Some(existing.join(", "));
                }
            }
        }
    }
}
