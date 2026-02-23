use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::review::model::Week;
use crate::shared::crypto::UserCrypto;
use crate::shared::error::AppError;
use crate::sync::model::IntegrationConfig;

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
    let phase = crate::review::model::BragPhase::find_by_id(pool, phase_id, user_id)
        .await?
        .ok_or(AppError::NotFound("Phase not found".to_string()))?;
    let phase_start = NaiveDate::parse_from_str(&phase.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::Internal("Invalid phase start date".to_string()))?;
    let phase_end = NaiveDate::parse_from_str(&phase.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::Internal("Invalid phase end date".to_string()))?;
    if date < phase_start || date > phase_end {
        return Err(AppError::BadRequest(format!(
            "Date must be within your review cycle ({} to {})",
            phase.start_date, phase.end_date
        )));
    }

    let week = Week::find_or_create_for_date(pool, phase_id, date).await?;
    let new_week_id = if week.id != entry.week_id {
        Some(week.id)
    } else {
        None
    };

    BragEntry::update(pool, id, user_id, input, new_week_id, crypto).await
}

/// Classifies a meeting entry: sets meeting_role and teams, optionally saves a rule.
pub async fn classify_entry(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    meeting_role: &str,
    teams: Option<&str>,
    save_rule: bool,
    crypto: &UserCrypto,
) -> Result<BragEntry, AppError> {
    let entry = BragEntry::find_by_id(pool, id, user_id, crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    // Encrypt teams if provided
    let enc_teams = teams
        .filter(|s| !s.trim().is_empty())
        .map(|s| crypto.encrypt(s.trim()))
        .transpose()?;

    sqlx::query(
        "UPDATE brag_entries SET meeting_role = ?, teams = COALESCE(?, teams) WHERE id = ?",
    )
    .bind(meeting_role)
    .bind(&enc_teams)
    .bind(id)
    .execute(pool)
    .await?;

    // If save_rule is true and entry has a recurring_group, create a meeting rule
    if save_rule
        && let Some(ref rg) = entry.recurring_group {
            let rule_input = crate::review::model::CreateMeetingRule {
                match_type: "recurring_group".to_string(),
                match_value: rg.clone(),
                meeting_role: meeting_role.to_string(),
                person_name: None,
            };
            let _ = crate::review::model::MeetingRule::create(pool, user_id, &rule_input).await;
            let _ = crate::review::model::MeetingRule::apply_meeting_rules(pool, user_id).await;
        }

    BragEntry::find_by_id(pool, id, user_id, crypto)
        .await?
        .ok_or(AppError::NotFound("Entry not found".to_string()))
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
