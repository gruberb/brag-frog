use serde::{Deserialize, Serialize};

/// Bundled profile fields for `User::update_profile`.
pub struct ProfileUpdate<'a> {
    pub display_name: Option<&'a str>,
    pub team: Option<&'a str>,
    pub manager_name: Option<&'a str>,
    pub skip_level_name: Option<&'a str>,
    pub direct_reports: Option<&'a str>,
    pub timezone: Option<&'a str>,
    pub week_start: Option<&'a str>,
    pub work_start_time: Option<&'a str>,
    pub work_end_time: Option<&'a str>,
}

/// Authenticated user, keyed by Google `sub` claim. Upserted on every login.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub google_id: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    /// Career level role (e.g., "Senior Software Engineer"). User-configured.
    pub role: Option<String>,
    /// Whether the user is targeting promotion (affects AI prompt context).
    pub wants_promotion: bool,
    pub created_at: String,
    pub last_login_at: String,
    // Profile fields (migration 002)
    pub display_name: Option<String>,
    pub team: Option<String>,
    pub manager_name: Option<String>,
    pub skip_level_name: Option<String>,
    pub direct_reports: Option<String>,
    pub timezone: Option<String>,
    pub week_start: Option<String>,
    // Work hours (migration 003)
    pub work_start_time: Option<String>,
    pub work_end_time: Option<String>,
}

/// Maps an email address to a human-readable display name. Users configure
/// these in Settings to replace raw emails (e.g. from calendar attendees)
/// with recognizable names throughout the UI.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PeopleAlias {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    pub display_name: String,
    pub team: Option<String>,
    pub created_at: String,
}
