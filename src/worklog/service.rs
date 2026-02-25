use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::cycle::model::Week;
use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;
use crate::integrations::model::IntegrationConfig;

use super::model::{BragEntry, UpdateEntry};

/// Updates an entry, reassigning it to the correct week if the date changed.
pub async fn update_with_week_reassignment(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    input: &UpdateEntry,
    crypto: &UserCrypto,
) -> Result<BragEntry, AppError> {
    let date = NaiveDate::parse_from_str(&input.occurred_at, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date format".to_string()))?;
    let entry = BragEntry::find_by_id(pool, id, user_id, crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;
    let phase_id = Week::phase_id(pool, entry.week_id).await?;

    // Validate date falls within the review cycle
    let phase = crate::cycle::model::BragPhase::find_by_id(pool, phase_id, user_id)
        .await?
        .ok_or(AppError::NotFound("Phase not found".to_string()))?;
    phase.validate_date_in_range(date)?;

    let week = Week::find_or_create_for_date(pool, phase_id, date).await?;
    let new_week_id = if week.id != entry.week_id {
        Some(week.id)
    } else {
        None
    };

    BragEntry::update(pool, id, user_id, input, new_week_id, crypto).await
}

/// Result of classifying a meeting entry, returned to the caller to orchestrate
/// any cross-module side effects (e.g., creating meeting rules in the cycle module).
pub struct ClassificationResult {
    pub entry: BragEntry,
    /// If the user opted to save a rule and the entry had a recurring_group,
    /// this contains the recurring group value to persist.
    pub save_rule_for_group: Option<String>,
    pub meeting_role: String,
}

/// Classifies a meeting entry: sets meeting_role and teams. Returns a
/// `ClassificationResult` so the caller can orchestrate meeting rule creation
/// without this module reaching into the cycle (review) module.
pub async fn classify_entry(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    meeting_role: &str,
    teams: Option<&str>,
    save_rule: bool,
    crypto: &UserCrypto,
) -> Result<ClassificationResult, AppError> {
    let entry = BragEntry::find_by_id(pool, id, user_id, crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    // Encrypt teams if provided
    let enc_teams = teams
        .filter(|s| !s.trim().is_empty())
        .map(|s| crypto.encrypt(s.trim()))
        .transpose()?;

    BragEntry::update_classification(pool, id, meeting_role, enc_teams.as_deref()).await?;

    let save_rule_for_group = if save_rule {
        entry.recurring_group.clone()
    } else {
        None
    };

    let updated = BragEntry::find_by_id(pool, id, user_id, crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    Ok(ClassificationResult {
        entry: updated,
        save_rule_for_group,
        meeting_role: meeting_role.to_string(),
    })
}

/// Excludes a Google Drive file: adds to exclusion list and soft-deletes all entries for it.
pub async fn exclude_drive_file(
    pool: &SqlitePool,
    user_id: i64,
    entry: &BragEntry,
) -> Result<(), AppError> {
    if entry.source != "google_drive" {
        return Err(AppError::BadRequest("Not a Google Drive entry".to_string()));
    }

    let file_id = entry
        .source_id
        .as_deref()
        .and_then(|sid| sid.split(':').nth(1))
        .ok_or_else(|| AppError::BadRequest("Missing file ID".to_string()))?
        .to_string();

    let title = entry.title.clone();

    IntegrationConfig::update_config_json(pool, user_id, "google_drive", |json| {
        let arr = json["excluded_files"]
            .as_array_mut()
            .cloned()
            .unwrap_or_default();
        let already = arr.iter().any(|v| v["file_id"].as_str() == Some(&file_id));
        if !already {
            let mut new_arr = arr;
            new_arr.push(serde_json::json!({
                "file_id": file_id,
                "title": title,
            }));
            json["excluded_files"] = serde_json::Value::Array(new_arr);
        }
    })
    .await?;

    BragEntry::soft_delete_by_file_id(pool, user_id, &file_id).await?;
    Ok(())
}

/// Excludes a Google Calendar event series: adds to exclusion list and soft-deletes all entries.
pub async fn exclude_calendar_event(
    pool: &SqlitePool,
    user_id: i64,
    entry: &BragEntry,
) -> Result<(), AppError> {
    if entry.source != "google_calendar" {
        return Err(AppError::BadRequest(
            "Not a Google Calendar entry".to_string(),
        ));
    }

    let raw_event_id = entry
        .source_id
        .as_deref()
        .and_then(|sid| sid.strip_prefix("calendar:"))
        .ok_or_else(|| AppError::BadRequest("Missing event ID".to_string()))?;

    let base_event_id = raw_event_id
        .split('_')
        .next()
        .unwrap_or(raw_event_id)
        .to_string();

    let title = entry.title.clone();

    IntegrationConfig::update_config_json(pool, user_id, "google_calendar", |json| {
        let arr = json["excluded_events"]
            .as_array_mut()
            .cloned()
            .unwrap_or_default();
        let already = arr
            .iter()
            .any(|v| v["event_id"].as_str() == Some(&base_event_id));
        if !already {
            let mut new_arr = arr;
            new_arr.push(serde_json::json!({
                "event_id": base_event_id,
                "title": title,
            }));
            json["excluded_events"] = serde_json::Value::Array(new_arr);
        }
    })
    .await?;

    BragEntry::soft_delete_by_event_id(pool, user_id, &base_event_id).await?;
    Ok(())
}
